# 0256. A namespace import is named as the module of its bindings

Status: Accepted. Extends [0028](0028-js-module-imports.md).

## Context

react.dev's BrandMenu imports Radix's context menu as a namespace and
renders its parts by it:

```tsx
import * as ContextMenu from '@radix-ui/react-context-menu';
<ContextMenu.Root>..</ContextMenu.Root>
```

A binding of a module's namespace, `#*.Root` (ADR 0028), was imported by
a name of the module's file, `import * as reactContextMenu`.

## Decision

**A namespace whose bindings are all in one Rust module, not the crate's
root, is imported by that module's name.**

```rust
mod ContextMenu {
    unsafe extern "Rust" {
        #[link_name = "@radix-ui/react-context-menu#*.Root"]
        pub safe fn Root(props: RootProps) -> JSX::Element;
    }
}
```

```jsx
import * as ContextMenu from "@radix-ui/react-context-menu";
<ContextMenu.Root>..</ContextMenu.Root>
```

- **Bindings spread over modules, or at the root, are named by the
  file**, as before.

## Why

- **It's the JS a person writes**, named as the Rust names it.
- **It's tested**: a compiler test calls two bindings of a namespace a
  module holds; a mutation names it by its file.
