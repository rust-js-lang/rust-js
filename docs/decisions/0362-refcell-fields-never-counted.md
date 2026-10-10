# 0362. A field's `RefCell` never counted is the value it holds

Status: Accepted. Extends [0288](0288-cell-fields.md) to the `RefCell`s of
[0328](0328-refcell-borrow-counts.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

JS shares an object and changes it in place: react.dev's MDXComponents
builds its nested table of contents by pushing each node onto its
parent's `children`, the parent found in a `Map` of ancestors:

```js
const node = {item, children: []};
currentParent.children.push(node);
```

Rust shares a node by `Rc`, so what changes it is a cell:
`children: RefCell<Vec<Rc<Node>>>`. A `RefCell` was `{ value }`, each
borrow checked against the guards it counts (ADR 0328):

```js
const node = { item, children: { value: [] } };
$borrowMut(currentParent.children).value.push(node);
```

A `Cell` in a field is already the value it holds (ADR 0288). A
`RefCell`'s `{ value }` holds its count, which a held guard sets.

## Decision

**A `RefCell` type each field of which, in the crate, is only ever
borrowed for a moment, where nothing can ask (ADR 0328's momentary
borrow), is the value it holds in a field, and its borrow of the field is
the field itself:**

```rust
let node = Rc::new(Node { item: Some(item), children: RefCell::new(vec![]) });
parent.children.borrow_mut().push(node.clone());
```

```js
const node = { item, children: [] };
parent.children.push(node);
```

- **Its count can't be anything but 0**: a momentary borrow is checked,
  not counted, so no guard of such a field is ever counted, and every
  check of one passes. Leaving them out changes no run.
- **Each field of the type, crate-wide**, borrowed only so: one lent
  whole, `&x.f`, whose callee may count, bound by a pattern, moved,
  replaced, cloned or compared, or of a derive (which reads fields
  itself), keeps the type a cell. So does one with a destructor, whose
  drop reads `value`; a generic one, which may be any; and a library's,
  whose consumers may do any of these.
- **A field only**: a local of the same type is a cell still, its borrows
  checked and counted.
- **As 0288's `Cell`**: a field read is a handle on the property, so
  `*self.hits.borrow_mut() += 1` is `counter.hits = counter.hits + 1`;
  what's made into a field is what the cell holds; it's declared as its
  value, `children: Node[]`.

## Why

- **It's the JS a person writes**, the original's object graph, with the
  same runs: no borrow it leaves out could have failed.
- **It's 0288's rule for `Cell`**, with ADR 0328's own analysis of which
  borrows are checked and not counted, asked of every body once.

## Alternatives

- **Keep `{ value }`**: the port's JS unlike the original's, for a check
  that can't fail.
- **An arena of nodes by index**: Rust without a cell, and JS further
  from the original.

## Consequences

- The corpus's `refcell_borrows` counter field is its number now,
  `counter.hits = (counter.hits + 1) >>> 0`, still as native Rust runs it.
- Mutations make every such field a cell, take a counted borrow, a lent
  field, a bound one and a derive's for one only borrowed for a moment,
  skip a local's check, and declare one `{ value }`.
