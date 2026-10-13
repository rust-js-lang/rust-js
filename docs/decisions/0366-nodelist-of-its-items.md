# 0366. A `NodeList` of its items' type, as TypeScript's `NodeListOf`

Status: Accepted. Extends [0024](0024-web-crate.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

WebIDL's `querySelectorAll` gives a `NodeList`, a list of `Node`s; the
webapi crate, generated from WebIDL, gave the same. TypeScript's DOM gives
a `NodeListOf<Element>`, and an input's `labels` a
`NodeListOf<HTMLLabelElement>`: a page reads an item's rect, or a label's
`htmlFor`, as it is. react.dev's home page does,
`Array.from(nodes).map((node) => node.getBoundingClientRect())`, and its
port had to cast each `Node` to the `Element` TypeScript says it is.

## Decision

**webapi's `NodeList` is `NodeList<T = Node>`, TypeScript's
`NodeListOf<T>`: its members give and take a `T`, and a member gives the
list TypeScript says it does.**

- `NodeList<T>`'s `item`, `values`, `entries`, `for_each` and
  `node_list::iter` are of `T`. A plain `NodeList` is of `Node`s, as
  before.
- The generator reads what each member's `NodeListOf<X>` lists from
  TypeScript's DOM, or its type parameter's constraint,
  `querySelectorAll<E extends Element>`'s `Element`; where `X` is no
  webapi type (`childNodes`' `ChildNode`, a union of SVG elements), it
  stays `Node`.
- Declarations name it `NodeListOf<T>`.

## Why

- **Types as TypeScript's** (ADR 0024 follows WebIDL for what's there,
  TypeScript for how it's typed): a port reads an item as the original
  does, no cast.
- **One type**: a default type parameter keeps every `&NodeList` as it
  was, where a second type, `NodeListOf`, would be two spells for one list.

## Alternatives

- **Keep `Node`s, cast at the use**: an `unsafe` cast in every program
  where TypeScript needs none.
- **`querySelectorAll` generic, `query_selector_all::<HTMLElement>(..)`**:
  TypeScript's `E` is a cast it doesn't check; the caller who wants one
  casts the item, as TypeScript's default leaves them to.

## Consequences

- `RadioNodeList` derefs to `NodeList<Node>` where TypeScript's extends
  `NodeListOf<HTMLInputElement>`; the generator reads extends by name only.
- `test/web_forms.rs` reads a `querySelectorAll` item's rect and a label's
  `htmlFor`; `test/declarations.test.ts` holds the `NodeListOf` names.

## Since

2026-10-13.
