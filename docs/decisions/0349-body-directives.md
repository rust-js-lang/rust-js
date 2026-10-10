# 0349. A body's `js::directive!` is its function's or its closure's

Status: Accepted. Extends [0192](0192-next.md)'s module directive.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`js::directive!("use client");` is a module's directive, its first
statement (ADR 0192). Next.js has directives of a function too: a Server
Action written in the Server Component that uses it, `"use server"` first in
its body, which captures what the component has; and a `"use cache"`
function or component, the way Next.js 16 caches anything. Neither could be
written: a `js::directive!` in a body, a `const _` with an attribute there,
was taken as its module's, since a module's items are the ones nested in its
functions too.

## Decision

**A `js::directive!` written in a function's body, or a closure's, is that
body's directive, its first statement; one at a module's top is the
module's.**

```rust
pub async fn cached(n: u32) -> u32 {
    js::directive!("use cache");
    n + 1
}

let lighten = async |_: &'static FormData| {
    js::directive!("use server");
    cookies().await.set("theme", "light");
};
```

```js
export async function cached(n) {
  "use cache";
  return (n + 1) >>> 0;
}

const lighten = async () => {
  "use server";
  (await cookies()).set("theme", "light");
};
```

- **Whose it is** is its item's parent, as rustc has it: a module, a
  function, or a closure, past the coroutine an `async fn`'s or an `async`
  closure's body is.
- **One with no function to be first in**, in an `async` block or a
  constant's value, is an error, at it.
- **It's the body's prologue**, printed as JS has a directive, before
  what's hoisted.

## Why

- **It's how Next.js writes them**: an inline Server Action and a
  `"use cache"` function are a directive in a body, which Next.js's
  compiler finds there, and nowhere else.
- **One spell for one job**: the same macro, its place telling whose it is,
  as JS's own directive is.

## Consequences

- `test/next.test.ts` submits a Rust closure's Server Action in a real
  Next.js app: it sets a cookie, and the route is rendered again with it.
- A `"use cache"` function needs Next.js's `cacheComponents` config, as one
  in JS does.
