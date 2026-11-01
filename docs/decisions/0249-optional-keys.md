# 0249. An element's key may be none

Status: Accepted. Extends [0041](0041-react.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's CodeDiagram gives each child it renders anew its own key,
which a child may not have:

```tsx
<CodeBlock key={child.key} {...child.props} noMargin={true} />
```

@types/react types a key `key?: Key | null`; the react crate's `Key` was
a string or a number only, so an element's own, `element.key()`, an
`Option`, wasn't one.

## Decision

**An `Option` of a key is a key: `None` is no key.**

```rust
jsx! { <li key={element.key()}>{..}</li> }
```

```jsx
<li key={element.key}>..</li>
```

## Why

- **It's React's type**, and JS's: `undefined` is no key.
- **It's tested**: a JSX test renders a list keyed by a key and by none,
  and reads React's keys of them.

## Amendment (2026-11-01): any of Rust's numbers is a key (#451)

A full-stack app keys its tasks by their `u64` ids, `key={task.id.0}`;
`Key` was a `&str`, a `String`, an `i32`, a `u32`, a `usize` or an `f64`,
so it wrote `key={id as u32}`, which cuts an id past `u32::MAX` and gives
two tasks one key. @types/react's `Key` is `string | number | bigint`, and
React makes text of a key, `'' + key`. Case A
([0262](0262-when-rust-and-js-disagree.md)): a `u64`, a BigInt (ADR 0086),
keys as its digits, `10n` `"10"`, as a `u32` 10 does, so nothing differs.

**Each of Rust's integers and floats is a key**: `i8` to `i128`, `isize`,
`u8` to `u128`, `usize`, `f32` and `f64`, one rule, not the few a port
asked for. Held by `test/jsx-types.test.ts`'s "an element's key may be a
64-bit integer", run in Node: `u64::MAX` keys as
`"18446744073709551615"`, an `i64` as `"-3"` and a `u8` as `"7"`. A
binding's `impl`s only: no compiler change, so no mutation.

