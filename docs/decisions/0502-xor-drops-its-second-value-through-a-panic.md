# 0502. `xor` drops its second value through a panic

Status: Accepted. Amends [0498](0498-xor-and-zip-drop-what-they-give-up.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

`Some(a).xor(Some(b))` drops `a`, then `b` (ADR 0498). Its two drops ran
one after the other: where `a`'s destructor panicked, `b`'s never ran,
where Rust's unwinding drops it.

## Decision

**`xor` of two `Some`s drops its own value, then the other's in a
`finally`: `try { drop(a) } finally { drop(b) }`, so `b` is dropped though
`a`'s drop panics.** `unpaired` keeps choosing what's given up; it's
`zip`'s, which drops at most one, as before.

```js
if (option != null && other != null) {
  try {
    loudDrop_drop(option);
  } finally {
    loudDrop_drop(other);
  }
}
```

## Why

- **It's std's**: `xor_drop_panics` drops two `Some`s whose first's
  destructor panics, with native Rust: both dropped, and the panic.

## Consequences

- **A panic while another is unwinding** is as rust-js's other drops
  have it, not an abort.
