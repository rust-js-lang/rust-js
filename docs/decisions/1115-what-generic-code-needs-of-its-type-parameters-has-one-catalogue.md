# 1115. What generic code needs of its type parameters has one catalogue

Status: Accepted (#1170), delivered in steps: the catalogue, its
boundary API and the architecture test first, with [1114](1114-a-reference-a-pointer-may-leave-with-is-its-places-pointer.md)'s
raw pointer as its first fact landed under it; the other routes migrate
onto it in the steps listed under Consequences. Extends
[0145](0145-type-facts.md), whose `TypeFact` it generalizes, and
[0454](0454-a-mut-given-as-a-type-parameter.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): none of its own: it
moves where each fact is decided, not what JS does.

## Context

rust-js writes one JS body per generic function, where Rust writes one
per instance. Wherever JS's form of a value, or what std does with it,
depends on `T`, that one body must learn what each instance has. Each
such fact found its own route, one P0 or refusal at a time: ADR 0145's
`size_of::<T>()`, given at run time; ADR 0454's `&mut` box, one form in
generic code that callers adapt; ADR 0051's boxed `Some`; #1009's
`Rc` counts and #1119's clones, decided over every caller's instance;
#1147 and #1163's dictionaries of std functions; and #1143's raw
pointer, which had no route, so generic code's pointer was a handle and a
slice caller's the array.

An audit of `src/lower` (2026-11-10) found 307 functions that read a type
parameter, `ty::Param`, `has_param()` or `is_unknown(ty)`, or ask one of
the routes. Most ask a structural question, a bound's self type or a
parameter's index. The rest decide a per-type fact in one of four ways:

| Route | How it's answered | Facts | Owner today |
|---|---|---|---|
| **Given** | at run time, a parameter or a dictionary the caller passes, or the value itself | a `Some`'s payload boxed or not (by its value, `$some(x)`; amended below); `TypeFact`'s 19 (size, align, name, id, needs_drop, a map's class, sorts, `Fused`, `TrustedLen`, ...), a bound's dictionary, std functions' `Iterator` dictionaries, drop slots | `type_fact_params`, `given_type_fact` (ADR 0145); `traits/dictionaries.rs`; `stepped_params` (#1147, #1163); `traits/drop_params.rs` (ADR 0421) |
| **Form** | one form in generic code, which a caller, knowing `T`, adapts at the call | a `&mut`, generic code's box; a raw pointer, a handle on its place; an iterator given as it is | `mut_ref_form`'s `GenericBox` and `mut_refs.rs`' `boxed_in_call`, `given_as_the_value`, `generic_result_values` (ADRs 0454, 1086); none (#1143); `iterators/knowing.rs`, `stepping.rs`, `mir/iter/sources.rs`, `closing.rs`' `slicing` (#992, #703) |
| **Instances** | at compile time, of every caller's instance, carried through generic callers; refused, or compiled apart, where they disagree | an `Rc` counted; a clone apart; where an object is stored; a `Some` unboxed by every caller; what JS gives; `eq` and zips std decides by `TrustedLen`; a function's copy apart | `instances.rs`' `instances_of`, shared by `rc_counts` (#1009), `clones` (#1119) and `stored_places` (#1114); `mutation.rs`' `unboxed_params`; `js_given.rs`; `callers_decide.rs` (ADR 1014); `copies.rs` |
| **Alone** | of the generic type, as one answer right for every instance | a dictionary call where `T` is unknown, `$ptrRecast`, a conservative drop or panic | each site |

The **Alone** row is where wrong answers hide: a site that picks a form
from `is_unknown(ty)` is right only if that form is every instance's.
Most ask a dictionary, which is. Some pick a form: whether a value moved
out is a copy apart, whether a hint reaches the crate's own iterator,
whether a clone of a source drops.

## Decision

**`generic_facts.rs` is the one catalogue of what generic code needs of
its type parameters: each fact, the route that answers it, and the
function that does. A site that needs a per-type fact of a type holding a
type parameter asks its owner through the catalogue; a fact's form is
adapted at a call by one boundary API.**

- **The catalogue** is `generic_facts.rs`' table: every fact, its route,
  `Given`, `Form` or `Instances`, and the function that answers it, which
  the architecture test finds. `TypeFact`'s facts are its `Given` facts, as
  they were.
- **The boundary API**, `form_given(form, (declared, instantiated), value)`
  and `form_back(..)`, is the one place a `Form` fact's value is adapted
  where a call crosses from a caller that knows `T` to generic code that
  doesn't, and back. Its first form is `Form::Handle`: a raw pointer whose
  pointee is a type parameter, a handle whose `value` is the referent, as
  ADR 0454's box is, given as `$ptrTo(x)` where the caller's is the value
  itself and read back as `.value`, at a call.
- **An architecture test** holds the sites that decide from a type
  parameter alone to the audited list, by file: a new one fails, naming
  `generic_facts.rs`, and a migrated one lowers its file's count.

### Why one catalogue

- **Each fact was found by its own bug.** Five routes for one question,
  "what does this instance have", each a P0 or a refusal when first met.
  A catalogue makes a new fact a row, decided once, not a sixth route.
- **The route is a choice, made once per fact**: given at run time costs
  a parameter; a form costs an adaptation at each call; instances cost a
  refusal where callers disagree. The catalogue says which, and why.
- **The boundary is one place**, so the adaptations of a call, a
  dictionary's, a `dyn`'s, a function pointer's, a closure's and a
  library's agree.

## Consequences

Migrations, each its own step with its tests, listed on #1170:

1. The `&mut` box's adaptations, `boxed_in_call`, `given_as_the_value`,
   `unboxed_by_function` and `generic_result_values`, onto `form_given`
   and `form_back`. Done (2026-11-10): `Form::Box` is the box's form and
   its one rule, `boxed_in_call`'s cases too, through shared references
   and a type parameter given a `&mut` (`declares_box`); a generic
   function's `fn` pointer adapts through both forms (#1187), a library's
   function's value takes both, a function given to generic code is
   adapted for both (`unboxed_by_function`, `takes_generic_form_itself`),
   and a direct call gives an object's `&mut` as the in-place cell every
   other boundary gives, `$cellOf(p)`, not a box copied back (ADR 0454,
   amended): a box is copied back only of a number's place.
2. A `Some`'s payload: `boxed_payload`'s callers at a call's boundary
   onto the API; its hundred readers inside a body keep asking
   `boxed_payload`, the form's owner. Done (2026-11-10) otherwise: it's
   **Given**, by the value, not a **Form**, so there's nothing to adapt
   (amended below).
3. The iterator given as it is: `given_as_it_is`, `given_as_iterator`,
   `given_iterable` and `slicing`. Done (2026-11-10) as it was: the
   adaptation already has one owner, `stepping.rs`' `given_iterators`,
   which a call's arguments and a function value's arrow both go through,
   `(x) => total($iterator(x, $rangeNext))` of a `fn` pointer, as the
   catalogue now says; `as_js_iterator` and `iterable_items`, which std's
   calls ask everywhere, make a JS iterator of a value, not the boundary's
   choice. `iterator_given_to_generic_code_by_every_route.rs` holds each
   route against native Rust, both builds and every rewrite: a call, a
   `fn` pointer, an `F`, a `dyn Fn`, a closure, a trait's method through a
   `dyn`, a `Box<dyn Iterator>`, and one given as it is, returned or in a
   struct.
4. The `Instances` analyses onto one carrier of callers' instances,
   `instances_of`: `unboxed_params`, `js_given`, `callers_decide`. Done
   (2026-11-10), each step byte-identical: one index of the calls that
   give generic code its parameters' values, `callers_of`, of
   `given_calls_of` (step A); `params_given`, the carrier's second
   question beside `instances_of`'s, whether any caller gives a parameter
   what isn't its own copy, clone or value (step B); `callers_decide` and
   `js_given` carry what they ask through the same index, `calls_by_caller`
   the other way round, so a body of MIR alone is seen too (step C); and
   one rule of whether a function's callers are all seen, `seen_whole(owner,
   fact)`, which asks whether a caller the crate doesn't see gets its own
   copy for the fact: of a copy, a `Clone` dictionary or an unboxed `Some`
   (`Fact::CopiedApart`) a consumer compiles its own and JS's is apart, so a
   `pub` function's crate calls are all; of an `Rc` counted, a place
   stored or a clone apart (`Fact::Shared`) none does, so a `pub` one's
   aren't. Its probes found #1217 (`reached` blind to a body of MIR alone)
   and held each policy against native. Two follow-ups, each changing
   output: #1233 (`CopiedApart` following a trait's generic method through
   its namings) and #1234 (`Rc` counts among what a consumer compiles its
   own for, #1112).
5. The **Alone** sites that pick a form, each checked against every
   instance or moved to a route: `mir/entries.rs`' `moved_apart`,
   `iterators/hint_asks.rs`, `iterators/clones.rs`' `dropped`.

## What holds it

- `test/architecture.test.ts`' "a type parameter's facts are asked of
  their owner": each file's count of `ty::Param(`, `has_param()` and
  `is_unknown(` sites, the audited table, a ratchet each migration lowers;
  growth names `generic_facts.rs`. And "each fact in the generic facts'
  catalogue names an owner that's there".
- Each fact's own tests, as before; 1114's corpus cases per call shape for
  the raw pointer.

## Amended (2026-11-10, #1170): a `Some`'s payload is given by its value

The audit filed a `Some`'s payload, boxed or not (ADR 0051), as a **Form**,
one form in generic code its callers adapt. It isn't one: `$some(x)` boxes
by the value at run time, only what looks like `None`, `undefined`, `null`
or such a box, so generic code's `Some(t)` of a `5` is `5`, as a caller's
`Some(5)` is, and its `Some(())` the box a caller's `Some(())` is, as a
`()`'s `Option` is boxed wherever it's held (`boxed_payload`, of a type that
can be nullish). The value tells generic code and its caller alike: a
**Given** fact, as a dictionary is, with nothing to adapt at the boundary.
Migration 2, moving `boxed_payload`'s callers at a call's boundary onto
`form_given`, would give an adapter to a fact that needs none.

What picks a form by its type keeps to where it's held, checked against
native Rust in `some_given_to_generic_code_by_its_value.rs` (both builds,
`METAMORPHIC=all`):

- A local `Option<Option<T>>` made and read where it's held is its payload
  (ADR 1017); given to a function, `back(n)` or `peek(&n)`, it's held
  boxed, `$some(undefined)`, as the choice is made only where nothing else
  reads it.
- An `Option` of an untagged enum whose variant is `null`, a map's `get`,
  crosses boxed, `$someGet`'s, as `untagged_null_option_held.rs` holds of
  one given to a function.
- Generic code that skips `$some` where no instance can look like `None`
  (`unboxed_params`, an **Instances** fact, which stays one) counts its
  callers through a `fn` pointer, a `dyn Fn`, a generic caller, a closure's
  bound, a trait's generic method and a `map`: each function given a
  number directly and a `()` or an `Option` one other way boxes.

The catalogue files it under **Given**, its owner `boxed_payload` still,
the representation's.

## Amended (2026-11-12, #1233): a trait's method's copies, clones and `Some`s by its namings

Case ([0262](0262-when-rust-and-js-disagree.md)): none, Rust's behavior
kept; output smaller. `params_given`'s facts, a copy (ADR 0289), a `Clone`
dictionary (ADR 0914) and an unboxed `Some` (ADR 0051), took a trait's
method, or an impl's, as unseen, so a trait's generic method always took a
copy function and a `Clone` dictionary and boxed its `Some`s. Now a method
of the crate's own trait, one no other crate can name or implement (not
public), is seen: `instances::method_family` is its declaration and each
impl's method of it, and a naming of any decides what each takes of a
parameter of the method's own, as generic code calls them by the trait's
dictionary, passing what the declaration takes. Each member, the
declaration too, which no item is, is marked, so a call through the trait
and the impl's JS agree. A member's parent parameters, its trait's `Self`
or its impl's own, which a call through the trait names only as `Self`'s,
stay unseen, as `given_on_the_way` takes them.

A public trait, and another crate's, std's or a library's, keep the rule:
another crate's code may call its method with what the crate never sees.
Neither is observable today, so neither has a mutation: a consumer that
gives a library's trait method more than it takes compiles its own of it,
the impl and the generic function calling it (crate pairs
trait_method_of_a_librarys_pub_trait and
impl_of_a_librarys_trait_called_by_it, each against native Rust), and std's
provided methods are rust-js's runtime, which calls a crate's `fold` with
an accumulator of its own, `$folded`'s array (corpus case
fold_override_given_an_option_by_std).

Proof: corpus case trait_method_given_by_its_callers, a `Copy` bound given
`u32`s alone (no copy function) and a struct (a copy), a `Clone` bound
given a `Vec`, a `Some` of what's never `None` (the value) and of an
`Option` given through a generic caller (a box), and an impl's own
parameter given an `Option` as `Self`'s (a box), against native Rust in
both builds; five corpus snapshots smaller, a copy function, a `Clone`
dictionary or a `$someValue` gone (`rc_counted_through_a_traits_generic_method`
by a `try` and a clone of what's its own clone). Mutations
`trait-family-unfollowed` (by its snapshot),
`trait-family-declaration-unmarked`, `trait-impl-params-seen` and
`trait-family-namings-unshared`, caught; `trait-method-copies-by-callers`,
which modelled a call through the dictionary unseen, removed as
`trait-family-namings-unshared` models it now.
