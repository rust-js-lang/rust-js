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

## Amendment (2026-11-25, #2533): declared in its own context

A private type declared where an item names it is declared in its own
context, the type entered (`Declarations::within`) as an item of the
module is, then the item that named it entered again. It was declared in
the namer's, its type parameters' erasures swapped in but not its owner,
so a field's `Option` asked how it holds its payload of the module's
context, the crate root's, which is no item's: a rustc ICE, `generics_of:
unexpected node kind Crate`, of a 17-line program whose private struct of
an `Option` field an impl of another module's trait names first. Corpus
`private_struct_of_an_option_declared_where_an_impl_names_it`; mutation
`private-type-declared-in-the-namers-context`.
