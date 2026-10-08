# 0259. A module's constant element, and an image's title

Status: Accepted. Extends [0113](0113-plain-rustc.md) and
[0192](0192-next.md).

## Context

react.dev's TopNav makes its icons once, as a module's constants, and
renders them in its buttons; its logo is an image with a tooltip:

```tsx
const darkIcon = <svg ..>..</svg>;
<button ..>{darkIcon}</button>
<Image alt="logo by @sawaratsuki1004" title="logo by @sawaratsuki1004" .. />
```

A module's constant is a `thread_local!`'s static, but a `jsx!` among its
tokens was never rust-js's: they're the macro's, not expressions, so rustc
expanded it as a plain rustc's placeholder. Nor could the element be read
out of it, as `JSX::Element` wasn't `Copy`. next/image's props had no
`title`.

## Decision

- **A `thread_local!`'s `jsx!` calls are rust-js's**, wherever they are
  among its tokens, `Some(jsx! { .. })` too.
- **`JSX::Element` is `Copy`**: React never changes an element once it's
  made, so a copy is the element itself.
- **next/image's `Image` takes a `title`**.

```rust
thread_local! {
    static darkIcon: JSX::Element = jsx! { <svg ..>..</svg> };
}
jsx! { <button ..>{darkIcon.with(|icon| *icon)}</button> }
```

```js
const darkIcon = <svg ..>..</svg>;
<button ..>{darkIcon}</button>
```

## Why

- **It's the JS a person writes**: an element made once, read where it's
  rendered.
- **It's tested**: a JSX test renders a module's constant elements, one of
  them inside `Some`, twice; mutations leave each to rustc. The Next.js
  build test titles an image.
