# 0255. next/router's events

Status: Accepted. Extends [0192](0192-next.md).

## Context

react.dev's usePendingRoute shows a route as pending a moment after
Next.js starts going to it, by the router's events:

```js
events.on('routeChangeStart', handleRouteChangeStart);
return () => events.off('routeChangeStart', handleRouteChangeStart);
```

The next crate's `NextRouter` had no `events`.

## Decision

**`NextRouter::events` is `router.events`, a `MittEmitter`, whose `on`
and `off` take a `RouterEvent` and a handler of the URL, `&'static dyn
Fn(&str)`.**

```rust
let started: &'static dyn Fn(&str) = Box::leak(Box::new(|url: &str| ..));
events.on(RouterEvent::RouteChangeStart, started);
move || events.off(RouterEvent::RouteChangeStart, started)
```

```js
const started = (url) => { .. };
events.on("routeChangeStart", started);
return () => { events.off("routeChangeStart", started); };
```

- **Its events are those whose handler is given the URL first**, all of
  @types' `RouterEvent` but `routeChangeError`, whose first is its error.
- **A handler is kept**, `'static`, as mitt keeps it till `off` is given
  the same function: a leaked box is the closure itself (ADR 0238).

## Why

- **It's the JS a person writes.**
- **It's tested**: the Next.js build test compiles a hook giving a
  handler on and off.
