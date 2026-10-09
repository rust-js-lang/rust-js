# 0320. An `Rc` whose counts are read is counted, as Rust counts it

Status: Accepted. Extends [0023](0023-strings-references-shared-state.md)
and [0098](0098-destructors.md); counted by
[0314](0314-std-data-structures.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)), A for an `Rc`
whose counts aren't read, D for one another crate may share.

## Context

An `Rc<T>` is the value it points at (ADR 0023): clones share it, and JS's
collector frees it. That's all most code asks of one. But
`Rc::strong_count`, `get_mut`, `make_mut`, `try_unwrap`, `into_inner`,
`unwrap_or_clone` and every `Weak` answer from counts JS doesn't keep, and
so did `ptr_eq` of a number, which JS compares by value. They were
refused: ADR 0314's count had `Rc` at 1 of its 23 methods, `Weak` at none,
and the Rust book's tree, a parent's `Weak` and its children's `Rc`s,
couldn't compile.

## Decision

**An `Rc<T>` of a `T` whose counts the crate reads is `{ value, strong,
weak }`, counted where it's made, cloned and dropped, as Rust counts it;
every other `Rc` is still its value.**

```rust
let a = Rc::new(5);
let b = Rc::clone(&a);
println!("{}", Rc::strong_count(&a));
```

```js
const a = { value: 5, strong: 1, weak: 0 };
try {
  const b = $rcClone(a);
  try {
    console.log(`${a.strong}`);
  } finally {
    $rcDrop(b);
  }
} finally {
  $rcDrop(a);
}
```

| Rust | JS |
|---|---|
| `Rc::new(x)`, `*rc`, `rc.clone()` | `{ value: x, strong: 1, weak: 0 }`, `rc.value`, `$rcClone(rc)` |
| a drop | `$rcDrop(rc)`, `$rcDrop(rc, dropNode)` where what it points at has a destructor: the last drops it |
| `strong_count`, `weak_count`, `ptr_eq` | `rc.strong`, `rc.weak`, `a === b` |
| `Rc::downgrade(&rc)`, `weak.upgrade()`, `Weak::new()` | `$downgrade(rc)`, `$upgrade(weak)`, `{ strong: 0, weak: 0 }` |
| `get_mut`, `make_mut` | `rc.strong === 1 && rc.weak === 0 ? rc.value : undefined`; `rc = $makeMut(rc, clone)` |
| `try_unwrap`, `into_inner`, `unwrap_or_clone`, `new_cyclic` | `$tryUnwrap`, `$intoInner`, `$unwrapOrClone`, `$newCyclic` |

- **Which are counted**: an analysis finds each `T` whose `Rc` or `Arc`
  the crate reads a count of, `ptr_eq`s, or makes a `Weak` of, in a body
  or a field. If one is a generic function's `T`, or a generic function
  holds an `Rc` while any is counted, every `Rc` is: the two must agree on
  what an `Rc` is.
- **Drops are ADR 0098's**: a counted `Rc` and a `Weak` are values with a
  destructor, so a scope holding one is a `try`, and generic code is given
  their drops. A type inside itself through an `Rc`, a tree's node, gets
  its drop function. `$rcDrop(item, (value) => { dropNode(value); })` is
  `$rcDrop(item, dropNode)`.
- **A `RefCell` drops what it holds**, now that one may hold a counted
  `Rc` or a `Weak`: its `value`, and a write through `borrow_mut()` drops
  the old one, as any place's does. A `Cell` of one stays refused: its
  `set` drops the old value where no place is written.
- **A `&mut` to what it points at**, `get_mut`'s or `make_mut`'s, is its
  `value` where that's an object, and the `Rc` itself for a number or text,
  the `{ value }` box a `&mut` is (ADR 0074). `make_mut` gives its place
  the `Rc` it makes, so the place must be one: `make_mut` of another `&mut`
  is refused.
- **What reads it reads its `value`**: `{}`, `{:?}`, `==`, `<`, a derive's
  `==` field by field, not `$eq` of the counts, JSON writing, a loop over
  it, and a plain cell (ADR 0287), which a counted `Rc<Cell>` isn't.
- **Refused, loud**: a counted `Rc` in a library, or of a type another
  crate may hold, where that crate's would be the value; reading one from
  JSON; `get_mut` of a `T` that looks like `None`. A map or set of `Rc`
  keys was refused already, and one that owns counted `Rc`s still is:
  maps don't drop their entries yet.

## Why

- **It's the same program**: each count is Rust's at each point, so what
  reads one, and what the last `Rc` drops, is Rust's.
- **It's the JS a person writes where it can be**: an `Rc` nobody counts is
  still its value, with no `try`.
- **It's tested**: two corpus cases run against native Rust, counts through
  clones, scopes, `drop`, `downgrade` and `upgrade`, `get_mut`, `make_mut`
  shared and not, `try_unwrap` and `into_inner`, the book's tree, a `Drop`
  type dropped by the last `Rc`, a derive's `Clone`, `==` and order, generic
  code, `Rc::new` as a function, loops, `Arc::new_cyclic`, `default()` and
  a counted `Rc<Cell>`. Mutations leave a clone or a drop uncounted, let a
  dangling `Weak` upgrade, share `make_mut`'s clone, unwrap a shared one,
  keep `new_cyclic`'s `Weak`, give `get_mut` while shared, leave
  `make_mut`'s place, count generic code apart, take a counted cell for a
  plain one, drop a `Weak` uncounted, refuse a `RefCell` of one, lose a
  recursive drop, compare counts, and show the box.
- `docs/std-coverage.txt`: `Rc` and `Arc` 11 of 23, `Weak` 5 of 8; the
  rest are raw pointers, uninitialized memory, `pin` and `downcast`.
