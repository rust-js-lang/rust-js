# 0598. A temporary is no part of a named variable's pattern

Status: Accepted. Amends tidy's destructuring
([0293](0293-cells-read-at-once.md)). Amended (2026-10-26): a binding the
program names of a variable rust-js named, `old` of `take()`, names both.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: one pattern or two
reads of the same properties, in the same order.

## Context

React's scheduler sets up its channel:

```js
const channel = new MessageChannel();
const port = channel.port2;
channel.port1.onmessage = performWorkUntilDeadline;
```

Its Rust, `let port = channel.port2(); channel.port1().set_onmessage(..)`,
became

```js
const { port2: port, port1: _12 } = channel;
_12.onmessage = ..
```

Two parts of a variable the program names, read in a row, are one
pattern, `let (a, b) = t;`'s. But the second was MIR's temporary for
`channel.port1()`, which the program never named, and would have been read
in place.

## Decision

**Of a variable the program names, a pattern takes the parts it names**:
a temporary among them ends the run, and is read where it's used,
`channel.port1.onmessage = ..`. A temporary's own parts, `const [a, b] =
f();`, are as before.

## What holds it

- `test/lowering.test.ts`, "a temporary is no part of a named variable's
  pattern".
- Mutation: a temporary's part kept in the pattern.

## Amendment: a `let` of a name alone is its own statement, and a cast's is the variable

React's `mapIntoArray` (ReactChildren.js) reads a lazy component's parts
of `children` as it is, `const payload = (children as any)._payload;
const init = (children as any)._init;`. Its Rust,

```rust
let lazy: &LazyComponent = unsafe { js::cast(children) };
let payload = &lazy.payload;
let init = lazy.init;
```

was `const lazy = children; const { _payload: payload, _init: init } =
lazy;`: two parts read in a row were one pattern, though the program
wrote two `let`s, and the cast a name JS has no use for.

**A pattern takes the parts its program's pattern names**: a `let` of a
name alone, `let x = p.x;`, ends the run, a statement the program wrote as
one, `const x = p.x;` (`LetNames::named`, of THIR's `let`s). MIR's
temporary of a part, as `channel.port1()`'s, followed only a `let`'s
name, so that rule is this one now. A `let`, a parameter's or an arm's
pattern is one destructuring, as before. **A `let` of a variable cast**,
`js::cast`'s or any binding that's its argument (`JsForm::This`,
`LetNames::casts`), **is that variable** where nothing reads it after and
nothing after sets either: `children._payload`, as TypeScript reads it;
else it keeps the name the program gave. Case N: the same reads, in the
same order.

- `test/lowering-parts.test.ts`, "a let of a part is its own statement,
  and a cast's variable is the variable", run.
- Mutations `named-let-part-of-a-pattern` (`tidy/destructuring.rs`),
  `cast-variable-read-after` (`tidy/bindings.rs`), `cast-let-named`
  (`mir/patterns.rs`).

## Amendment: a binding the program names of a variable rust-js named

React's ReactFiberLane, `markRootFinished`, takes each lane's hidden
updates:

```js
const hiddenUpdatesForLane = hiddenUpdates[index];
if (hiddenUpdatesForLane !== null) {
  hiddenUpdates[index] = null;
```

Its port's `if let Some(hidden_updates_for_lane) =
hidden_updates[index].take()` printed `const old = hiddenUpdates[index];
hiddenUpdates[index] = undefined; if (old) { for (const update of old)`:
`take()` holds what it gives in a variable rust-js names `old`, the
binding is a copy of it, `const hiddenUpdatesForLane = old;`, and tidy's
`copied bindings` read the copy as the variable, by the variable's name.

**Where the variable is rust-js's, no local the program names (`Named::
user`), declared in the statements the `if` is in, and the binding's name
is the program's and nowhere else in them, both are the binding's**:
`const hiddenUpdatesForLane = hiddenUpdates[index]; .. if
(hiddenUpdatesForLane) { for (const update of hiddenUpdatesForLane)`
(`bindings.rs`'s `named_for_bindings`). One variable renamed in its own
scope, to a name nothing else there has: case N, nothing that runs changes.

Test: `lowering-shadows`'s "a binding of what take() gives names it", run.
Mutation `taken-binding-named-old` (`lower/mir/tidy/bindings.ts`).
