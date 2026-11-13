# 1132. A move of a local whose address is taken copies it

Status: Accepted (2026-11-12, #1535, #1500). The coordinator approved
option A on #1500: one change that fixes #1535 and lifts #1500. Beside
[0562](0562-a-pointer-is-a-place-and-an-address.md), pointers, and
[0020](0020-structs-and-tuples.md), copies.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change (an
address, and the value a move left), so Rust's behavior is preserved.

## Context

In a crate that asks an address, a local's address is its JS object's
place. `let kept = m;` was `const kept = m;`: one object, so `kept`
had `m`'s address, where natively it's its own allocation, as each live
local is (#1535, a wrong answer). Since #1484 a place a pointer is taken
to, assigned whole again, is written over in place, `$assign(m, v)`, so
it keeps its address. After a move that write would have changed the value
moved, so it was refused (#1500), and so was a write where the place may
hold a value or not.

## Decision

A move of a place a raw pointer is taken to or into
(`pointers/rewritten.rs`' `moved_apart_places`, an object JS holds as
itself) is a copy, its form `copies.rs`' `moved_apart`:

- What Rust's move copies, the value's bytes: `[m[0], m[1]]` or
  `{ ...o, inner: [o.inner[0], o.inner[1]] }`, each struct or tuple
  inline in it copied in turn. What it owns elsewhere, a `Vec`'s items, is
  shared, as the move hands it over. An array is `a.slice()`, its items
  shared, as nothing writes into one over its place yet (#1553 adds the
  copy when it does).
- A value of std's type, which JS holds in a form of its own, a `Vec` as
  one object with its items, a `HashMap` a `Map`, is refused where it's
  moved (#1551): its copy would be another's items, and no copy would have
  the same address. That gave a wrong answer silently before.

Since the place moved out keeps its object, a write after the move is
over it, `$assign`, as any write again is. Where the place may be unmade
on some way here, `let mut d; if c { d = .. }`, the write tests it:
`if (d === undefined) d = v; else $assign(d, v);` (`Rewritten::Tested`),
whatever rustc's dataflow says of it there: a value with drop glue is
dropped before the write, which leaves it unmade on each way. Moved out or
dropped on some way but made on each, it holds its object, no test. A
parameter is made where it's given, so its one write is a write again,
which was made anew before.

A place is a local's own storage: its fields and items, and the contents
of each `Box` reached so, `(*s.b).f` (`within_items`, one rule for
#1484's writes and this one's moves). So `Box::new(*b)` copies `*b` as
`let k = m;` copies `m`, and `*b = v` is written over in place, where it
made a new object at another address (a #1484 gap the metamorphic net's
`boxed` rewrite found).

Only moves of such places change: crates that ask no address, and locals
no pointer is taken to, give the JS they gave. `a.slice()` is a read
(`Expr::read`), which `mem::forget` drops as it does any read.

## Consequences

- `let kept = m;`, `f(m)` and `return m` of an addressed local give a
  copy at its own address; assigned again after, the local is written
  over in place.
- Proof: corpus case address_of_a_local_moved_out against native wasm32:
  a move, a move then a write again, a parameter given, a nested struct
  with a pointer into it, a loop, an array, a move on one way, a value
  made on one way run both ways, one with drop glue run both ways, a
  parameter moved on one way, and a `Box`'s contents moved out and written
  again, each passing the metamorphic net's rewrites. And
  diagnostics cases for a `Vec` and a `HashMap` moved. Mutations in
  rewritten.ts, copies.ts, assign/rewritten.ts and expr_walk.ts.
- #1553 (an array assigned again over an item's pointer crashes) was found
  here and left; it predates this change.

## Amendment (2026-11-13, #1553): an item reached by an index is each item, and a moved array's items are copied

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. An array an item of which a pointer was taken to
by an index read at run time, `&raw const items[i]`, assigned whole
again, kept the item at `items[_19]`, the index local of the pointer's own
statement, which the write never declares: `ReferenceError: _19 is not
defined`.

- `kept_within` (`pointers/rewritten.rs`) now takes an item reached by an
  `Index`, past the place written, of an array of a known length as each
  of its items, `ConstantIndex` 0 to N: the write writes over each, so a
  pointer at any of them reads the new item where it was, and it never
  reads the index to tell them apart.
- Writes now keep an array's items, so a move of such an array copies
  each item that's an object held inline, `a.map((item) => [item[0],
  item[1]])`, where it shared them, `a.slice()` (`copies.rs moved_apart`):
  a later write over the place the array was moved from changed the moved
  array's items with it.

Corpus case address_of_an_array_item_kept_when_written_whole against
native Rust; address_of_a_local_moved_out's snapshot copies its items.
Mutations `kept-item-at-its-pointers-index` (`pointers/rewritten.ts`) and
`moved-array-items-shared` (`copies.ts`). A write through the `&mut` that
`mem::replace` is given isn't one of #1484's yet, #1616.
