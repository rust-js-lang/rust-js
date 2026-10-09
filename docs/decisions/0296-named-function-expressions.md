# 0296. A function a block makes and gives is a named function expression

Status: Accepted. Builds on [0248](0248-thread-local-default-export.md)'s
memoized components; amended by [0308](0308-local-functions.md), where a
function a block doesn't give is.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev memoizes its components with a function written where it's
wrapped, named as the component:

```ts
export const IconCanary = memo(function IconCanary(props) { .. });
export default memo(function CodeBlockWrapper(props) { .. });
```

A port wrote `fn Canary(..)` and `memo(Canary)`: Rust can't name a module's
function and its static alike. So the JS had `memo(Canary)` and a function
`Canary` apart from it, a name React's tools show that the original
doesn't have. A function made inside a thread-local's initializer was taken
for the initializer itself and broke the module ("Duplicated export").

## Decision

**A function a block makes and gives as its value, and that nothing else
names, is JS's named function expression where the block is:**

```rust
thread_local! {
    pub static IconCanary: MemoExoticComponent<IconCanaryProps> = memo({
        fn IconCanary(props: IconCanaryProps) -> JSX::Element { .. }
        IconCanary
    });
}
// export const IconCanary = memo(function IconCanary(props) { .. });
```

- It's named in itself alone, so it takes no name of the module's: the
  static keeps `IconCanary`. If its body reads that name, which then means
  the static, it's named apart, `function IconCanary$1`.
- One that calls itself, or that the block also calls, or a generic one,
  which its dictionaries are given to where it's named, stays a function
  of its own: a declaration in the body it's written in (ADR 0308), or the
  module's.
- A function a thread-local's `init` makes is that function, not the `init`.

The function is lowered as any item is, and put where it's named.

## Why

- **It's the JavaScript a person writes**: the original's
  `memo(function IconCanary ..)`, under the original's name.
- **It's the same program**: a function nothing else names is the same
  wherever it's defined; a named function expression's name is its own.

## Consequences

- React's developer tools show a memoized component by the original's
  name.
