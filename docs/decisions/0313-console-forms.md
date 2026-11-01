# 0313. Loops, spreads and calls as a person writes them

Status: Accepted. Amended (2026-11-01): prepare's rule is the only one that leaves an argument out, lowering marking a lone one that's none. Amends the range `for` loop's end (ADR 0025) and
`chain` (ADR 0036). Amended by [0640](0640-a-rest-parameter-is-a-tuple.md):
`console` takes a tuple of its arguments.

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
  `setProgram(undefined)`. (Amended by ADR 0330: of rust-js's own calls
  only; one given to JS is kept, as it may count its arguments.)
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
  (Amended by ADR 0640: a tuple of any number, each its own type,
  `console::warn(("%s", x))`.)

## Amended (2026-11-01): the one rule

Two rules left out a trailing `undefined`: prepare's, after another
argument, of every call, and lowering's `calls::left_out`, of a `None` of
an `Option` parameter and a `()`, alone too (ADRs 0441, 0443), with
`binding_calls`' three loops of a binding's optional parameter, each
dropping them before prepare saw them. Most of what prepare leaves out no
type says: a writer's formatter `options` (103 of 263 in the corpus, the
lowering suites and the guide), a runtime helper's optional parameter,
`$assertFailed("Eq", a, b)` (39), a dictionary's drop function of none.

**One rule leaves an argument out, `js::undefined_arguments::left_out`**:
a trailing `Undefined` after another argument, and one alone where
lowering marks it no argument, `Known::Absent`. Prepare asks it of every
call; lowering of each call it makes of what it knows the types of, so
what reads the call before prepare sees the arguments its JS has.
Lowering decides nothing else of it; it says of each trailing `undefined`
what it is (`calls.rs`): of a parameter that may be left out, a `None` of
a binding's optional one or of a function value's or the crate's own
function's `Option`, or a `()` (`absent`), `Absent`; given to JS
otherwise, `GivenUndefined`, which stays (ADR 0330, `given_to_js_but`);
of the crate's own call, else as it is (`own_absent`). What prints is as
before.

The user first decided (2026-11-01) that a lone trailing `undefined` of
rust-js's own call be left out too, `alone()` and `setProgram()`, as case
N ([0262](0262-when-rust-and-js-disagree.md)): rust-js's own functions
never read `arguments.length`, and a default parameter takes the two
alike. Built so, the corpus's `as_deref_rc_in_place` panicked in Node
where native Rust printed: lowering writes JS's own methods of a std
call with a plain `undefined`, `fiberStack.push(undefined)` of
`vec.push(None)`, which alone would be `push()`, pushing nothing. A lone
`undefined` is a value where what's called counts its arguments: third
case, results change. **The user's decision of 2026-11-01, after A was
found to change results: "alone is a value" stands**, `alone(undefined)`.

And after another argument, the rule was already wrong of such a method
(a wrong answer, found by the probe this amendment made):
`v.insert(1, None)` was `v.splice(1, 0)`, inserting nothing, and
`v.extend([Some(2), None])` `v.push(2)`. An item a std call gives JS's own
method is now `GivenUndefined` (`Expr::given_item`): `v.splice(1, 0,
undefined)`, `v.push(2, undefined)`. The corpus's `trailing_none_items`
compares `push`, `insert`, `extend`, `extend_from_slice`, `splice`,
`resize`, `append`, a `VecDeque`'s, `concat`, `chain`, a map's and a set's
of `None`s with native Rust (ADR 0330, amended).

Held by `test/lowering-loops.test.ts`'s "Console's loops, spreads and
calls read as written", `shown(2) + alone(undefined)`, the corpus's
`option_arguments`, `unit_arguments`, `as_deref_rc_in_place` and
`trailing_none_items` against native Rust, and the mutations
`trailing-undefined-kept`, `trailing-undefined-alone` and
`absent-alone-kept` (`js/undefined_arguments.ts`), `unit-argument-given`,
`given-absent-kept` and `given-absent-of-any` (`lower/calls.ts`),
`own-call-none-given`, `own-closure-none-given` and `fn-value-none-given`
(`lower/mir/calls.ts`), `pointer-arguments-given` (`lower/mir/dispatch.ts`),
`static-none-given`, `binding-trailing-none-given`,
`rest-of-none-keeps-none` and `bound-none-kept` (`lower/binding_calls.ts`),
`pushed-none-left-out` (`lower/combinators.ts`) and
`inserted-none-left-out` (`lower/combinators/adapters.ts`).
