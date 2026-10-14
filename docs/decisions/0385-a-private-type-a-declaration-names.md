# 0385. A private type a declaration names is declared, not exported

Status: Accepted. Extends [0196](0196-typescript-declarations.md) and
[0210](0210-declarations-import-module-types.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): types, which JS
doesn't have.

## Context

react.dev's DiagramGroup takes props of an interface it doesn't export:

```tsx
interface DiagramGroupProps {
  children: ReactNode;
}

export function DiagramGroup({children}: DiagramGroupProps) {
```

Its Rust's struct isn't `pub`, as the interface isn't exported, and the
`.d.ts` gave the function `props: any`: looser than the original, a
TypeScript caller's props unchecked.

## Decision

**A module's private type that its declarations name is declared in its
`.d.ts`, not exported, before what names it, as TypeScript declares the
interface: `interface DiagramGroupProps { .. }`, then
`export function DiagramGroup(props: DiagramGroupProps)`.**

- **Another module's private type is still `any`**: it isn't this one's to
  declare, and its own module doesn't export it.
- Another crate's untagged enum a declaration names, declared here the same
  way (ADR 0225), is before what names it too.
