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

## Amendment: a `Cell` it owns through what it owns (#989)

`clone_shares` asked each capture `is_freeze`, which looks only at its own
fields: a `Box<Cell<i32>>` and a `Vec<Cell<i32>>` are `Freeze`, so a
closure that took one was its own clone, `const counter2 = counter`, which
shared the cell a clone of the box copies (`1 2` where native prints `1
1`). It asks `owns_interior_mutability`, which looks through what the
capture owns, not through what it shares, an `Rc`'s, an `Arc`'s, a
`Weak`'s or a reference's, whose clone shares the cell as the closure
itself would. Corpus case `closure_clone_owned_cell`.

## Amendment: a library's `impl Trait + Clone + '_`, its lifetimes erased (#1201)

A library's public function's `impl Trait + Clone` is revealed so that a
consumer may clone what it hides (`given_out_clone`). The function's
output was read with its lifetimes left bound, and what an
`impl Iterator + Clone + '_` hides holds the function's own lifetime,
`Chars<'_>` or `Split<'_, char>`. Asking whether that type is `Clone`
crashed rustc's trait selection on its escaping bound variable. The
output is read with its lifetimes erased, as every other clone question
is. This is a crash, not a choice between Rust's and JS's behavior, so
ADR 0262's table doesn't apply. Held by the crate pair
`library_opaque_clone_of_a_borrow` and the mutation
`given-out-clone-bound-regions`.

## Amendment: a library's closure is cloned as its library made it (#1202)

A consumer's clone of a library's `impl Iterator + Clone`,
`v.into_iter().map(|s| s.len())`, threw `f[$cloneKey] is not a function`.
The two crates decided the one closure apart. The library sees what its
closure captures, nothing, and made it a plain function, its own clone.
The consumer can't see another crate's captures, so `clone_shares` took
a closure of another crate's as writing unless rustc infers it `Fn`;
rustc infers this one `FnMut`, the bound `map` gives it. It cloned it
apart, `$clone(f)`, of a function made with no clone.

The crate that makes a closure decides how it's cloned, and its consumers
ask it: a library's contract lists each closure it clones, or gives a
consumer to clone, with whether it's cloned apart, `cloned_closures`
(library ABI 26), and a consumer's `ClosureFacts` takes that answer for a
closure its library decided. One its library didn't decide, a closure
the consumer makes itself from std's or a library's MIR (ADR 0914), is
decided as before. `clone_value` takes the crate's decision of each
closure it clones, `ClosureFacts::clone_is_itself`, and no longer asks
`clone_shares` again, which of a library's closure answers otherwise; it
asks it only of one the analysis didn't see cloned, and refuses where a
clone could tell.

`given_out_clone` now reveals what a library's `impl .. + Clone` hides
through each `impl Trait` it holds in turn. A public function that
returns a private one's `impl Iterator + Clone` left that one's closure
undecided, a plain function its consumer then cloned apart.

A library's contract is read in one place, `Exports::read`, for a
manifest and for `@rust-js/std` alike, with each of its fields named, so
a fact one reading takes the other takes too.

ADR 0262's case: results change (a throw), so Rust's: a clone of the
closure behaves as native Rust's. Held by the crate pair
`library_closures_cloned_as_made`, of a closure its library made plain
and three it cloned apart, one hidden behind a private function's
`impl Iterator + Clone`, against native Rust, and by its mutations.

## Amendment: a copy of a closure cloned apart is its clone, in whatever holds it (#1211)

A clone of what holds a closure that changes what it captured, `Some(f)`,
`(f, 1)`, a `Vec`, a `Box`, a `Result` or a struct that derives `Clone`,
was right only where the holder wasn't `Copy`. A `Copy` closure, `move ||
{ n += 1; n }` of an `i32`, makes its `Option` and its tuple `Copy`, and
std's `Option::clone` and a derived `Clone` of a `Copy` struct are a copy:
rust-js copies only what's changed in place, so `held.clone()` was
`held`, the one JS function, and the clone's calls counted on from the
original's count, `1 2` where native Rust prints `1 1`. A copy of the
holder, `let copied = held;`, shared it the same way, where a copy of the
closure itself was refused (ADR 0246).

The crate already clones such a closure apart wherever it's cloned, held
or not, as the types it clones are walked for the closures they hold, and
a `Copy` bound is a `Clone` one. So a copy of one is its clone, as Rust's
copy is a value of its own captures: `copies.rs` takes a `Copy` closure
that writes what it took (`writes_what_it_took`, `clone_shares`' rule) and
that the crate clones apart as a type changed in place, whose copy is
`$clone(f)`, so each holder's copy copies it: `[$clone(pair[0]), pair[1]]`,
`held == null ? held : $clone(held)`. A library's, as its library decided
(#1202), and so in both builds. A copy of one the crate never clones,
held by value in a tuple, an array, a struct or an enum's field or
another closure's capture, is refused as the closure's own copy is.

ADR 0262's case: results change, so Rust's. Held by
`closure_cloned_in_a_holder.rs` (each holder, cloned and copied),
`copied_closure_bound.rs`, now run, and `copied_closure_in_a_holder.rs`
and `copied_closure_in_a_struct.rs` (refused), against native Rust, the
crate pair `library_closures_cloned_in_a_holder`, and their mutations.

## Amendment: `given-out-clone-bound-regions` retired (#1218)

Since #1202 (87f78a3b9) what a library's `impl .. + Clone` hides is read
part by part, each instantiated from its opaque type and its regions erased
before its `Clone` is asked, so the output read with its lifetimes left
bound no longer reaches a question that holds them:
`library_opaque_clone_of_a_borrow` and every other crate pair pass with
it. The mutation survived main's check and is removed; the erasure stays,
as what the function's signature says.
