# 1086. A `&mut`'s JS form is `MutRefForm`'s

Status: Accepted (2026-11-04, steps 1 and 2 of 3; step 3 begun, its
first operation, a move out, `moved_out`). Amends
[0099](0099-mut-references.md) and [0454](0454-a-mut-given-as-a-type-parameter.md):
what a `&mut` to a type is, a box, a generic code's box, the object, a
guard's object, a value given as it is or the value itself, is one answer,
`mut_ref_form`'s, which each predicate those decisions name is now made of.
Amends [1077](1077-a-function-js-cant-call-as-declared-isnt-declared.md): a declaration
asks the same owner, `SignatureForms`, for a `&mut`'s TypeScript type,
`MutRefForm::ts_type`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes who
answers, not what's answered: every corpus snapshot is byte-identical.

## Context

A `&mut T` lowers to one of several JS forms: the object itself, written
in place (`$assign`); a `{ value }` box, of a number, a string, an enum
with a fieldless variant or a flattened state; the same box of a type
parameter, which generic code is compiled once for; a get/set handle,
where the borrow outlives a call; an in-place cell `$cellOf(v)`, whose
`value` *is* the object (ADR 0454); and a flattened state's handle on its
keys (ADR 1044). Every operation on a `&mut` must know which form it
holds, and a study of 83 wrong answers found 12 (#466 #469 #472 #478 #482
#487 #527 #528 #536 #547 #548 #614) where two operations had decided it
apart. #614 is the plainest: `Cell::from_mut` asked `is_boxable`, a
parameter `is_cell_pointee`, and they disagree on a type parameter.

Before this decision, four predicates in `representation.rs` answered
"what is a `&mut` to this type", each public to lowering, and 80 call
sites in 30 files chose among them:

| predicate | answer | callers |
|---|---|---|
| `is_boxable(ty)` | a concrete value JS can't change in place | 21: std types' `get_mut`, `as_mut`, `Pin`, `Rc`, `OnceCell`, `LazyCell`, `MaybeUninit`, `Any`, `Result` combinators, drop glue |
| `is_generic_boxed(ty, env)` | a type parameter, not bound by `Fn` or `Iterator` | 10: generic calls' boxes (`boxed_in_call`, `unboxed_by_function`, `nested_mut_object`) and a call's argument |
| `is_cell_pointee(ty)` | either of those | 43: whole writes, reborrows, captures, `Cell::from_mut`/`get_mut`, raw pointers, `.d.ts` |
| `written_in_place(ty)` | not a cell, not a guard, an object or a value given as it is | 6: `*r = v`, `mem::replace`/`take`/`swap`, `moved_out`, wrapping ops |
| `handles_mut(ty)` | `is_boxable` or any type only a caller knows | 17: slice and map std calls that give a closure a `&mut` |

And the operations that consume the choice, with what each asked:

| operation | where | decided by |
|---|---|---|
| whole write `*r = v` | `mir/places.rs` `place_expr`, `mir/assign.rs` `mir_store` | `is_cell_pointee`, `written_in_place` and `LocalFacts::place_reference` |
| field write `r.f = v` | `mir/places.rs`, `representation.rs` `project` | `is_cell_pointee` on the deref; a flattened field's `Storage::InPlace` handle |
| move out (`replace`/`take`/`swap`, `Cell::replace`) | `mut_refs.rs` `moved_out`, `mir/entries.rs`, `std_types/cell.rs` | `is_unknown || written_in_place`, gated by `is_unknown` in one site and not another |
| copy-back after a call, on return and on unwind | `mir/assign.rs` `boxed_args`, `mut_refs.rs` `copied_back`, `mir/structure.rs` | the `Value` variant given (`Ref` or not), not the type |
| reborrow, a held box given on | `mir/rvalues.rs`, `mir/assign.rs` `boxed_args` | `is_cell`, `param_is_box` (callee's environment), `is_cell_pointee` (caller's), `boxed_in_call`, `borrow_outlives_call`, and the JS shapes `Member(_, value)`, `held_cell`, `naming.boxes()` |
| closure capture | `mir/closures.rs`, `mir/operands.rs` `captured_place`, `mir.rs` | `is_cell_pointee` of the capture's pointee |
| `Cell::from_mut`, `get_mut` | `std_types/cell.rs` | `is_cell_pointee` (since 73f5056ec, #614) |
| a std call's handle on an item | `std_types/slice.rs`, `map.rs`, `cell.rs` guards | `handles_mut` |
| `.d.ts` of a `&mut` | `declarations.rs` | `is_cell_pointee` in the item's context (#619) |
| flattened state's handle | `representation.rs` `project`, `js/make.rs` `Storage`, tidy's `read_handles` | the JS shape `Handle(_, Storage::InPlace)` |

## Decision

What a `&mut` to a type is in JS is one enum, `MutRefForm`, made by one
function, `mut_ref_form` (`mut_ref_form_in` for a callee's types, in its
environment), in `representation/mut_ref.rs`:

| form | JS | of |
|---|---|---|
| `Box` | `{ value }` | a concrete value JS can't change in place |
| `GenericBox` | `{ value }`, or `$cellOf(o)` where the caller has an object | a type parameter not bound by `Fn` or `Iterator` |
| `Object` | the object, written whole in place, `$assign` | a JS object |
| `Guard` | the guard's object, never written whole in place | a `RefCell` or lock guard |
| `GivenAsItIs` | the caller's own value, written in place | a type parameter given as it is (ADR 0411) |
| `Itself` | the value, changed only by calling or stepping it | a function, an iterator, a type parameter bound by one |

`MutRefForm::is_cell`, `is_the_object`, `written_in_place` and `ts_type`
each match every form, so a form added is a compile error at each, and an
operation that matches the form must say what it does of each.
`is_boxable` and `is_generic_boxed` are private to the owner; every other
module asks `mut_ref_form`. `is_cell`, `through_refs`, `has_cell_layer`,
`param_is_box`, `param_form` and `handles_mut` are the owner's too, made of
the form. A site that asks for `MutRefForm::Box` alone, not a generic box,
is listed by an architecture test, each one a place a type parameter's
`&mut` may be given another form than its parameter's (#614's kind).

A callee's type is asked in the callee's terms: `mut_ref_form_in`
normalizes it in the callee's environment first, asks whether it's a
generic box before anything else, and asks whether it's a type only a
caller knows there too. Where a caller asked `is_generic_boxed` of a
callee's projection alone, as `nested_mut_object` does of every part of a
signature, the form asked more of it, and normalizing it in the caller's
environment, where the callee's `CAP` isn't, was rustc's internal error
(`deref_projection_generic.rs`); a value given as it is is the function
being lowered's own type parameter, never a callee's.

A declaration asks the owner through `SignatureForms`, the closures the
pipeline makes of a context of the item declared (#619, #620): its
`mut_ref` gives the form, and `MutRefForm::ts_type` the TypeScript type
(`{ value: T }` of a cell, `T` of any other). `JsSignature` is to be built
on it.

The work lands in steps:

1. This one: the owner, its consumers migrated, no output changed.
2. A corpus matrix, generated and run against native Rust: pointees
   (number, `String`, `Vec`, struct, `Copy` struct, enum with and without a
   unit variant, flattened state, type parameter, iterator) × holders
   (local, parameter, field, item, `Result` payload, closure capture,
   generic fn, library default) × operations (whole write, field write,
   `mem::replace`/`take`/`swap`, `Cell::from_mut(..).replace`, a panic
   after the write). Weighted nightly, a path-filtered sample per push.
3. Each wrong answer the matrix finds fixed, and the operation-level forms
   (the get/set handle of `borrow_outlives_call`, `$cellOf`, the flattened
   handle, the copy-back's `$assign` or `=`) made a `MutRefForm` method,
   `given_to(callee, i)`, so the `Value` variant and the JS shapes stop
   deciding them.

## Consequences

- The questions the predicates answered apart are now visible as forms,
  and where they still answer apart is said in one place:
  - `handles_mut` gives a handle of a `GivenAsItIs` or an `Itself` of a type
    parameter (bound by `FnMut` or `Iterator`), where a parameter of that
    type is the value itself: `retain_mut` of a `Vec<F>` in generic code
    gives its closure a handle it calls as the function;
  - the 21 sites that ask `Box` alone give a type parameter's `&mut` the
    value, where a parameter is a box: `Option::as_mut`, `Rc::get_mut`,
    `Pin::set` and the others, in generic code;
  - `moved_out` copies apart a type parameter bound by `Fn` (`Itself`),
    which the form says isn't written in place.
  Each is a matrix case of step 2; one that's a wrong answer is filed and
  fixed there, not here, since step 1 changes no output.
- Asking the form computes a little more than `is_cell_pointee` did (a
  non-cell's guard and object tests); `check:fast` showed no step slower.
- Alternatives: a per-local `LocalFacts` entry, as the study proposed,
  answers per place, but every predicate here is of a type, and the
  per-place choices (handle, in-place cell, copy-back) are per call
  argument: step 3's `given_to`. Leaving the predicates public with a
  lint was rejected: #614 shows a name can't say which question it answers.

## Step 2: the matrix (2026-11-04)

`test/mut-ref-matrix.ts` writes a program for each pointee (a number, a
`String`, a `Vec`, a struct, a `Copy` struct, an enum with and without a
unit variant, a flattened state, an iterator) and holder (a local, a
parameter, a field, a `Vec`'s item, a `Result`'s payload, a closure's
capture, a generic function, a trait's default method): 72, each through
every operation (changed in place, a whole write, a field write,
`mem::replace`, `take` and `swap`, `Cell::from_mut(..).replace`, and a panic
after the write, caught). The type parameter is each generic holder's
pointee, instantiated with each of the others. `test/mut-ref-matrix.test.ts`
runs each beside native Rust: seven in the nightly's suite and in `check:fast`
where a `&mut`'s form or an operation on one changes, all 72 in the
nightly's `mut-refs` job. What it refuses is listed in `refused`, and what
it gets wrong in `wrong` by its issue, each list only shrinking; an
operation a pointee refuses is left out of its programs with why
(`refused`). A flattened state's field holder is its own, `held`: the
class's field through its guard, and lent to a function (#658).

Its first run of all 72 found two wrong answers and a refusal: #636,
what's moved out of a flattened state lent through its guard is the class
itself; #638, a `&mut` to a `Range` that's a `Vec`'s item is a new JS
iterator of it, `Iterator.from($index(items, 1))`; and #639, `Default` of a
`Range` refused (P2), since lifted (c47767a32), so the iterator's programs
take one too. The other 70 run as native Rust does.

## Step 3, begun: the matrix's wrong answers (2026-11-04)

- **#638, a `&mut` std gives to an iterator in a place is that place.**
  `std_value_as` (`mir/dispatch.rs`) made a JS iterator,
  `Iterator.from(..)`, of any std call's value whose type implements
  `Iterator`, and a `&mut I` does: `&mut items[1]` of a
  `Vec<Range<i32>>` was a new iterator of the item, which `r.next()`,
  `mem::replace` and `swap` changed apart from it. It asks the form now:
  a `&mut` is made a JS iterator only where it's the value itself,
  `MutRefForm::Itself` (`is_the_value`, which matches every form), a std
  iterator's; of a range, `Object`, it's the place, as a `&mut` to one
  held any other way already was. Case (ADR 0262): results change, so
  Rust's. Mutation `lent-iterator-place-made-a-js-iterator`
  (`mir/dispatch.ts`), caught by the matrix's `iterator__item`, now in
  its sample.
- **#636, what's moved out of a flattened state through its guard is
  refused.** `let mut state = task.state.borrow_mut()` holds the
  flattened handle, `Storage::InPlace` (ADR 1044), in a local tidy
  inlines later, so `mem::replace(&mut *state, ..)` reached
  `mir_swap_or_replace` as `state.value`, whose read is the class's
  object: `const old = task;`, which the write then changed; `take`,
  `swap` and `Cell::from_mut(..).replace` alike. `mir_store` records
  each local that holds such a handle (`Made::flattened`), and
  `through_flattened` (`mir/rvalues.rs`) asks it of a place of a
  flattened state's type; `mem`'s three and the cell's own calls,
  `Cell::from_mut` of a `&mut` the guard lends among them, refuse it
  with one message, `std_types/cell.rs`' `FLATTENED_TAKEN_WHOLE`. Case
  (ADR 0262): Rust's behavior can't be kept yet, so a compile error,
  consistent with ADR 1044's refusal of the cell's own `replace`. Lifted
  by the next item.
- **#658, #659, what's moved out of a `&mut` to a flattened state is a
  copy of its variant's keys.** The same value lent to a function that
  moves it out, `fn f(s: &mut Status) -> Status { mem::replace(s, ..) }`,
  gave back the object too (#658, P0). The move out is now the owner's
  operation: `moved_out` moved from `mut_refs.rs` to
  `representation/mut_ref.rs`, beside `is_flattened_state`, and gives a
  flattened state's copy of the keys its variant has, by its tag, the
  user's decision of 2026-11-04 (ADR 1044's amendment, which says the
  copy's form), as well as `$apart` of an object or a type parameter.
  `mir_swap_or_replace` asks it of a flattened state's `&mut` too, and the
  cell's own `replace` reads its handle's object; #636's refusals,
  `through_flattened` and `Made::flattened` are gone. A handle on a
  flattened state's handle's `value`, `&mut *state` handed on, is that
  handle, whose setter sets each key (tidy's `read_handles`, #678). The
  lend site isn't
  changed: every function a flattened state's `&mut` is lent to has it as
  the flattened enum, and copies where it moves it out; generic code,
  whose `$apart` would copy the class's object whole, is never lent one,
  as the borrow is refused where that code may meet it. Case (ADR 0262):
  results change, so Rust's. The matrix's `flattened_state__field` runs
  every operation, through the guard and lent to a function; its
  `heldRefused` is gone.
