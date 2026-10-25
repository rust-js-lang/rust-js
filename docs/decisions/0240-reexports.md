# 0240. A `pub use` of another module's function is re-exported from it

Status: Accepted; amended by [0874](0874-a-modules-pub-uses-are-re-exported-as-written.md)
(every `pub use` of what has JS, in the order written, a run of one
module's one statement, or refused; a JS module's namespace too).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

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

- **A module of nothing but them is a file of them**, as react.dev's
  Sidebar/index is. (Amended: it had no file.)

## Why

- **It's the JS a person writes**: no wrapper, no local, the function as it
  is.
- **It's Rust's**: what `pub use` makes public is what the module exports.
- **It's tested**: a compiler test re-exports one name and one renamed,
  beside a private `use`, and calls them; a declarations test checks
  TypeScript that imports them. Mutations re-export the private `use`,
  drop the rename, and leave out the JS's or the `.d.ts`'s statement.

## Since

- **A default that's another module's function is re-exported as it**:
  `js::export_default!(super::page::page)` is `export { page as default }
  from "./page.js"`, with the module's `pub use`s, as react.dev's
  errors/index makes the errors page its own. A module of only that is a
  file of it.

## Amendment: a binding's, from its JS module

**A `pub use` of a binding of a JS module's export is `export { .. } from`
that module**, in the JS and the `.d.ts`, as react.dev's Challenges/index
re-exports its still-TypeScript Challenges:

```rust
mod bound {
    #[cfg_attr(rust_js, rust_js::link_name = "./Challenges#Challenges")]
    pub fn Challenges(props: ChallengesProps) -> JSX::Element { unreachable!() }
}
pub use bound::Challenges;
```

```js
export { Challenges } from "./Challenges";
```

- **Its module named as its imports are** (ADR 0028): from the root's JS
  as written, from this file's folder as JS reads it, `../math.js` a
  folder down; a default by the name the `use` gives it, `export { default
  as Half }`.
- **A binding's `static` is re-exported too**, and a module of only one's
  `pub use` is a file of it. One function, `analysis::is_reexport`, says
  what a `pub use` re-exports, which both the module's file and its
  re-exports ask; the output names every module a re-export or a dynamic
  `import()` is from by one closure, the crate's or a JS module's.
- **A `pub fn` binding isn't re-exported**: it's an import its crate's
  modules call, and a binding crate's thousands of `pub` bindings export
  nothing. Only `pub use` re-exports, as it does a function of the crate's.
  Decided with the user.
- **A JS module's namespace, `#*`, is refused**: `export * as ns from`,
  which TypeScript's re-export model doesn't hold yet; re-export each of
  its exports by name.
