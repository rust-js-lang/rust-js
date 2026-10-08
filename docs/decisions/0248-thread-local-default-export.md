# 0248. A `thread_local!` is a module's default export too

Status: Accepted. Extends [0192](0192-next.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's CodeBlock/index exports a memo'd component by default, which
MDXComponents imports:

```tsx
export default memo(function CodeBlockWrapper(props) { .. });
```

A memo'd component is a `thread_local!` in Rust, which `thread_local!`
makes a `const` of its `LocalKey`, and `js::export_default!` named a
function only: the module had no default export to give.

## Decision

**`js::export_default!` names a function of its module or a
`thread_local!` of it.**

```rust
thread_local! {
    static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label);
}
js::export_default!(Memoized);
```

```js
const Memoized = memo(Label);
export default Memoized;
```

```ts
declare const Memoized: NamedExoticComponent<LabelProps>;
export default Memoized;
```

## Why

- **It's the JS a person writes**, a value exported by default.
- **It's tested**: a declarations test exports a memo'd component by
  default, renders it, and checks its `.d.ts`; mutations refuse the
  `thread_local!` and declare it a function.
