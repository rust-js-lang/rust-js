# 1151. A fact of a call is of its node, asked of each node at MIR's span

Status: Accepted (2026-11-14, #1681's change 3, M5, the lead's plan, taken
beside m6-lily's M3; in progress, a fact a commit, each recorded here).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's. A fact keyed by a span was taken of every call a macro's
expansion writes at that span (#1787, #1788, #1791, wrong answers); where
Rust's can't be kept yet, a compile error (#1789). Elsewhere no output
changes.

## Context

Some facts the body's analysis finds, before MIR is lowered, choose a
form for a call: a `resume_unwind` of a payload as it was given throws it
again (rd550); a `function::call` of a `js::method` whose `this` is its
path's value is the method's own call (rd334). Each was a set of spans,
the call's `fn_span`, which MIR's call is looked up by. Two calls a
macro's repetition writes, `$(f(..);)*`, share that span, and a fact of
one was taken of the other: `focus_each!(a => a, a => b)` called `a`'s
`focus` of `a` where Rust gives `b` (#1787).

## Decision

**Each such fact is a set of THIR nodes, and MIR's call asks it of each
THIR call at its span (`span_join::each_call_at`): the fact holds
whichever node it is, of each there, and there's one.** Of two calls
alike, both, the fact is the call's; of two apart, neither's is taken of
the other. Never a raw span. Where a fact licenses leaving out a check,
its join is still the one node there or none (`the_one_at`,
`the_call_at`, ADR 0292); `each_call_at` is for a fact that chooses a
form, where a call of two alike keeps the form it had.

Each fact moves in a commit of its own, with a case where two forms share
a span inside a macro:

1. `resumed_as_given` (`body_queries.rs`, read by `std_types/unwind.rs`):
   corpus case catch_unwind_resumed_in_a_macro, two alike, `throw
   payload` each, and two apart, one of a payload made there, neither
   taken as given. Mutation resumed-as-given-by-a-shared-span, any call
   at the span taken, caught.
2. `method_calls` and `called_methods` (`body_queries/method_calls.rs`,
   read by `binding_calls.rs` and `mir/calls/method_paths.rs`): of two
   alike, each its method's own call, `a.focus(undefined)`; of two apart,
   neither, `a.focus.call(b, undefined)`, and a key's refused, as it
   reads no method's own call (#1787). Mutations method-call-by-a-shared-span
   and called-method-by-a-shared-span, caught.

3. The collect sources (`body_queries/collect_sources.rs`), the `Vec` a
   `collect()` std may make in place is made of, whose capacity is the
   collect's (#1259): two collects a macro's repetition writes at one
   span, of sources `a` and `b`, each took `b`'s capacity, `1 3` where
   Rust's first is `1 10` (#1788, a wrong answer). Its lowering side, the
   source's name by the collect's span (`mir/locals.rs`'
   `in_place_source_names`, read by `mir.rs`' `in_place_source`), waits
   on m6-lily's S4, which makes it a map to a `VarId`. Till then the
   analysis refuses collects at one span whose sources differ, naming
   #1789, which keys both sides by node; of one source alike, or none, as
   each expansion's own, they compile. Corpus case
   collect_in_place_in_a_macro, the guide's limitations; mutation
   collects-at-one-span-of-sources-apart, caught.

4. The in-place adapters (`body_queries/in_place.rs`, the `InPlace`
   decision in `body_queries.rs`), whether a `rev()`, an `enumerate()`
   or a `map()` is the recognizers' form or std's compiled struct (ADR
   0871): of two a macro's repetition writes at one span, one given to a
   generic function's `I`, which steps it as a JS iterator, and one to a
   function that holds it as std's struct, both were made in place, so
   the held one stepped the in-place form from its back: an
   `Enumerate`'s `next_back` gave `7` where Rust's is `3007`, a `Rev`'s
   and a `Map`'s threw (#1791, a wrong answer). Both stages hold THIR
   calls, and the join is `InPlace`'s own, one API for every reader:
   `made_at` (`mir/iter.rs`' `adapter_in_place`, `type_facts`, which asks
   it of a THIR call's own span so what it asks is what the lowering
   makes) and `ground_made_at` (`back_stepped`, before the held stage).
   What the analysis asks of another adapter to make one in place asks
   the same join: the call it's given to (`a.iter().rev()` a macro gives
   its `enumerate()`), the `rev()` a variable is made of (`ma` of
   `$f(ma.rev())`), and the `flat_map` whose closure gives what's made in
   place; a node's own answer alone made `ma` `$mapEnds(..)` beneath std's
   held `Rev`, which threw. Of two apart, both are std's struct; of two
   alike, the recognizers' form, as one alone. Corpus case
   in_place_adapters_in_a_macro, each adapter both ways round and alike,
   a variable's `rev()` and a chain the macro is given; mutations
   in-place-by-a-shared-span (any call at the span taken),
   in-place-given-to-a-node-of-a-shared-span and
   in-place-made-of-a-node-of-a-shared-span (a node's own answer), caught,
   and the module's mutations retargeted to its THIR calls.
   `back_stepped`'s `ground_made_at` takes the same join; any call at the
   span there survives the case, as a held `rev()`'s source is marked by
   the variable's own rules too, so it has no mutation of its own.

Still to move: the collect sources' lowering side (#1789), after
m6-lily's S4.

## Consequences

- A body fact read by MIR's lowering is keyed by THIR's node; a span is
  only how MIR finds it, through `span_join`.
- A call a macro writes twice at one place keeps its form where both
  copies agree, and loses it where they differ: correct either way.
