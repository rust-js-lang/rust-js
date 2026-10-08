# 0172. Bytes to text, as Rust validates UTF-8

Status: Accepted. Extends [0063](0063-text.md) and
[0138](0138-string-byte-counts.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Code that reads bytes makes text of them: a parser of a URL's
percent-escapes, a regex's literals, a protocol's frames.

```rust
match str::from_utf8(bytes) {
    Ok(text) => text.to_owned(),
    Err(e) => format!("bad at {}", e.valid_up_to()),
}
```

`str::from_utf8`, `String::from_utf8`, `from_utf8_unchecked` and
`String::from_utf8_lossy` were errors, which stopped regex-syntax,
percent-encoding, writeable and serde_core.

## Decision

**Bytes are checked as Rust checks them, and valid ones decoded by JS's
`TextDecoder`:**

| Rust | JS |
|---|---|
| `str::from_utf8(bytes)` | `$fromUtf8(bytes)`: `Ok` of the text, or `Err` of a `Utf8Error` |
| `String::from_utf8(bytes)` | `$fromUtf8(bytes, true)`: its `Err` a `FromUtf8Error` |
| `str::from_utf8_unchecked(bytes)` | `$utf8Decode(bytes)` |
| `String::from_utf8_lossy(bytes)` | `$utf8Lossy(bytes)`: a `Cow`, borrowed where they're valid, else owned |

- **`$utf8Check` is `core::str::validations`'s:** where the first bad
  sequence starts, and how many bytes long it is, by the second byte's range
  that rules out overlongs, surrogates and past U+10FFFF, or none where the
  bytes end inside one. That's `Utf8Error`'s `valid_up_to()` and
  `error_len()`, and its message.
- **A `Utf8Error` is `{ valid_up_to, error_len }`, a `FromUtf8Error`
  `{ bytes, error }`:** their methods read them, `utf8_error()`,
  `into_bytes()`, and their `Display` and derived `Debug` are std's, so
  `unwrap()` panics with Rust's message.
- **`from_utf8_lossy` replaces each bad sequence with one U+FFFD,** as
  Rust's `Utf8Chunks` does: `TextDecoder` replaces by the same maximal
  subpart, which a mutation that replaced only the first found.
- **A `Cow<str>` is `{ TAG, _0 }`, as an enum is (ADR 0033):** its text, by
  `Deref`, `into_owned()`, `{}` or `{:?}`, is `_0`, borrowed or owned.

## Why

- **It's exact:** the `utf8_decoding` corpus case compares valid text of
  every width, a stray byte, sequences cut short at each width, a bad
  continuation, a surrogate, an overlong, `F5` and past U+10FFFF, with
  their messages, `Debug`, `valid_up_to()` and `error_len()`;
  `String::from_utf8`'s error and its bytes; lossy text borrowed and owned;
  and `from_utf8_unchecked`, with native Rust; `utf8_unwrap_panic` the
  panic of `unwrap()`.
- **It's the JS a person writes:** `TextDecoder` for the text, and a check
  of its own where the answer is Rust's, not the decoder's.
