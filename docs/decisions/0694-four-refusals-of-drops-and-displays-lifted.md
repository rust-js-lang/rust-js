# 0694. `ManuallyDrop::drop`, `extend` of what holds nothing, a matched call's value and `{}` of an `impl Display`

Status: Accepted. Amends [0406](0406-a-union-is-the-field-it-holds.md) and
[0098](0098-destructors.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): R where something is
dropped (what Rust drops, when), N otherwise.

## Context

Four of #171's refusals were of programs Rust is fine with, each from a
check that didn't know a case it covers:

- **`unsafe { ManuallyDrop::drop(&mut m) }`** wasn't recognised.
- **`v.extend((2..4).map(D))`** of a `Vec<D>`, `D` with a destructor:
  `extend` was accepted only where its iterator ends holding nothing.
  `(2..4).map(D)` holds nothing with a destructor at all; its items are
  made as they're stepped, each moved into the `Vec`.
- **`match w.r() { Some(_) => .. }`**, a `&Option<D>` a method gives, of a
  value with a destructor: the drop's `try` flushed the pending
  discriminant before its switch. Its subject, the call, doesn't read the
  same anywhere, so it was written as a value and refused.
- **`println!("{a}")` of an `impl Display` a function gives**: the shown
  type was the opaque one, whose `Display` nothing names.

## Decision

- **`ManuallyDrop::drop(&mut m)` is `Std::DropPointee`**: the destructor of
  what `m` holds, through the `&mut` (`through_refs`), a `ManuallyDrop`
  being what it holds (ADR 0406). It's not `mem::drop`, which of a
  reference drops nothing.
- **`extend` is given what holds nothing to drop** as it's given what
  ends holding nothing: each item it makes is moved into what's extended.
- **A pending discriminant of what doesn't read the same is held first**,
  its subject a `const` where it's flushed, and the discriminant kept for
  its switch, as the switch holds a scrutinee with effects.
- **The shown type is revealed** (`shown_type`), so `{}` and `{:?}` of an
  `impl Display` are of what it hides.

## Consequences

- Held by `test/corpus/refused_drops_and_displays.rs` against native Rust,
  and the mutations `manually-drop-drop-skipped`,
  `extend-of-nothing-held-refused`, `discriminant-of-a-call-refused` and
  `shown-impl-display-hidden`.
- #171's serde items, a `Box<dyn Iterator>` of a struct with `Drop`, and
  `BTreeMap::split_off` of values with one, stay refused.
