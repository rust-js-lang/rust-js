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
  `#[rust_js::on_load]`, in a `const _` marked so too, which nothing calls;
  rust-js writes its body where the module's statements go, and neither the
  function nor the `const`.
- **What they use is imported**, a runtime helper's too, and **linked as a
  function's body is**: a call of another module's function is its import,
  whose alias is none of their names, a closure's parameters' neither.
  (Amended: they were left out of linking, so such a call reached the
  printer unresolved, and an alias could be one of their names.)

## Why

- **It's JS's, as the module has it**: a statement among its imports, and
  no variable to hold what it gives.
- **It's tested**: a compiler test's module calls a JS function when it's
  loaded, through a helper; mutations write a function of it, leave its
  statements out, and its helper unimported. A link test calls another
  module's function from a closure, beside a local of that function's name;
  mutations leave their names unreserved, or their imports unresolved.
- Each body's locals are its own, as a function's are: a later body's is
  none of an earlier one's in its module, `x$1` beside `x`, as none is a
  function's or an import's of it. (Amended: two that bound one name
  declared it twice at the module's top, which JS won't load.)
