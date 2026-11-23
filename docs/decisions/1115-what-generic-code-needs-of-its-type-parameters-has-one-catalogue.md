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
| **Form** | one form in generic code, which a caller, knowing `T`, adapts at the call | a `&mut`, generic code's box; a raw pointer, a handle on its place; an iterator given as it is | `mut_ref_form`'s `GenericBox` and `mut_refs.rs`' `boxed_in_call`, `given_as_the_value`, `generic_result_values` (ADRs 0454, 1086); none (#1143); `iterators/knowing.rs`, `stepping.rs`, `iterators/mir_steps/sources.rs`, `closing.rs`' `slicing` (#992, #703) |
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
   `iterators/hint_asks.rs`, `iterators/clones.rs`' `dropped`. Done
   (2026-11-13), each checked: none picks a form one instance would
   disagree with, so none moves, and the audit's counts stay as they are.
   `moved_apart` copies apart what's moved out of a type parameter's place
   through a reference, which generic code holds as its box, a caller's
   in-place cell (`Form::Box`); a copy apart is the value of every
   instance. `clones.rs`' clone of a `copied()` of what generic code is
   given is `$given`'s form (`stepping.rs`' `given_iterators`) cloned by
   its `Clone` dictionary (**Given**), and refused where a caller gives a
   source that drops (`given_drop`, **Instances**), or where an associated
   type's source may (`drops`, of every instance). `hint_asks.rs`' reads
   are routes already: a size given (`given_type_fact`, **Given**), the
   extends of a collection generic code doesn't know, which its callers
   decide (`refuse_unknown_capacity_extends`, **Instances**), and the
   refusal of a bound's `extend` of items whose size is a caller's.
   `test/corpus/generic_code_right_for_every_instance.rs` holds each with
   instances that differ, against native Rust in both builds: a `copied()`
   cloned of a std iterator, the crate's own and an adapter; a `Vec`
   collected and extended, its capacity read, of items and iterators of
   each kind; an unknown collection extended; a `swap` through a type
   parameter's `&mut` of a `Vec`, a number and a tuple.

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

## Amended (2026-11-16, #1874): a comparison of what every caller gives a number is JS's own

Case ([0262](0262-when-rust-and-js-disagree.md)): none, so JS's behavior,
as JS's `<`, `<=`, `>`, `>=`, `===` and `!==` order and compare numbers as
Rust's `PartialOrd` and `PartialEq` do, `NaN` unordered and unequal to
itself, `-0` equal to `0`. Generic code compared a type parameter through
its dictionary wherever every instance was a JS number: `clamp<T:
PartialOrd>` called with `i32`s and `f64`s alone was `TPartialOrd.lt(value,
min)`, given `f64PartialOrd` and `$partiallyOrdered` by its callers. Now,
the lead for the user:

- **Instances: `compared_params`** (analysis/mutation.rs), beside
  `cloned_params`, by the carrier's `params_given`: the type parameters a
  bound compares, `PartialOrd`, `PartialEq` or a trait that extends one,
  `Ord` (`compare_bounded`), that some caller gives what isn't a JS number,
  an integer of 32 bits or fewer or a float (`ordering::compares_by_operator`),
  carried through generic callers. A function `seen_whole` says isn't seen
  by `Fact::CopiedApart`, as of a clone, or that another crate or JS can
  name, `Fact::Shared`, compares by its dictionary: no library's contract
  says yet what an export compares by, so exports keep it.
- **One answer, `compares_by_operator_param`,** which the lowering of a
  comparison asks (`operator_value`, `eq_value`, `own_ne_between`) and the
  bounds a body reads (`read_bounds`): `<`, `<=`, `>`, `>=`, `==` and `!=`
  of such a parameter are JS's own and read no dictionary, so a bound
  nothing else reads is given none (#1915). `partial_cmp`, `cmp`, `max` and
  `min` still ask it, an `Option<Ordering>` or a choice no operator is.
- **Not a `char` or text,** which `<` orders by UTF-16 units (ADR 0183),
  nor a 64- or 128-bit integer, a `BigInt`, which keep their dictionary.

An `Ord` bound's dictionary stays given though `<` reads none of it, as
`Ord` extends `Eq`, whose bounds #1915's first step leaves to their old
owner (`answered_elsewhere`): `at_most<T: Ord>(a, b)` is `a <= b`, still
taking `TOrd`.

Proof: corpus cases compared_by_operator (a `clamp`, `largest`, `==`, `!=`
and the four orders of `i32`s, `u32`s, `u8`s, `i16`s, `f32`s and `f64`s with
`NaN` and `-0`, a `u32` instance and an `i32` one each compared both ways, a
generic caller giving its parameter on, an `Ord` bound and a function
pointer) and compared_keeps_its_dictionary (a struct, `char`s, `i64`s, a
caller giving on a `String`, `partial_cmp`, a `pub` function), against native
Rust in both builds, and their metamorphic rewrites. Mutations
`generic-number-compared-by-dictionary`, `generic-number-equal-by-dictionary`,
`generic-number-not-equal-by-dictionary`, `compared-every-caller-a-number`,
`chars-compared-by-operator`, and by their snapshots
`bigints-compared-by-operator`, `compared-export-by-operator`,
`compare-bounded-direct-only` and `read-bound-operator-reads`, caught.

Five corpus snapshots and one example's read so now, each run as before:
generic_unstable_sorts' `(a, b) => a < b` and
copied_default_asks_its_drops_in_the_impls_terms' `key$1(a) < key$1(b)`,
once `TOrd.PartialOrd().lt`; generic_mut_lent_by_std's and
loop_exit_tested_after_the_loop's `===` and `!==`; option_some_mut's
`read(o, v)`, given no `$strictlyEqual`; and the versions example's `x >
best`.

## Amended (2026-11-18, #2031, #2050): how generic code calls what it's given, `on_the_way`

Case ([0262](0262-when-rust-and-js-disagree.md)): R. A wrong answer's owner.

A closure rustc infers `Fn` or `FnMut` drops nothing as JS calls it; Rust's
`FnOnce::call_once` of it runs it and then drops it. Whether generic code
calls one by value is an **Instances** fact, its catalogue's row: each call
by value's type parameter, of every caller, through every generic caller.
The carrier answers it with one worklist beside `instances_of`,
`on_the_way`, sharing its namings, `givings`, which `given_on_the_way`
now reads too, its answers for the other facts as they were:

- **`on_the_way(caller, arg)`** follows a call's argument back through
  every naming that gives it: each function's type parameter whose value
  it is, given on as itself, in a `Box` or an `AssertUnwindSafe`, or by a
  reference (a call by value of a reference calls by `&`), and each
  closure it may be. Each parameter is followed once, where
  `instances_of` follows each way to it: answered so, the facts of
  itertools' adapters took more than nine minutes to compile one crate.
- **`Fact::Called(&Held)`**, its namings: only the crate's own calls give
  its closures, as `Marked`'s. And an impl's own parameter, which every
  other fact takes as any type (a dictionary's), is what's in its place in
  the impl's type, matched (`match_in`): of an impl of the crate's own
  trait, in what each naming of the trait's method gives the trait,
  `Once<{closure}>` with `Once<F>`; of another crate's, which code the
  crate can't see calls, std's `Drop` or `Iterator`, in each type the
  crate's bodies hold of the impl's (`held_of`). A caller's own parameter
  met in the impl's type is matched in its values in turn; one met again
  on that way, an adapter that wraps itself, is the outer match's. A
  `dyn` of the trait hides its impl, as do the patterns `match_in` can't
  read: those, and a trait's own method's `Self` of another crate's
  trait, are `None`, and the closures given to generic code are then taken
  as called by value. Over the corpus, the crate corpus and every test
  file, none was.
- A boxed `Fn` the walk meets is said apart: given as the value of a
  parameter a naming gives, the call that gives it can give it as one
  whose call drops it (ADR 0470, #2033); held in what's given, or given
  back, none can, and it's refused (#2050).


## Amended (2026-11-23, #2376): what std's own code gives the crate's impl

A crate's impl that std's own body calls, `FromIterator::from_iter` of a
collection of the crate's, is named by no body the crate has: `collect()`
calls it inside std. So its type parameter had no instance the carrier
saw, and generic code's questions of it went unasked. A crate's
`from_iter` that asks `size_hint()` of what it's given got JS's own
`values().take(n)`, `(0, None)`, where std's `Take` is exact.

**The rule.** `given_calls_of` adds, beside each naming of a std consumer,
what that call's lowering calls of the crate's impl, as a naming of the
trait's method with the arguments it's given there:
`recognition::crate_impl_called_by`, the one answer the lowering and the
carrier both read.

- `collect()` into a collection of the crate's calls its `from_iter`
  given the iterator as it is.
- Into an `Option` or a `Result` of one, it calls the `from_iter` given
  std's `GenericShunt` of it (`fallible_collect`, which the lowering's
  `try_processed_into` reads too), whose part the runtime's shunt plays.
- `sum()` and `product()` call the crate's `Sum::sum` and
  `Product::product`.

What std's body reaches but no lowering calls, `partition`'s and `unzip`'s
crate `Extend`s, which the runtime's `$partition` replaces (#2377), is no
naming: the carrier holds only what the JS calls.

**Whole, by a check.** In the debug build every test runs,
`instances::check_std_reach` follows std's MIR from each call the crate
makes of std's generic code given something of its own (an instance once;
only while what it's given holds something of the crate's; where std's
MIR is there). Each crate impl it reaches whose own type parameter is an
`Iterator` or an `IntoIterator` must be what `crate_impl_called_by`
answers of that call. One it isn't is kept, its decision `std-reach`'s
`unanswered`, and said where the compile ends without an error, naming
the std method and the impl and saying what to do: add the std method to
the answer, as its lowering calls the impl, or refuse the call. A compile
a refusal ends says the refusal alone, a tuple's `collect()` into the
crate's `Extend`s, `partition`'s (#2377), any later one's: a compile that
succeeds has every reach answered, with no catalogue of refusals. So a
std path no answer covers fails a test, not a program. rustc's own mono collection was
measured first, 4.5 ms a compile, and refused: it needs every
dependency's optimized MIR, which the port's binding crates, built
metadata-only, don't have (77 errors).

**Sound for each reader.** An added naming gives an instance the JS
really gives.

- `hint_asked` (`Fact::Marked`) only marks what's made knowing its hint,
  and a form that knows its hint is right wherever it's made. Its
  `may_leave` reads the same answer: a value a std consumer gives the
  crate's impl leaves the function that makes it.
- `clones`, `stored_places` and `cell_borrows` (`Fact::Shared`) each
  answer of the set of what callers give, a property any instance has. A
  real instance can only add one the JS meets, never take one away.
- `rc_counts` (`Fact::Counted`) counts what's given, and a real instance
  is one it should count.
- `callers_decide` and `closures::called` read each naming as a direct
  call of the trait's method, which this is.
- `Fact::CopiedApart`'s readers (copies, `traits`, `mutation`,
  dictionaries) never follow an impl's method, so they're unchanged.

**Held by** the corpus's `hint_asked_by_a_crates_own_collect_and_sum.rs`,
against native Rust: a `take(n)`, `skip(n)`, `filter`, `Result` collect
and `Sum`. Also held by `lowering-generic-iterators.test.ts`' check of an
unanswered path found, a tuple's `collect()` into two `Extend`s of the
crate's, which its refusal says, the check's rule's unit test,
and by a mutation of each rule. Two snapshots move: a `from_iter` that
collects generically, which std's `Vec` collect asks the hint of, is given
`$mapEnds`, not JS's `map`.


## Amendment: too many instances to list is not no instance (#2406)

`instances_in` stops listing a type's instances past `MOST_INSTANCES`
(64, #1009), each choice for each parameter put in its place. It said so
with `None`, the same answer as "a caller can't be seen".
`seen_instances_of` turned that `None` into an empty set. So when
generic code asked the hint of what it's given,
`fn hint_of<I: Iterator>(it: I) { it.size_hint() }`, and had 65
callers, none of their iterators was made knowing its hint. JS's own
`.map` of an array's items then gave `(0, None)` where Rust's `Map` gives
`(5, Some(5))`. Fewer than 65 callers was right. ADR 0262: the results
change, so Rust's behaviour is kept.

`instances::Seen` is now the answer: `These(types)`, `Many` (past what
it lists, never none) or `Unseen` (any type). Each reader says what it
does with each case:

- hint_asked's `of_params`: on `Many`, `seen_parts_of`. That's each type
  each parameter is given, and what each generic caller makes on the
  way, without their product (`Combine::Parts`, which can't overflow).
  Every instance's parts are among them, so marking each part marks
  what each instance's walk would. `Unseen` marks nothing, as before
  (the runtime's `$hintAsked`, #2277).
- hint_asked's `grown`, clones' `as_any_instance` and its cloned types,
  rc_counts' pointees, cell_borrows' `read_asks`, and lent.rs's
  `addressable`, `lent_offset` and boxed payload, plus type_facts' lends:
  `Many` as `Unseen`, any type, through `Seen::listed_or_any`. Each of
  these was already conservative for "any type".
- lent.rs's `known_within`: `Unseen` is offset 0 (a `pub` function's
  consumers, as before). `Many` is refused: generic code has one JS for
  every instance, and with too many to check that each holds its object
  at the same offset, it is never assumed to be 0.

This is held by `test/corpus/hint_asked_of_more_instances_than_listed.rs`
(65 callers, both builds) against native Rust, and by the mutations
`many-instances-marked-as-none` and `parts-put-together`. The
conformance table's `| hint given |` rows, picked and batched together,
had failed this way. They pass now.
