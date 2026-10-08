# 0218. What JSX holds stays where it's written

Status: Accepted. Amends [0040](0040-jsx.md): a handler or child that
prints on several lines stays in its JSX, where it went in a `const` first.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

oxc's printer indented code by the statement it was in, not by where it
sat in JSX, so ADR 0040 put a handler or child it would print on several
lines in a `const` first, named after the attribute: `const onKeyDown =
(e) => { .. };`, `const items = crumbs.map((crumb) => { .. });`. Since ADR
0065, oxfmt lays the JS out, as Prettier does, and indents what JSX holds
where it is. react.dev's own components write both in place:
`breadcrumbs.map((crumb, i) => ..)` in their `<div>`, a handler of several
statements in its `onClick`.

## Decision

**A handler, a list or any other child stays in its JSX, of one line or of
several, as a person writes it:**

```jsx
<input
  onKeyDown={(e) => {
    if (e.key === "Enter") {
      add();
    }
  }}
/>
<div>
  {crumbs.map((crumb) => {
    const t = crumb.title;
    ..
  })}
</div>
```

- **What keeps Rust's order still goes first**: a prop read before a child
  whose statements do more than read (ADR 0194), and an element's inputs
  before a later operand's statements, as before. An input that reads the
  same wherever it's read stays: a constant, a function, a variable nothing
  writes again, and a comparison of them, `===`, `!==`, `== null` or `!`,
  which runs no code of its own. `{version === "canary" ? .. : undefined}`
  stays in place before `status.filter(..)`'s `const`; `n === 0` before a
  later child's `n += 1` still goes first.
- **A handler of one call is still an arrow of it**, `() => setCount(1)`,
  as React ignores what it returns.

## Why

- **It's the JS a person writes**, and oxfmt lays it out as Prettier would.
- **Nothing moves**, so nothing about the order it runs in needs proving.
