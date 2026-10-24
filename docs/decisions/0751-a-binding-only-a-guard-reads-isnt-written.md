# 0751. A binding only an arm's guard reads isn't written in the arm

Status: Accepted. Extends [0364](0364-mir-lowering.md)'s assignments of
what nothing reads.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
what's left out is a read of what the guard has just read, which does
nothing; one that does something is still written, as a statement.

## Context

React's `getArrayKind` (shared/ReactPerformanceTrackProperties.js) tests
a tuple's first item in the test itself:

```js
if (isArray(value) && value.length === 2 && typeof value[0] === 'string') {
  // Key value tuple
```

The port binds it in its pattern, for its guard:

```rust
Kind::Array(&[first, _]) if matches!(js::classify(first), Kind::String(_)) => {
    // Key value tuple
```

rustc gives an arm with a guard two locals of each binding: one the guard
reads through, `BindingForm::RefForGuard`, and one the arm's body has,
assigned once the guard holds. This arm's body reads no `first`, so rustc
warns of none, and rust-js wrote the body's anyway, as it writes every
variable the program names: `const first = value[0];`, read by nothing.

## Decision

**A binding of a guarded arm's pattern that nothing in the arm reads is
written as a temporary nothing reads is**: what makes it, if that does
something, and nothing else. `locals::bound_for_a_guarded_arm` says
which local that is: the one a `RefForGuard` local names.

## Why

- **It's upstream's JS**: the test, and no `const` of what it tested.
- **Only that binding**: a `let x = el.offsetWidth;` the program wrote, a
  read a getter may answer, stays; rustc warns of a binding the program
  names and never reads anywhere else.

## Alternatives

- **Leave out every unread variable whose value does nothing**: `has_effects`
  says a member read does nothing, but a getter's may, which a `let` the
  program wrote keeps.

## Consequences

- `test/lowering-guards.test.ts` holds it; the port's `getArrayKind` has
  no `first`.
