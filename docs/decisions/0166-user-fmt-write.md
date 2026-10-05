# 0166. A writer of the crate's is given its text a `write!` at a time

Status: Accepted. Amends [0148](0148-write-to-string.md).

## Context

A writer of the crate's own is how text is built where it goes somewhere
other than a `String`: anyhow's, writeable's and serde_core's (docs/
crate-corpus.md).

```rust
impl fmt::Write for Collector {
    fn write_str(&mut self, s: &str) -> fmt::Result { .. }
}
write!(collector, "{} and {:?}", 1, "two")?;
```

ADR 0148 refused it: Rust's `write!` calls `write_str` for each piece, a
literal's, each argument's, padding a `char` at a time, where rust-js
makes a `write!`'s text one string.

## Decision

**A writer of the crate's is given its text whole, one `write_str` for a
`write!`, and `write_char` its `char`, where it keeps std's.** Its text is
Rust's; how many calls carry it isn't. That's a difference from native
Rust, in the semantics page's list.

```js
collectorWrite_write_str(collector, `1 and ${$debugStr("two")}`);
```

- **What it writes itself is called as it is:** its own `write_char` or
  `write_fmt`.
- **A writer that fails is an error, saying so:** a `fmt::Result` is always
  `Ok` in JS (ADR 0054), so a `write_str` that returns `Err(fmt::Error)`
  is refused, rather than taken as `Ok`.
- **Still an error:** a generic `W: fmt::Write` given a `Formatter`, as
  bitflags' `to_writer` is. (Amended: generic code that writes to any
  writer is given a dictionary, [ADR 0180](0180-generic-writers.md).)

## Why

- **It's what such writers see:** they append or count what they're given,
  and the text is the same. A writer counting its calls is the one that
  could tell, and it's listed.
- **It's exact otherwise:** the `user_fmt_write` corpus case compares the
  text and its byte count, through `write!`, `writeln!`, a `Display` inside
  one, padding, `write_char` std's and the writer's own, with native Rust;
  a diagnostics test the refusal of one that fails.

## Alternatives

- **Keep it refused**, ADR 0148's choice: exact, but anyhow, writeable and
  serde_core stay stopped. Chosen against: the difference is in how text
  is handed over, not in what it is.
