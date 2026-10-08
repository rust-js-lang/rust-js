# 0249. An element's key may be none

Status: Accepted. Extends [0041](0041-react.md).

## Context

react.dev's CodeDiagram gives each child it renders anew its own key,
which a child may not have:

```tsx
<CodeBlock key={child.key} {...child.props} noMargin={true} />
```

@types/react types a key `key?: Key | null`; the react crate's `Key` was
a string or a number only, so an element's own, `element.key()`, an
`Option`, wasn't one.

## Decision

**An `Option` of a key is a key: `None` is no key.**

```rust
jsx! { <li key={element.key()}>{..}</li> }
```

```jsx
<li key={element.key}>..</li>
```

## Why

- **It's React's type**, and JS's: `undefined` is no key.
- **It's tested**: a JSX test renders a list keyed by a key and by none,
  and reads React's keys of them.
