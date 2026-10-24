# 0625. A default constant is its value; a comment right above the first code is its

Status: Accepted; amended (one its module reads, below; one a `pub use`
of its module renames, by [0773](0773-a-held-default-is-exported-once.md)). Extends
[0611](0611-a-default-export-is-where-its-macro-is.md) and amends [0383](0383-a-files-leading-comments-are-its-jss.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: what a module exports
and how its comments sit, nothing a program does.

## Context

React's `shared/ReactVersion` is a constant, its default:

```js
// TODO: this is special because it gets imported during build.
// ..
export default '19.3.0';
```

`js::export_default! { fn noop() {} }` declares a function as the
default (ADR 0611); the macro took no `const`. Named,
`js::export_default!(REACT_VERSION)`, it's `const REACT_VERSION =
"19.3.0"; export default REACT_VERSION;`, a name upstream doesn't have.
And a file's leading comments, its license, are its JS's header (ADR
0383), up to its first code: the `TODO` above the export was taken into
the header, a blank line after it, where upstream has it on the export.

## Decision

**Of a `const` itself, `js::export_default! { const V: T = value; }`, its
value is the default**, `export default "19.3.0";`, as JS has no
declaration of a constant as one. Another module of the crate reads it as
its default, `import REACT_VERSION from "./ReactVersion.js"`, as it does
a default function (ADR 0251). Its `.d.ts` declares it, then exports it,
`declare const REACT_VERSION: string; export default REACT_VERSION;`, as
TypeScript has no default `const`.

**A run of comments right above the first code, no blank line between,
is that code's** (ADR 0444), where a blank line parts it from what the
file starts with: the license is the header, and the `TODO` is the
export's, as upstream has them.

## Why

- **The macro holds what it exports**, a function or a constant, one
  rule: the declaration as the default, and of a constant, its value.
- **A comment on its code is its code's** wherever it is: the first
  code's too, past the license.

## Alternatives

- **The naming form for a constant**: a name the JS doesn't need.
- **The header to the last comment before code, as before**: a comment
  that's the first item's sits apart from it, and in the corpus was lost
  where the module printed no header.

## Amended: one its own module reads is declared, then exported

A constant the macro held and its own module read, `VERSION.len()`, was
`export default "19.3.0";` and `$byteLen(VERSION)`: a read of a name JS
never declared, a `ReferenceError`, a wrong answer where Rust runs.

**A constant the default holds, read by name in its own module, is
declared, then exported by default**, `const VERSION = "19.3.0"; export
default VERSION;`, a `pub` one too, which is the default alone (ADR 0611,
amended): not exported by its name. Which it is, `pipeline.rs` asks the
uses lowering records, each item's of another (`read_at_home`), not the
JS; its `.d.ts` is as before, `declare const VERSION: string; export
default VERSION;`. One no code of its module reads is its value, as
before.

## Consequences

- Three corpus programs keep a comment that was lost before, now above
  their first function: `integer_bits`, `str_bytes`, `tagged_pointers`.

## What holds it

- `test/modules.test.ts`, "a default export is the constant's value where
  the macro holds it": its JS, its `.d.ts`, and a module reading it.
- `test/emission.test.ts`, "a file's leading comments are its JS's": a
  run right above the code after a license.
- Mutations `default-constant-a-mark`, `default-constant-unheld`,
  `default-constant-declared`, `default-constant-declared-as-default`,
  `leading-run-above-code-kept`.
- `test/modules.test.ts`, "a default constant its own module reads is
  declared, then exported". Mutations: `default-constant-read-undeclared`,
  `default-constant-read-exported`.
