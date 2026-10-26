# 0946. An `if` of `&&`, `||` and `!` mixed is its one test

Status: Accepted. Extends [0613](0613-and-or-mixed-are-returned-as-written.md),
a value of `&&` and `||` mixed returned as one, and
[0790](0790-a-break-the-program-wrote-is-where-it-is-written.md)'s `or_blocks`,
an `if` of `!a || b`, to any `if` whose condition MIR made a tree of
`if`s that leave a block; [0879](0879-an-if-and-a-closures-captures-read-as-written.md)'s
negations as written then write it as the program did.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The test runs each
part where the tree ran it, in the same order, none twice, and gives
`true` on exactly the paths that went on.

## Context

React's `popHydrationState`:

```js
if (
  tag !== HostRoot &&
  tag !== HostSingleton &&
  !(
    tag === HostComponent &&
    (!shouldDeleteUnhydratedTailInstances(fiber.type) ||
      shouldSetTextContent(fiber.type, fiber.memoizedProps))
  )
) {
```

Its port writes the same condition in Rust. MIR makes it branches: each
way it's false jumps to where the `if` ends, which the JS wrote as a
block MIR made and `if`s that leave it, `bb42: { if (..) { if (tag ===
HostComponent) { if (..) { if (..) { break bb42; } } else { break bb42; }
} .. } }`, a label upstream hasn't.

## Decision

**A block MIR made, `l: { if (p) { tree; t } }` or `l: { tree; t }`,
whose `tree` is `if`s of tests of a logical the program wrote, each
branch an `if` of the same or a `break l`, is `if (p && test) { t }`**
(`logicals::skipped_tests`, in tidy's `chains` stage): `test` is where
`tree` goes on past its end, each `if (c) { x } else { y }` of it `c &&
x` where `y` leaves, `!c || x` where `y` goes on, and the `if`s one after
another joined with `&&`. `written_negations` then writes it as the
program wrote it, `!(tag === HostComponent && (!a || b))`.

Not where a path's value is `c ? x : y`, which `&&` and `||` don't write;
nor where `t` breaks the block too.

## Why

- **It's the program's condition**, as written, where JS writes one.
- **Each part runs as it did**: `&&` and `||` short-circuit as the
  branches did, so the calls are made in the same order, each once, on
  the same paths.

## Consequences

- React's `popHydrationState`'s two conditions read as upstream's, no
  `bb42:` or `bb69:`.
- An `else` that tests, `if (c) { break l; } else { if (d) { .. } }`, is
  `if (c) { break l; } if (d) ..` before this runs, so the tree has none.

## What holds it

- `test/lowering-written-branches.test.ts`, "an if of && || and ! mixed
  is its one test", each value of each part, and the order of the calls.
- Mutations: `skipped-tests-*`.
