# 1102. What's made in turn drops as a later part unwinds

Status: Accepted (#897, #898, #902, #903; the form approved by the
coordinator for the user, 2026-11-08). Amends
[0052](0052-std-trait-impls.md) (a derived or std `Clone` and `Default`
rust-js writes), [0507](0507-a-vectors-edits-of-destructors.md)
(`resize`, `fill`) and [0509](0509-more-std-calls-of-destructors.md)
(`unwrap_or_default`'s `Err`).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behaviour is preserved: a drop that runs natively and not in JS is
a line, a count or a resource the program sees.

## Context

#854's unwind table runs each operation beyond iterators once each point
a panic can be, every make, clone, drop and comparison of a `D`, against
native Rust in both builds. Its first run found 163 cases, 326
case-builds, of one mechanism in four issues: **a value made of parts,
whose making of a later part panics, leaks the parts it made already.**

- #897: a derived `Clone`'s fields and a tuple's; a clone of a `Vec`, a
  `VecDeque`, a `BinaryHeap`, an array, a `BTreeMap` or a `HashMap`, its
  items; `to_vec` and `extend_from_slice`'s items' own clones.
- #902: a derived `Default`'s fields and a tuple's: `mem::take`,
  `unwrap_or_default`.
- #898: the value `resize`, `fill` and `vec![x; n]` clone from.
- #903: what `inspect` and `inspect_err` give back, as their closure
  panics; an `Err` `unwrap_or_default` drops, as `T::default()` panics.

In Rust each is the same rule, which drop elaboration gives every body:
what a frame has made, a temporary of an aggregate's or a local, is
dropped as a later call in it unwinds. The MIR rust-js lowers has those
edges, so a struct literal the program writes is right (the table's
`value struct` rows pass). None of these four is lowered from MIR:
`clone_value` and `default_value` (`std_impls.rs`, `std_impls/defaults.rs`)
write a derived or std `Clone` and `Default` as an expression,
`{ ...p, a: clone(p.a), b: clone(p.b) }` and `items.map(clone)`; the
runtime's `$resize`, `$fill` and `$repeat` loop by hand; `inspect` is an
`if` around the call. Each wrote the parts in order and no unwind edge.

## Decision

**One rule, asked where rust-js writes a value of parts itself: a part
with a destructor made before a later part whose making may panic is
dropped as that one unwinds.** Its owner is `std_impls`, its rule `std_impls/in_turn.rs`:

- `clone_may_panic` (moved there from `mir/iter/collect.rs`, its one
  answer, which a `cloned()` iterator's steps ask too) and
  `default_may_panic` say whether making a part may panic. A `Copy`
  value's clone never does, nor an `Rc`'s or a `Weak`'s, which counts; a
  hand-written impl's may, unless each of its methods never unwinds (ADR
  0902); a derived one's as its fields' do (before, `clone_may_panic`
  answered "never" of every derived `Clone`, so a `cloned()` of a derived
  type took its clone to be quiet); std's types' as what they hold's do,
  except an empty collection's `Default`, which makes none.
- `held_in_turn` says which parts are held: each with a destructor made
  before the last whose making may panic. Where none is, the value is
  written as it always was.

Two forms, one per order Rust drops in:

- **Parts of a fixed number**, a derived `Clone`'s or `Default`'s fields,
  a tuple's, a map entry's key and value, an array's `Default`'s items,
  dropped the latest first, as Rust drops an aggregate's operands:
  `$madeInTurn((made) => ({ ...p, a: made(clone(p.a), drop), b:
  clone(p.b) }))`. `made(part, drop)` gives the part back as it holds
  it, and `$madeInTurn` drops what it holds as `build` throws. A field
  that needs no clone of its own, a `{ ...p }` copy, is held too where it
  drops, as the derived `Clone` made one.
- **Items**, a clone of a slice's, a `VecDeque`'s, a `BinaryHeap`'s, an
  array's or a map's entries, dropped in order, as std's clone drops what
  its guard holds: `$madeEach(items, clone, drop)`, `Array.from`'s of
  `clone` but for that.

Both are `src/runtime/made.js`'s, and each asks `$unwinds`, the
runtime's one guard of what drops as a panic unwinds, whose
`$dropsUnwinding` is the one place a drop's second panic aborts
(`test/architecture-iterators.test.ts` holds it so). The runtime's own
loops ask it too: `$resize` holds
the value it was given across a drop of what it cuts off, the room check
and each clone; `$fill` across each clone and each drop of what it
replaces; `$repeat` across each clone, then drops the clones made, as
`extend_with`'s frame unwinds before `from_elem`'s drops its `Vec`.
`inspect` and `inspect_err` drop the value they give back as their closure
unwinds, where it may (`call_may_unwind`, ADR 0902's answer, which a
fold's stages ask too): `try { f(x); } catch (error) { drop(x); throw
error; }`, `drops::dropped_unwinding`'s. `or_dropped`, of
`unwrap_or_default`'s `Err` and its kind, holds what it gives up across
what it makes otherwise where that may throw.

## Alternatives

- **A named clone function per type**, `function cloneP(p) { const a =
  clone(p.a); const b = $unwinds(() => clone(p.b), () => drop(a));
  return { ...p, a, b }; }`, as a recursive type's is (`recursive_fns`).
  It reads more like JS a person writes. But `clone_value` and
  `default_value` make expressions, under an `Option`'s `p == null ? p :`
  and an enum's variant tests, where no statement can go before the
  part; each such type would need its function, an enum's of `if`s, and
  `default_value` its statements, which it has none of. The helpers make
  the guard an expression wherever the clone already is. Not chosen.
- **Lower a derived `Clone` and `Default` from their MIR**, whose cleanup
  edges are rustc's. That gives up rust-js's spread clone, which shares
  what nothing changes (ADR 0020), for `const value = ..; let value$live`
  of every field, and std's `Vec` clone has no MIR rust-js can lower. Not
  chosen.
- **Guard every clone of parts with a destructor.** Simpler to decide;
  but most clones can't panic, and `Vec<String>`'s would read worse for
  nothing. Not chosen: the guard is only where `held_in_turn` says.

## Consequences

- #897's, #898's, #902's and #903's rows of the unwind table pass, 306
  case-builds, but `Vec clone_from`'s and a `fill` of a `Vec` of `Vec`s:
  a `Vec`'s `clone_from` runs std's `SpecCloneIntoVec`, another order of
  clones and drops with no panic at all, filed as #919.
- One corpus snapshot changed, as Rust's behaviour needs:
  `refcell_generic_borrow_meets_held`'s clone of a `Vec<Node>`, whose
  `Clone` is hand-written and whose drop drops a `Weak`, is `$madeEach`.
- `test/lowering-made-in-turn.test.ts` holds both forms: none of `Copy`,
  `String` and `Vec<String>` fields, and of a derived `Clone` of them;
  each guarded form of a panicking `Clone` and `Default`.
- Corpus: `clone_unwinds_what_it_made`, `default_unwinds_what_it_made`,
  `resize_fill_repeat_unwind`, `inspect_unwinds_its_value`, each failing
  before in both builds.
