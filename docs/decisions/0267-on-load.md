# 0267. js::on_load! is what a module runs when it's loaded

Status: Accepted. Extends [0110](0110-stable-syntax.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A JS module may run statements when it's loaded. react.dev's Page
prefetches CodeBlock, its own statement among its imports:

```tsx
import(/* webpackPrefetch: true */ '../MDX/CodeBlock/CodeBlock');
```

Rust has nothing that runs when a module loads, and rust-js nothing to
write one: a port made it a `thread_local!`, an unused `const prefetched`.
`js::import!` is a static import, which bundles the module where it is.

## Decision

**`js::on_load! { .. }` is the module's own statements, run when it's
loaded**, after its `const`s:

```rust
js::on_load! {
    let _ = import_module::<()>("../MDX/CodeBlock/CodeBlock");
}
```

```js
import("../MDX/CodeBlock/CodeBlock");
```

- **rustc checks them as a function's body**: the macro writes a function
  `#[rust_js::on_load]`, in a `const _`, which nothing calls; rust-js writes
  its body where the module's statements go, and no function.
- **What they use is imported**, a runtime helper's too.

## Why

- **It's JS's, as the module has it**: a statement among its imports, and
  no variable to hold what it gives.
- **It's tested**: a compiler test's module calls a JS function when it's
  loaded, through a helper; mutations write a function of it, leave its
  statements out, and its helper unimported.
