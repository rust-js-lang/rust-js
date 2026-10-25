# 0574. What React's scheduler heap asks: a nullable result, an object's `Some`, a runtime that loads anywhere

Status: Accepted; amended by [0578](0578-a-nullable-result-where-its-written.md),
which writes `value ?? null` where the value is written. Amended: its
declaration is `T | null`, never `undefined`. Extends [0275](0275-nullable-writes-null.md), whose
`#[rust_js::nullable]` it puts on a function's result,
[0051](0051-generic-options.md), whose box it leaves out where a bound
says it's never needed, and [0307](0307-binding-trait-methods.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A for each. A JS
caller sees `null` where it tests for it, and nothing a program does
changes.

## Context

The first file of React's port (rust-js-lang/react-in-rust) is its
scheduler's min-heap, `SchedulerMinHeap.js`: `push`, `peek` and `pop` of
any object with an `id` and a `sortIndex`. Its Rust is a trait of
bindings, ADR 0307's, which takes no dictionary and reads
`node.sortIndex`:

```rust
pub trait Node {
    #[cfg_attr(rust_js, rust_js::link_name = "get sortIndex")]
    fn sort_index(&self) -> f64 { unreachable!() }
    ..
}
pub fn peek<T: Node>(heap: &Heap<T>) -> Option<&T> { heap.first() }
```

React's own tests of the scheduler, still JavaScript, ran on the JS
rust-js wrote of it, and found three things:

- **47 of 64 failed on `undefined`.** `Scheduler.js` asks `peek(taskQueue)
  !== null`, and a `None` is `undefined` (ADR 0030): an empty queue
  read as a task.
- **`Some(first)` was `$some(first)`, and `heap.first()` `$someAt(heap,
  0)`:** a type parameter's `Some` is boxed, as it might be an `Option`
  itself (ADR 0051). A `T: Node` can't be: its `sortIndex` is read of it.
- **26 failed on loading the runtime.** Importing `$swap` loaded the
  package, whose JSON reader made a `TextDecoder` as it loaded; Jest's
  jsdom has none.

## Decision

**A function's `#[rust_js::nullable]`, alone, is of its result: its
`None` is `null`.** As a field's is (ADR 0275): a `None` it makes is
`null` where it's made, `Some(x)` is `x`, and another `Option` it returns
is `value ?? null`. On a function whose result isn't an `Option`, it's
rust-js's error. Named, `#[rust_js::nullable(replacer)]`, it's still a
binding's parameter.

```js
export function peek(heap) {
  return heap[0] ?? null;
}
```

**A type parameter a binding of its bound reads is never nullish**
(`read_by_its_bounds`): `Some` of one is the value itself, as `Some` of an
object is. The binding is taken on trust (ADR 0307), and a value it reads
a property of is no `undefined`.

**`@rust-js/runtime` loads where JavaScript does.** What a helper needs of
the host, the JSON reader's `TextDecoder`, it makes when it's first
asked, not as the package loads.

## Why

- **The caller's test is the contract**: `!== null` is what the
  scheduler's own code and tests say, and TypeScript's `T | null`.
  `nullable` is already how rust-js says it, of a field and a parameter;
  of a result it's one more place, not another spelling.
- **The box is for a value that can be `undefined`**, and a bound that
  reads the value says it can't be.
- **A package imports by name.** A module that uses `$swap` runs in Node,
  a browser, a worker or jsdom alike; one helper's need of the web
  isn't every module's.

## Alternatives

- **A `js::Nullable<T>` result type**: a second way to say what
  `#[rust_js::nullable]` says, and a type no Rust caller would want.
  Rejected.
- **Make every `None` `null`**: changes what every module writes, and
  `undefined` is JS's own absence (ADR 0030). Rejected.
- **Box unless proved otherwise, and leave the bound alone**: `$some` and
  `$someAt` in a heap written as `heap[0]`. Rejected.
- **Give Jest a `TextDecoder`**: the port's setup to work around the
  runtime. Rejected.

## Consequences

- React's scheduler's 63 tests pass on the heap's JS, and one is
  skipped, as upstream's are.
- A crate's `Option` of a binding-bounded type parameter loses its box
  wherever it's made.

## What holds it

- `test/bindings.test.ts`, "a nullable function's None is null, and a
  binding bound's Some is the value": `heap[0] ?? null`, `return null;`,
  `return first;`, no `$some`, run in Node; a non-`Option` result
  refused.
- `test/runtime-package.test.ts`, "@rust-js/runtime loads in a host with
  none of the web's APIs".
- Mutations: the result left `undefined`, a `Some` coalesced, the bound's
  `Some` boxed, a non-`Option` result let through.

## Amendment: a nullable result is declared `T | null`

A nullable function's `.d.ts` was any `Option`'s, `peek(heap: Node[]):
Node | null | undefined`, where its `None` is `null` where it's made (ADR
0578), as React's `peek` is typed `T | null`: a TypeScript caller's
`const first: Node | null = peek(heap)` was an error, and its `=== null`
test didn't narrow away an `undefined` that's never there.

**A `#[rust_js::nullable]` function's result is declared `T | null`**
(`declarations.rs`, `function`), its `T` as written, an alias's name too.
Another `Option`'s result keeps `T | null | undefined`.

Held by `test/declarations.test.ts`, "a nullable function's result is
declared T or null", which TypeScript checks, and the mutation
`nullable-result-declared-undefined` (`lower/declarations.ts`).
