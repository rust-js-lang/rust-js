# 1151. A fact of a call is of its node, asked of each node at MIR's span

Status: Accepted (2026-11-14, #1681's change 3, M5, the lead's plan, taken
beside m6-lily's M3; in progress, a fact a commit, each recorded here).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's. A fact keyed by a span was taken of every call a macro's
expansion writes at that span (#1787, a wrong answer). Elsewhere no output
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

Still to move: the in-place adapters (`body_queries/in_place.rs`,
`iterators/in_place.rs`) and the collect sources (`body_queries/
collect_sources.rs`; its lowering side waits on m6-lily's S4, which makes
`mir/locals.rs`' `in_place_source_names` a map to a `VarId`).

## Consequences

- A body fact read by MIR's lowering is keyed by THIR's node; a span is
  only how MIR finds it, through `span_join`.
- A call a macro writes twice at one place keeps its form where both
  copies agree, and loses it where they differ: correct either way.
