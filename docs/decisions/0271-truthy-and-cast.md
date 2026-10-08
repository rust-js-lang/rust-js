# 0271. A JS value's truthiness, and a value vouched to be a type

Status: Accepted. Extends [0225](0225-unknown-values.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's errors page revives elements from JSON and reads React's error
codes, which `fetch` gives as `any`:

```tsx
if (!Type) {
  console.error('Unknown type: ' + Type);
  Type = Fragment;
}
// ..
const errorCodes: {[key: string]: string} = await response.json();
if (code && !errorCodes[code]) {
```

`!Type` asks JS's question, whether a value is falsy: `undefined`, `null`,
`""`, `0`, `NaN` or `false`. A `js::Unknown` had no way to ask it short of a
`classify` and a `match` of each kind. And TypeScript gives `any` a type by
saying so; ADR 0225 has every step out of an `Unknown` written, by
`classify`, `get` or a `match`, each a check.

## Decision

- **`js::truthy(value)` is `!!value`**, of an `Option<&T>` of anything
  never `undefined` (`js::Defined`): JS's truthiness, by a link name of its
  own, `"!!"`. A `!` of it is `!value`, `!!!value` being `!value` for every
  value, and a test of it is `if (value)`, as a test makes a `bool` of what
  it's given.
- **`unsafe { js::cast::<T>(value) }` is `value`**, an `Option<&Unknown>` as
  a `T`, unchecked. `unsafe` is Rust's own way to say "I vouch", which
  TypeScript's `any` says by being given a type: the step out of the types
  is written, and its caller answers for it.

```rust
if !js::truthy(Type) {
    Type = Some(js::unknown_of(FRAGMENT));
}
let message: Option<String> = unsafe { js::cast(js::get(errorCodes, code)) };
```

```js
if (!Type) {
  Type = Fragment;
}
const message = errorCodes[code];
```

## Why

- **Each is the JS the page has**, with nothing of rust-js's.
- **Truthiness is JS's, asked of a JS value**: a program asks it by name, and
  gets JS's answer, so nothing a Rust program could observe differs. Rust's
  own tests, `is_empty()` or `is_none()`, keep Rust's answers.
- **A cast is checked by no one**, as TypeScript's isn't: a `T` that isn't
  what JS has is the caller's bug, which `unsafe` marks where it's made.
- **It's tested**: a compiler test asks truthiness of eleven values, and a
  cast gives the value; mutations print `!!` where a test has it, and
  `!!!`.
