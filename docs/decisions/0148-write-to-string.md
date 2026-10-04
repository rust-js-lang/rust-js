# 0148. `write!` into a `String` adds to it, and a `fmt::Result` is always `Ok`

Status: Accepted. Extends [0054](0054-display.md).

## Context

`write!` and `writeln!` into a `String`, through `std::fmt::Write`, are how
Rust builds text a piece at a time:

```rust
use std::fmt::Write;

let mut out = String::new();
for (name, n) in items {
    writeln!(out, "{name:<6}{n:>3}").unwrap();
}
```

Each was an error, `write_fmt` and the `fmt::Result`'s `unwrap()` both.
ADR 0054 made a `fmt::Result` nothing in JS, as a `Formatter`'s writes
can't fail, and gave it no methods.

## Decision

**`write!(s, ..)` into a `String` is `s += ..`, as `push_str` is, and a
`fmt::Result`'s `unwrap()`, `expect()`, `is_ok()` and `is_err()` are what
an `Ok` gives:**

```js
let out = "";
for (const item of items) {
  out += `${$pad(item[0], 6, "<")}${String(item[1]).padStart(3)}\n`;
}
```

| Rust, `s` a `String` | JS |
|---|---|
| `write!(s, "..")`, `writeln!(s, "..")` | `s += \`..\``, `s += \`..\n\`` |
| `s.write_str(t)`, `s.write_char(c)` | `s += t`, `s += c` |
| of a `&mut String` given to a function | `out.value += ..`, its box (ADR 0074) |
| `.unwrap()`, `.expect(..)` of the `fmt::Result` | nothing: `()` |
| `.is_ok()`, `.is_err()` | `true`, `false` |
| `?` of it | nothing, as in a `fmt` (ADR 0054) |

- **Writing to a string can't fail:** std's `impl fmt::Write for String`
  always returns `Ok`, so a `fmt::Result` of one is `Ok`, and nothing.
- **The write runs:** `write!(t, "!").is_ok()` is `t += "!"`, then `true`.

## Why

- **It's exact:** std's `String` writes can't fail, so each method's answer
  is the one Rust's gives. The `write_to_string` corpus case compares
  `writeln!` with options, `write!` through `&mut`, `?` in a function
  returning `fmt::Result`, `write_str`, `write_char`, `let _ =` and
  `is_ok()` with native Rust.
- **It's the JS a person writes:** `out += ..`.

## Consequences

- `write!` into a `String`, and a `fmt::Result`'s `unwrap`, `expect`,
  `is_ok` and `is_err`, compile.
- A user's `impl fmt::Write` is still an error, and so are a `fmt::Result`'s
  other methods, `map_err`, and a `match` of one. (Amended: a user's
  `impl fmt::Write` is given its text a `write!` at a time, ADR 0166.)
