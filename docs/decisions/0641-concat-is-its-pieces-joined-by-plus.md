# 0641. A `concat!` of literals is its pieces joined by `+`

Status: Accepted. Amends [0066](0066-template-literals.md), whose string
written across lines was the one form a literal kept of its source.
Amended: a format string of a `concat!` is its pieces too; and a comment
between its pieces is before the piece it's written before.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe: `"a " + "b"` is the string `"a b"` is.

## Context

React splits a long string over lines as a `+` of literals, as prettier
lays it out:

```js
export const OMITTED_PROP_ERROR =
  'This object has been omitted by React in the console log ' +
  'to avoid sending too much data from the server. Try logging smaller ' +
  'or more specific objects.';
```

and so is nearly every `console.error('..' + '..', a, b)` message it
writes. The Rust a port writes is std's way of splitting a string over
lines, `concat!`:

```rust
pub const OMITTED_PROP_ERROR: &str = concat!(
    "This object has been omitted by React in the console log ",
    "to avoid sending too much data from the server. Try logging smaller ",
    "or more specific objects."
);
```

rustc expands `concat!` to one string literal, and rust-js wrote it as
one, a line of 150 characters where React has three.

## Decision

**A `concat!` of literals is its pieces joined by `+`, as written** (the
user's decision, 2026-10-23): each argument its own JS string, in order.

- **Every argument is its own piece**, a string's text, and a `char`'s,
  an integer's, a float's or a `bool`'s as `concat!` writes it:
  `concat!("v", 2, ".", 1)` is `"v" + "2" + "." + "1"`. One rule for each
  kind, the one a reader guesses from the Rust: a number isn't a JS
  number, which `+` would add, `1 + 2 + "x"` is `"3x"`, nor written as JS
  writes it, `1.50` is `"1.50"` in Rust; and it isn't merged into a
  neighbour, which would need a rule of which.
- **A piece written across lines is a template literal of them**, as one
  literal is (ADR 0066): `` `<a>\n</a>` + "!" ``. A `\` ending a line
  continues one literal, one piece, as it's one in Rust.
- **Everywhere a string literal is lowered**: a `const`'s or a
  `static`'s value, a call's argument, `console.error("a " + "b")`, a
  `let`, a return, a pattern, a JSX child or prop, `{"a " + "b"}`, and
  `panic!(concat!(..))`'s message, `throw new Error("a " + "b")`.
- **A format string of a `concat!` is its pieces too**, each a template
  where it holds a placeholder (amended below; it was the one template).
- **A `concat!` of anything but literals is the one string it is**:
  `concat!("a", stringify!(b))`, `concat!(env!("V"), "x")` and a nested
  `concat!` are expanded by rustc first, and their pieces aren't in the
  call's source; so is one a `macro_rules!` writes of its `$` arguments.

The JS tree's string written otherwise than as one literal on one line is
one kind, `Written`, of its pieces, `StrPiece { text, lines }`: the
`Lines` of a string across lines is a `Written` of one piece. `Expr::
written` makes a single piece on one line a `Str`, so a string has one
form for each way it's written. `src/lower/literals.rs`'s
`string_literal` is the one answer of what pieces a literal is written as,
asked by THIR's literals, MIR's constants and a `const`'s or `static`'s
value (`const_string`), where three functions in `lower.rs` answered
whether one was written across lines.

rustc reports no pieces: the literal's span is the macro call's, with
`concat!`'s expansion as its context. `recognition::from_concat` knows
std's `concat!` by its diagnostic item, `macro_concat`, not its name,
which a crate's macro may have; `literals.rs` reads the call's arguments
by Rust's lexer and what rustc makes of each token,
`LitKind::from_token_lit`, and keeps the pieces only where their text,
joined, is the literal rustc made: anything else is the one string.

## Why

- **The output reads as the original does**: React's three lines are
  three lines, laid out by prettier's rule for a `+` chain, broken after
  each `+` where it's too long.
- **The value is rustc's**: the pieces are checked against it, so a
  reading of a token that disagrees with `concat!`'s can only lose the
  pieces, never change the string.
- **A literal is still a constant** (`is_constant`): moved, inlined,
  compared and given to JSX as the one literal was.

## Alternatives

- **A `Binary` `+` of `Str`s in the JS tree**: every pass that asks
  whether a value is a constant would see an operator, and a temporary,
  a JSX child's or a pattern's rules would change with it.
- **Non-string arguments merged into a neighbouring string**,
  `"v2" + ".1"`: which neighbour is a rule to learn, and the output no
  longer follows the Rust's arguments.
- **A `concat!` the port writes as a `\`-continued literal instead**: one
  JS string, which isn't React's, and a dialect where `concat!` is std's
  way.

## Consequences

- No corpus program or snapshot writes `concat!`: none changes.
- A `const`'s struct of a `concat!` field, `Point { name: concat!(..) }`,
  is rustc's value, whose text is one string: no span of the field's
  literal is kept.
- A `static` of a string written across lines is a template literal now,
  as a `const`'s is.

## What holds it

- `test/lowering-strings.test.ts`, "a concat! of literals is its pieces
  joined by +": React's constant, a call's argument, each kind of
  argument, a `\` continuation, a piece across lines, a trailing comma, a
  `stringify!`, a `let`, a `static` and a `panic!`, each run in Node for
  its value.
- `test/jsx-children.test.ts`, "a concat! child or prop is its pieces
  joined by +", rendered.
- Mutations in `scripts/mutations/lower/literals.ts`,
  `scripts/mutations/lower/mir/operands.ts`,
  `scripts/mutations/lower/recognition/std_items.ts` and
  `scripts/mutations/to_oxc/expressions.ts`.
- The guide's [Basics: strings](../guide/for-js-developers/basics.md#strings-and-template-literals).

## Amendment: a format string of a `concat!`

React's `formatProdErrorMessage` ends its message as prettier lays out a
`+` of a template and two strings:

```js
  return (
    `Minified React error #${code}; visit ${url} for the full message or ` +
    'use the non-minified dev environment for full errors and additional ' +
    'helpful warnings.'
  );
```

The port writes std's way, `format!(concat!("..{}..{}.. ", "..", ".."),
code, url)`, which was one template of 160 characters.

**A format string of a `concat!` of literals is its pieces joined by `+`**
(the user's decision, 2026-10-23), wherever `format_args!` is:
`format!`, `print!`, `println!`, `write!`, `panic!`, `assert!`'s message.
Each piece is a template literal where it holds a placeholder, and
otherwise a string, `` "n is " + `${n}` + "!" ``; one written across lines is a
template literal with its line breaks, as a literal's piece is. Of
positional, named and spec'd arguments alike, `{1}`, `{x:.1$}`, `{:>8}`:
each is formatted as it is and put in its piece.

- **One template where a piece holds part of a placeholder or an
  escape**: `concat!` joins first and `format_args!` parses the string
  it made, so `concat!("{", "}")` is a placeholder only the whole has,
  and `concat!("a{", "{b")` a `{{`. So is one whose template rustc wrote
  otherwise than the source has it: a literal argument written in,
  `format!(concat!("v ", "{}{}"), "lit", n)`, or a nested `format_args!`
  flattened into it.
- **The template is rustc's**: rustc keeps no span of the format string,
  so `literals.rs`'s `format_string` reads the call's source, its first
  string literal or `concat!` of literals, and `format_args.rs`'s
  `written_pieces` splits rustc's template by the pieces' text and
  placeholders only where they're what the template is, piece for piece;
  anything else is the one template. A reading that's wrong can lose the
  pieces, never change the string. `println!`'s line break, which it adds
  after the pieces, is the last piece's, there and in a `println!` of a
  `concat!` with nothing to show, `console.log("a " + "b")`.
- **The JS tree's template says how its source breaks it**:
  `Template(texts, values, TemplateLayout)`, the layout the `lines` of
  ADR 0066 and the `TemplateBreak`s, each a text and an offset in it, as
  a value is never broken. Every pass sees one template, as it is: its
  values are evaluated in the order they were, and a `+` of strings is
  the template's string. `Expr::template_pieces` makes it of the pieces;
  of no values left, rustc's literal folded in, its pieces are a
  `Written`'s, a constant, `"n + 1 is " + "4"`. Tidy's splice of a
  value's template into the one it's in moves the breaks with the texts.
  The printer writes each piece, `+` between.
- **No source spans of their own**: a piece's span is the macro call's,
  as a literal's pieces' are; rustc's spans are of the call.

What holds it: `test/lowering-strings.test.ts`, "a format string of a
concat! is its pieces joined by +", React's message as upstream writes it,
each kind of argument, each case of one template, a `println!`, a
`write!`, a `Display`'s `write!` and a `panic!`, each run in Node; the
corpus's `format_concat_pieces.rs`, against native Rust; mutations in
`scripts/mutations/lower/format_args.ts`, `lower/literals.ts`,
`lower/display.ts`, `lower/mir/std_calls.ts`, `lower/mir/tidy/values.ts`,
`js/make.ts` and `to_oxc/expressions.ts`.

## Amendment: a comment between the pieces

React's `lazy()` splits `import` in its message with a comment of why:

```js
console.error(
  'lazy: Expected the result of a dynamic imp' +
    'ort() call. ' +
    'Instead received: %s\n\nYour code should look like: \n  ' +
    // Break up imports to avoid accidentally parsing them as dependencies.
    'const MyComponent = lazy(() => imp' +
    "ort('./MyComponent'))",
  moduleObject,
);
```

Its port writes the comment between `concat!`'s arguments, and the
pieces had no spans: the comment had nothing to be before but the next
argument, `moduleObject`, or, of a `const`, nothing at all. **Each piece
of a `concat!` of a literal's, `string_literal`'s and `const_string`'s, is
where its argument is written, `StrPiece::span`; a comment before one,
past the first, is before it, on lines of its own after the `+`**, as a
conditional's branch's is (ADR 0658, `to_oxc::arm_starts`, its line
split after the `+` before it). A format string's pieces keep the call's
span, as before: rustc's template is split by their text, and none has a
span of its own.

What holds it: `test/lowering-strings.test.ts`, "a comment inside a
concat! is before the piece it's written before", a `const`'s and a
call's argument's; mutations `concat-piece-unplaced`
(`lower/literals.rs`), `concat-piece-no-start` and `concat-split-not-at-plus`
(`to_oxc.rs`).
