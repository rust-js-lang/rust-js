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

## Amendment: a let chain's binding between its tests (2026-10-28)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, each value read once
where Rust reads it, as below.

React's attachSuspenseRetryListeners writes `if (inProgressLanes !== null
&& inProgressRoot !== null) { .. } else { throw .. }`; its port writes the
let chain `if let Some(lanes) = IN_PROGRESS_LANES.get() && let Some(root)
= IN_PROGRESS_ROOT.with_borrow(Clone::clone) { .. } else { panic!(..) }`
(cm5 of the CommitWork port's gaps). MIR binds the second `let`'s value
between the tests, `if (lanes == null) { break l; } const root$1 =
inProgressRoot; if (root$1 == null) { break l; }`, so the tree of tests
this decision folds ended at the binding, and the chain stayed a labeled
block, its `else` after it.

**A `const` between the tests of a variable's read or of a path of
properties from one, `const x = a.b;`, before another test, is the first
of the statements the tests guard, and each test of it a test of what it
reads, `inProgressRoot == null`** (`logicals::skipped`), where the tests
call nothing: the read, of what no test changes, reads the same where it
was and after the tests. The chain is then `if (lanes != null &&
inProgressRoot != null) { .. }`, whose `else` `written_elses` writes as
the program did.

Held by `test/corpus/let_chain_else_of_reads.rs`, against native Rust;
mutation `chain-binding-between-tests-kept` (`mir/tidy/logicals.ts`).

## Amendment: a let chain with an `else` inside an `if` (2026-10-28)

React's attachSuspenseRetryListeners tests its let chain inside `if
(enableX && ..)`, and goes on after that `if`. MIR's block for the chain's
`else` is then the whole `if`'s: `bb23: { if (enabled && present) { if
(lanes != null) { if (root != null) { s; break bb23; } } throw .. } }`,
which `branch_values`' `&&` chain (`or_chain`) didn't see, as the block
isn't the chain alone, and the label stayed.

**A block MIR made whose one statement is an `if` of no `else`, of such
`if`s down to an `&&` chain to the block's end, is those `if`s, the chain
one `if` of its tests and its `else`:** `if (enabled && present) { if
(lanes != null && root != null) { s } else { throw .. } }` (the pass
"chains in blocks", `tidy/branches.rs`, after the passes that name the
chain's temporaries and join its `if`s). A `break` of the block from the
chain's end goes where the `if`s end, the block's end, so nothing moves.

A let chain's binding between its tests, `const r = root;` of a variable
or a path of properties from one, goes into the chain's body, each test
of it a test of what it reads, `if (a != null && root != null) { const r
= root; .. }`: read only once each test before it holds, as it was, and
the same value, as no test calls anything; where one does, `&& swap()`,
the chain is left as it was.

Case N of [ADR 0262](0262-when-rust-and-js-disagree.md): the same tests in
the same order, and the same statements under them.

Tested by the corpus case `let_chain_else_inside_if`; mutations
`chain-in-blocks-unfolded`, `sunk-binding-untested`,
`chain-bindings-sunk-past-calls` (`tidy/branches.ts`).

## Amendment: only the chain's tests are asked whether they call

The port's attachSuspenseRetryListeners kept `bb27`: its chain's body
calls `restorePendingUpdaters(inProgressRoot, inProgressLanes)`, and
`chained` asked the whole `if`, body and all, whether anything in it
calls, so it kept the bindings before the chain's tests and left the
label. **What the chain's tests and the bindings between them call is
asked, not its body**, which runs once they've held, after each binding
is read either way. Held by `let_chain_else_inside_if`'s `restore`, whose
body calls now (native-compared), and the mutation
`chain-body-calls-counted`.
