# Promises and async

An `async fn` is an `async function`, `.await` is `await`, and a future is
a JS promise. There's no executor to set up: JS's event loop runs it.

Every example was compiled by rust-js as it is today. The JS shown is what
it writes, without its header comment and source-map line.

## Contents

- [`async fn` and `.await`](#async-fn-and-await)
- [A future starts when it's made](#a-future-starts-when-its-made)
- [Run a future without waiting: `js::spawn`](#run-a-future-without-waiting-jsspawn)
- [`js::Promise`](#jspromise)
- [Rejections](#rejections)
- [Fetch](#fetch)
- [Timers](#timers)
- [Many promises at once](#many-promises-at-once)
- [Streams](#streams)
- [What's refused](#whats-refused)

## `async fn` and `.await`

```rust
pub async fn double_later(n: u32) -> u32 {
    let half = half_later(n).await;
    half * 4
}

async fn half_later(n: u32) -> u32 {
    n / 2
}
```

```js
export async function doubleLater(n) {
  const half = await halfLater(n);
  return (half * 4) >>> 0;
}

async function halfLater(n) {
  return (n / 2) >>> 0;
}
```

- **A JS caller gets a `Promise`**: `await doubleLater(10)` is `20`.
- **An `async` block and an `async` closure are async arrows**:
  `async |id: u32| id * 10` is `async (id) => ..`.
- **An `async fn` in a trait** is an async function in its impl's
  dictionary, awaited where it's called.
- **`std::future::ready(v)` is `Promise.resolve(v)`** ([ADR 0571](../../decisions/0571-pin-macro-and-a-ready-future.md)), and
  `pin!` of a future is the promise itself.

## A future starts when it's made

In Rust a future does nothing until it's polled. In rust-js it's a
promise, which runs to its first `await` at once and on to its end
though nothing awaits it ([ADR 0029](../../decisions/0029-async-await.md)). That's what lets JS call an
`async fn` and get an ordinary promise, with no Rust executor. A program
that relies on a future not having started yet sees it run.

## Run a future without waiting: `js::spawn`

An event handler can't `await`. `js::spawn` starts a future that no one
awaits:

```rust
# mod later {
#     pub async fn double_later(n: u32) -> u32 {
#         let half = half_later(n).await;
#         half * 4
#     }
#
#     async fn half_later(n: u32) -> u32 {
#         n / 2
#     }
# }
#
# use later::double_later;
#
pub fn start() {
    js::spawn(Box::new(async {
        let n = double_later(10).await;
        println!("{n}");
    }));
}
```

```js
import { doubleLater } from "./later.js";

export function start() {
  (async () => {
    const n = await doubleLater(10);
    console.log(`${n}`);
  })();
}
```

The promise is left unawaited: a panic inside it is an unhandled
rejection, which the host reports.

## `js::Promise`

`js::Promise<T>` is a JS promise of a `T`. A binding to a JS function
that returns a promise returns one, and `.await` on it is `await`
([Write bindings](bindings.md)):

```rust
use js::{JsError, Promise};

#[cfg_attr(rust_js, rust_js::link_name = "./api.js#loadUser")]
pub fn load_user(id: u32) -> Promise<String> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./api.js#saveUser")]
pub fn save_user(name: &str) -> Promise<Result<u32, &'static JsError>> {
    unreachable!()
}

pub async fn rename(id: u32) -> String {
    let name = load_user(id).await;
    match save_user(&name).await {
        Ok(saved) => format!("saved {name} as {saved}"),
        Err(error) => format!("not saved: {}", error.message()),
    }
}
```

```js
import { loadUser, saveUser } from "./api.js";
import { $settle } from "@rust-js/runtime";

export async function rename(id) {
  const name = await loadUser(id);
  const value = await $settle(saveUser(name));
  if (value.TAG === "Ok") {
    const saved = value._0;
    return `saved ${name} as ${saved}`;
  }
  const error = value._0;
  return `not saved: ${error.message}`;
}
```

The `js` crate has the rest of JS's `Promise`:

| JS | Rust |
|---|---|
| `new Promise((resolve, reject) => ..)` | `js::promise::new(\|resolve, reject\| ..)`, which runs before it returns, so it may set the variables around it; `reject` takes any value, an `Option<&Unknown>`, `reject(error)` ([ADR 0782](../../decisions/0782-a-promises-executor-borrows-and-then-takes-two-handlers.md)) |
| `Promise.resolve(v)`, `Promise.reject(e)`, `Promise.reject()` | `js::promise::resolve(v)`, `js::promise::reject(Some(e))` of any value, `js::promise::reject(None)` |
| `Promise.all([a, b])` | `js::promise::all(vec![a, b])`; of two of different types, `all2((a, b))`, up to `all4` |
| `Promise.allSettled`, `race`, `any` | `js::promise::all_settled`, `race`, `any` |
| `Promise.withResolvers()` | `js::promise::with_resolvers()` |
| `p.then(f)`, `p.catch(f)`, `p.finally(f)` | `p.then(f)` (`f` gives a promise), `p.then_resolve(f)` (`f` gives a value), `p.catch(f)` (`f` gives a value, a promise, or of an `Option`'s nothing, `p.catch(\|_\| {})`, [ADR 0698](../../decisions/0698-a-catch-handler-gives-a-value-or-a-promise.md)), `p.finally(f)` |
| `p.then(f, g)` | `p.then_with(f, g)`: each gives a value or a promise, as `catch`'s does ([ADR 0782](../../decisions/0782-a-promises-executor-borrows-and-then-takes-two-handlers.md)) |
| an `async` call handed on, not awaited | `js::promise(future)`: the future as the `Promise` it is |

## Rejections

`.await` of a rejected promise throws, as a panic does: the `async fn`'s
own promise rejects, and the destructors on the way run.

To handle a rejection, ask for a `Result`:

- **A binding that returns `Promise<Result<T, &JsError>>`** settles either
  way: its `Ok` is what the promise fulfils with, its `Err` what it's
  rejected with, `$settle(saveUser(name))` above.
- **`js::settle(promise)`** does the same for a promise whose binding says
  `Promise<T>`, as webapi's `fetch` does.

It's the binding's declared result that says so, not a type parameter a
caller fills: `js::promise(parse(text))` of an `async fn parse() ->
Result<u32, String>` is that promise, fulfilled with the `Result` the
future gives, never settled ([ADR 0588](../../decisions/0588-a-binding-throws-by-its-declared-result.md)).

A rejection's reason, as a handler or `settle` reads it, is a `&JsError`,
whose `message()` and `name()` are the `Error`'s
([Errors, panics and exceptions](exceptions.md)); what rejects one takes
any value, as TypeScript's `reason?: any`.

A promise is a handle JS shares: `p.then(f)` leaves `p` as it was. One of
a `Copy` value is `Copy`, so it's kept, a `Cell<Option<Promise<..>>>`
field, and given handlers again, `info.value.get().unwrap().then_with(..)`.
One of any other value, a `Vec`'s, is consumed by its first handler, as
Rust's: two would each be given the one array
([ADR 0782](../../decisions/0782-a-promises-executor-borrows-and-then-takes-two-handlers.md)).

## Fetch

`window.fetch` is webapi's. A network error rejects, so settle it:

```rust
use js::JsError;
use webapi::window;

pub async fn load_text(url: &str) -> Result<String, &'static JsError> {
    let response = js::settle(window.fetch(url, None)).await?;
    if !response.ok() {
        return Ok(format!("HTTP {}", response.status()));
    }
    Ok(response.text().await)
}
```

```js
import { $settle } from "@rust-js/runtime";

export async function loadText(url) {
  const result = await $settle(window.fetch(url));
  if (result.TAG === "Err") {
    return result;
  }
  const response = result._0;
  if (!response.ok) {
    return { TAG: "Ok", _0: `HTTP ${response.status}` };
  }
  return { TAG: "Ok", _0: await response.text() };
}
```

- `fetch`'s `init`, an options object, is an `Option`: `None` leaves it
  out, `window.fetch(url)`.
- `?` on the settled `Result` returns the `Err` as it is.

## Timers

`js::set_timeout` and `js::set_interval` are every JS runtime's timers;
`js::clear_timeout` and `js::clear_interval` take what they give. A delay
is a promise a timer resolves:

```rust
use js::Promise;

pub fn delay(ms: f64) -> Promise<()> {
    js::promise::new(move |resolve, _reject| {
        js::set_timeout(Box::new(move || resolve(())), ms);
    })
}

pub async fn countdown() {
    for n in (1..=3).rev() {
        println!("{n}");
        delay(1000.0).await;
    }
    println!("liftoff");
}
```

```js
import { $iterator, $rangeInclusiveNextBack } from "@rust-js/runtime";

export function delay(ms) {
  return new Promise((resolve, reject) => {
    setTimeout(() => {
      resolve();
    }, ms);
  });
}

export async function countdown() {
  for (const n of $iterator({ start: 1, end: 3 }, $rangeInclusiveNextBack)) {
    console.log(`${n}`);
    await delay(1000);
  }
  console.log("liftoff");
}
```

`resolve(())` is `resolve()`: a `()` argument is no argument.
A browser has no `setImmediate`: `js::is_defined(js::set_immediate)` asks
([ADR 0597](../../decisions/0597-whether-the-host-has-a-global-is-is-defined.md)).

## Many promises at once

```rust
pub async fn total(ids: Vec<u32>) -> u32 {
    let load = async |id: u32| id * 10;
    let all = js::promise::all(ids.into_iter().map(|id| js::promise(load(id))).collect()).await;
    all.iter().sum()
}
```

```js
export async function total(ids) {
  const load = async (id) => (id * 10) >>> 0;
  const all = await Promise.all(ids.map((id) => load(id)));
  return all.reduce((a, b) => (a + b) >>> 0, 0);
}
```

Each `load(id)` starts when it's called, so they run side by side;
`Promise.all` waits for them all.

## Streams

Rust has no `for await`. Read a stream by awaiting its reader's `read()`
until it's `done`:

```rust
use webapi::{readable_stream_default_reader, window};

pub async fn count_chunks(url: &str) -> u32 {
    let response = window.fetch(url, None).await;
    let Some(body) = response.body() else {
        return 0;
    };
    let reader = readable_stream_default_reader::new(body);
    let mut chunks = 0;
    while reader.read().await.done != Some(true) {
        chunks += 1;
    }
    chunks
}
```

```js
export async function countChunks(url) {
  const response = await window.fetch(url);
  const body = response.body;
  if (body == null) {
    return 0;
  }
  const reader = new ReadableStreamDefaultReader(body);
  let chunks = 0;
  while (!(await reader.read()).done) {
    chunks = (chunks + 1) >>> 0;
  }
  return chunks;
}
```

## What's refused

There's one thread, and a future is a promise. rust-js refuses, at
compile time:

- **Awaiting a `Future` your crate implements itself**, which isn't a
  promise ([ADR 0425](../../decisions/0425-a-crates-own-future-and-reader-are-allowed-never-lowered.md)):

  ```text
  error: rust-js does not support `Ready` as a promise yet: its `Future` is the crate's own, which rust-js never compiles (ADR 0425)
  ```

- **`std::thread::spawn` and `std::thread::sleep`**: wait with a timer,
  `delay(ms).await`, above.
- **`std::future::poll_fn`, a `Wake` of your own**, and an `IntoFuture`
  of your own given to generic code.

The whole list is in [the limitations](../limitations.md#async-threads-and-time).
