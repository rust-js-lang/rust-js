# 0616. An import read only by a `const` is the import by that name

Status: Accepted. Extends [0352](0352-reclaimed-names.md)'s reclaimed
names.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same value is
read; only its name changes.

## Context

React's `shallowEqual` imports its helpers by the names it calls them:

```js
import is from './objectIs';
import hasOwnProperty from './hasOwnProperty';
```

Its Rust reads a thread-local of another module, which holds them, into a
variable, `let is = IS.with(|is| *is);`, so the JS was `import IS from
'./objectIs.js'; .. const is = IS;`, and `import hasOwnProperty$1 ..;
const hasOwnProperty = hasOwnProperty$1;`.

## Decision

**An import its module reads once, a `const` at a function's top that
copies it, is the import by that `const`'s name**, the `const` gone
(`reclaim::aliased_imports`): `import is from './objectIs.js';`, a named
one `import { Empty as Selected }`. Where the module reads it again, or
declares the name anywhere else, both stay.

## What holds it

- `test/modules.test.ts`, "an import read only by a const is the import
  by its name", with a module that reads it twice.
- Mutations: the `const` kept, renamed where it's read twice.
