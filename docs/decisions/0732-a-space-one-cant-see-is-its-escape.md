# 0732. A space one can't tell from `' '`, or a mark one can't see, is its escape

Status: Accepted. Amended below: an escape's hex is in lowercase, and
rust-js writes each escape of a string, a control's too.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
an escape is the code point it names.

## Context

React's ReactPerformanceTrackProperties writes

```js
const UNCHANGED = ' \xa0';
```

a figure space, then a no-break space. The port's `"\u{2007}\u{a0}"`
printed as `" \xA0"`: oxc escapes U+00A0, U+2028 and U+2029 and the
controls, and writes every other code point as it is, so the figure space
read as a plain one. Nothing in the JS said it wasn't.

## Decision

**In a string's or a template's JS, a space but `' '` and a line's end,
and a mark that takes no room, are written as their escapes**, ` `,
`\xA0` (`to_oxc::expressions::unseen`): each code point Rust's
`char::is_whitespace` gives, and U+00AD, U+061C, U+180E, U+200B, U+200E,
U+200F, U+202A–U+202E, U+2060–U+2064, U+2066–U+2069 and U+FEFF. Not what
joins or varies what's seen, U+200C, U+200D and the variation selectors,
which an emoji or a script is made of and which reads as it is.

A string's escape is written by oxc: given as a lone surrogate is,
U+FFFD and its four hex digits, which it writes as `\uXXXX`, a U+FFFD
itself as its own `fffd`. A template's is written with its text
(`template_raw`), `\x` for what's below U+0100, as oxc's `\xA0` is.

## Consequences

- The port's `UNCHANGED` reads `" \xA0"`, upstream's but for case.
- An app's zero-width space in a string, `"a\u{200b}b"`, reads as one.

## Amendment: an escape's hex is in lowercase, and rust-js writes each

React writes its escapes in lowercase, `' \xa0'` and `'\u200b'`, as JS
sources mostly do, and Prettier keeps an escape as it's written; rust-js
wrote them in uppercase, `"\u2007\xA0"`, oxc's spelling. The user decided
(2026-10-27, pt5(a)): **an escape's hex is in lowercase, everywhere**.

oxc writes U+00A0 as `\xA0` and a lone surrogate's escape as given, so a
string's escapes came half from rust-js (through oxc's lone-surrogate
marker) and half from oxc, which also wrote a control other than its own
few as it is: `"\x7f"` printed an invisible DEL, `""`. Now **a string
with a code point to escape, `unseen`'s or a control, is written as its
source by rust-js** (`string_raw`, given to oxc as source text, as a regex
literal is), and a template's text as before (`template_raw`); both spell
each escape with one rule, `escape`: JS's own, `\n`, `\r`, `\t`, `\b`,
`\v`, `\f`, `\u2028`, `\u2029`; a control, every one below U+00A0, `\x`
and its two digits, `\x00`, `\x7f`, not `\0`, which a digit after would
make an octal escape; and `unseen`'s, `\x` below U+0100, `\xa0`, `\xad`,
`\u` and four digits above, `\u2007`, `\ufeff`, each in lowercase. A
string with none goes to oxc as before. The formatter then picks the
quote with fewer escapes, as Prettier does, `'\u200b"q"'`.

Case N ([0262](0262-when-rust-and-js-disagree.md)): an escape is the code
point it names, however it's spelled. Held by
`test/lowering-strings.test.ts`'s "a space one can't tell from ' ' is
written as its escape", run, a quote, a backslash, a tab, a line's ends,
a control and U+2028 among them; mutations `escape-hex-uppercase`,
`string-raw-quote-unescaped` and `string-raw-backslash-unescaped`
(`to_oxc/expressions.ts`). The corpus' and the snapshots' strings are
re-blessed: their escapes in lowercase, a tab written `\t` where a string
has another escape, and controls seen, `"\x7f\x9f"`.
