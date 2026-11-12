# 1086. A `&mut`'s JS form is `MutRefForm`'s

Status: Accepted (2026-11-04, steps 1 and 2 of 3; step 3 begun, its
first operation, a move out, `moved_out`; 2026-11-08, every site asks
the form by its methods, #821). Amends
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
| `is_generic_boxed(ty, env)` | a type parameter, not bound by `Fn` or `Iterator` | 10: generic calls' boxes (`boxed_in_call`, since `generic_facts.rs`' `Form::Box`, ADR 1115; `unboxed_by_function`, `nested_mut_object`) and a call's argument |
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
| reborrow, a held box given on | `mir/rvalues.rs`, `mir/assign.rs` `boxed_args` | `is_cell`, `param_is_box` (callee's environment), `is_cell_pointee` (caller's), `boxed_in_call` (since `Form::Box`, ADR 1115), `borrow_outlives_call`, and the JS shapes `Member(_, value)`, `held_cell`, `naming.boxes()` |
| closure capture | `mir/closures.rs`, `mir/operands.rs` `captured_place`, `mir.rs` | `is_cell_pointee` of the capture's pointee |
| `Cell::from_mut`, `get_mut` | `std_types/cell.rs` | `is_cell_pointee` (since 73f5056ec, #614) |
| a std call's handle on an item | `std_types/slice.rs`, `map.rs` | `handles_mut`; a guard's deref, `cell.rs`, `mut_ref_form` (#1016) |
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
    `Pin::set` and the others, in generic code (a wrong answer, #1007:
    each asks `is_cell` now, below);
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
- **#687, the flattened state's cell's own moves out.** Its `into_inner`,
  `replace_with` and `swap` copy its variant's keys by the same owner's
  `moved_out`, and a cell of one moved whole out of its field by
  `moved_out_of_cell`; its `get_mut` and `try_borrow` lend the handle a
  `&mut` to it is (ADR 1044's amendment).
- **#1007, a `&mut` a std call lends generic code is a box.** The 17
  std calls that asked `mut_ref_form(item) == MutRefForm::Box`, a box of
  a known type alone, gave a type parameter's `&mut`, `GenericBox`, the
  value, which generic code reads as a box, as every `&mut T` there is
  (ADR 0099): `Result::as_mut` and `iter_mut`, `Pin::set`, `Rc::get_mut`,
  `OnceCell::get_mut`, `LazyCell::force_mut`, `Peekable::peek_mut` and
  `VecDeque::range_mut` threw on the write in JS, `BinaryHeap::peek_mut`
  didn't sift, and `dedup_by` and `pop_if` were refused. Each asks
  `is_cell`, a box of either kind, as a parameter does; no site asks for
  a known type's box alone. No corpus case or matrix program had reached
  one with a type parameter: the corpus and every `&mut` matrix program
  are byte-identical, and `generic_mut_lent_by_std.rs` holds each. Found
  refactoring the sites into the form's methods (#821). Case (ADR 0262):
  results change, so Rust's.
- **#821, every site asks the form by its methods.** Some 30 sites
  outside `representation/mut_ref.rs` compared the form, `==
  MutRefForm::Box` or `== MutRefForm::GenericBox`, each choosing for every
  other form, a new one too, by its silence; the 17 of the first were
  #1007. The rest, eight in `mut_refs.rs` of what generic code and its
  dictionaries adapt, ask `is_generic_box`, a method beside `is_cell`,
  `is_the_object`, `is_the_value` and `written_in_place`, each a match
  that names every form; `traits/impls.rs` asks `is_cell` by a closure.
  `MutRefForm` isn't comparable any more, and
  `test/architecture-mut-refs.test.ts` refuses a form named outside its
  module, but the one site that makes one, a trait's dictionary entry's
  adapted parts, generic code's box whatever its impl's
  (`traits.rs`), listed with why, and a method of the form's own whose
  match has a `_` arm, a `matches!` or an `==`. Byte-identical: the
  corpus, `METAMORPHIC=all` and every program of the `&mut` and `None`
  matrices. What's left of step 3 is the operation-level forms, the
  get/set handle of `borrow_outlives_call`, `$cellOf`, the flattened
  handle and the copy-back's `$assign` or `=`, made one method,
  `given_to(callee, i)`.

## Step 4: the matrix grown to the space (2026-11-08, #855)

The matrix's 72 programs were 9 pointees × 8 holders through 8 operations,
in the default build alone. #855 grows it to the space the forms meet:

- **Pointees** (14): a `bool` and an `f64`, boxes as a number is; an
  `Option<i32>`, whose `None` is a value of its own; a tuple, an array JS
  changes in place; and a `Box<i32>`, its value.
- **Holders** (15): an `Option`'s payload, a tuple's field, a `Box`'s
  value, a `RefCell`'s guard (`MutRefForm::Guard`, which no holder had
  given), each item `iter_mut` steps, a method's field of `self`, and a
  `BTreeMap`'s value by `get_mut`.
- **Operations** (14): a reborrow changed; the `&mut` given on to the
  pointee's own method, to generic code, and to generic code that panics
  after its change, which a box copies back as it unwinds (#487); `*self`
  written whole by a trait's default method; and a closure that captures
  it, called twice.

210 programs, each built twice, plain and `--std-iterators` (#855 asks
both): all 210 a night, twelve in the nightly's suite and in `check:fast`
where a `&mut` changes, the seven of before and five of the new holders.
`refused` and `wrong` are keyed `<build>:<program>`; an operation refused
of one holder alone is `<op>@<holder>` in its pointee's `refused`. Its first
run: every program runs as native Rust does in both builds, 10.5 s on the
Mac in 12 parts, but a flattened state's three operations that lend it past
its class's guard to generic code or a trait's default method, which
rust-js refuses (its borrows are its object's own keys, which count none),
left out of its `field` holder with why.

## Amendment: a guard of a caller's iterator derefs as `mut_ref_form` says (#1016)

The metamorphic net's `rc-refcell` rewrite (#1008) put an iterator of a
type only a caller knows in a `RefCell`, `Rc::new(RefCell::new(source
.items()))` of an `S::Iter`, and stepped it through its guard,
`(*items.borrow_mut()).next()`: it threw `it.next is not a function`, as
did `step(&mut *held)` of an `I: Iterator`. A guard's `deref_mut` asked
`handles_mut`, a std call's handle on an item, which is a handle of any
type only a caller knows; a `&mut I` of an `I` bound by `Iterator` is the
iterator itself (`MutRefForm::Itself`), which every other site, and the
function it's given to, reads as it. A guard's deref now asks
`mut_ref_form`, the cell, the box that is, of a `&mut` that's a cell, its
`value` of any other, so a `&mut F` of an `F: FnMut` is the function, as
it was. And std's `&mut` to a type only a caller knows is the place, not
made a JS iterator, `Iterator.from(held.value)`, which `*held.borrow_mut()
= other` couldn't write, a crash of the compiler: what's there is the JS
iterator its caller gave (`std_lends_a_place`). The table's "a std call's handle on an item"
keeps `handles_mut` for `retain_mut` and the like; a guard's deref is
`mut_ref_form`'s. Results changed
([0262](0262-when-rust-and-js-disagree.md)), so Rust's are kept; no other
case's JS changed. Held by the corpus case
`iterator_in_a_ref_cell_given_generic_code` and the mutations
`guard-of-an-iterator-the-cell` and
`std-mut-ref-of-a-callers-iterator-made-one`; the net's five
`rc-refcell` lines of #1016 run as native.

## Amendment: a `&mut` to a shared reference to an object is a cell (#1021)

`*m.as_mut().unwrap() = &other` of an `Option<&HashMap<..>>`, the
net's `option` rewrite of `loop_exits`, was `$assign(map, other)`:
a `&mut` to `&HashMap` was `MutRefForm::Object`, the map itself, as
`is_object` of the reference said, so a write of the whole copied
`other` into the map it was to, where Rust makes the reference another
(native `Some(5)`, JS `None`). A `&mut` to a shared reference to an
object is a cell (`is_boxable`), as one to a reference to a struct's
array was, so the write is the reference's. Results changed
([0262](0262-when-rust-and-js-disagree.md)), so Rust's are kept; no case's
JS changed. Held by the corpus case
`written_through_a_guard_or_a_reference_to_an_object` and the mutation
`mut-ref-to-a-shared-ref-the-object`.


## Amendment: a `&mut Box<dyn FnMut>` made a `&mut dyn FnMut` is the function its cell holds (#1222)

A `&mut` to a boxed function, `&mut Box<dyn FnMut(i32)>`, is a cell, `{
value }`, as `mut_ref_form` of the box says, since a write through it,
`*r = Box::new(g)`, replaces the function. Made a `&mut dyn FnMut`, the
unsizing coercion of the `dyn` branch (`traits/dyns.rs`,
`unsize_trait`) gave that cell as the `dyn FnMut`, and its call threw
`f is not a function` (found by #1204's `dyn-fn` rewrite: three corpus
cases). A `dyn FnMut` can't be written whole, and the box can't be while
it's lent, so the `dyn FnMut` is the function the cell holds,
`Expr::held(value)`, read where it's made: the coercion asks
`mut_ref_form` of the pointee as every other reader of a `&mut`'s form
does. Results changed ([0262](0262-when-rust-and-js-disagree.md)), so
Rust's are kept. Corpus case `mut_ref_of_a_boxed_function_as_dyn`,
mutation `mut-ref-of-a-boxed-function-its-cell`.

## Amendment: what `*r` reads is `r` only where `read_is_itself` says (#1311)

An `Option<&mut i32>` read by value gave the item's handle where Rust
gives the number, `Some([object Object])`: `copied()`, `cloned()` and
`map(|x| *x)` of `it.next()`, `nth(k)`, `last()`, `first_mut()` or
`get_mut(i)`, in both builds. So did `copied()` of a `Result<&mut i32, E>`,
`as_deref()` of an `Option<&mut i32>`, and `map(|s| s.as_str())` of an
`Option<&mut String>`. Results changed
([0262](0262-when-rust-and-js-disagree.md)'s third case), so Rust's are
kept. Each took `*r` of a `&mut` for `r` itself, as it is of a shared
`&T`, which is its value in JS. Three sites decided so, apart:
`closure_gives_its_argument`'s step through `[Deref]`, `OptionCloned` and
`ResultCloned`, which clone the value as `Option<&T>`'s do, and
recognition's `as_deref`, which took any reference's pointee for the
option.

The question is the form's: `MutRefForm::read_is_itself`, whose match
names every form. It is false of a cell's, `Box` and `GenericBox`, whose
read is its `value`. It is true of the object, a guard's object, what's
given as it is, and the value itself. `closure_gives_its_argument`'s
`[Deref]` step of a `&mut` asks it. `copied()` and `cloned()` of an
`Option<&mut T>` of a cell read its `value` and give `Some` of it as
`map` does: `o?.value`, `o != null ? $some(o.value) : undefined` of an
`&mut Option<i32>`. The `Some`-where-present tail `map` wrote inline is
now `some_where`, which both use. `read_through_held` (combinators'
values) does the same for a `Result<&mut T, E>`'s `copied()` and
`cloned()`, `r.TAG === "Ok" ? { TAG: "Ok", _0: r._0.value } : r`, and for
`as_deref` of either. Recognition's `deref_is_itself` keeps `as_deref_mut`
of a `&mut` and `as_deref` of a `&T` as the option itself. `as_deref` of
a `&mut` goes to `Comb::AsDeref`, which asks the form. A generic `T`'s
`&mut`, `GenericBox`, is read the same way. `*o.unwrap()` and
`map(|x| *x + 1)`, which lowered the read, were right before.

`test/corpus/option_of_a_mut_ref_read_by_value.rs` holds each, of a
number, a `String`, an `Option<i32>` (some, none and none outside), a
generic `T` and a `Result`'s `Ok` and `Err`, against native Rust in both
builds. Mutations: `mut-ref-cell-read-is-itself`,
`closure-deref-of-mut-ref-gives-itself`,
`option-copied-of-mut-ref-gives-handle`,
`result-copied-of-mut-ref-gives-handle` and
`as-deref-of-mut-ref-is-itself`.

## Amendment: a `&mut Box<I>` of an iterator is what a `&mut I` is (2026-11-11, #1314)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's are kept.

A `&mut` to a box was a cell, `{ value }`, as a write of the whole through
it, `*b = Box::new(..)`, gives the box another value (#1222). Of a box of
an iterator, that cell was lent where the iterator was asked:
`boxed.by_ref().take(2)` gave generic iterator code the cell, which it
stepped as an iterator, and threw. The user decided: when `I` is an
iterator, a `&mut Box<I>` takes a `&mut I`'s form, the iterator itself,
as a `Box` is what it holds in JS. That covers `by_ref()`, a `&mut boxed`
given to generic code, a `Box<dyn Iterator>`'s and every other route,
asked of the one owner, `mut_ref_form`, which gives the box's `&mut` its
contents' form, of an iterator the type's bounds say where it's asked.

What reads a `&mut`'s form follows it: a write of the whole through one,
`*b = Box::new(v)`, is `$assign(b, v)`, and `mem::replace`'s `$take`, as
of a `&mut I`. Two more needed the box seen through:

- **What's supported.** `support.rs` checked a `&mut Box<I>` by the
  type's own predicates, which no box matches, and refused it, "values of
  type `&mut Box<Range<i32>>`"; it checks it as a `&mut I` now, where its
  form isn't a cell.
- **A write of what the box holds,** `**b = v`, was written as the
  parameter given another value, `b = v`, the caller's iterator left as
  it was: wrong in both builds. It's the `&mut`'s whole write now,
  `$assign(b, v)` (`mir/assign.rs`' `through_box`).

One snapshot changes: `boxed_iterator_stepped_as_what_it_boxes`'
`stepped(ranged)` takes the range itself, `$rangeNext(boxed)`, where it
made a cell and wrote it back. `test/corpus/mut_ref_of_a_boxed_iterator_is_the_iterators.rs`
(`by_ref()` of a `Vec`'s boxed iterator and of a `Box<dyn Iterator>`, a
`&mut boxed` given to generic code, `*b = Box::new(..)` of a `dyn`, `**b =
..` of a `Vec`'s iterator and of a range, `mem::replace(&mut **b, ..)` and
`mem::replace(b, ..)`, a reborrow of it by its deref and a step through
`(**b)`) runs against native Rust in both builds. Mutations:
`mut-ref-of-a-boxed-iterator-a-cell` (`lower/representation/mut_ref.ts`),
`mut-ref-of-a-boxed-iterator-unsupported` (`lower/support.ts`) and
`write-through-a-boxed-iterator-lost` (`lower/mir/assign.ts`).

## Amendment: what gives what it's given reads through a `&mut`'s cell its result isn't behind (2026-11-12, #1486)

`s.as_ref()` of an `s: &mut String` is std's forwarding impl, `AsRef<U> for
&mut T`, its receiver a `&&mut String`; `r.borrow()` of an `r: &mut i32` is
`Borrow<T> for &mut T`. The recogniser answers both `Std::Same`, the same JS
value, which is right of a `&mut` to an object, the object itself, and wrong
of one to text or a number, a cell: the `&str` it gives is the text in it.
`fn show(s: &mut String) -> usize { let b: &str = s.as_ref(); b.len() }`
printed 0, and `println!("{b}")` `[object Object]`, with no error.

`Same` says a call's result is its argument; it never said the two are held
alike. `same_as` (`representation/mut_ref.rs`) now does, of the argument's
type and the result's: the value as it is where both are behind a `&mut`'s
cell or neither is, and where only the argument is and the result is a `&`,
what's in the cell, `through_refs`' read. What's given on as a `&mut` keeps
its cell: `identity(&r)`'s `&&mut i32`, `Box::new(&mut *r)`'s box, and
`r.borrow()` to a `&&mut i32`, std's `Borrow<T> for T`. Rust's behaviour is
preserved (ADR 0262's "results change").

Found by compiling 195 one-probe programs of #1477's 21 trait methods in
shapes the corpus doesn't write, each against native Rust, which
[finding wrong answers](../finding-wrong-answers.md) lists.
`test/corpus/as_ref_and_borrow_through_a_mut.rs` (a `&mut String`'s
`as_ref()` and `borrow()` as a parameter, a local, a field and a closure's
parameter, through a `&` to it and written out; a `&mut` to an `i32`, a
`bool`, a `char` and an enum borrowed; a `&mut Vec`'s, unchanged; generic
code given a `&mut` as its `K: Borrow<i32>`; and the three that keep the
cell) runs against native Rust in both builds. Mutations
`same-through-a-mut-the-cell`, `same-of-a-ref-to-a-mut-read-through` and
`same-into-what-owns-a-mut-read-through` (`lower/representation/mut_ref.ts`).

## The matrix crosses std's impls by type (2026-11-12)

Three operations more, each program's (#1459's third rule): `borrowed`,
`Borrow::borrow(&r)`, a `&` to what a `&mut` is to (std's `Borrow<T> for
&mut T`, #1486's); `borrowed_mut`, `BorrowMut<T> for T`, the `&mut` itself
(#1487's); and `given_back`, `identity` of a reborrow. Their first run of all
210 found one wrong answer, #1514, of one holder, a closure's capture: a
`&mut` held as its place's own name, the outer variable a closure reads, gave
`borrowed undefined`, as #1486's `same_as` asks the types whether a cell is
read through, and a shared reference to such a `&mut` was the place itself,
no cell. Now that reference is a handle on the place, the cell its type says
(`held_as_its_place`, `mir/temporaries.rs`, which owns what a local holds), and `same_as`'s answer by type is
true of it: all 210 run as native Rust does. Corpus case
`borrow_of_a_captured_mut`, mutation `captured-mut-ref-no-handle`.

