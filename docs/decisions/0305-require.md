# 0305. A binding marked `require` is read through `require(module)`

Status: Accepted. Amends [0028](0028-js-module-imports.md): how a module's
export is read.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A binding of a module's export, `#[link_name = "pkg#name"]`, is imported:
`import { name } from "pkg"`. react.dev's runESLint, an ES module its
bundler builds, reads a CommonJS package's export through `require`:

```ts
const reactRules = require('eslint-plugin-react-hooks').rules;
```

And a getter of a module's export, `get pkg#name`, was no import at all:
lowering read a local nothing had imported.

## Decision

**A binding marked `#[rust_js::require]` is read through `require(module)`
where it's read, and nothing imports its export; a getter of a module's
export, `get pkg#name`, is imported as a function's is.**

```rust
unsafe extern "Rust" {
    #[link_name = "get eslint-plugin-react-hooks#rules"]
    #[rust_js::require]
    safe fn rules() -> &'static ReactHooksRules;
}
thread_local! {
    static reactRules: &'static ReactHooksRules = rules();
}
// const reactRules = require("eslint-plugin-react-hooks").rules;
```

- An export one binding marks is read through `require` by each binding
  of it: one module, read one way.
- `require` is a name no local of the module takes.

## Why

- **It's the JavaScript a person writes**: the original's `require(..)`.
- **It's the same program**: its bundler, or Node, loads the module
  where `require` asks for it, as the original's does.
