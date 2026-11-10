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
| **Given** | at run time, a parameter or a dictionary the caller passes | `TypeFact`'s 19 (size, align, name, id, needs_drop, a map's class, sorts, `Fused`, `TrustedLen`, ...), a bound's dictionary, std functions' `Iterator` dictionaries, drop slots | `type_fact_params`, `given_type_fact` (ADR 0145); `traits/dictionaries.rs`; `stepped_params` (#1147, #1163); `traits/drop_params.rs` (ADR 0421) |
| **Form** | one form in generic code, which a caller, knowing `T`, adapts at the call | a `Some`'s payload boxed or not; a `&mut`, generic code's box; a raw pointer, a handle on its place; an iterator given as it is | `boxed_payload` (ADR 0051), asked at about a hundred sites; `mut_ref_form`'s `GenericBox` and `mut_refs.rs`' `boxed_in_call`, `given_as_the_value`, `generic_result_values` (ADRs 0454, 1086); none (#1143); `iterators/knowing.rs`, `stepping.rs`, `mir/iter/sources.rs`, `closing.rs`' `slicing` (#992, #703) |
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
  itself and read back as `.value`, at a call and where a generic function
  is a value.
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
   `unboxed_by_function`, `takes_box_itself` and `generic_result_values`,
   onto `form_given` and `form_back`; and a generic function's instance as
   a `fn` pointer a value of its own taking its type's form, which an
   object's or a slice's `&mut T` is refused as till then (#1187).
2. A `Some`'s payload: `boxed_payload`'s callers at a call's boundary
   onto the API; its hundred readers inside a body keep asking
   `boxed_payload`, the form's owner.
3. The iterator given as it is: `given_as_it_is`, `given_as_iterator`,
   `given_iterable` and `slicing`.
4. The `Instances` analyses onto one carrier of callers' instances,
   `instances_of`: `unboxed_params`, `js_given`, `callers_decide`.
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
