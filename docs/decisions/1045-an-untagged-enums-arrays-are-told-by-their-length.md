# 1045. An untagged enum's arrays are told by their length, and changed in place

Status: Accepted (rd490). Extends [0214](0214-untagged-enums.md): two
arrays whose types fix their lengths in one untagged enum. Amends 0214's
"a `&mut` to one is a place" (#347) and
[0863](0863-a-refcell-of-an-object-is-the-object.md): of an enum each of
whose values is an array it holds itself, a `&mut` is the array, and a
`RefCell` of one the array it holds.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the output's shape,
for the tests: each value of a variant has its length, which the others'
haven't, so the test tells each as Rust's discriminant does. C, Rust's
answer, for a copy and a clone of one changed in place, which shared the
array before.

## Context

React's Fizz server keeps what a prerender postponed as replay nodes,
each a plain array: `[name, key, children, slots]`, or a boundary's
`[name, key, children, slots, fallback, rootSegmentID]`. It tells them
apart by `node.length === 4`. `trackPostponedBoundary` grows a node of
four, which its parent's children and the working map both hold, to a
boundary's in place, `suspenseBoundary[4] = fallbackReplayNode;
suspenseBoundary[5] = boundary.rootSegmentID;`, and `untrackBoundary`
shrinks it back, `boundaryNode.length = 4; boundaryNode[2] = [];
boundaryNode[3] = null;`. The nodes are the postponed state, which the
host serializes as JSON and gives back to `resume`.

The port's Rust is an untagged enum of two tuple structs, each an array
(ADR 0020), which rust-js refused: "`Boundary` and `Element` both hold an
array" (ADR 0214). Its draft wrote one tuple struct whose last two fields
were absent until set, told by `rootSegmentID`'s presence, which no form
of Rust shrinks: an untracked boundary kept six items, `null` and
`undefined`, which `JSON.stringify` writes as a boundary's, a wrong answer
on resume (gap rd490, P0).

## Decision

**An array whose type fixes its length, a tuple's, a tuple struct's or a
`[T; N]`'s, is an array of that many items** (`untagged::Kind::Tuple`),
told from another of another length by its `length`:

```rust
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ReplayNode {
    Element(ReplayElement),   // struct ReplayElement(name, key, children, slots)
    Boundary(ReplayBoundary), // .., fallback, rootSegmentID
}
```

| Of the enum's arrays | The test of one of four items |
|---|---|
| it alone | `Array.isArray(node)`, as before |
| two or more, beside what isn't an array | `Array.isArray(node) && node.length === 4` |
| every variant one | `node.length === 4`, as upstream |

- A `Vec`'s, or a slice's, is an array of any length, so beside a tuple
  it's two of one kind, refused where they're told apart, as two strings
  are.
- A `#[repr(transparent)]` struct is its field, as everywhere (ADR 0584):
  `struct Id(f64)` is a number, where it was taken for an array of one.
- An object's test leaves out every array, of any length,
  `!Array.isArray(entry)`.

**An untagged enum each of whose values is an array it holds itself, not
through a reference nor an `Rc`, is changed in place as an array is**
(`untagged::arrays_in_place`): a `&mut` to one is the array, and a
`RefCell` of one the array it holds (`drops::holds_itself`), so a node
its holders share is `Rc<RefCell<ReplayNode>>`, the array itself, and a
write of another variant changes it where each holder sees it:

```rust
let mut node = boundary_node.borrow_mut();
if let ReplayNode::Boundary(ReplayBoundary(name, key, ..)) = &*node {
    // Downgrade to plain ReplayNode since we won't replay through it.
    *node = ReplayNode::Element(ReplayElement(name.clone(), key.clone(), vec![], ResumeSlots::None));
}
```

```js
if (node.length === 6) {
  // Downgrade to plain ReplayNode since we won't replay through it.
  node.length = 4;
  node[2] = [];
  node[3] = null;
}
```

- **A write that keeps some items, each its own read where it is,
  writes only the others** (`mir/assign.rs::kept_items_written`), its
  `length` first, but where no value of the type has more items and the
  last is written, which makes it that: the grow is `node[4] = fallback;
  node[5] = id;`. What reads more than variables is held first, as
  `$assign` reads each item before it writes any. A write that keeps none
  is `$assign(node, [..])`, as any array's.
- **A copy or a clone of one changed in place is its own array**: of
  arrays of what needs no copy of its own, `node.slice()`, whatever its
  variant (`copies.rs::sliced_untagged`); else each variant's payload's
  copy, told by its test. A clone of one of `(f64, f64)` was the array
  itself, which `*r = v` then changed.
- One of a string, a number or an object is a place still, as #347 made
  it: no `*r = v` makes a number another's, or an array an object.

## Why

- **The JS is upstream's**: the node is the array React writes, its JSON
  the postponed state upstream's resume reads, and its test, grow and
  shrink are upstream's lines.
- **Idiomatic Rust**: a value its holders share and change is an `Rc` of a
  `RefCell`; a variant's change is `*node = ..`, and a variant's field a
  field.
- **`length` is what tells them**: a tuple's array has as many items as
  its type has fields, wherever it's made, so the test holds of its
  values only, whatever order they're tried in.

## Alternatives

- **A fresh value through a `{ value }` box**, as a `RefCell` of an
  untagged enum was: right in Rust's terms, but the node is `{ value:
  [..] }`, not the array the postponed state's JSON carries.
- **Making a fresh node and putting it in each holder's place**: upstream
  changes it in place because two holders hold it; the Rust would have to
  find each, which upstream doesn't.
- **One tuple struct of optional fields**, the draft's: no write of Rust's
  shrinks an array.
- **Writing `length` always**, or after the items: a grow to the longest
  variant's has no need of it, as upstream writes none.
- **`Array.isArray(v) && v.length === n` wherever**: noise where the
  enum's one array is told by `Array.isArray` alone, as before, or where
  every value is an array.

## Consequences

- The Fizz port's replay node is the untagged enum of two tuple structs,
  held `Rc<RefCell<ReplayNode>>`, its children
  `Rc<RefCell<Vec<Rc<RefCell<ReplayNode>>>>>`; upstream's `boundaryNode[3]
  = segment.id` a write of the boundary's field, `if let
  ReplayNode::Boundary(b) = &mut *node.borrow_mut() { b.3 = .. }`.
- Held by the corpus's `untagged_told_by_length`, against native Rust:
  the replay node told by its length, matched, shown, cloned and
  compared; tuples through an `Rc`; arrays beside a string and an object;
  a transparent struct's number; a node two holders share grown, given
  another fallback, shrunk and renumbered through a `&mut`; a write
  through a `&mut`, `mem::replace` and `RefCell::replace`; and a copy and
  a clone apart from what changes after. Mutations `untagged-tuple-any-array`,
  `untagged-tuple-struct-any-array`, `untagged-length-unread`,
  `untagged-lengths-always-tested`, `untagged-lengths-never-alone` and
  `untagged-transparent-an-array` (`untagged.ts`);
  `untagged-arrays-a-place` (`representation.ts`);
  `untagged-arrays-boxed-in-a-cell` (`ref_cells.ts`);
  `untagged-arrays-copy-shared` and `untagged-arrays-slice-unread`
  (`copies.ts`); `untagged-arrays-clone-shared` (`std_impls.ts`); and
  `kept-items-length-never-written`, `kept-items-length-always-written`,
  `kept-items-unheld` and `kept-items-unwritten` (`assign.ts`). Seven
  mutations' code moved: `untagged-copy-shared`,
  `untagged-written-in-place`, `untagged-clone-shared`,
  `untagged-object-takes-arrays`, `untagged-subclass-left-out-twice`,
  `untagged-slice-kindless` and `untagged-member-left-in`.
