# 0341. A byte slice's `escape_ascii` and `utf8_chunks`

Status: Accepted. Extends [0327](0327-cell-deque-char-methods.md) and
[0172](0172-utf8-decoding.md); counted by [0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A byte slice's `escape_ascii()`, which shows bytes as ASCII text, and
`utf8_chunks()`, which cuts them into runs of valid UTF-8 and the bad
bytes after each, were errors.

## Decision

**`escape_ascii()` is its text, as a `char`'s escapes are (ADR 0327);
`utf8_chunks()` is an array of `{ valid, invalid }`, cut where std's
`Utf8Chunks` cuts, by the check `from_utf8` makes (ADR 0172).**

```rust
println!("{}", bytes.escape_ascii());
for chunk in mixed.utf8_chunks() {
    println!("{:?} {:?}", chunk.valid(), chunk.invalid());
}
```

```js
console.log(`${$escapeAscii(bytes)}`);
for (const chunk of $utf8Chunks(mixed)) {
  console.log(`${$debugStr(chunk.valid)} [${chunk.invalid.map((item) => String(item)).join(", ")}]`);
}
```

- **Each byte as `u8::escape_ascii` escapes it**: `\t`, `\r`, `\n`, `\'`,
  `\"` and `\\` by name, a printable one as itself, any other as `\x` and
  two lowercase hex digits.
- **A sequence the bytes end inside of is one bad run**, as std's is, and
  any other bad sequence is as many bytes as `from_utf8`'s `error_len`.
- **Still refused**: iterating `escape_ascii()`'s bytes, as its text isn't
  them, and `{:?}` of a `Utf8Chunk`.

## Why

- **It's the same program**: the same text, and the same runs.
- **It's tested**: the `slice_bytes_text` corpus case runs, against
  native Rust, `escape_ascii` of each named escape, `\x7f`, `\x00` and
  `\xff`, as `to_string()`, of nothing; and `utf8_chunks` of a bad byte, a
  sequence cut short in the middle and at the end, collected and of
  nothing. Mutations write a tab in hex, print `\x7f`, leave hex
  unpadded, drop a cut-short end, cut each bad byte alone, and refuse
  `{}`, the loop and `valid()`.
- `docs/std-coverage.txt`: `slice` 122 of 133.
