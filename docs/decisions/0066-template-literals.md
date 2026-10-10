# 0066. Text with values in it is a template literal

Status: Accepted. Changes [0034](0034-strings-and-chars.md); extends [0065](0065-format-with-oxfmt.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`format!`, `write!`, `{:?}` and `panic!` build their string from text and
values joined with `+` (ADR 0034):

```js
name + ": " + String(n) + " item" + (n === 1 ? "" : "s")
```

It's correct, but it isn't what a person writes today, and once formatted
(ADR 0065), a long one is laid out one piece per line.

## Decision

**Strings joined of text and values are a template literal:**

```js
`${name}: ${n} item${n === 1 ? "" : "s"}`
```

- **All in one place.** Every string these build is joined by one function,
  `join`, which decides:

  | Pieces | JS |
  |---|---|
  | text only | a string: `"a dot"` |
  | text and values | a template: `` `Some(${x})` `` |
  | values only | `a + b`, or the value alone |

- **`String(x)` is `x` in `${}`**: a template makes each value a string as
  `String` does, so `${n}` is `String(n)`.
- **Joins inside a join are taken apart**, so a template doesn't nest one
  that it could hold: `` `Some((${a}, ${b}))` ``. A value that's a choice
  keeps its own: `` `${x == null ? "None" : `Some(${x})`}` ``.
- **The text is escaped as a template needs:** `` ` ``, `\` and `${`,
  and controls as in a string (`\n` stays `\n`, not a line break).
- `{:?}` of a constant `Option` is known: `Some(1.0)` is `"Some(1.0)"`, not a
  test of `1 == null`.

## Why

- **It's how JS is written now**, and it reads as the Rust template does:
  `"{name}: {n} item"` and `` `${name}: ${n} item` `` line up.
- **Formatters leave a template on one line**, as a person would, rather
  than a chain of `+` broken one piece per line.

## Alternatives

- **`+` everywhere** (as before): no nesting, but noisy, with a `String(..)`
  around each number.
- **A template even for values alone** (`` `${a}${b}` ``): uniform, but
  `a + b` of two strings is plainer.

## Since

- **A string literal written across lines is a template literal with its
  line breaks**, `r#"<!DOCTYPE html>` and the lines after it, as
  react.dev's Sandpack template writes a file's code: the same text,
  written as the Rust is. One written with `\n` keeps it, and so does one
  whose break is left out by a `\` ending the line. A `` ` ``, `\` or
  `${` in it is escaped. Case N: it's the same string.
- **So is a format string written across lines**, `format!`, `println!`
  and the rest: the template its value is in, with its line breaks, as
  react.dev's DownloadButton writes the page it downloads. rustc keeps no
  span of the format string's own, so it's the macro call's first string
  literal, by Rust's lexer. One written with `\n`, or whose break a `\`
  leaves out, keeps it. Case N: it's the same string.

## Amendment: `String(x)` the program wrote stays

`String(n)` of a Rust number is `n` in a template, as before. `String(x)`
the program wrote, `js::string(arg)` of any value, stays `${String(arg)}`:
a template throws on a symbol, which `String` names, as react.dev's
Console names what a page logged. It was `${arg}`. A number's text is its
own node, which only it unwraps; a bindings test shows a symbol, and
mutations unwrap any `String`, and keep a number's `fmt` an arrow.
- **A `const` of a string written across lines is a template literal**
  (2026-10-10), `const PAGE = \`<main>..\``, as one in place was, where it was
  a string of `\n`s: its value is rustc's, whose text the literal it's
  written as says how to write. react.dev's SandpackWithHTMLOutput writes
  its sandbox's files so; the playground's own test runner's text is laid
  out anew. A lowering test has one of a raw string and one of `\n`s,
  which stays a string; mutations write each either way.
