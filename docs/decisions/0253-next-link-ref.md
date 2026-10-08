# 0253. next/link takes a ref and passHref

Status: Accepted. Extends [0192](0192-next.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's SidebarLink scrolls its link into view, by a ref to it, and
passes its `href` on:

```tsx
<Link href={href} ref={ref} title={title} target={target} passHref ..>
```

Next.js's `Link` is a `ForwardRefExoticComponent` of React's
`RefAttributes<HTMLAnchorElement>`, with `passHref?: boolean`; the next
crate's `LinkProps` had neither.

## Decision

**`LinkProps` has `pass_href`, `passHref`, and `r#ref`, its `<a>`'s
ref, each `None` unless given.**

```rust
jsx! { <Link href="/" ref={Some(anchor)} passHref={Some(true)}>{"Home"}</Link> }
```

```jsx
<Link href="/" ref={anchor} passHref>Home</Link>
```

- **Its children are its last field**, as Next.js types them and JSX
  makes them: after the props, so a class a call makes is made first,
  where it's written, and nothing is held for the children. (Amended: they
  were second, which Rust made before any prop after them.)

## Why

- **It's Next.js's type**, optional both.
- **It's tested**: the Next.js build test compiles a client component's
  link of a ref and `passHref`, and of a class and children each a call
  makes, in JSX's order.
