# 0256. A namespace import is named as the module of its bindings

Status: Accepted. Extends [0028](0028-js-module-imports.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

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

## Amended by

- [0373](0373-one-binding-form.md): A binding of a JS function is an ordinary function, and a JS type a struct

## Amendment: camelCase, as every Rust name is (2026-10-23)

The module's name is camelCase in JS, as ADR 0573 has every Rust name:
`mod side_menu` holding `./side.js#*` is `import * as sideMenu`, where it
was `side_menu`; `mod ContextMenu` is `ContextMenu` still. Found porting
React's `shared/ReactSharedInternals`, whose `react#*` binding its module
`react_shared_internals` holds: `reactSharedInternals`.

## Amendment: only a module of the namespace's own (2026-10-26)

A module that held the bindings of a namespace was its name, whatever
else it held: React's `ReactFiberCacheComponent`, a module of the
reconciler's code that binds two of `scheduler#*`'s members, imported it
as `import * as cache from "scheduler"`, named after the importer, and
`ReactSharedInternals`, a module of types too, `import * as
reactSharedInternals from "react"`. A file of other code isn't the
namespace; `mod ContextMenu`, holding only its parts, is, as JS's
`import * as ContextMenu` is.

**A module names the namespace only where it holds no code of its own**:
bindings of the namespace, the types they take, their props, with what a
derive or a macro writes of them (`jsx!`'s `impl Lifetimes` and props
macro, ADR 0213) and impls of bindings, and `use`s. One of other code
too, a function, a static or a `const` that isn't a binding of it, or of
another module's bindings, leaves it named by what it's imported from, as
the crate's root does: `import * as scheduler from "scheduler"`, `import *
as react from "react"`. Upstream's `Scheduler` and `React` are capitalized: only a
module of the namespace's own named so, `mod Scheduler`, gives that, and
rustc's `non_snake_case` lint warns of it, so a lint-clean port reads
`scheduler`. Case N
([0262](0262-when-rust-and-js-disagree.md)): a name.

Held by `test/modules.test.ts`'s namespace test, which adds a module of
code calling `./scheduler.js#*.unstable_now`, one holding
`./timers.js#*.start` beside `./clock.js#tick`, and props that derive
`Clone` to `mod ContextMenu`; `test/jsx.test.ts`'s `<Menu.Item>`, whose
props `jsx!` writes of; and the mutations
`namespace-of-any-holding-module`, `namespace-module-of-other-items`,
`namespace-module-of-another-namespace`, `namespace-module-of-props-unread`,
`namespace-module-written-impls-unread` and `namespace-module-jsx-macro-unread`.
