# 0602. A binding taken as a `fn` is its JS function, read where it's taken

Status: Accepted. Amends [0039](0039-generic-bindings.md)'s binding as a
value. Amended by [0667](0667-console-is-a-value-and-rest-arguments-are-one.md):
a variadic binding taken as a `fn` of rest arguments as a value,
`&dyn js::Rest<T>`, is its function too, which its call spreads them to.
Extended by [0887](0887-std-items-that-are-math-are-math.md): a std function that's `Math`'s, taken as a `fn`, is
`Math`'s function, `Math.clz32`.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results would change:
Rust's, a `fn` fixed once it's taken.

## Context

React's scheduler captures what it calls, in case a polyfill overrides it:

```js
const localSetImmediate =
  typeof setImmediate !== 'undefined' ? setImmediate : null;
```

and its test "does not use setImmediate override" sets `setImmediate` after
the scheduler loads, which it mustn't call. ADR 0039 made a binding used
as a value an arrow of its own parameters, `(callback) =>
setImmediate(callback)`, so `.map(parseInt)` isn't given each index. That
arrow reads the global at each call: the polyfill was called.

## Decision

The user's choice: **a binding taken as a `fn`, a function pointer, is the
JS function itself, read where it's taken**, `const localSetImmediate =
setImmediate`, as a Rust `fn` is the one function from where it's taken.
Where its arrow does more than pass its parameters on, a `Result`'s
`try`, it stays that arrow.

**A binding handed to a JS callback itself is still its arrow**,
`.map(encode)` `.map((text) => encodeURIComponent(text))`: that's its
function item, not a pointer. And what hands a `fn` pointer to JS gives it
no more than Rust does, as it did: `.map((item) => parse(item))`.

## Consequences

- A binding's arrow didn't make its arguments as its call does, a
  `#[rust_js::nullable]` parameter's `null`: found here, fixed by
  [0604](0604-a-bindings-value-gives-what-its-call-gives.md).

## What holds it

- `test/lowering.test.ts`, "a binding taken as a fn is its JS function,
  read where it's taken": a global set after the module loads isn't
  called, and a pointer handed to `map` gets no index.
- The guide's [binding as a value](../guide/interop/bindings.md#a-binding-as-a-value).
- Mutations: the arrow kept, the cast not asked.

## Amendment: called through what holds it, with no `this` (#427)

A binding's JS function held in a field, `Config { schedule:
webapi::global::queue_microtask }`, and called through it,
`(CONFIG.schedule)(cb)`, was `CONFIG.schedule(cb)`: JS gave the host
function the config as its `this`, which a browser's own refuses,
`TypeError: Illegal invocation` (Chromium's `queueMicrotask`, `setTimeout`,
`requestAnimationFrame`), where Rust's call runs. The second case of ADR
0262 (results change): Rust's behavior is kept.

**A fn pointer that may be a binding's JS function the crate made one of
is called with no `this`, its callee held first, `const schedule =
CONFIG.schedule; schedule(cb)`**, by the one rule a `with_this` function's
call through a field is (#402, `called_unbound`): `FnPointers::may_read_this`
answers both, `Called::Function` of a `dyn Fn`'s signature, `Called::Pointer`
of a fn pointer's type. A temporary MIR holds the read in, which tidy would
read in place, is held named as its field. A fn pointer the crate's
functions alone may be is called through its field as before,
`CONFIG.count(n)`.

Held by `test/lowering-passed-on.test.ts`'s "a fn field that may hold a
host function is called with no this, in Chromium" (run in Chromium, the
real `queueMicrotask`), and mutations `host-fn-field-called-through`
(`analysis/fn_pointers.ts`), `host-fn-temporary-unheld` (`mir/calls.ts`).
