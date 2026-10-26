# 0945. A sort by UTF-16 units is JS's `sort()`

Status: Accepted, the user's decision of 2026-10-26 that the port writes
the Rust of JS's order and rust-js prints JS's `sort()`. Extends
[0183](0183-code-point-order-and-trim.md), whose code-point order stays
`sort()`'s, `sort($cmp)`, and [0342](0342-unstable-sort.md), whose
unstable sort stays every other comparator's.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: the comparator is
the order JS's `sort()` sorts strings by, so its result is the same, and
of an unstable sort too, whose ties are equal strings, the same string,
which no order of them tells apart.

## Context

React's setToSortedString sorts names with JS's `sort()`, which orders
strings by their UTF-16 code units:

```js
return array.sort().join(', ');
```

Rust's `sort()` of strings orders them by code point, which rust-js keeps,
`array.sort($cmp)` (ADR 0183). The two differ where two strings first
differ at a character past U+FFFF, two units from U+D800, and one from
U+E000 to U+FFFF: JS puts `"b😀"` before `"b\u{E000}"`, Rust after. The
port must keep JS's order, which Rust writes:

```rust
array.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
```

rust-js printed it as a comparator stepping both strings' units,
`array.sort((a, b) => $iterCmp(Iterator.from($encodeUtf16(a)),
Iterator.from($encodeUtf16(b)), $cmp))`.

## Decision

**A `sort_by`, or a `sort_unstable_by`, of strings whose closure is
`|a, b| a.encode_utf16().cmp(b.encode_utf16())` is JS's `sort()`**, given
no function:

```rust
names.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
```

```js
names.sort();
```

- **The closure is read as Rust wrote it**, its THIR
  (`body_queries::orders_by_utf16_units`): a call of `Iterator::cmp` of
  `str::EncodeUtf16` (`recognition::compares_utf16_units`), its receiver
  the first parameter's `encode_utf16()`, its argument the second's,
  each through its borrows and derefs, a `&String`'s `Deref` too. Any
  other closure, `b` before `a`, a function of the two units, is a
  comparator as before.
- **Of strings**: `&str`, `String`, as their items are JS strings, which
  JS's `sort()` compares as they are; not `char`s.
- **A `sort_unstable_by` of it is `sort()` too**: strings equal by their
  units are the same string, so Rust's unstable order of ties, which
  `$sortUnstable` keeps for any other comparator (ADR 0342), can't be
  told from JS's stable one.
- **`sort()` stays code points**, `sort($cmp)`, Rust's meaning (ADR 0183).

## Why

- **It's what the program says**: Rust's order of the strings' UTF-16
  units, which is exactly JS's `sort()`'s, so the JS upstream wrote is
  the JS of the Rust that means it.
- **Idiomatic Rust**: `encode_utf16()` and `Iterator::cmp`, std's, no
  rust-js API for an order std can say.

## Alternatives

- **A `js` crate function, `js::sort(&mut names)`**: a rust-js
  substitute for an order std says.
- **Rust's `sort()` made JS's**: a wrong answer where strings differ past
  U+FFFF.

## What holds it

- `test/corpus/utf16_sort.rs`: strings that differ above U+FFFF and from
  U+E000 to U+FFFF, by `sort()`, by `sort_by` of `&str`s and of
  `String`s and by `sort_unstable_by`, run natively and as JS.
- `test/lowering-strings.test.ts`, "only a sort by the first's units then
  the second's is JS's sort()": descending, and by a function of the
  units, stay comparators, run as JS. Not corpus cases: their comparators
  are the `$iterCmp` this decision removes elsewhere, which the
  readability gate counts.
- `scripts/matrix.ts`'s strings: each pair sorted by its units.
- Mutations `utf16-order-of-either-parameter`, `utf16-order-of-any-cmp`
  (`body_queries.rs`), `utf16-sort-by-kept` (`std_types/vec.rs`) and
  `utf16-sort-unstable-kept` (`std_types/slice.rs`).
