# 0239. A local of a component's function type is that component as a tag

Status: Accepted. Extends [0220](0220-jsx-tag-values.md) and
[0213](0213-props-as-written.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Challenges picks its heading as it renders, and gives it props
by name:

```jsx
const Heading = isRecipes ? H4 : H2;
<Heading id={titleId} className={..}>{titleText}</Heading>
```

`jsx!` read a capitalized local as a DOM element's tag (ADR 0220), whose
attributes are an element's, or as a component only given its props whole:
a local has no props companion of its name, `Heading!`, which a
component's named props are built by.

## Decision

**A capitalized local or parameter written of a component's function type,
`fn(P) -> JSX::Element`, is that component as a tag: its props are built
by `P`'s companion, as a component's are, and it's rendered as the
component it holds.**

```rust
let Heading: fn(HProps<'static, &str>) -> JSX::Element = if is_recipes { H4 } else { H2 };
jsx! { <Heading id={Some(title_id)} className={..}>{title_text}</Heading> }
```

```jsx
const Heading = isRecipes ? H4 : H2;
<Heading id={titleId} className={..}>{titleText}</Heading>
```

- **The type is what says it**: `jsx!` reads the written type, `P` of
  `fn(P) -> ..`, as it reads a local's name; without one, a local is a
  DOM element's tag, as before.
- **Its props are as a component's**: what's written by name, children
  too, and a base, `{..base}`; a `key` is the element's. It takes no `ref`
  nor type arguments, which are of a component's own macro.

## Why

- **It's the JS a person writes**: one local, rendered where it's used.
- **It's Rust's**: the local's type is a function's, which the props are
  checked against, as a component's are.
- **It's tested**: a JSX test picks between two components of one props
  type, renders each with a `key` and props by name, and its mutations
  read the type as nothing and the local as a DOM element's tag.
