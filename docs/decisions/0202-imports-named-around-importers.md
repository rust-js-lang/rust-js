# 0202. An import is named around the modules that import it

Status: Accepted. Amends [0028](0028-js-module-imports.md).

## Context

An import's name is the same in every file that has it, unique among the
crate's imports, and was named around every module's items. react.dev's
`ButtonLink` imports next/link's default, `Link`, and its `IconLink`
module has a function of its own, `fn Link`, which ButtonLink's file
never sees: every file's import was `import Link$1 from "next/link"`.

## Decision

**An import is named around the globals, the crate's other imports, and
the items of each module that imports it, or every module's for a
library's export, whose importers are known only once they're lowered:
another module's item of its name is no reason to rename it.**

```js
// components/ButtonLink.jsx, beside components/Icon/IconLink.jsx's `function Link`
import Link from "next/link";
```

- **Still one name in every file that has it**, so a module's locals
  avoid it, as they did.

## Why

- **A name is the file's**: one in a file that doesn't import it is no
  name of that file's import. A module of the crate's own, imported by
  the importing one, is the linker's to alias, as it was.
- **It's tested by the `imports` snapshot**: `inner/leaf.rs` has a `fn
  join` of its own, and the root's `import { join } from "node:path"`
  keeps its name.
