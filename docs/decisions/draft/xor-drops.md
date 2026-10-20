# `xor` drops what it gives up

Status: Draft. Would extend [0496](../0496-and-drops-what-it-gives-up.md),
whose `and` drops its argument by the one table of what std leaves
unused, and [0490](../0490-a-value-std-leaves-unused-is-dropped-there.md).

Case: C ([0262](../0262-when-rust-and-js-disagree.md)): what's dropped,
and when, as std's.

## Context

`Option::xor` of values with a destructor is refused: `Some(a).xor(Some(b))`
is `None`, and std drops both `a` and `b` as it returns; `Some(a).xor(None)`
is `Some(a)` and drops nothing. ADR 0496 gave `and` its argument's drop
through `unused_arguments`, the existing table, and its subject's through
`kept_or`. `xor` needs a condition involving both options.

## Decision

**Extend the existing unused-value decision to express `xor`'s joint
condition: both `Some`, drop the subject's payload then the argument's;
one `Some`, move that option to the result; neither, return `None`.**
The pinned std's `Option::xor` matches the pair and releases the discarded
pair in that order.

Evaluate the subject and argument once, in Rust's order, before selecting
the branch. Use the existing `Option` representation helpers: `Some` may
contain a falsy value or a payload whose JS representation is `undefined`.
JS truthiness is not an option-discriminant test.

The unused-value owner decides which values are discarded; the existing
drop machinery emits their cleanup. Do not leave an old refusal or a
second drop condition in another lowering path. A new standalone `$xor`
helper is not required merely to add destructor support.

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
- **It's the existing owner's**: one place says what std leaves unused;
  extend it rather than invent a parallel `std_leaves_unused` table.

## Consequences

- Remove the specific `xor` refusal only when these cases pass.
  Do not infer that an entire dependent crate is supported.
