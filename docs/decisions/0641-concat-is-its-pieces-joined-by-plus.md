# 0641. A `concat!` of literals is its pieces joined by `+`

Status: Accepted. Amends [0066](0066-template-literals.md), whose string
written across lines was the one form a literal kept of its source.

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
- **A format string of a `concat!` with a placeholder** is the template
  it was, `` `n is ${n}` ``: `format_args!` reads the one string rustc
  made, and its pieces are the template's text.
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
