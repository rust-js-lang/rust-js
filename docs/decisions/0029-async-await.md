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
  and streams. A type's own `impl Future` is allowed and never lowered,
  and awaiting one, as no JS promise is one, is refused (ADR 0425). Several futures are joined by the js crate's
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

## Amended by

- [0373](0373-one-binding-form.md): A binding of a JS function is an ordinary function, and a JS type a struct

## Amendment: what a future gives isn't adopted

A future is a JS promise, and a promise resolved with a thenable adopts
it, which Rust's future never does: an `async fn` giving a struct with a
function named `then` never resolved, and one giving `Some(future)` was
flattened, `None` (#172). Where a future's output may be a thenable,
`may_be_thenable`'s answer, a future, an `Option` or a `Box` of one, a
struct with a function named `then`, or a type only a caller knows, what
resolves it, an async body's `return` or `ready(v)`, gives
`$settled(value)`, which boxes it only if it's a thenable, and its
`.await` takes `$unsettled(..)` of what it gives. Any other output is as
it was. Held by the lowering test "an awaited value that's a thenable
isn't adopted".

## Amendment: an `async` block drops what it took

An `async move` block, an async arrow called at once, never dropped what
it took: its coroutine's captures had no types where its body was lowered,
and nothing dropped them (#173). Its body drops them as it ends, where
Rust's future, awaited, is dropped; one dropped unawaited has run, as a
promise does, and dropped them then, so dropping the future elsewhere drops
nothing. An `async` closure drops what it took where it's dropped, as a
closure does, and one whose body took part of it drops what's left. What a
closure, an `async` closure or an `async` block holds is one answer,
`held_types`. Held by `test/lowering-functions.test.ts`, "an async block or
closure drops what it took".

## Amendment: a future runs as it's made only where nothing can tell (#965, the user's decision, 2026-11-08)

Case ([0262](0262-when-rust-and-js-disagree.md)): N where nothing can tell
a promise's running as it's made from Rust's future, which runs once it's
polled; R where results change, which is refused, as no lowering of a
future that runs once it's polled is a JS promise.

A rust-js future is a promise, which runs as it's made. Rust's does
nothing until it's polled. So a future held while code that may be seen
runs, then awaited, runs its body before that code in JS and after it in
Rust; one dropped unawaited never runs in Rust and has run in JS. #942's
async table left those cases out, and ADR 0262 listed no exception.

**Kept where nothing can tell, refused where it shows**
(`analysis/unwinding/eager_futures.rs`):

- A future of the crate's, an `async fn`'s, an `async` block's or an
  async closure's, is followed from where it's made through what holds
  it, a move, a borrow, `pin!`, `Box::new`, `Box::pin` or a field of a
  local, to where it's awaited, dropped, given back to its caller or
  given to a binding.
- **Kept:** awaited where it's made, or past only code that can't be seen
  (`unwinding::quiet`'s rule, of each statement and call: no write but to
  the body's own locals, no call but a quiet one, no panic, no drop that
  runs code); an `.await` of what runs nothing, `ready(v)` or a quiet async
  body, pinned or not, is one too. A future whose own body can't be seen,
  `async fn five() -> u32 { 5 }`, is kept wherever it goes, dropped
  unawaited too: its running can't be told. An async fn that only awaits
  the futures its caller gives, `async fn awaited(f: impl Future)`, is
  quiet where each it's given is (`quiet_but_their_parameters`).
- **The JS boundary's:** a future given to a binding, `js::promise`,
  `js::spawn`, `webapi`'s functions, is a running promise by design, as a
  JS caller's is; given back to its caller, its caller's to say.
- **Refused,** naming where: one held past what may be seen and awaited
  after ("code that may be seen runs before it's awaited"), one dropped
  unawaited ("it's dropped unawaited"), and one given where its `.await`
  can't be followed, a Rust function's parameter or a value of another
  crate ("it's given where rust-js can't follow when it's awaited"). To
  start futures together, as the playground's downloads do, each is
  `js::promise(f)`, a running promise held and awaited in turn.

The quiet owner learns async bodies: a coroutine is a candidate, its
`yield` and its end where its future is dropped (`CoroutineDrop`) run
nothing, an `.await`'s `get_context`, `into_future` and `poll` of a quiet
future are quiet (`awaits_quietly`), a drop by a quiet `Drop`, its fields'
too, is (`quiet_drops`), and so are a pin's or a box's making and a pin's
views, and `future::ready`.

The amendment above's "one dropped unawaited has run" (#173) is of a
future whose body can't be seen alone now; `lowering-functions`' test of
it drops no block unawaited. `test/async.rs`'s `held` awaited `double(2)`'s
future past `first.await`:
refused now, it holds two binding promises instead. The playground's
loader (`wasm/web/rust/compiler.rs`) started its downloads by holding
futures: each is `js::promise(..)` now, as the boundary takes them.

Held by the corpus cases `future_held_past_what_is_seen_refused`,
`future_dropped_unawaited_refused`, `future_held_in_a_field_refused`
(`//@ compile-fail`) and `future_held_where_nothing_shows`; the async
table's families "held past nothing seen", "held past a ready future's
await", "dropped unawaited, its body quiet" (as native Rust), and "held
past what's made", "held past an await" and "dropped unawaited"
(refused), ADR 1090, amended; and the mutations of
`lower/analysis/unwinding/eager_futures.ts` and `unwinding.ts`'s `yield-seen`,
`coroutine-drop-seen`, `quiet-drop-impl-seen`, `get-context-seen`,
`parameters-future-awaited-seen`, `pin-views-seen`, `ready-made-seen` and
`ready-awaited-seen`.
