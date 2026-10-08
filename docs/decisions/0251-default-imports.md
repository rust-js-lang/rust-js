# 0251. A module's default export is imported as its default

Status: Accepted. Extends [0248](0248-thread-local-default-export.md) and
[0202](0202-imports-named-around-importers.md).

## Context

react.dev's CodeBlock/index exports one thing, by default, which its
CodeDiagram and PackageImport import by a name of their own:

```tsx
import CodeBlock from './CodeBlock';
```

A module of a rust-js crate that used another's item imported it by the
item's name, which the other then exported by that name too, as well as
by default: `import { Memoized } from "./CodeBlock/index.jsx"`, and
`<Memoized />`, though the Rust wrote `use ..::Memoized as CodeBlock`.

## Decision

**A module's default export, used by another module of its crate, is
that one's default import, named as its `use .. as` names it, and isn't
exported by its own name for it.**

```rust
use label::Memoized as Label;
```

```js
import Label from "./label.jsx";
```

- **Named imports of the same module follow it**, `import Label, { a }
  from`, as JS writes them.
- **One a library's consumers reach is still exported by name**, as they
  import it (ADR 0100).

## Why

- **It's the JS a person writes**, each module's default its own.
- **It's tested**: a declarations test imports a module's memo'd default
  under another name and renders it; mutations export it by name too,
  import it by name, name it as its item is, and print it braced.
