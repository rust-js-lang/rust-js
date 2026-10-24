# 0841. A nullable place stores what's never `undefined` as it is

Status: Accepted. Amends [0791](0791-a-cells-storage-says-what-it-stores.md),
whose `Storage::written` kept as it is only a constant and what MIR's
`nulled` says; and [0781](0781-a-none-only-nullables-read-is-null-where-its-made.md),
whose variable only nullable fields read was of `Option`s made, now of a
call's too.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A nullable place's
`None` is `null` either way; what's written is what was, without a `??
null` that changes nothing.

## Context

React's port wrote a `?? null` of what can't be `undefined`:

```js
// createContext: a Some of an object made there, written through a handle
context.Consumer = {$$typeof: REACT_CONSUMER_TYPE, _context: context} ?? null;
// lazy: a map's None, undefined, then coalesced
debugTask: (createTask ? createTask('lazy()') : undefined) ?? null,
```

Upstream writes `context.Consumer = { .. }` and `debugTask:
console.createTask ? console.createTask('lazy()') : null`.
`Storage::written(made, nulled)`, the one answer to what a nullable place
stores, coalesced whatever wasn't a constant or `nulled`, and asked
again with `nulled` false where tidy writes a handle's place in its stead
(`read_handles`), where MIR's answer was gone. A binding's nullable
parameter answered it a second time, the same rule in `mir/calls.rs`.

## Decision

**What's never `undefined` is stored as it is.** `Expr::may_be_undefined`
is the one answer of whether a JS value may be: never a constant but
`undefined`, `null` among them, nor an object, an array, a function, a
`new`, a template, a regex or JSX made there, nor what an operator gives
but an `&&`, `||` or `??` that may give an operand that may be, nor a test
of two that aren't; nor what lowering knows isn't, `Known::Defined`.
Anything else, a variable or a call, may be.

- **Lowering says what it knows**: what MIR's `nulled` says is `null` or
  a value already is marked `Known::Defined` where it's written, so a
  handle's write tidy asks again keeps it, `context.Consumer =
  consumerOf(context);`; and an `Option`'s `map` marks its `Some`, not
  boxed, its value, which can't be `undefined` (ADR 0051).
- **A test's branches each as they're stored**: `c ? f(c) : undefined` is
  `c ? f(c) : null` where `f(c)` is never `undefined`; else the whole is
  `?? null`.
- **A call's `Option` only nullable fields read is written where the
  call's made** (`nullable_written`, ADR 0781's `null_nones`), its `None`
  `null`: `debugTask: createTask ? createTask("lazy()") : null`. Not a
  clone, which `nulled` gives what it's a clone of.
- **One answer**: a binding's nullable parameter asks
  `Storage::written` too.

| Rust | JS |
|---|---|
| `context.consumer.set(Some(Box::leak(Box::new(Consumer { .. }))))` | `context.Consumer = { $$typeof: REACT_CONSUMER_TYPE, _context: context };` |
| `context.consumer.set(Some(consumer_of(context)))` | `context.Consumer = consumerOf(context);` |
| `context.consumer.set(consumer)`, an `Option` given | `context.Consumer = consumer ?? null;` |
| `debug_task: console::create_task().map(\|t\| t("lazy()"))` | `debugTask: createTask ? createTask("lazy()") : null` |

## Why

- **It's the JS a person writes**, and what React writes; and the same
  value: `?? null` of what's never `undefined` gives it back.
- **One owner, told what it can't see**: the JS's form says what it can;
  lowering's facts ride on the expression (`Known`), as a getter's read
  does (ADR 0672), so no later pass guesses them.

## Alternatives

- **Each caller checks its value's form**: three answers to one
  question. Rejected.
- **Tidy's handle write skips storing**: a write the cell's call didn't
  store, a closure's through the variable it captured (#192), needs it.
  Rejected.

## Consequences

- A `#[rust_js::nullable]` field given `n.map(|n| n + 1.0)` is `n != null ?
  n + 1 : null`, where it was `(..) ?? null`.

## What holds it

- `test/lowering-nullable.test.ts`, "a nullable field is written what's
  never undefined as it is": React's `createContext` and `lazy`, a
  handle's write, an `Option` given still coalesced, run with and
  without `console.createTask`.
- Mutations in `scripts/mutations/js/make.ts` (`null-for-none-objects-coalesced`,
  `null-for-none-branches-coalesced`, `nulled-unmarked`,
  `defined-may-be-undefined`), `scripts/mutations/lower/std_types/option.ts`
  (`map-some-undefined`), `scripts/mutations/lower/mir/patterns.ts`
  (`null-nones-calls-unwritten`, `null-nones-clones-written`) and
  `scripts/mutations/lower/mir/assign.ts` (`call-option-written-as-given`).
