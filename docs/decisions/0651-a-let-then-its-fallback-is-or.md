# 0651. A `let`, then its fallback where it's falsy, is `||`

Status: Accepted. Extends [0635](0635-a-filter-by-truth-then-a-fallback-is-or.md)
and [0646](0646-a-held-chain-and-its-fallback-is-one-or.md), whose `||`
was a filter's and a held chain's, to a variable set again where its
test fails.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: no program can tell.

## Context

React's `DefaultPrepareStackTraceV8` falls back on `'Error'` where an
error has no name:

```js
const name = error.name || 'Error';
```

Its port writes it as Rust does, a variable set again where it's empty:

```rust
let mut name = error.name();
if name.is_empty() {
    name = "Error".to_string();
}
```

and rust-js wrote it as it's written, `let name = error.name; if (!name) {
name = 'Error'; }`. Its test is already the very `!name` JS reads a
string's truth by: rust-js writes `is_empty()` of text as `!s`, text's
emptiness its truthiness (ADR 0266), and a number's `== 0.0` as `f ===
0`, which `NaN` fails where `!f` holds.

## Decision

**A `let`, then an `if` of the `!x` of it whose one statement sets it, `if
(!x) { x = c; }`, is `const x = e || c`**: `||` gives `e` where it's
truthy, else makes `c`, as the `if` did. **And of the fallback where it's
none, `x ??= c`**, which preparation's `coalescing` makes of `if (x ==
null) { x = c; }`, **`const x = e ?? c`**. Preparation's `fallen_declared`
writes each, beside ADR 0646's `fallen_held`, after a block's tests are
read as tests.

- **Only `!x` of that variable**: the test is JS's own truth of the value,
  whatever made it; which Rust tests are written so is decided where tests
  are written, so a number's `f === 0`, a string's `name === "x"`, keep
  their `if`.
- **Not of a `c` that reads `x`**, `s = format!("{s}?")`, which the
  declaration would read before it's made.
- **A `const` where nothing sets `x` after**, by an assignment, in a
  closure too, or a `&mut`'s handle; else a `let`, `let label = s ||
  "none"; label += "!";`.
- `??` and `||` together are written with their parentheses, `(o ?? "")
  || "x"`, as JS requires.

## Why

- **No program can tell**: `e` is made once, first, then `c` only where `e`
  is falsy, or none, as the `if` made it, and the variable holds what it
  held after the `if`.
- **It's the JS React writes.**
- **One owner each**: whether a test is a value's truth is the test's
  writer's; that a declaration's fallback is `||` or `??`, preparation's,
  beside the other fallbacks it writes.

## Consequences

- None of the corpus's or the snapshots' JS changes; React's port reads as
  upstream.

## What holds it

- `test/lowering-options.test.ts`, "a let then its fallback where it's
  falsy is one ||": React's `prepareStackTrace`'s name and message, one
  set after, one whose fallback reads it, a number's test and a string's
  that aren't its truth, `??` and both together, each run of the values
  they tell apart, `NaN`, `0`, `""` and `undefined` among them.
- Mutations in `scripts/mutations/prepare.ts`: the `if` kept, a fallback
  that reads it, a `const` set after, any test, `??` as `||`.
