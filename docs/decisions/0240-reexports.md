# 0240. A `pub use` of another module's function is re-exported from it

Status: Accepted.

## Context

react.dev's Challenges/index re-exports its Challenges, which MDXComponents
imports from it:

```tsx
export {Challenges} from './Challenges';
```

A `pub use` of another module's function wrote nothing, in the JS or the
`.d.ts`: what imported it from the module that re-exports it found nothing.

## Decision

**A module's public `use` of another of its crate's modules' function is
`export { .. } from` that module, in its JS and its `.d.ts`; a `use .. as`
is `export { a as b }`.**

```rust
mod inner;
pub use inner::helper;
pub use inner::other as renamed;
```

```js
export { helper, other as renamed } from "./inner.js";
```

- **One statement for each module**, its names in the order written.
- **A private `use` is the module's own**, and is imported as before, not
  exported.
- **The `.d.ts` is TypeScript's**, printed by its factory through
  @rust-js/typescript's new `export-from` kind, and read back the same way.

## Why

- **It's the JS a person writes**: no wrapper, no local, the function as it
  is.
- **It's Rust's**: what `pub use` makes public is what the module exports.
- **It's tested**: a compiler test re-exports one name and one renamed,
  beside a private `use`, and calls them; a declarations test checks
  TypeScript that imports them. Mutations re-export the private `use`,
  drop the rename, and leave out the JS's or the `.d.ts`'s statement.
