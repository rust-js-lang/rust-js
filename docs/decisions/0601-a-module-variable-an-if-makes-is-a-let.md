# 0601. A module's variable an `if` makes is a `let` each branch assigns

Status: Accepted. Extends [0270](0270-module-variables.md)'s
thread-locals. Amended by [0657](0657-an-if-of-one-value-each-is-a-conditional.md):
an `if` of one value each, a function too, is a `const` of a conditional;
and by [0870](0870-a-let-chains-value-reads-its-binding-first.md): a
closure of `const`s and then its value is the module's `const`s.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same value, made
once when the module loads.

## Context

React's scheduler picks how it schedules work when it loads:

```js
let schedulePerformWorkUntilDeadline;
if (typeof localSetImmediate === 'function') {
  schedulePerformWorkUntilDeadline = () => { .. };
} else if (typeof MessageChannel !== 'undefined') {
  const channel = new MessageChannel();
  ..
} else {
  ..
}
```

Its Rust is a thread-local an `if` makes. A value made of statements is a
closure called where it's made, so it was

```js
const schedulePerformWorkUntilDeadline = (() => {
  if (typeof setImmediate !== "undefined") {
    return () => { .. };
  }
  ..
})();
```

## Decision

**A module's `const` a closure called where it's made makes, not
exported, is `let name;` and the closure's body**, each `return v` at its
ends `name = v`, an early `return`'s `if` the rest's `else`. What follows
stays as JS writes it: `let x;` and what sets it are one block, no blank
line between.

It stays the call where the body declares a name at its top, which would
be the module's, and where a `return` isn't at its ends, a loop's.

The variable is a `let`, which nothing else assigns: Rust's thread-local
is set once, and JS has no `const` set in branches.

## What holds it

- `test/lowering.test.ts`, "a thread-local an if makes is a let each
  branch assigns", with one that declares at its top and one of a loop,
  which stay calls.
- Mutations: the pass off, a top declaration taken, the blank line kept.

## Amendment: `undefined` is no assignment (ADR 0704)

A `return undefined` at an end, an `Option`'s `None`, assigns nothing:
`let name;` is `undefined` already, so `else { name = undefined; }` is
gone, and a `then` left with nothing is the `else`'s under its test
negated ([0704](0704-a-module-variable-is-assigned-only-what-isnt-undefined.md)).

## Amendment: a closure of `const`s and its value is the module's (ADR 0870)

A closure whose body is `const`s and then the value it returns, `const at
= Math.random(); return at < 2 ? 1 : 2;`, a let chain's value read first
(ADR 0870), is those `const`s and the value, the module's: `const at =
Math.random(); const SEEN = at < 2 ? 1 : 2;`, each named apart from what
the module names elsewhere (`prepare::initializers::declared_at_top`).
A body that declares at its top and does more, `let at = ..; at *= 2;`,
stays the call.

## Amendment (2026-11-18): a body that makes a local and gives it (the vercel-ui SDK's port, G1)

A body that declares a name at its top stayed the call, as that name would
be the module's. But one whose only declaration at its top is the local it
gives at its end, its only `return`, is the module's statements, that local
the item's name (`made_and_given`):

```js
const regionDays = new Map();
for (const [day, regions] of [..]) {
  for (const region of regions.split(" ")) {
    regionDays.set(region, day);
  }
}
```

as the SDK writes the table its `thread_local!` `REGION_DAYS` holds,
`{ let mut region_days = HashMap::new(); for .. { .. } region_days }`,
where it printed `const regionDays = (() => { const regionDays = new
Map(); ..; return regionDays; })();`. Nothing else enters the module's
scope: what else it declares is its loops' and blocks' own. One that
declares more at its top, or gives a value early, whose `return` no
module's top may have, stays the call. ADR 0262's first case: the same
statements, at the same place, as the module loads (`lazy_locals`
decided it's made there).

Where it's done: preparation, as `assigned_in_place` already makes an
`Item::Statements` of such a `const`, after every reader of a module's
`Item::Const` by its name has read it: linking and the library's exports
(`library.rs`, `library/internal.rs`, `output.rs`, of exported ones,
which this never is), the class prototypes' order (`classes/prototypes.rs`)
and `plain_statics`' cells, all in lowering; what reads items after it,
preparation's own passes and printing, takes a `Statements` item as it
takes the one `assigned_in_place` makes.

Corpus case `thread_local_table_made_at_the_top`, against native: the
SDK's table, read by the module's function; another module's table, read
by its exported function, which the first module imports; one that gives
early and one that declares another name after its local, both still the
call. No other snapshot moves. Mutations `made-and-given-left-a-call`,
`made-and-given-past-an-early-return` and `made-and-given-of-more-names`,
caught.

