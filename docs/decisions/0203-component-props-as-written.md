# 0203. A component's props are as written

Status: Accepted. Amends [0192](0192-next.md) and [0200](0200-binding-props-borrow.md).

Case: N, A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A component's props were written in its struct's order, a cost ADR 0192
named: react.dev's `ButtonLink` gives next/link's `Link` its classes,
its rest, its `aria-label` and its `target`, in that order, which React
writes as the `<a>`'s attributes, but rust-js wrote `target` first, as
`LinkProps` declares it, and the page wasn't react.dev's. And as its
children came before a field a base gave, each prop was read into a
`const` before them.

## Decision

**A component's props are written as `jsx!` is given them, its children
last, then what a base gives, in its struct's order: what Rust makes, in
the order it makes them.**

```rust
<Link href={href} className={Some(classes.as_str())} rest={rest} aria-label={label} target={Some(target)} {..Default::default()}>
```

```jsx
<Link href={href} className={classes} {...rest} aria-label={label} target={target}>
```

- **A prop is read before the children only where the order shows**:
  one after them, a base's, that does something, or that reads what
  children that do something change, `<Tally {..base}>{bump(&mut base)}</Tally>`.

## Why

- **It's JSX's**: a person writes props in an order, and React keeps it,
  in the attributes it writes and in which of two a spread replaces.
- **It's Rust's order**: a struct literal's fields are made as written,
  its base read after them, so the JS makes them as Rust does.
- **It's tested**: a JSX test writes a component's props against its
  struct's order and finds them as written, with nothing read into a
  `const`, and one whose children change its base renders what Rust's
  does; `test/next.test.ts`'s `Link` is given its rest first.
