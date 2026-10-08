# 0241. Text right before text is in braces

Status: Accepted. Extends [0040](0040-jsx.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Challenge writes two texts side by side:

```jsx
Challenge {currentChallenge.order} of{' '}
{totalChallenges}
```

Its children are `" of"` and `" "`, two text nodes, which React renders
apart: `1<!-- --> of<!-- --> <!-- -->4`. rust-js wrote each string child
as JSX text, so the two were `of ` together: one text, one node, a page
that differs.

## Decision

**A string child is JSX text, but not right before a child that is JSX
text: then it's in braces.**

```rust
jsx! { <p>{"Challenge"}{" "}{order}{" of"}{" "}{total}</p> }
```

```jsx
<p>
  {"Challenge"} {order}
  {" of"} {total}
</p>
```

- **The earlier is in braces, not the later**: oxfmt, as Prettier, makes
  a `{" "}` within a line a space, of the text before it, which would join
  them again. A text after a string in braces stays text.
- **It's read from the last child back**, so of three, `a{"b"}c`.

## Why

- **It renders what React renders**: each string, a text node of its own.
- **It's the JSX a person writes**: text as text, braces only where JSX
  needs them.
- **It's tested**: a JSX test renders adjacent strings to HTML, with
  React's marks between text nodes; a mutation writes every string as
  text.
