# 0180. Generic code that writes to any writer is given a `fmt::Write` dictionary

Status: Accepted. Extends [0166](0166-user-fmt-write.md),
[0148](0148-write-to-string.md), [0099](0099-mut-references.md) and
[0106](0106-generic-traits.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Code that formats into whatever it's given takes a `W: fmt::Write`, and
writes to it a `char`, a `&str` or a `write!` at a time:

```rust
fn write_two(w: &mut (impl Write + ?Sized), v: u8) -> fmt::Result {
    w.write_char((b'0' + v / 10) as char)?;
    w.write_char((b'0' + v % 10) as char)
}
```

chrono formats each date so, given a `String` or the `Formatter` its
`Display` has; bitflags' `to_writer` takes its writer by value, a `&mut
Formatter`. ADR 0166 gave the crate's own writers their text, but a
generic writer was an error, which stopped chrono, at 6 places, and
bitflags.

## Decision

**A `W: fmt::Write` is given a dictionary, after its arguments, as a
bound of the crate's own traits is, of `write_str`, `write_char` and
`write_fmt`; the writer is given in a box, as a generic `&mut T` is:**

| Writer | Given | Its dictionary |
|---|---|---|
| `&mut s`, a `String` | `{ value: s }`, taken back after | `$stringWriter`: what's written is added to `value` |
| `f`, the `Formatter` a `Display` has | `{ value: f }` of its text, taken back after | `$stringWriter` |
| a writer of the crate's own | itself, in a box | its impl's: its `write_str`, and std's `write_char` and `write_fmt` as that `write_str` |
| `f` or `&mut s` taken by value, `W = &mut ..` | as above | `$mutStringWriter`, through the box it's given in; a writer of the crate's, its own |

```js
function write_two(w, v, Write) {
  Write.write_char(w, String.fromCharCode((48 + ((v / 10) & 255)) & 255));
  return Write.write_char(w, String.fromCharCode((48 + (v % 10)) & 255));
}

function timeDisplay_fmt(time) {
  let f = "at ";
  const w = { value: f };
  write_time(w, time.hours, time.minutes, "", $stringWriter);
  f = w.value;
  f += ".";
  return f;
}
```

- **`write!(w, ..)` is one `write_fmt` of its text,** as ADR 0166's writers
  are given it in one `write_str`; that's ADR 0166's difference, in the
  semantics page's list, not a new one.
- **std's `write_char` and `write_fmt` write with `write_str`,** a `char`'s
  text or a `write!`'s, so a writer that keeps them has its `write_str` in
  their places, named once: `{ write_str, write_char: write_str, .. }`. One
  it writes itself is its own.
- **Where the writer's type is known, nothing changes:** `write!(s, ..)` is
  `s += ..` (ADR 0148), and a writer of the crate's is called as ADR 0166
  calls it. A dictionary is for generic code.

## Why

- **It's exact:** the `generic_writers` corpus case writes through generic
  helpers, a `char`, a `&str` and a `write!` at a time, to a `String`, to
  the `Formatter` a `Display` has, padded where it's shown, and to a writer
  of the crate's that counts what it's given; and through a writer taken by
  value, a `Formatter`, a `&mut String` and a `&mut` to the crate's writer,
  with native Rust.
- **It's what a generic `&mut T` already is:** the box, and its value
  taken back (ADR 0099), and the dictionary a bound of the crate's own
  traits is given (ADR 0106).

## Not yet

- **A generic writer's `&mut writer`,** of one taken by value, is a handle
  made at each call, `{ get value() { .. }, set value(v) { .. } }`, which a
  handle made once for the variable would make shorter.
