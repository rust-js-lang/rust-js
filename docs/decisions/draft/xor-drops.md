# `xor` completes cleanup when a discarded payload panics

Status: Draft. Would amend
[0498](../0498-xor-and-zip-drop-what-they-give-up.md), which now implements
ordinary `xor` and `zip` drops through `unpaired`.

Case: C ([0262](../0262-when-rust-and-js-disagree.md)): what's dropped,
and when, as std's.

## Context

`Option::xor` of values with a destructor is now supported.
`Some(a).xor(Some(b))` is `None`, and `unpaired` emits the drop of `a`
then `b`. Its two drop-statement lists are concatenated. If `a`'s drop
throws, execution never reaches `b`'s list.

The normal-path corpus case in ADR 0498 proves the ordinary pairings,
but its destructors do not panic. The earlier draft's general proposal
is therefore superseded; this draft is only the remaining unwind work.

## Decision

**Keep `unpaired` as the one owner of `xor` and `zip` selection, and use
the existing drop/unwind machinery to finish both discarded payloads'
cleanup.** For `xor` of two `Some`s, drop the subject first and arrange
the argument's drop as cleanup that runs even when the first panics.
The pinned std's `Option::xor` releases the discarded pair in that order.

Evaluate the subject and argument once, in Rust's order, before selecting
the branch. Use the existing `Option` representation helpers: `Some` may
contain a falsy value or a payload whose JS representation is `undefined`.
JS truthiness is not an option-discriminant test.

Do not add another unused-value table or a standalone `$xor` helper.
`unpaired` already knows the joint condition; drop emission owns how to
clean up what that branch discards. `zip`'s existing selection and
ownership must stay unchanged.

**Cleanup must survive a panic in the first destructor.** Transfer
ownership before each drop and arrange the second's cleanup through the
existing unwind machinery. Two consecutive calls would skip it if the
first throws. An option moved into the result must not also be cleaned
up by the caller.

For a type parameter, use the existing caller-provided drop evidence.
Keep the same selection rule whether that evidence does work or is absent.
General double-panic behavior remains the existing documented limitation;
this draft does not claim to resolve the [panic proposal](catch-unwind.md).

## Why

- **It's std's**: corpus cases cover all four pairs, concrete and generic
  values, nested options, falsy payloads and side effects while making
  the two arguments. Record the result and the exact destructor sequence.
- **Unwind cleanup is tested**: run a case whose first discarded payload
  panics and whose second records its drop. Use the existing native/JS
  panic oracle; `catch_unwind` support is not a prerequisite.
- **Mutations prove the checks**: reverse the drop order, skip cleanup
  after a panic, drop the surviving result, evaluate an operand twice,
  or test option presence by truthiness.
- **It's the existing owner's**: `unpaired` keeps its selection rule;
  cleanup reuses the drop machinery rather than duplicating it.

## Consequences

- Ordinary `xor` support is already accepted. After this change, its
  two-payload cleanup also survives a first destructor's panic.
- Amend ADR 0498's sequential-drop example when implemented. Do not infer
  that every dependent crate or every unwinding case is then supported.
