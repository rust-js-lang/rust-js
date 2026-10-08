# 0187. A writer's `Err(fmt::Error)` is thrown, with what it wrote, and each consumer takes it as std's does

Status: Accepted. Amends [0054](0054-display.md), [0148](0148-write-to-string.md)
and [0166](0166-user-fmt-write.md); extends [0100](0100-separate-crates.md).

Case: C, D, B ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A writer returns the string it writes (ADR 0054), and a `fmt::Result` is
nothing: writing to a string can't fail, so `Err(fmt::Error)` was an error,
and `is_ok()` was `true` (ADR 0148). chrono's formatting returns one, for
a format it can't write, and stopped there, at four places.

In Rust the error goes up through each `?` to what consumes the text, and
what was written before it stays written. Each consumer takes it as its
own:

| Consumer | What Rust does |
|---|---|
| `write!(s, ..)` to a `String` | gives the `Err` back; `s` holds what was written before it |
| `to_string()` | panics, `a Display implementation returned an error unexpectedly: Error` |
| `format!` | panics, `a formatting trait implementation returned an error when the underlying stream did not: Error` |
| `print!`, `println!` | prints what was written before it, then panics, the same without `: Error` |

## Decision

**`Err(fmt::Error)` is thrown, a `$FmtError`, and a writer it passes out of
puts what it wrote in front of what the error carries. Each consumer catches
it and does what std's does.** `Ok` is `undefined`, as it was.

```js
function spanDisplay_fmt(span) {
  let f = "";
  try {
    f += hoursDisplay_fmt(span[0]);
    f += "-";
    f += hoursDisplay_fmt(span[1]);
  } catch (error) {
    throw $fmtWritten(error, f);
  }
  return f;
}
```

- **A `fmt::Result` as a value is `undefined`, or the error caught:**
  `$fmtTry(() => show(s, h))`; `?` of one passes it on, `is_ok()` is
  `r === undefined`, and `{:?}` is `Ok(())` or `Err(Error)`.
- **A write of what may fail is piece by piece,** `t += "a"; t += x;`, so
  what came before is written; a `write!` to a `String` takes what the
  failing writer wrote, `t += $fmtPartial(error)`.
- **Only what may fail is written so.** A crate's functions that may return
  one are found as its drops are: one that makes one, or calls or formats
  one that may, of the crate's or a library's, which its manifest says. A
  generic `T`'s may where any of the crate's or its libraries' may. A crate
  none of whose do is lowered as before, `is_ok()` `true`.
- **A writer of the crate's that fails is still an error:** it's given each
  `write!`'s text whole (ADR 0166), where Rust gives it a piece at a time,
  so it would fail where Rust's doesn't.

## Why

- **It's exact:** corpus cases compare a `write!` to a `String` that fails,
  its text and its result, a function's `?` of one, and `to_string()`,
  `format!` and `println!` of one, with native Rust; a library's writer
  that fails, given to a consumer's `write!`, too.
- **A crate with no `fmt::Error` is the JS it was:** the corpus's JS is
  unchanged but where a case makes one.

## Alternatives

- **One panic for every consumer:** less code, but `to_string()`'s and
  `println!`'s messages are std's own, and a `write!` to a `String` gives
  the error back, without a panic.
- **A writer writing into its consumer's buffer,** as Rust's does: what it
  wrote would be there without being handed up, but every writer's JS would
  take a buffer, where it returns a string (ADR 0054).

## Costs

- **A library's generic code given the crate's type that fails:** compiled
  without the crate, it doesn't catch the error, which is then `format!`'s
  panic, whichever consumer it was.
- **What may fail is written with `try`s,** a writer's and its consumers',
  where it was a template.
