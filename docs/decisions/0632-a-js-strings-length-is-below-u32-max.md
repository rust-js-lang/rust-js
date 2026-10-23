# 0632. A JS string's `length` is below `u32::MAX`, and a counter below it doesn't wrap

Status: Accepted. Extends [0576](0576-what-a-bodys-integers-can-be.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): the program can't
observe it: no engine holds a string as long as `2^32 - 1` units.

## Context

React's `normalizeConsoleFormat` steps through its format string with
`for (let i = 0; i < formatString.length - 1; i++)`. Its port, `i: u32`
below `js::string::length(format_string)`, the js crate's binding of a
string's `length`, read

```js
while ((i + 1) >>> 0 < formatString.length) {
  ...
  i = (i + 1) >>> 0;
```

The integer ranges (ADR 0576) knew a slice's `len()`, but not a JS
string's `length`, any `u32` to them; and of `i + 1 < n` they learned
nothing of `i`, and of a counter that kept growing round a loop, nothing
at all. So the test's `i + 1` and each step kept their wrap, though
none can wrap.

## Decision

Three facts the ranges know, each where they decide it:

- **A JS string's `length` is at most `2^32 - 2`.** The spec allows
  `2^53 - 1`, but no engine holds a longer string than `2^31 - 1` units:
  V8's are below 2^29, SpiderMonkey's at most 2^30 - 2, JavaScriptCore's
  2^31 - 1. Which call reads one is the binding's to say: a `get length`
  of a string `this`, `bindings.rs`'s `reads_string_length`, the js
  crate's `string::length` and any program's own binding of the same
  shape. It reads without changing anything, so a test of it narrows
  what it's compared with, as a slice's `len()` does. An array's
  `length` may be `2^32 - 1` by the spec, a sparse one's, and a typed
  array's more, so no other `get length` is bounded: they are any `u32`,
  as before.
- **`i + k op n` narrows `i` to `op n - k`**, where `k` is a literal and
  `i + k` can't have wrapped: `i`'s range with `k` added stays in its
  type. Where it may have, `u32::MAX + 1` is 0, below any `n`, and
  nothing is learned.
- **A counter still growing round a loop is taken first as at most one
  below its type's most**, where its `+ 1` can't wrap, and only when it
  grows past that, as anything. This is a widening with one threshold:
  the walk still goes round until what's known holds, so what's found
  holds of every turn.

So `while i + 1 < js::string::length(s)` and the steps below it are
`while (i + 1 < s.length)` and `i++`, and a step of one below a slice's
length, `while i + 1 < xs.len() { .. i += 1; }`, is `i + 1 <
xs.length` and `i++` too.

## Why

- **Generated code is a product**: the loop reads as upstream's does.
- **The program can't observe it** (ADR 0262, its first case): a string
  of `2^32 - 1` units can't be made in any engine, so a `u32` counter
  below one can't reach a wrap.
- **One place answers each question**: what a binding reads is the
  binding decoder's, as its `JsForm` is; the ranges ask it, in the test
  of what's pure and in what a call gives, and keep no list of names.

## Alternatives

- **A list of the js crate's length bindings in the ranges**: a second
  place that knows what a binding is, and a program's own binding of a
  string's `length` would be left out.
- **Every JS `length` below `2^32 - 1`**: an array's can be `2^32 - 1`,
  so a counter two steps below one could wrap, and a typed array's is
  more.
- **The string's bound alone**: `i + 1 < length(s)` showed nothing of
  `i`, and a growing counter was forgotten at once, so the bound changed
  nothing.

## Consequences

- No corpus program, snapshot or guide example changed: none tests
  `i + k` below a length. `docs/readability.txt` doesn't move.

## What holds it

- `test/lowering-numbers.test.ts`, "a counter below a JS string's length
  doesn't wrap", run in JS, with an array's `get length` that keeps its
  wraps and an `i + 1` that may have wrapped.
- Mutations `string-length-any-u32`, `string-length-impure`,
  `sum-narrowed-wrapped`, `sum-narrowed-unshifted`,
  `growing-counter-forgotten`, `any-length-a-strings`.
