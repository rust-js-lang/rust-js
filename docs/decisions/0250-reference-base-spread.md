# 0250. Props updated from a reference are a spread

Status: Accepted. Extends [0213](0213-props-as-written.md).

## Context

react.dev's CodeDiagram renders each `<pre>` child anew as a CodeBlock,
given the child's own props and two more:

```tsx
<CodeBlock key={child.key} {...child.props} noMargin={true} noMarkers={true} />
```

Its Rust reads the child's props through a reference and updates them,
`{..*props}`. rust-js read each field of them where it is,
`className={props.className} noShadow={props.noShadow} ..`: the same for a
field the props' type names, but any other key of them was lost.

## Decision

**A struct updated from one read whole through a shared reference,
`..*props`, whose other fields need no copy of their own, is that one
spread, then the fields named: `{ ...props, noMargin: true }`, in JSX
`{...props} noMargin`.**

```rust
jsx! { <Panel wide={Some(true)} {..*props} /> }
```

```jsx
<Panel {...props} wide />
```

- **The spread is first, wherever the base is written**: in Rust the
  named fields win, and in JSX what's after does.
- **A field that's copied as it's read**, a `Copy` struct the crate
  changes in place (ADR 0020), keeps each field read on its own.

## Why

- **It's the JS a person writes**, and every key of the props passes.
- **It's tested**: a JSX test updates props through a reference, renders
  the field it names over the base's, and finds a key the type doesn't
  name passed on. Mutations read each field, and put the spread last.
