# 0971. An escape is the array of what it gives

Status: Accepted. Amends [0327](0327-cell-deque-char-methods.md) (a text
escape was its text) and [0341](0341-byte-slice-text.md) (a byte slice's
`escape_ascii()` was its text); extends [0558](0558-a-bytes-escape-is-its-bytes.md)
and [0070](0070-function-values.md)'s case mapping to every escape. Fixes
#379 and a wrong answer found with it. Amends [0939](0939-an-iterable-s-into-iter-is-its-js-iterator.md):
its `IntoIterator` dictionary's text-escape arm is gone.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. A held
escape stepped by `next()` threw, and a byte slice's `escape_ascii()`
stepped `char`s where Rust steps `u8`s.

## Context

An escape had three representations:

- a `char`'s or a `str`'s `escape_default()`, `escape_debug()` and
  `escape_unicode()`: its text, a string;
- a byte slice's `escape_ascii()`: its text too;
- a byte's `escape_ascii()`: the array of its bytes, `$escapeByte(b)`, as a
  case mapping, `c.to_uppercase()`, is the array of its `char`s.

A string is an iterable but no iterator, so six places wrapped a text
escape in JS's iterator where it was stepped: the `for`'s `into_iter`,
`flat_map`'s inner, a std call's result, the `IntoIterator` dictionary,
MIR's source and its display. Each place was one more that could be
missed, and two were:

- `let mut e = 'é'.escape_default(); e.next()` was `$next` of the string,
  `TypeError: it.next is not a function` (#379), and `{}` of one stepped
  showed all of its text, not what was left.
- `b"a\n".escape_ascii()` stepped the escape's `char`s: `collect::<Vec<u8>>()`
  gave `["a", "\\", "n"]`, and `.map(u32::from).sum()` gave 0.

A string iterator also can't be shown without being stepped, so a held
escape had no `Display` that leaves it as it was.

## Decision

An escape is the array of what it gives, made where it's made:

- a text escape, its `char`s, `Array.from($escapeDefault(c))`, as a case
  mapping's;
- a byte slice's `escape_ascii()`, its bytes, `$escapeAscii(bytes)`, and a
  byte's, `$escapeAscii([b])`: one helper, which gives bytes.

All three, and the case mapping, are recognized where an iterator over an
array made where it's made is (`iterates_items`). So each is what the array
iterators already are: `$iter` of the array held, stepped from either end,
its `len()` the items left, its `{}` the text of what's left (`$restStr`,
`$restBytes`). Every escape-specific arm of stepping goes.

The array made of a string's `char`s carries `Known::Chars`. Tidy folds
`Array.from(text).join("")` back to the text by that fact, not by the
names `toUpperCase` and `toLowerCase` it read before. A text escape's
helpers only read what they're given and make a value, so they're `PURE`.

## Consequences

`'é'.escape_default().collect::<String>()` is `$escapeDefault("é")`, where
it was `Iterator.from($escapeDefault("é")).toArray().join("")`.
`.count()` is `Array.from(..).length`. A held case mapping's or escape's
`len()` and `next_back()` compile; they were refused.

Shown without being stepped, a byte slice's escape reads
`$restBytes($iter($escapeAscii(bytes)))`, where it read
`$escapeAscii(bytes)`. Three escapes held for one `println!` are each
named, as a held case mapping is, because `$iter` isn't `PURE`: it makes a
mutable iterator.

`test/corpus/escapes_stepped.rs` checks held escapes, a byte slice's bytes
and a case mapping's length against native Rust. The mutations are:

- `text-escape-not-chars`
- `escape-ascii-text`
- `escape-ascii-not-bytes`
- `chars-joined-unfolded`

The five that targeted the removed arms are gone with them.
