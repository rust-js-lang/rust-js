# 0257. A closure that only spawns an async block is an async arrow

Status: Accepted.

## Context

react.dev's BrandMenu copies a color as a menu item's handler:

```tsx
<MenuItem onSelect={async () => { await navigator.clipboard.writeText('#58C4DC'); }}>
```

The handler's Rust is a `Box<dyn Fn()>`, as another item's handler isn't
async: `|| js::spawn(Box::new(async { .. }))`, which rust-js wrote as an
arrow calling an async one:

```js
() => {
  (async () => { await navigator.clipboard.writeText("#58C4DC"); })();
}
```

## Decision

**A closure giving `()` whose body only spawns an async block is that
block's async arrow.**

```js
async () => { await navigator.clipboard.writeText("#58C4DC"); }
```

## Why

- **It runs the same**: calling it runs the block to its first `await`,
  as calling the async arrow did; it gives back a promise where it gave
  `undefined`, which a `()` is never read for.
- **It's the JS a person writes.**
- **It's tested**: a compiler test calls such a handler and finds what
  it spawned done; a mutation writes the arrow again.
