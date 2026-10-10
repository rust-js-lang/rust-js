# 0352. A name given apart is the name where nothing it'd shadow is read

Status: Accepted. Amends [0038](0038-js-names-and-destructuring.md)'s
"the module's own names are never reused".

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A local never took a module's name (ADR 0038), so a destructured prop of
a module function's name was `{ label: label$1 }`, where the function never
read the module's `label`; and a name JS has, `Error`, was named apart
for good, `import Error$ from "next/error"`, where its module never read
JS's `Error`. react.dev writes `{ label }`, and Next.js's docs `import
Error from "next/error"`. The names are chosen as a body is lowered, before
what it reads is known.

## Decision

**Once a module is lowered, a name given apart from another, `x$1` or
`x$`, is that name where its scope never mentions it:**

- **A local's**, where its item, a function, a type's methods, or a
  constant's value, neither declares nor reads the name anywhere.
- **An import's**, where its module never does.
- **Never a word JS keeps**, `class$`; a global rust-js names apart,
  `Error`, `String`, `Math`, may be the name, where nothing reads the
  global, a template's conversion to a string, `String(n)`, among the
  reads.
- **Not an `on_load!` body's**, whose names are the module's (ADR 0267).

```js
export function show({ label }) { .. }      // was { label: label$1 }
import Error, { catchError } from "next/error";
```

## Why

- **It's the same program**: a name renamed to one its scope never
  mentions keeps every read of it its own, so nothing can tell, but the
  reader.
- **It's what JS writes**: a parameter shadows a function it doesn't call,
  and an import takes a global's name it doesn't use.

## Consequences

- Seven corpus cases, five snapshots, a JSX snapshot, and three react.dev
  modules read shorter: `match`, `entry`, `{ value }`, `<OpenInCodeSandboxButton />`
  of an import named apart from nothing it reads; 0 of react.dev's 823
  pages differ.
- A name its scope reads elsewhere keeps its `$1`, `f$1` beside an `f`.
