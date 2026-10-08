# 0253. next/link takes a ref and passHref

Status: Accepted. Extends [0192](0192-next.md).

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

## Why

- **It's Next.js's type**, optional both.
- **It's tested**: the Next.js build test compiles a client component's
  link of a ref and `passHref`.
