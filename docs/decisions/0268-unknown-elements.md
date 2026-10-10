# 0268. An element's type, key and props may be JS values of any shape

Status: Accepted. Extends [0225](0225-unknown-values.md) and
[0234](0234-element-type.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's errors page revives its elements from JSON: each is an array
of a type, a key and props, the type an MDX component's name, or
`"wrapper"` for a fragment:

```tsx
let Type = val[1];
if (Type === 'wrapper') {
  Type = Fragment;
}
return <Type key={key} {...props} />;
```

Read from JSON, each is a `js::Unknown` (ADR 0225). React's `ElementType`
was made of a tag or a component only, a key of text or numbers, and
JSX's spread only of a struct or a `Rest`.

## Decision

- **`ElementType::from_unknown(value)`** is the value as what JSX renders,
  unchecked: the value itself.
- **`react::FRAGMENT`** is React's `Fragment` as a value, an `ElementType`.
- **A `js::Unknown` is a key**, as React takes any.
- **A JS value of any shape is spread as props**, its own properties: a
  `js::Unknown`, or another JS object type.

```rust
let mut Type = ElementType::from_unknown(r#type);
if matches!(classify(r#type), Kind::String("wrapper")) {
    Type = FRAGMENT;
}
jsx! { <Type key={key} {...props} /> }
```

```js
let Type = type;
if (type === "wrapper") {
  Type = Fragment;
}
return <Type key={key} {...props} />;
```

## Why

- **It's what JSX does with these values**, as the site does.
- **It's tested**: a JSX test renders a tag and a wrapper from values of
  any shape, keyed by a number; a mutation refuses their props.

## Since

- **An element and an element's type are JS values of any shape too**,
  `js::Defined`, as a reviver gives `Fragment` or the element it made:
  `js::unknown_of(jsx! { <b /> })` is the element itself.
- **Props that may be none are spread too**, an `Option` of a JS value:
  `{...props}`, which JS spreads as nothing where it's `undefined`, as a JSON
  element may have no props.
- **A type that may be none is one too**: `ElementType::from_unknown` takes
  an `Option` of a value, as TypeScript's `any` may be `undefined`, React's
  to refuse when it renders. `let Type = ElementType::from_unknown(Type);`
  is `Type` itself (ADR 0277): `<Type key={key} {...props} />`, as the
  errors page has it.

- **A struct spread through a reference is the struct's spread**:
  `{...&rest}`, as react.dev's Image spreads `rest` and reads
  `rest.src` after it, is `{...rest}`. A reference is the object it's
  of (2026-10-10).
