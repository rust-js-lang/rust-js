# 0718. A typed array is rest arguments, spread, and a rest of `number` takes any JS number

Status: Accepted. Extends [0640](0640-a-rest-parameter-is-a-tuple.md),
whose rest arguments are a tuple, a slice or a `Vec`, and whose `f64`
rest took an `f64` alone.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each call gives JS
the arguments it gave; a typed array's spread is its elements, as JS's
`...` steps it.

## Context

React's `shared/binaryToComparableString.js`:

```js
return String.fromCharCode.apply(String, new Uint8Array(view.buffer, view.byteOffset, view.byteLength));
```

The js crate bound no `String.fromCharCode`, so the port collected each
byte, `.values().map(char::from).collect()`, which printed an iterator's
`map`, `toArray` and `join("")`. TypeScript declares it
`fromCharCode(...codes: number[]): string`: a rest of `number`. A rest of
`f64` took only `f64`s, so neither a `Uint8Array` nor a `u8` was one, and
no rest took a typed array at all.

## Decision

- **`js::string::from_char_code(codes: impl Rest<f64>)` is
  `String.fromCharCode(..)`**, a variadic binding of TypeScript's `number`.
- **A typed array is rest arguments, spread**, as a slice is: `&Uint8Array`
  is `Rest<T>` of a `T` that takes a `u8`, so `from_char_code(bytes)` is
  `String.fromCharCode(...bytes)`. Lowering spreads what's given a variadic
  binding that's a JS object, which the js crate makes rest arguments only
  of an iterable, a typed array (`binding_calls.rs`).
- **A rest of `f64`, TypeScript's `number`, takes any number JS has as
  one**: what `f64::from` takes, `i8` to `u32` and `f32`, and `f64`; not an
  `i64`, a `BigInt`. So `math::max((1, 2.5))` and a `Uint8Array`'s bytes
  are a rest of numbers. A rest of a narrower number, `Uint16Array.of`'s
  `u16`, still takes its own type, so a literal there is inferred as it.

```rust
string::from_char_code(uint8_array::new_with_buffer(view.buffer(), Some(view.byte_offset()), Some(view.byte_length())))
```
```js
return String.fromCharCode(...new Uint8Array(view.buffer, view.byteOffset, view.byteLength));
```

This is upstream's call by the one rule a rest parameter has, the spread
`...`, where upstream wrote it `.apply(String, ..)`: the same arguments,
the same limit on how many an engine takes.

## Why

- **TypeScript's own type**: `number`, a rest of it, which any JS number
  is.
- **One rule of rest arguments**: what's spread is any iterable of them,
  as JS's `...` spreads it; a typed array is one, as an array is.

## Alternatives

- **`from_char_code(codes: impl Rest<u16>)`**, a code unit's own type:
  narrower than TypeScript, and a `Uint8Array`'s bytes still aren't
  `u16`s.
- **Each rest of a number takes what converts into it**, `u16` of a `u8`
  too: a literal `(72, 105)` would be ambiguous between them, and rustc's
  fallback, `i32`, no one's; and an `i64`, a `BigInt`, would take an `i32`,
  a JS number, which `BigInt64Array.of` throws of.
- **`.apply(String, bytes)`, a binding of `Function.prototype.apply`**: a
  second way to give rest arguments.

## What holds it

- `test/js-builtins.test.ts`, "String.fromCharCode takes its codes as rest
  arguments, a typed array spread": a tuple, a `Vec`, a `Uint16Array`,
  none, and upstream's function of a view, run.
- Mutation `rest-typed-array-refused`.
