# 0869. A web API's callback it calls once is an `FnOnce`

Status: Accepted. Amends [0260](0260-webapi-callbacks.md): a callback
parameter is a `Box<dyn FnMut(..)>`, but one the API calls once.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The JS is the same
closure either way; only what Rust lets the closure do changes.

## Context

React's `act` waits a few microtasks before it warns:

```js
const queueSeveralMicrotasks =
  typeof queueMicrotask === 'function'
    ? (callback: () => void) => {
        queueMicrotask(() => queueMicrotask(callback));
      }
    : queueMacrotask;
```

webapi typed every callback a `Box<dyn FnMut(..)>`, `queueMicrotask`'s
too, so the outer closure couldn't give away the `callback` it holds:
the port kept it in an `Option` to `take()`, which became a `let`, a
`$dropping` and an `if`. TypeScript types it `queueMicrotask(callback:
VoidFunction)`, and the HTML spec runs a microtask once; the js crate's
`setTimeout` already takes a `Box<dyn FnOnce()>`, as the guide says a
callback JS calls once is.

## Decision

**A callback an operation calls once at most is a `Box<dyn FnOnce(..)>`**:
webapi's generator lists those operations (`CALLED_ONCE`), as WebIDL
doesn't say it and each spec's algorithm does: a request for one later
call, `queueMicrotask`, `requestAnimationFrame`, `requestIdleCallback`,
`requestVideoFrameCallback`, `Scheduler.postTask`, `LockManager.request`;
or for one answer, `toBlob`, `getAsString`, `getCurrentPosition`,
`decodeAudioData`, the file system entries' `getFile`, `getDirectory`,
`getParent`, `file` and `readEntries`, `Notification.requestPermission`,
and `RTCPeerConnection`'s legacy callbacks. One JS calls again, a
listener's, an observer's, `watchPosition`'s, `forEach`'s, stays an
`FnMut`.

```rust
|callback| {
    webapi::global::queue_microtask(Box::new(move || {
        webapi::global::queue_microtask(callback);
    }));
}
```

```js
(callback) => {
  queueMicrotask(() => {
    queueMicrotask(callback);
  });
}
```

## Why

- **It's the type the API has**: called once, it may use up what it
  holds, as `FnOnce` says; a closure that's `FnMut` or `Fn` is one too, so
  every caller before still compiles.
- **One rule, the guide's**: a callback JS calls once is `Box<dyn
  FnOnce(..)>`, the js crate's timers' and webapi's alike.

## Alternatives

- **`FnMut` everywhere**: the port's `Option` and `take()`, Rust bent to
  fit the binding.
- **Guess it from WebIDL**: nothing there says how often a callback is
  called; a name, `request*`, isn't a rule (`requestPermission`'s
  promise and callback, but `MediaSession`'s handler is called again).

## What holds it

- `test/webapi-bindings.test.ts`, "a callback a web API calls once moves
  what it holds": React's `queueSeveralMicrotasks`, run, its callback
  after two microtasks; a `requestAnimationFrame` of a closure that
  moves what it took; and an observer's callback still an `FnMut`.
- The generated `webapi/src/lib.rs`, checked against its generator.
