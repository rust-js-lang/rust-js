# 0568. A backtrace is disabled unless forced

Status: Accepted. Extends [0034](0034-strings-and-chars.md)'s amendment and
[0427](0427-track-caller.md).

Case: B ([0262](0262-when-rust-and-js-disagree.md)): a `capture()` is
disabled where native Rust's depends on `RUST_BACKTRACE`, which a page
can't read; listed under Consequences.

## Context

anyhow keeps a `std::backtrace::Backtrace` in every error it makes,
`Backtrace::capture()`'d where the error is, shown by its `Debug` when it
was captured, and asked its `status()` to know. rust-js refused the call,
and with it anyhow, after its pointer casts (ADR 0562) and
`Arguments::as_str` (ADR 0034).

What `capture()` gives in Rust depends on the environment: with
`RUST_BACKTRACE` or `RUST_LIB_BACKTRACE` unset, a disabled backtrace,
whose `status()` is `Disabled`, which shows as `disabled backtrace` and
`<disabled>`; set, a captured one with frames. `force_capture()` captures
whatever the environment says; `disabled()` is disabled. JS has no such
environment in a browser, and a stack of its own, `new Error().stack`.

## Decision

A `Backtrace` is `undefined`, disabled, or a string, JS's stack:

- `Backtrace::capture()` and `Backtrace::disabled()` are `undefined`.
- `Backtrace::force_capture()` is `$backtrace()`, JS's own stack, its
  frames after the error's first line.
- `b.status()` is `$backtraceStatus(b)`: `"Disabled"` of `undefined`,
  else `"Captured"`, a `BacktraceStatus` being its variant's name as
  std's enums are, compared with `===`.
- `{}` of one is `$backtraceShown(b, false)`, `disabled backtrace` or the
  frames; `{:?}` is `$backtraceShown(b, true)`, `<disabled>` or the
  frames.

`std_types/backtrace.rs` owns the operations and what they lower to;
recognition names them; `runtime/backtrace.js` the three helpers.

## Why

- **It's what Rust does where nothing asks.** With the environment unset,
  as a program's users run it, `capture()` is disabled; rust-js makes it so
  always, since a page has no `RUST_BACKTRACE` to read. The program sees
  what native Rust shows by default (ADR 0262's second case: the
  difference is an environment variable, and listed here).
- **Forced is honest.** `force_capture()` asks for frames whatever the
  environment says; JS's stack is the frames it has, the best answer a
  page can give, and `Captured` says it was.
- **A string and `undefined`** are JavaScript's building blocks for it: no
  object, no class, the shape a person would write.

## Alternatives

- **Capture JS's stack in `capture()` too.** It would show frames where
  native Rust shows `disabled backtrace` by default, and cost a stack
  capture on every error anyhow makes, as Rust avoids unless asked.
- **Refuse `Backtrace`.** It refuses anyhow, and every crate that keeps
  one, for a value a page can only ever show.
- **Read `RUST_BACKTRACE` from `process.env`** where there is one. Two
  answers for one program depending on where it runs, which ADR 0262
  refuses; a page has no `process`.

## Consequences

- anyhow compiles past it, and stops next at `Display::fmt` called as a
  function, `Display::fmt(&self.0, f)`.
- `{:?}` of a forced backtrace shows JS's frames, not Rust's `Backtrace [{
  fn: .. }]`: the frames a page has, listed here as a difference.
- `BacktraceStatus::Unsupported` is never given.

## What holds it

- `test/corpus/backtrace.rs`: an error's captured backtrace, its status,
  its `Display` and `Debug`, a disabled one's, and a forced one's status
  and non-empty text, against native Rust run with `RUST_BACKTRACE`
  unset, as the suite runs it.
- Mutations: `backtrace-captured` (`capture()` captures), `backtrace-status-captured`
  (every status `Captured`), `backtrace-shown-raw` (a disabled one shows
  as nothing).
