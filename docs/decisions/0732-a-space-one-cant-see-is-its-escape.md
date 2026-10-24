# 0732. A space one can't tell from `' '`, or a mark one can't see, is its escape

Status: Accepted.

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
