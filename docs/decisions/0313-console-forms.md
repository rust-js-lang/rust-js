# 0313. Loops, spreads and calls as a person writes them

Status: Accepted. Amends the range `for` loop's end (ADR 0025) and
`chain` (ADR 0036).

Case: A ([0262](0262-when-rust-and-js-disagree.md)), but for a dropped
trailing `undefined`, B: only `arguments.length` tells.

## Context

Ported, react.dev's Console read unlike its original in five places:

```js
if (escaped == null || !escaped) ..        // if (!escaped)
const end = args.length;                   // i < args.length
for (let i = 0; i < end; i++) ..$index(args, i)  // args[i]
prev.concat(newLogs)                       // [...prev, ...newLogs]
listen((message) => { .. }, undefined)     // listen((message) => { .. })
```

And webapi had no `console`, whose `console.warn` it calls.

## Decision

- **`x == null || !x` is `!x`**: `null` and `undefined` are falsy.
- **A range's end of a length nothing in the loop's body changes,
  `xs.len()`, is `i < xs.length`**, as is a change before the loop, a
  closure's too: Rust's borrows let nothing outside the body change `xs`
  while the loop reads it, read each time round as
  JS's is; one the body may change is still a `const`, as Rust reads it
  once. A `VecDeque`'s length and index are a `Vec`'s, so `args[i]` in it
  is in bounds.
- **`a.chain(b).collect()` is `[...a, ...b]`**, an array written out its
  items, `[header, ...lines]`.
- **A trailing `undefined` after another argument is no argument**, `f(x)`,
  as a person leaves an optional one out. One given alone is a value,
  `setProgram(undefined)`.
- **webapi binds the `console` namespace** (WHATWG Console), as it does
  `WebAssembly`.

## Why

- **It's the JavaScript a person writes**: each is react.dev's.
- **It's the same program**: each reads the same values, but a dropped
  `undefined`, which a function can tell only by `arguments.length`, rare
  and harmless.
- **It's tested**: a lowering test runs each; mutations keep each
  form, read a changed length again each time round, and drop a lone
  `undefined`.

## Consequences

- A JS function that counts its arguments sees one fewer where a trailing
  `None` is left out.
- webapi's `console` takes one value per call: WebIDL's `any... data` is
  one generic argument, as the generator binds a variadic `any` today.
