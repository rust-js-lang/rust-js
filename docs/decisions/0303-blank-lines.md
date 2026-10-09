# 0303. Top-level statements that span lines are set apart

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

oxc prints a module's statements back to back. rust-js put a blank line
before each function, and after a line `};`, read from oxc's text before
it was laid out (ADR 0117). So react.dev's CustomPreset, three memoized
components, came out as one block:

```js
export const CustomPreset = memo(function CustomPreset({ .. }) {
  ..
});
const SandboxShell = memo(function SandboxShell({ .. }) {
```

and a constant oxc wrote across lines, which oxfmt puts on one,
`const ORIGIN = { x: 0, y: 0 };`, was set apart from the next.

## Decision

**Two top-level statements are set apart by a blank line where either is
a function or spans lines, as laid out.** The rule reads the laid-out
module's parse, so a line of a template literal is never taken for a
statement, and the map is moved to match.

- One-line statements stay together: `const a = 1;` and `const b = 2;`.
- A module's imports stay one block (ADR 0295), an import across lines
  too.

## Why

- **It's the JavaScript a person writes**: react.dev's modules separate
  their components and multi-line constants.
- **It's the same program**: a blank line between statements changes
  nothing.
