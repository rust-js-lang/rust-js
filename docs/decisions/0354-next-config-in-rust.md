# 0354. An app's `next.config` is Rust

Status: Accepted. Part of [0346](0346-next-coverage.md).

## Context

Next.js reads the app's config from `next.config.js`, its default export:
the config, or a function of the phase that gives it, as `NextConfig`
types it. rust-js-next compiles the crate before Next.js starts (ADR
0192), so a `next.config.rs` in the crate is `next.config.js` by the time
Next.js reads it. What was missing was `NextConfig` itself, 65 options,
and a custom server's `next()`, which takes one too.

## Decision

**`next::NextConfig` is `NextConfig`, and `next::config` holds its options'
types; the app's config is a function its module exports by default:**

```rust
pub fn nextConfig(_phase: &str) -> NextConfig<'static> {
    NextConfig { powered_by_header: Some(false), ..Default::default() }
}

js::export_default!(nextConfig);
```

```js
export function nextConfig(_phase) {
  return { poweredByHeader: false };
}

export default nextConfig;
```

- **A function, not a `static`**: a config's callbacks, `headers` and the
  others, are boxed closures, which no `static` holds.
- **`Promise<T> | T` callbacks are `Promise<T>`**, the form Next.js's docs
  write, `async headers() {}`; `js::promise(async { .. })` in a closure is
  an async arrow.
- **Each union is an untagged enum**, a string literal of it a unit
  variant (ADR 0214); `false | {..}` is `Bool(bool)` beside the object.
- **`Record<string, T>` is `&Dict<T>`**, and `any`, webpack's config and
  Sass's options, `&Unknown` given and `Json` taken.
- **`experimental` isn't bound yet**: its absence is the coverage's `-`.

## Consequences

- next's members: 381 of 433, `NextConfig` 64 of 65.
- `node:http`'s `createServer` and `next()` run a custom server of the
  config in the build test, as `node server.js`.
