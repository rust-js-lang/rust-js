# 0206. TypeScript, read and written by TypeScript's own parser and printer

Status: Accepted.

## Context

rust-js meets TypeScript both ways. It writes a crate's `.d.ts` (ADR
0196), so far as strings it put together, which TypeScript only checks
afterwards. And it has TypeScript to read: react's attributes are typed by
`@types/react` alone, `AnchorHTMLAttributes` and its chain, and a
TypeScript codebase moving to Rust is declarations to read too. Reading a
`.d.ts` by patterns breaks on what a pattern doesn't foresee, a type over
several lines, a comment in one, an alias named elsewhere. As the JS is
printed by oxc from a tree (ADR 0018), TypeScript should be read and
printed by a parser and a printer, TypeScript's own.

## Decision

**`@rust-js/typescript`, the workspace's `typescript/`, is TypeScript's own
parser and printer, through TypeScript 7's API, and a model of
declarations between them:**

```js
import { open } from "@rust-js/typescript";

const ts = await open(["node_modules/@types/react/index.d.ts"]);
const { declarations } = await ts.read("node_modules/@types/react/index.d.ts");
const text = await ts.print({ header: "// ..", declarations });
await ts.close();
```

- **The model is JSON**: imports, interfaces with what they extend and
  their members, type aliases, functions, constants, `export default` and
  namespaces; and types: keywords, literals, references with their
  arguments, unions, intersections, arrays, tuples, functions and object
  types. What it doesn't take apart is `other`, as it was written.
- **`read` is TypeScript's parser**, `typescript/unstable/async`'s
  program of the files a session opens: each file's syntax tree, as the
  model.
- **`print` is TypeScript's factory and printer**: the model as
  TypeScript's syntax tree, each declaration printed by TypeScript, a
  blank line between them, as TypeScript writes them.
- **What it prints is erasable syntax only**, as TypeScript's
  `erasableSyntaxOnly` says: there's no `enum`, parameter property or
  `import =` in the model, and a namespace holding values is refused but a
  `declare`d one's.
- **TypeScript is 7.0.2**, its latest release, pinned, as its API is
  `unstable`: a session is its native process, the async API's, which
  works under Node and Bun alike, where the sync API needs Node's.

## Why

- **It's TypeScript's**: what it reads, TypeScript parsed, and what it
  writes, TypeScript printed, so neither is rust-js's guess at TypeScript.
- **One model, both ways**: what's read can be printed, and printed, read
  as it was, which the module's test checks of a file of every kind of
  declaration; and what rust-js writes and what it reads are the same
  shape, for its generators and for moving TypeScript to Rust.
- **It's tested**: reading a file of each kind of declaration, printing a
  model, the two as one round trip, and `@types/react`'s
  `AnchorHTMLAttributes` taken apart.

## Costs

- **A session is a process of TypeScript's**, which starts in a second
  or so the first time on a machine.
- **The API is `unstable`**: a TypeScript release may change it, and the
  pin is moved deliberately.
- **The model is what rust-js needs**: classes, enums, overloads,
  conditional and mapped types are `other`, written as they were.
