# 0522. A closure cloned apart is made again

Status: Accepted. Extends [0022](0022-closures.md),
[0246](0246-function-parameters-need-no-copy.md) and
[0484](0484-a-returned-closure-carries-its-drop.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a clone holds its own
copy of what the closure took, as Rust's does.

## Context

A closure is its JS function, which shares the variables it took. A clone
of one was the function itself where nothing could tell, a `Fn` holding no
`Cell`, and refused otherwise ("cloning a closure that changes what it
captured"). A clone of a `Fn` holding a counted `Rc` could tell: it counts
one more, and drops on its own. It was the function itself, so counts and
drops came out wrong. That was `closure_clone_shared` under the
[counted-`Rc`s-across-crates draft](0523-libraries-share-one-counted-rc.md)'s
trial, the last of its four gaps.

## Decision

**A closure a clone of which isn't itself is made by a function of what it
took, `make`, and carries its clone, `$cloning(f, () => make(clone(x)))`;
`$clone(f)` makes it again.**

```js
const make = (count) =>
  $cloning(
    () => { count.value = (count.value + 1) | 0; return count.value; },
    () => make({ value: count.value }),
  );
const counter = make(count);
const counter2 = $clone(counter);
```

- **Cloned apart: a closure the crate clones that `clone_shares` says a
  clone can't be the closure itself.** `clone_shares` is the one answer:
  a `Fn` holding nothing that's a `Cell`, or whose clone counts or drops.
  `ClosureFacts` finds the closures cloned: `Self` of a `Clone::clone`
  call, or what a `Clone` bound of the function called is given, a trait's
  that extends it too.
- **One cloned apart carries its drop** (ADR 0484), as each clone drops
  what it holds itself; a closure that carries its drop is dropped by
  `$dispose(f)` wherever it is, its maker's body too.
- **A type parameter's function may be cloned apart**: its copy is its
  `Clone` dictionary's, not the function (ADR 0246), where the crate clones
  a closure apart or is a library, whose callers' may be.
- **A struct holding one clones it**: its derived `Clone` is a field's.
- **`drop(f)` and `forget(f)` take what they're given**: neither drops a
  function again after the call, as a function given to std otherwise is.

What a closure is cloned by unseen, where `clone_value` meets one the
analysis didn't, is refused: "cloning a closure where it isn't seen to be
cloned".

## Why

- **It's the same program**: `closure_clone`, `closure_clone_cell` and
  `closure_clone_counted` clone closures that change what they took, hold
  a `Cell`, and hold a counted `Rc` through a `T: Clone` and a derived
  `Clone`, and drop each, against native Rust; rustc's
  `functions-closures/clone-closure.rs` passes.
- **Nothing changes where a clone can't tell**: `closure_clone_shared`
  keeps its output.

## Consequences

- `docs/guide/limitations.md` loses "A clone of a closure that changes what it
  captured".
- `FnDyns` is `ClosureFacts`, `analysis/closures.rs`: what the crate does
  with its closures.
- Mutations stop seeing clones, share what can tell, keep what the clone
  takes, drop by the maker's variables, and drop `drop(f)`'s twice.

## Amendment: what's returned as `impl Trait + Clone` is cloned as what it is

`clone_value` read the type as it's written, and a value returned as
`impl Trait + Clone` is an opaque type, which its walks know nothing of:
a clone of it was the value itself, `const b = a`, a struct's and a
closure's alike, so changing one changed both (#100). The type is
revealed first, and the clone is the struct's or the closure's,
`$clone(tick)` of a closure cloned apart. ADR 0262's case: results would
change, so Rust's. Held by `test/corpus/impl_trait_clone.rs` and the
mutation `clone-value-opaque-unrevealed`.

## Amendment: a closure that moves what it took out

A clone of a closure was the closure itself where it never wrote what it
took, but one that moves it out, an `FnOnce`'s `move || { let mut v = v;
v[0] *= 5; v }` or `move || consume(v)`, hands the one value to what may
change it, so the clone gave the value changed (#149). A closure the
compiler infers is `FnOnce` is cloned apart too. Held by
`test/corpus/closure_once_cloned.rs`.
