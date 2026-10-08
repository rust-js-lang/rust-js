# 0242. A `dyn ReactNode` is the node itself

Status: Accepted.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's ErrorDecoder splits a message at its links, a list of text
and elements, `React.ReactNode[]`:

```tsx
return segments.map((message, i) => {
  if (i % 2 === 1) {
    return <a key={i} href={message}>{message}</a>;
  }
  return message;
});
```

@types/react's `ReactNode` is one type, of every node. React's crate has
it as a trait, which each node's type implements, so a list of nodes of
two types had no Rust type: rust-js refused `dyn ReactNode`, and
`Box<dyn ReactNode>` wasn't a node.

## Decision

**`Box<dyn ReactNode>` and `&dyn ReactNode` are a node of any type: in JS,
the node itself, and to TypeScript, a `ReactNode`.**

```rust
fn urlify(text: &str) -> Vec<Box<dyn ReactNode>> {
    text.split('|').enumerate().map(|(i, part)| -> Box<dyn ReactNode> {
        if i % 2 == 1 {
            return Box::new(jsx! { <a key={i} href={part}>{part}</a> });
        }
        Box::new(part.to_string())
    }).collect()
}
```

```jsx
function urlify(text) {
  return text.split("|").map((part, i) => {
    if (i % 2 === 1) {
      return <a key={i} href={part}>{part}</a>;
    }
    return part;
  });
}
```

- **It's what `dyn Any` is already**: a trait of no methods has nothing
  to look up, so a `dyn` of it is the value, as a `Box` is.
- **Its `.d.ts` type is the trait's**, `#[rust_js::types =
  "react#ReactNode<>"]`, as a parameter bound by it is (ADR 0196).

## Why

- **It's the JS a person writes**: the string and the element, returned.
- **It's Rust's**: a value of any type a trait has is a `dyn` of it.
- **It's tested**: a JSX test renders a list of text and links, and a
  node by `&dyn`; a declarations test types a list of them
  `ReactNode[]`. Mutations refuse the `dyn` and type it a function.
