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

## Amendment: a global scope's attributes are its module's functions (rd195)

React DOM's `estimateImageBytes` reads the window's `devicePixelRatio`
where there may be no window, `typeof devicePixelRatio === 'number' ?
devicePixelRatio : 1`; webapi had it only as `window.device_pixel_ratio()`,
which throws where no `window` is. **A global scope's attributes are
functions of its module, read bare at each call**, as TypeScript declares
them, `declare var devicePixelRatio: number`: `webapi::window_global::
device_pixel_ratio()` is `devicePixelRatio`, and `webapi::global::
is_secure_context()` `isSecureContext`, beside its operations. Not an
event handler property, nor one a global value names already, `document`,
`window`, `performance` and the rest of `GLOBALS`: one way to read each.
Getters only; a setter of a global is `globalThis`'s, as above.

```rust
pub fn pixel_ratio() -> f64 {
    if js::is_number(webapi::window_global::device_pixel_ratio) {
        webapi::window_global::device_pixel_ratio()
    } else {
        1.0
    }
}
```

```js
export function pixelRatio() {
  return typeof devicePixelRatio === "number" ? devicePixelRatio : 1;
}
```

Its test is upstream's, `typeof devicePixelRatio === 'number'`, the user's
decision: `js::is_number`, ADR 0942's third question (amended). It had
been ADR 0597's `typeof X !== "undefined"`, which differs of a
`devicePixelRatio` a script replaced, as `[Replaceable]` lets one, with
what isn't a number.

Held by `test/compiler.test.ts`, "the webapi crate's bindings become plain
JS" (web_forms' `pixel_ratio`, its JS, run with and without the global),
and `test/webapi-coverage.test.ts`. No mutation: the generator's.

## Amendment: a window's listener functions are called bare (2026-10-30, #431)

TypeScript's DOM declares `addEventListener` and `removeEventListener`
globals, of a window's event map, and React DOM's Fizz runtime calls the
bare one, `addEventListener('submit', ..)`: the global scope's, where a
page's `window` may be another, as React's tests' JSDOM window is, so
`window.addEventListener` listens elsewhere. **`window_global::add_event_listener(event,
listener, options)` and `remove_event_listener` are those globals**,
each typed as a window's method is (`Window: Listen<E>`), its listener a
closure of the event or one JS gave (ADR 1021). Case
([0262](0262-when-rust-and-js-disagree.md)): none, the JS the program
names. Held by `webapi-bindings.test.ts`'s "a window's listener functions
are called bare, typed by its events", run in Node.

## Amendment (2026-10-31): a global read as the global object's property (rd319)

React's SchedulerPostTask holds the postTask scheduler at load, `const
scheduler = global.scheduler;`, a property of the global object, which is
`undefined` where the host lacks it, Node, Safari. Its port's
`webapi::global::scheduler()` reads the bare `scheduler`, which throws a
`ReferenceError` as the module loads there: the module held where
upstream's loads.

**`webapi::global_this` is the `globalThis` global, the global object, a
`GlobalThis` of a window's members through its `Deref`, as TypeScript's
`typeof globalThis` has them**: its getters read the global as its
property, `global_this.scheduler()`'s `globalThis.scheduler`. A type of
its own, not a `Window`, so `js::is_defined(Window::new_error_event)`'s
global is still `window`'s, the one static of a `Window`. The one form of a property read of the global
object, a static at the crate root beside `window`, which the generator
writes (`GLOBALS`); a bare read stays `webapi::global`'s. `globalThis`,
not Node's `global`, as every host has it. Case
([0262](0262-when-rust-and-js-disagree.md)): N, the read upstream writes,
`undefined` where the host lacks it, as TypeScript types it.

Held by `test/webapi-bindings.test.ts`'s "a global read as the global
object's property is undefined where the host lacks it", run in a host
without `scheduler`, the bare read throwing beside it; and
`test/webapi-coverage.test.ts`, which holds the crate to what the
generator writes.

