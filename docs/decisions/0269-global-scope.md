# 0269. What every global scope has is called bare

Status: Accepted. Extends [0024](0024-web-crate.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's errors page fetches React's error codes in Next.js's
`getStaticProps`, which runs in Node:

```tsx
await fetch('https://raw.githubusercontent.com/react/react/main/scripts/error-codes/codes.json')
```

The webapi crate binds `fetch` as a window's, `window::fetch(window, url)`,
`window.fetch(url)`, and Node has no `window`. WebIDL says `fetch` is the
`WindowOrWorkerGlobalScope` mixin's, which every JS global scope has, a
window's, a worker's, and Node's.

## Decision

**`webapi::global` is that mixin's functions, called bare**:
`global::fetch(url)` is `fetch(url)`, and `queue_microtask`,
`structured_clone`, `report_error`, `atob` and `btoa` are too. Not its
timers, whose handler there is text to run: the builtins crate's take a
closure (ADR 0102).

## Why

- **It's the JS a person writes**, and it runs wherever JS does.
- **It's tested**: a compiler test fetches and queues a microtask, against
  globals of its own, with no `window` in its JS.

## Amendment: a window's own functions, `webapi::window_global`

A window's own operations are called bare too, from
`webapi::window_global`: `window_global::confirm_with_message("..")` is
`confirm("..")`, as TypeScript's DOM declares them globals and react.dev's
NavigationBar calls it. They're apart from `webapi::global`, which keeps to
what every scope has: Node has no `confirm`. `window.confirm(..)` is still
the method, as TopNav's `window.matchMedia(..)` is written.
