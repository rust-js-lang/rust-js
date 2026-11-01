# 0842. A let chain after a test of its own reads its place once

Status: Accepted. Amends tidy's `and tests`, which
[0672](0672-a-property-a-test-reads-is-read-once.md) amended for a getter's
read. Amended below: a binding read before a test that runs code stays
before it; and a chain whose `if`s `settle branches` made is joined.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: the nested `if`s
are the JS MIR made of the chain, its tests in the order written and each
short-circuiting as Rust's do; only which of two equal forms is printed
changes.

## Context

React's `lazy()` reads `ioInfo.value` once where `__DEV__` holds:

```js
if (__DEV__) {
  const ioInfo = payload._ioInfo;
  if (ioInfo != null) {
    ..
  }
}
```

Its port writes the let chain clippy's `collapsible_if` asks for:

```rust
if debug && let Some(value) = info.value.get() {
    js::set(value, "status", "fulfilled");
}
```

Tidy's `and tests` joined MIR's `if (debug) { const value = info.value;
if (value != null) { .. } }` into one `&&`, the binding's read in the
test, `bound_into`'s, and the `const` kept for the body:

```js
if (debug && info.value != null) {
  const value = info.value;
  value.status = "fulfilled";
}
```

`info.value` is a Rust struct's field, which reads the same again (ADR
0672), so the answer is right; but the place is read twice where Rust
reads it once and React writes it once. A test that reads it twice,
`let Some(count) = counts.count.get() && count > 3`, was `debug &&
counts.count != null && counts.count > 3`, the same.

## Decision

**A chain isn't joined where joining reads a binding's place twice and
its first test is a test of its own.** One function answers it, tidy's
`logicals::read_twice_apart`, which `and_tests` asks before it joins:

- **twice**: the second test reads the `const`, and it, or what follows
  the `const`s, the body or a later binding, reads it again, so the join
  would write its value at each, or keep the `const` and read the place in
  the test too;
- **a test of its own**: the bound value reads no variable the first test
  reads, `info.value` of `debug`. Such a chain stays MIR's, nested:

```js
if (debug) {
  const value = info.value;
  if (value != null) {
    value.status = "fulfilled";
  }
}
```

A chain that takes apart what its first test tested is joined as before:
`examples/options.rs`'s `chained_statement`, `let Some((a, b)) = if h > 0
{ .. } else { None } && a < 10`, binds `const a = value[0]` of the
`value` its first test tests, and reads `if (value && value[0] < 10) {
const a = value[0]; const b = value[1]; .. }`, as its snapshot has it,
the test and the binding of one value. A binding only
the test reads once is read in place, its `const` gone, as before: one
read either way.

## Why

- **One read, as Rust's**: the chain reads the place once; React's JS,
  which a person writes, does too.
- **The JS MIR made**: nothing is moved; the `if`s stay as they were, so
  evaluation order and short-circuiting are Rust's by construction.
- **Idiomatic Rust in**: clippy's `collapsible_if` makes the let chain
  the only Rust a port may write; the compiler gives the JS upstream wrote.

## Alternatives

- **Every chain that would read a place twice**: `chained_statement`
  would nest too, `if (value) { const a = value[0]; const b = value[1];
  if (a < 10) { .. } }`, two levels where the joined test reads as the
  chain does; its first test is of the value the binding takes apart.
- **Reading the place into a `const` before the chain**, `const value =
  info.value; if (debug && value != null)`: reads it where `debug` is false,
  which Rust doesn't, and a getter's read would then run where Rust's
  doesn't.

## Consequences

- The corpus, the snapshots and the guide's examples don't change.
- A place a pass before `and tests` already reads in place at each use,
  `node.next` of `if debug && let Some(b) = &node.next { b.n + b.n }`, a
  plain read MIR held that `inlined` writes where it's read, is joined as
  before, `if (debug && node.next != null) { return node.next.n +
  node.next.n; }`: there's no `const` left to keep. That's the inlining's
  rule for Rust's data, ADR 0672's, not the chain's.

## What holds it

- `test/lowering-written-branches.test.ts`, "a let chain after a test of
  its own reads its place once": `settle`'s and `many`'s nested `if`s, run.
- The mutations `chain-place-read-twice`, `chain-tests-read-place-twice`
  and `chain-taken-apart-nested`, the last held by the `options` snapshot.

## Amendment: a binding read before a test that runs code stays before it

Case ([0262](0262-when-rust-and-js-disagree.md)): results change; Rust's
are kept. A wrong answer, fixed.

`if let Some(fiber) = fiber && let Some(task) = fiber.task.get() &&
probe(fiber) { return task.len(); }` printed `if (fiber && fiber.task !=
null && probe(fiber)) { const task = fiber.task; return $byteLen(task);
}`: joining MIR's `if`s moved the binding's `const` past `probe(fiber)`,
which clears the field, so `task` was `undefined` where Rust's is what was
read before, and the JS threw. **A chain whose next test runs code isn't
joined where a binding before it is read after it, by the body or a later
binding** (`logicals::read_past_code`): `if (fiber) { const task =
fiber.task; if (task != null && probe(fiber)) { .. } }`, the binding
read where Rust reads it. One the test reads alone, in its place, goes as
before; one past a test that does nothing, `task.len() > min`, is joined
as before, nothing between able to change what it read.

Held by `let_chain_binding_past_a_call.rs` against native Rust, and the
mutation `chain-binding-read-past-code` (`tidy/logicals.ts`).

## Amendment: a chain whose `if`s `settle branches` made is joined (2026-11-02)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Joining nested `if`s
with no `else` into one `&&` runs the same tests in the same order, each
short-circuiting.

React Fizz's `use(usable)` is `if let Some(usable) = usable &&
js::type_of(Some(usable)) == "object" { .. }`. Upstream writes it
`usable !== null && typeof usable === 'object'`, one test. It printed
`if (usable != null) { if (typeof usable === "object") { .. } }`. When an
arm makes a value with a drop, such as `let context: Rc<Context> = ..;
return read(&context);`, MIR leaves the chain as guard `break`s out of a
block. `settle branches` (the `chains` stage) turns those into the nested
`if`s, but only after `and tests` has already run, and no second `chains`
run follows in a function with no drops left to unwind. **`and tests` runs
again right after `settle branches`** (`and tests settled`). It is the same
rule with the same checks, so a chain the first run left apart, such as
one that would read a place twice, stays apart. No other corpus case's JS
changed. The playground's result frame did change. Its `match
message_data(e) { Some(report) if from_frame && report.run == Some(run) =>
report, _ => return }` was two nested `if`s; it is now one guard, `if
(report == null || !fromFrame || report.run !== run) { return; }`, as its
`_ => return` is written. Its two `frame` bindings, now in one block,
became a `let` read again, which runs the same.

Held by the corpus case `let_chain_settled_and` against native Rust, and
the mutation `and-tests-settled-skipped` (`lower/mir/tidy/stages.ts`).
