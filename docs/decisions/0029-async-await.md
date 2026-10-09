# 0029. `async`/`.await` are JS's `async`/`await`; a future is a promise

Status: Accepted.

Case: B, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

rust-js's own playground does its work through promises: `fetch`,
`WebAssembly.instantiate`, and timers. To write it in Rust, rust-js needs
async code. How the others do it (checked in local clones):

- **ReScript** has `async` and `await` keywords, and a `promise<'a>` type
  for JS promises. `let get = async key => await I.get(key)` compiles to
  `let get = async key => await I.get(key);`.
- **Scala.js** has `js.async { .. }` and `js.await(p)` on a
  `js.Promise[A]`. They compile to an async arrow and a JS `await`
  (`Closure` with the `async` flag, and `JSAwait`, in its IR).

In Rust, an `async fn` returns a *coroutine*, a state machine that does
nothing until something polls it. rustc writes `e.await` as a loop:

```text
match IntoFuture::into_future(e) {
    mut __awaitee => loop {
        match Future::poll(Pin::new_unchecked(&mut __awaitee), get_context(_task_context)) {
            Ready(result) => break result,
            Pending => {}
        }
        _task_context = yield ();
    }
}
```

Native Rust needs an executor (tokio, or `wasm-bindgen-futures` in the
browser) to drive that loop. JS has one built in: the event loop.

## Decision

**Async Rust is async JS, one to one.**

| Rust | JS |
|---|---|
| `async fn f(x: u32) -> u32 { .. }` | `async function f(x) { .. }` |
| `e.await` | `await e` |
| `async move { .. }` | `(async () => { .. })()` |
| `async \|y\| ..` | `async (y) => ..` |
| a future (`impl Future`, `dyn Future`, a `webapi::Promise<T>`) | a JS promise |
| `spawn(Box::new(async move { .. }))` | `(async () => { .. })();`, not awaited |

- **An `async fn`'s body is its coroutine's body.** rustc moves each
  parameter into the coroutine with `let x = x;`. In JS it's one function,
  so the inner `x` is the parameter itself. A pattern parameter (`(a, b)`)
  arrives as rustc's `__arg0`, named `param`, as in a plain `fn`.
- **`.await` is recognized whole**, like `for` (ADR 0025): the `match`
  on `IntoFuture::into_future(e)` whose one arm is the poll loop.
  `yield`, `poll` and the task context never reach the JS.
- **JS promises have a Rust type**, `webapi::Promise<T>`, which implements
  `Future` with `Output = T`, so rustc accepts `.await` on it. Its `poll` is
  never compiled: rust-js turns `.await` into `await`. The webapi crate's
  WebIDL promise results are now included, with the Fetch Standard:
  `window::fetch_with_str(window, url).await` is `await window.fetch(url)`,
  and `response::text(r).await` is `await r.text()`. `extern` functions
  can return one too:

  ```rust
  #[link_name = "node:timers/promises#setTimeout"]
  safe fn later(ms: u32, value: u32) -> Promise<u32>;
  ```
- **`webapi::spawn`** runs a future without waiting for it, for event handlers.
  It's an `extern` function of the `"this"` form (ADR 0024), so it's the
  promise itself.
- **A rejected promise throws at its `await`**, the way a panic does. In
  the playground, a panic in async code is reported like any other.

**One difference from Rust, chosen on purpose: a future starts when it's
made.** A Rust future does nothing until it's first polled. A JS promise runs
at once, up to its first `await`. So the code before a future's first
`.await` runs when the future is created, and a future that's never awaited
still runs. `test/async.rs` and the countdown example pin this down.

## Why

- **The JS reads like the Rust**, and like hand-written JS. ReScript and
  Scala.js do the same.
- **No runtime.** The event loop is the executor. There's no state machine,
  no waker and no polling in the output.
- **JS callers get what they expect**: an exported `async fn` returns a
  promise they can `await`.
- **The difference rarely shows** in UI code, which `.await`s what it
  starts. When it does show, what happens is what the JS says.

## Alternatives

- **Keep Rust's laziness**: a future becomes a function that starts the
  work (`() => promise`), and `.await` calls it. Every future would then need
  handling by its type (a JS promise or a Rust future), JS callers of an
  `async fn` would get a function, and the output would stop reading like
  JS.
- **Compile the state machine**, from MIR, with an executor in JS like
  `wasm-bindgen-futures`: faithful, but unreadable, and big.
- **Generators** (`function*` and a driver), as Babel did before ES2017:
  only worth it where JS has no `async`, and it has.

## Consequences

- `Promise` and `spawn` live in the webapi crate, though they're JS and not
  the web platform. A `js` crate may take them later.
- `extern` functions can't be generic, so a program declares `new Promise`
  for each result type it needs. The countdown example declares one for
  `Promise<()>`.
- Rust has no `#[test] async fn` (rustc rejects it), so tests can only see
  what runs before the first `.await`. The countdown's test checks that
  a click shows "3" at once.
- A future handed on unawaited, to a binding that takes a promise, is
  `js::promise(f())`: in JS the call's promise itself, as an `async fn`'s
  call is one already.
- Not yet: `async` functions in traits, `IntoFuture` for your own types,
  and streams. Several futures are joined by the js crate's
  `promise::all` (ADR 0283), `Promise.all`.
- A pattern parameter, `(a, b)` or `Context { params, .. }`, Rust gives
  the future as `__arg0`, taken apart in its body. Its JS takes it apart
  where it's given, `([a, b])` or `({ params })`, as a plain `fn`'s does,
  where the body's first statements only read its parts and nothing else
  reads it: as react.dev's errors page's `getStaticProps` takes Next.js's
  context. (Amended: it was `param`, read part by part.)
- An awaited reference given to a generic `&T`, `same(text().await)`, is
  reborrowed by rustc inside the `.await`, `&*loop {..}` its arm: still
  `await`, as a reborrow is the reference itself in JS. (Amended: it was
  rejected, as a call of `IntoFuture::into_future`.)

## Amendment: a closure of an `async` block

`move || async move { .. }`, a closure whose body is an `async` block, is
`async () => { .. }`, as react.dev's useSandpackLint writes its
`loadLinter`. It was the block called in a closure, `() => (async () => {
.. })()`: calling either runs the block up to its first `await` and gives
its promise. Rust writes it so where an `async` closure's future would
borrow the closure. A compiler test runs one, spawned; a mutation keeps
the call.
