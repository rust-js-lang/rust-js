# 0320. An `Rc` whose counts are read is counted, as Rust counts it

Status: Accepted. Extends [0023](0023-strings-references-shared-state.md)
and [0098](0098-destructors.md); counted by
[0314](0314-std-data-structures.md). Amended: a shared `Rc<RefCell<dyn T>>`
is counted as the `Rc` it's made of (below); and by
[0519](0519-a-cell-of-what-has-a-destructor.md): a `Cell` or a `OnceCell` of
a value with a destructor is allowed, and drops it as Rust does; and by
[0520](0520-a-counted-rc-of-a-dyn-is-its-pair.md): a counted `Rc<dyn T>` is
its pair, `{ value, impl, strong, weak }`; and by
[0523](0523-libraries-share-one-counted-rc.md): a library, and a crate using
one, counts every `Rc`, and a `&mut` to a counted `Rc` is a cell.

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
| `get_mut`, `make_mut` | `rc.strong === 1 && rc.weak === 0 ? rc.value : undefined`; `rc = $makeMut(rc, clone, drop)` |
| `try_unwrap`, `into_inner`, `unwrap_or_clone`, `new_cyclic` | `$tryUnwrap`, `$intoInner`, `$unwrapOrClone`, `$newCyclic` |

(Amended: `make_mut` and `unwrap_or_clone` of a shared `Rc` clone what it
points at while it's still shared, so a `clone` sees every owner, and then
drop the `Rc`, the last of it dropping what it points at with `drop`; a
clone that panics leaves `make_mut`'s as it was, and `unwrap_or_clone`'s
dropped as it unwinds. They had dropped a count first, which a review
found.)

- **Which are counted**: an analysis finds each `T` whose `Rc` or `Arc`
  the crate reads a count of, `ptr_eq`s, or makes a `Weak` of, in a body
  or a field. (Amended: and each whose drop runs a destructor, which the
  last `Rc` of it drops where Rust's does; an `Rc` of one was refused.
  What a type drops depends on which `Rc`s are counted, a value holding a
  counted `Rc` drops it, so it's found to a fixed point, by the drops'
  own question, `drop_runs`. The `rc_destructors` corpus case drops
  through clones, a `Vec` of them, a `RefCell` in one, one a function
  gives back, and one of a value holding another, with native Rust.)
- **An `Rc<dyn T>` of a counted `Rc`** (amended) is `{ value: { value,
  impl }, strong, weak }`, the counted `Rc` of the pair, as every use of
  one reads it: made of one made where it's unsized, `Rc::new(x)`, its
  value made the pair. One made elsewhere, which its other owners share as
  the type it is, an upcast of one, and a `Weak`, are refused. It had been
  a pair around the `Rc`, which each use misread, since an `Rc` of a value
  with a destructor was counted (`rc_dyn_destructors`). If one is a generic function's `T`, or a generic function
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

## Amendment: a shared `Rc<RefCell<dyn T>>`

A listener a bus and its owner both hold, `vec![logger.clone()]` as a
`Vec<Rc<RefCell<dyn Listener>>>`, was refused wherever any `Rc` was
counted, a tree's `Weak` parent beside it: a `dyn` was taken to drop
something wherever any type might, so its `Rc` was counted, and one made
of another `Rc` was refused. Three rules now:

- **A `dyn` of the crate's trait drops what one of its implementors
  does**, each of them known where nothing else can implement it: a `dyn
  Listener` of `Logger`s drops nothing, and its `Rc` is its value, as its
  owner's is. Of another crate's trait, a library's, or one a generic impl
  implements, it may drop anything, as before.
- **An `Rc` made a counted one is counted** (`made_others`), to the fixed
  point: the two share their counts, `strong_count` 2 of either.
- **A counted `Rc<RefCell<C>>` made an `Rc<RefCell<dyn T>>`** is the same
  `Rc`, its cell given the `impl` in place, as ADR 0397 gives an uncounted
  one's, which its other owners share. A `Weak`'s is still refused.

`rc_dyn_listeners`, `rc_dyn_sinks` and `rc_refcell_dyn_shared` run each
against native Rust. An `Rc<dyn T>` of a counted `Rc` made elsewhere,
with no `RefCell`, is still refused: its pair is a value of its own.
