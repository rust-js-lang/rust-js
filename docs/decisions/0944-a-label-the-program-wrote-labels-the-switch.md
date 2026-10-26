# 0944. A label the program wrote labels the `switch` an inner `switch` leaves

Status: Accepted. Amends [0742](0742-a-label-the-program-wrote-is-its-name.md),
whose name was the program's as written, `get_tag`: it's camelCase now;
and extends [0790](0790-a-break-the-program-wrote-is-where-it-is-written.md)'s
"Where a label is still needed" to the `switch` a `match` the program
labeled is.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A label, and which
statement it's on, is how the JS reads: each `break` goes where it went.

## Context

React's `createFiberFromTypeAndProps` labels its `switch`:

```js
getTag: switch (type) {
  case REACT_FRAGMENT_TYPE:
    return createFiberFromFragment(..);
  ..
  default: {
    if (typeof type === 'object' && type !== null) {
      switch (type.$$typeof) {
        case REACT_CONTEXT_TYPE:
          fiberTag = ContextProvider;
          break getTag;
        ..
      }
    }
    ..
    fiberTag = Throw;
  }
}
```

Its port writes the label Rust has, a labeled block around the `match`,
`'get_tag: { match .. { _ => { .. match .. { Some(REACT_CONTEXT_TYPE) =>
{ ..; break 'get_tag; } .. } .. } } }`. MIR has no labels, and placed the
block's end where the outer `match`'s arms meet: the JS read `bb63: {
switch ($$typeof) { case ..: ..; break; .. default: break bb63; } break; }`
inside `default:`, a label no one wrote, as the inner `switch`'s `break`
can't leave the outer one. ADR 0790 had kept a label only for a block
that ends no case; here a `break` that a plain `break` would leave
instead.

## Decision

The user's decision: **a program's own label is printed where upstream
writes it.** A block the program labeled around a `match` that's a
`switch`, a `break` of which is in a `switch` or a loop inside a case,
labels the `switch`: `getTag: switch (type) { .. break getTag; .. }`, by
its camelCase name, as every name is (ADR 0573). A `break` of it in the
case itself is the case's `break;` (ADR 0790).

MIR leaves the block in one of three places, each the same label
(`labeled_switches`, in tidy's `settled` stage, before a block's `break`
past a `switch` is the case's):

- around the `switch`, `getTag: { switch (..) { .. } } return tag;`, each
  case's end MIR's copy of the `return` after it, which is the case's end;
- around the inner `switch`, in a case, followed by a jump that leaves
  the outer `switch`: the case's `break;`, a `break` of a block the outer
  `switch` ends, or MIR's copy of the `return` after it. Then its `break`s
  leave the outer `switch`: the block's statements are in its place, the
  jump after them gone where they never go on to it, and the outer
  `switch` is labeled.

A label is printed on the `switch` it labels, `getTag: switch`, not on a
block around it. Labels stay refused wherever the program wrote none, a
block MIR made (`bbN`), and wherever a structured form exists: a label
nothing names once the case's own `break`s are plain is gone.

## Why

- **It's the JS React writes**, and the one JS has: a `break` in an inner
  `switch` leaves only that one.
- **It's the program's name**: the label a person wrote in Rust, as a
  loop's is (`loop`'s otherwise, ADR 0629).
- **camelCase, as every name**: `getTag`, not `get_tag`.

## Alternatives

- **A flag, `let done = false`, tested after the inner `switch`.**
  Rejected: a variable no one wrote, where JS has the label.
- **Keep MIR's block, named by the program, `getTag: { switch .. }`
  inside `default:`.** Rejected: not where the program's label is, nor
  React's.

## Consequences

- The port's `'get_tag` is upstream's `getTag: switch (type)` where its
  `match` is a `switch`. Where its guarded arms that fall to the next
  return a call, `return createFiberFromViewTransition(..)`, the `match`
  isn't one yet: MIR shares the arm's body, which only a `return` of one
  value is copied into each `break` of (ADR 0755); the port reads `bbN:`
  blocks there still.
- A label the program wrote of a loop or a block is camelCase too:
  React's `find_changed_dep` is `findChangedDep`.

## What holds it

- `test/lowering-labels.test.ts`, "a label the program wrote is the
  switch's whose inner switch leaves it", of each place MIR leaves the
  block.
- The guide's [`switch` and `match`](../guide/for-js-developers/basics.md#switch-and-match).
- Mutations: `labeled-switch-*`, `never-goes-on-past-a-label`,
  `written-label-snake-case`.
