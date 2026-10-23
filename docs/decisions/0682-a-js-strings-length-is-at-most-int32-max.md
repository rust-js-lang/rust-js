# 0682. A JS string's `length` is at most `2^31 - 1`, so two add without a wrap

Status: Accepted. Amends [0632](0632-a-js-strings-length-is-below-u32-max.md),
whose bound it lowers from `2^32 - 2`, and extends
[0576](0576-what-a-bodys-integers-can-be.md)'s ranges.

Case ([0262](0262-when-rust-and-js-disagree.md)): 1, the program can't
observe it. No engine makes a string longer than `2^31 - 1` units, so a
Rust program compiled by rust-js can't hold one, and a sum it can't reach
can't wrap.

## Context

React's `shared/ReactSerializationErrors.js`, ported, writes

```js
} else if (substr.length < 10 && str.length + substr.length < 40) {
```

Its port reads each `length` with `js::string::length`, a `u32` (ADR
0247), and rust-js wrote

```js
} else if (substr.length < 10 && (str.length + substr.length) >>> 0 < 40) {
```

ADR 0011's wrap of a `u32`'s `+`, since the ranges (ADR 0576) took a JS
string's `length` as at most `2^32 - 2` (ADR 0632), and two of those can
pass `u32::MAX`. But no engine's string is that long:

| Engine | Most units | Where it says so |
| --- | --- | --- |
| V8 (Chrome, Node, Deno) | `2^29 - 24`, `2^28 - 16` on 32 bits | [`String::kMaxLength`, `include/v8-primitive.h`](https://github.com/v8/v8/blob/main/include/v8-primitive.h) |
| SpiderMonkey (Firefox) | `2^30 - 2` | [`JS::MaxStringLength`, `js/public/String.h`](https://searchfox.org/mozilla-central/source/js/public/String.h) |
| JavaScriptCore (Safari, Bun) | `2^31 - 1`, `int32_t`'s most | [`StringImplShape::MaxLength`, `Source/WTF/wtf/text/StringImpl.h`](https://github.com/WebKit/WebKit/blob/main/Source/WTF/wtf/text/StringImpl.h) |

MDN's [`String: length`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/length)
lists the same three. Making a longer one throws a `RangeError`
(`"a".repeat(2 ** 29 - 23)` in V8), so a string past the most never
exists to be measured.

## Decision

**A JS string's `length` is in `[0, 2^31 - 1]`** (`ranges.rs`'s
`STRING_LENGTH_MOST`), the largest of every engine's. Which call reads one
is still the binding decoder's, `bindings.rs`'s `reads_string_length`
(ADR 0632): the js crate's `string::length` and a program's own `get
length` of a string `this`.

So the sum of two lengths is at most `2^32 - 2`, within `u32`, and is JS's
own `+`:

```js
substr.length < 10 && str.length + substr.length < 40
a.length + b.length
```

What can still wrap keeps its wrap, as ADR 0576's ranges carry it:

- **A third length** may pass `u32::MAX`: `a.length + b.length` is exact,
  and the `+ c.length` of it wraps, `(a.length + b.length + c.length) >>>
  0`.
- **A subtraction**, `length - 1`, is `u32::MAX` of an empty string in
  release Rust (ADR 0011), so `(s.length - 1) >>> 0`, unless what comes
  before shows the string isn't empty.

**A counter widened round a loop keeps what it was widened to** where a
later round finds it within it. ADR 0632 took a growing counter first as
at most one below its type's most; a counter below a string's length, now
`2^31 - 1`, is found smaller than that the next round, and a range that
shrank was forgotten, so `i + 1 < length(s)`'s counter lost what it had.
What a round finds within the head's range is the head's: the head still
holds, and the walk settles.

## Why

- **The JS reads as upstream's**: `str.length + substr.length < 40`, as
  React wrote it.
- **The program can't observe it**: a wrap of `length(a) + length(b)`
  needs a string no engine makes. Rust's own `u32` sum of two such
  lengths is the same number JS's `+` gives.
- **The largest engine's, not the smallest's**: V8's `2^29 - 24` would let
  more sums go unwrapped, but a program run in Safari or Bun can hold a
  string of `2^31 - 1` units, and three of those pass `u32::MAX`.

## Alternatives

- **The spec's `2^53 - 1`**: no engine reaches it, and every `+` of a
  length would keep its wrap. Rejected for what engines do.
- **V8's `2^29 - 24`**: up to eight lengths would add unwrapped, but in
  JavaScriptCore three strings of `2^31 - 1` units each can be made, and
  their `u32` sum wraps. Rejected: results would change.
- **Drop the wrap of any sum of lengths, however many**: three may pass
  `u32::MAX`. Rejected.

## Consequences

- The corpus's `string_length_sum` is the only JS that changes: no other
  corpus program or snapshot adds two string lengths. The guide's
  basics page shows `fits`, ReactSerializationErrors' test.
  `docs/readability.txt` counts the new case's 36 lines and no new shape.
- A JS engine that one day makes a string longer than `2^31 - 1` units
  would be one this bound is wrong for; each of the three keeps its
  length in a 32-bit signed field, so that's a change to its string
  representation.

## What holds it

- `test/lowering-numbers.test.ts`, "two JS string lengths add without a
  wrap", run in JS: ReactSerializationErrors' test, a sum of two, a sum of
  three that keeps its wrap, and a `- 1` of an empty string that wraps;
  and "a counter below a JS string's length doesn't wrap" for the
  widened counter.
- `test/corpus/string_length_sum.rs`, a program's own `get length`
  binding, counting UTF-16 units natively, run in both.
- Mutations `string-length-past-int32` (the most `2^31`, so two may wrap),
  `string-length-below-engines` (`2^30 - 1`, so three don't wrap) and
  `widened-counter-within-forgotten`.

## Amendment: an array of sized items is shorter still (ADR 0701)

A `Vec` or a slice of items that take up room is at most `isize::MAX`
items long, as no Rust allocation is larger in bytes, so its `len()` is
at most `2^31 - 1` too, and `len() as isize - 1` is `length - 1`
([0701](0701-a-signed-index-tested-not-negative-is-a-js-index.md)). One of
items of no size may still be `u32::MAX` long.
