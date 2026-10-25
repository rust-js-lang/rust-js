# 0848. `function::apply` is `Function.prototype.apply`, and its `this` may be `null`

Status: Accepted. Extends [0740](0740-a-functions-members-are-read-of-any-function.md),
whose `js::function` had `name`, `length`, `toString` and `call`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a binding's shape:
the call TypeScript's `apply` is, given what TypeScript lets it be given.

## Context

React's `cache` calls what it caches as upstream writes it:

```js
return fn.apply(null, arguments);
```

The js crate had `function::call(f, this_arg, args)`, `f.call(thisArg,
...args)`, whose `this_arg` is `Option<&dyn IntoUnknown>`, `None` of
`undefined`; no `apply`, and nothing gave `null` as a `this`, which a
strict function sees apart from `undefined`.

## Decision

**`function::apply(f, this_arg, arg_array)` is `f.apply(thisArg,
argArray)`, TypeScript's `apply(this: Function, thisArg: any, argArray?:
any): any`: each an any value, `Option<&dyn IntoUnknown>`, as `call`'s
`this_arg` is; `None` of `this_arg` is `undefined` and of `arg_array` no
arguments, left out as a trailing optional one is (ADR 0443). JS's
`null`, as a value of any shape, is `Kind::Null`, the variant that is
it (ADR 0676): `Some(&Kind::Null)`.**

```rust
function::apply(r#fn, Some(&Kind::Null), Some(&args))
function::apply(f, None, None)
function::apply(f, Some(&"self"), Some(&args))
```

```js
fn.apply(null, args)
f.apply()
f.apply("self", args)
```

## Why

- **It's TypeScript's member**, typed as it types it, so a port writes
  upstream's call.
- **`null` is a value JS has**, and the one Rust value that is it is
  `Kind::Null`, an untagged variant whose JS is `null`: no second
  spelling of it, `js::NULL`, and no attribute making a `None` of `apply`
  `null` while `call`'s is `undefined`, two meanings of one `None`.

## Alternatives

- **`#[rust_js::nullable(this_arg)]`, `None` of `null`**: `apply(f, None,
  ..)` would be `null` where `call(f, None, ..)` is `undefined`; and an
  `undefined` `this` would need a `Some` of a `()`.
- **`arg_array: impl Rest<dyn IntoUnknown>`**: `apply` takes one array,
  TypeScript's `any`; spreading is `call`'s.

## What holds it

- `test/js-object.test.ts`, "a function is applied to a this of any value
  and an array of arguments", of a strict function: `null`, `undefined`
  and a string its `this`. `test/rest-parameters.test.ts`, React's
  `fn.apply(null, args)`, run against upstream's.
