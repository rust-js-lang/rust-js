# The crates rust-js ships

rust-js ships five crates of bindings, one for each part of JavaScript a
program calls, and compiles Rust's own `std` itself. A binding crate is
declarations only: it's never compiled to JS, and a call of one of its
functions is the JS it names, `document.get_element_by_id("app")` being
`document.getElementById("app")` ([ADR 0102](../../decisions/0102-js-and-webapi.md)).

| `use` | Binds | Cargo package | npm package | What it covers |
|---|---|---|---|---|
| `js` | JS's own language: promises, errors, JSON, regular expressions, timers, typed arrays, `Intl` | `rust-js-builtins` | `@rust-js/builtins` | TypeScript's ES2024 library where Rust has no type of its own: 689 of 689 members |
| `webapi` | the browser, generated from W3C's WebIDL | `rust-js-webapi` | `@rust-js/webapi` | TypeScript's DOM: 8,372 of 8,419 members |
| `node` | Node.js's modules | `rust-js-node` | `@rust-js/node` | `@types/node`: 160 of 1,535 exports, what ports have needed |
| `react` | React and React DOM, 18.0 on | `rust-js-react` | `@rust-js/react` | `@types/react` and `@types/react-dom`: 244 of 244 exports |
| `next` | Next.js | `rust-js-next` | `@rust-js/next` | Next.js's public modules: 2,099 of 2,099 exports |

An app names each crate it uses in its `Cargo.toml` by the name it's
`use`d by, and installs its npm package, which holds the crate; Cargo finds
it in `node_modules` ([ADR 0118](../../decisions/0118-bindings-on-npm-only.md)):

```toml
[dependencies]
js = { package = "rust-js-builtins", version = "~0.0.2" }
webapi = { package = "rust-js-webapi", version = "~0.0.3" }
```

`bun create @rust-js` writes these at the release's versions. How an app
gets its crates is [Dependencies](../project/dependencies.md).

## `js`: the JavaScript language

The JS that every runtime has, browsers', workers' and Node's, and that
Rust's `std` has no type for ([ADR 0283](../../decisions/0283-js-crate-covers-typescripts-es-library.md)).
What `std` has, a string, an array, a `Map`, a number's methods, rust-js
writes itself, below.

```rust
use js::{Kind, classify, json, reg_exp, set_timeout};

pub fn describe(text: &str) -> String {
    match json::parse(text) {
        Ok(Some(value)) => match classify(value) {
            Kind::String(s) => format!("text {s}"),
            Kind::Number(n) => format!("number {n}"),
            Kind::Array(items) => format!("{} items", items.len()),
            _ => "something else".to_string(),
        },
        Ok(None) => "null".to_string(),
        Err(error) => js::js_error::to_string(error),
    }
}

pub fn is_number(text: &str) -> bool {
    reg_exp::new(r"^\d+$", "").test(text)
}

pub fn later(name: String) {
    set_timeout(Box::new(move || println!("hello, {name}")), 100.0);
}
```

```js
import { $displayF64, $try } from "@rust-js/runtime";

export function describe(text) {
  const value$1 = $try(() => JSON.parse(text));
  if (value$1.TAG === "Ok") {
    if (value$1._0 != null) {
      const value = value$1._0;
      if (typeof value === "string") {
        return `text ${value}`;
      }
      if (typeof value === "number") {
        return `number ${$displayF64(value)}`;
      }
      if (Array.isArray(value)) {
        return `${value.length} items`;
      }
      return "something else";
    }
    return "null";
  }
  const error = value$1._0;
  return String(error);
}

export function isNumber(text) {
  return /^\d+$/.test(text);
}

export function later(name) {
  setTimeout(() => {
    console.log(`hello, ${name}`);
  }, 100);
}
```

What it has:

- **A value of any shape is a `js::Unknown`,** TypeScript's `unknown`:
  what `json::parse` gives, or a binding's `any`. `classify(value)` tells
  what it is, a `Kind` to `match`, by `typeof` and `Array.isArray`;
  `get(value, key)`, `set(value, key, to)` and `has(value, key)` are
  `value[key]` and `key in value`; `object::keys` is `Object.keys`
  ([ADR 0225](../../decisions/0225-unknown-values.md)). A `Json` is a JSON
  value to `match`, and a `Dict<T>` a plain object of `T`s by name.
- **Whether the host has a global is `is_defined(binding)`,** `typeof X
  !== "undefined"`: `is_defined(set_immediate)`, which a browser hasn't
  ([Access to the global scope](../interop/global-scope.md#a-global-some-hosts-lack)).
- **A promise is a `Promise<T>`, to `.await`;** `settle(p).await` is a
  `Result` of what it fulfils or rejects with, `spawn` starts a future no
  one awaits, and `promise(future)` is a future's `Promise` for JS
  ([Promises and async](../interop/async.md)).
- **What JS throws is a `&JsError`:** a binding that may throw returns
  `Result<T, &JsError>`, `js_error::to_string(e)` is `String(e)`, and
  `e.message()` an `Error`'s ([Errors](../interop/exceptions.md)).
- **`json`** is `JSON.parse` and `JSON.stringify`; a Rust value's JSON is
  serde's, below.
- **`RegExp`**, made by `reg_exp::new`, a literal `/^\d+$/` of a pattern
  written out ([ADR 0243](../../decisions/0243-regex-literals.md)), with
  `test`, `exec` and `reg_exp::replace`.
- **Timers:** `set_timeout`, `set_interval`, their `clear_*`.
- **Where JS's answer isn't Rust's, JS's by name:** `js::string`'s
  methods count UTF-16 as JS's do, `string::slice`, `string::index_of`,
  for an index a JS API gives or takes; a place found in a Rust string is
  std's `find` and slicing, which are JS's own there (ADR 0642);
  `js::number::to_fixed` rounds a tie away from zero; `js::math`'s
  `round`, `sign`, `max`, `min` and `random` are `Math`'s
  ([ADR 0247](../../decisions/0247-js-string-methods.md),
  [ADR 0365](../../decisions/0365-js-math-where-rust-differs.md)). A
  `str`'s and an `f64`'s own methods keep Rust's.
- **The rest of the ES library:** `Date`, `Intl`'s formatters, typed arrays
  and `ArrayBuffer`, `Atomics`, `Symbol`, `WeakMap`, `WeakRef`,
  `FinalizationRegistry`, `Proxy`, `Reflect`, `encode_uri_component` and
  the other global functions.
- **The module's own JS:** `js::import!` imports a file for its effects or
  an item lazily, `js::import_module!` a module, `js::export_default!` a
  module's default export, `js::directive!("use client")` its directive,
  `js::on_load!` what it runs when loaded
  ([Emitting modules](../project/modules.md)).

## `webapi`: the browser

The web platform, **generated from W3C's WebIDL**, the source TypeScript's
`lib.dom.d.ts` comes from ([ADR 0024](../../decisions/0024-web-crate.md)).

```rust
use webapi::events::Click;
use webapi::tags::Button;
use webapi::{EventTargetExt, document, window};

pub fn mount() {
    let app = document.get_element_by_id("app").expect("the page has an #app");
    let button = document.create_element(Button);
    button.set_text_content("Save");
    button.add_event_listener(Click, |event| {
        let x = event.client_x();
        if let Ok(storage) = window.local_storage() {
            let _ = storage.set_item("clicked-at", &x.to_string());
        }
    }, None);
    app.append((button,));
}

pub async fn load(url: &str) -> String {
    let response = window.fetch(url, None).await;
    response.text().await
}
```

```js
import { $try, $unwrap } from "@rust-js/runtime";

export function mount() {
  const app = $unwrap(document.getElementById("app"), "the page has an #app");
  const button = document.createElement("button");
  button.textContent = "Save";
  button.addEventListener("click", (event) => {
    const x = event.clientX;
    const localStorage = $try(() => window.localStorage);
    if (localStorage.TAG === "Ok") {
      const storage = localStorage._0;
      try {
        storage.setItem("clicked-at", String(x));
      } catch {}
    }
  });
  app.append(button);
}

export async function load(url) {
  const response = await window.fetch(url);
  return await response.text();
}
```

How WebIDL maps to Rust, one rule for each:

- **Names are Rust's, made of the JS name:** an interface is its type,
  `HTMLInputElement`; a method or an attribute is snake_case,
  `getElementById` as `get_element_by_id`; a constructor, a static or a
  constant is in a module of its interface's name, `event::new("ping",
  None)` being `new Event("ping")`; a JS name that's a Rust keyword is its
  raw identifier, `r#type` ([ADR 0376](../../decisions/0376-keyword-names-are-raw.md)).
- **An attribute is a getter, and a setter if it's writable:**
  `input.value()`, `input.set_value("x")`.
- **Inheritance is `Deref`:** an `&HTMLButtonElement` has `Element`'s and
  `Node`'s methods, and goes where an `&Element` does.
- **An argument a caller may leave out is an `Option`,** and a trailing
  `None` is no argument: `window.fetch(url, None)` is `window.fetch(url)`
  ([ADR 0443](../../decisions/0443-a-value-a-caller-may-leave-out-is-an-option.md)).
  An options object is a struct of `Option`s, `RequestInit { method:
  Some("POST"), ..Default::default() }`.
- **A result that may be `null` is an `Option`;** one that may be
  `undefined` too.
- **An API that throws for what a program can't check first returns a
  `Result`,** as `window.local_storage()` and `storage.set_item` do; one
  that throws only when misused, an invalid selector, panics
  ([ADR 0442](../../decisions/0442-a-web-api-that-may-throw-is-a-result.md)).
- **An event's name and a tag are types,** `webapi::events::Click` and
  `webapi::tags::Button`: a listener is given the event its target and
  name give, a `PointerEvent` of a button's click, and `create_element`
  the element of its tag
  ([ADR 0223](../../decisions/0223-webapi-event-and-tag-maps.md)).
  `EventTargetExt` gives every target `add_event_listener` and
  `remove_event_listener` ([ADR 0282](../../decisions/0282-event-listener-methods.md)).
- **A union is each of its members as it is:** `window.fetch(url, None)`
  and `window.fetch(request, None)`; a union a JS API gives is an enum of
  its members ([ADR 0215](../../decisions/0215-webapi-unions.md)).
- **A rest parameter is a tuple, each argument of its own type:**
  `el.append((node, "text"))` is `el.append(node, "text")`, and
  `console::error(("%s is %s", name, kind))` is `console.error("%s is %s",
  name, kind)`; `(x,)` is one, `()` none, and a `None` before none is no
  argument, `console::time_log(None, ())` being `console.timeLog()`. A
  slice or a `Vec` of them is spread: `el.append(&children)` is
  `el.append(...children)`
  ([ADR 0640](../../decisions/0640-a-rest-parameter-is-a-tuple.md)).
- **A cast is `unchecked_from`:** `html_input_element::unchecked_from(el)`.

## `node`: Node.js

Node's modules, typed as `@types/node` types them, for the Rust of a
program that runs in Node, a Next.js page's data, a build script's
([ADR 0272](../../decisions/0272-node.md)). It binds what ports have
needed so far: all of `path`, `buffer` and `querystring`, most of
`process`, `url` and `events`, and parts of `fs`, `http` and `util`.

```rust
use node::{BufferEncoding, fs, process};

pub fn read_page(name: &str) -> Result<String, &'static js::JsError> {
    let root = process::cwd() + "/content";
    fs::read_file_sync(&format!("{root}/{name}.md"), BufferEncoding::Utf8)
}
```

```js
import { readFileSync } from "fs";
import { $try } from "@rust-js/runtime";

export function readPage(name) {
  const root = process.cwd() + "/content";
  return $try(() => readFileSync(`${root}/${name}.md`, "utf8"));
}
```

## `react` and `next`

React's and React DOM's whole API, as of React 19, for whichever React from
18.0 on the app has installed: components, `jsx!`, hooks, context and the
rest ([ADR 0041](../../decisions/0041-react.md),
[ADR 0043](../../decisions/0043-react-versions.md)). `next` binds every
public module of Next.js: `next/link`, `next/navigation`, `next/image`,
`next/font/google`'s fonts, a route's data and metadata
([ADR 0192](../../decisions/0192-next.md)). They're
[React](../README.md#react)'s pages, and [Next.js](../react/nextjs.md).

```rust
use react::{JSX, jsx, use_state};

pub fn Counter() -> JSX::Element {
    let (count, set_count) = use_state(|| 0);
    jsx! {
        <button onClick={move |_| set_count.update(|count| count + 1)}>
            {"Count is "}{count}
        </button>
    }
}
```

```jsx
import { useState } from "react";

export function Counter() {
  const [count, setCount] = useState(0);
  return <button onClick={() => setCount((count) => (count + 1) | 0)}>Count is {count}</button>;
}
```

## serde

A struct's or an enum's JSON is serde's, as on a server: an app depends
on `serde` with its `derive` feature and on `serde_json`, from crates.io,
as any Rust program does. rustc builds them, and rust-js writes what each
derive means, read from its attributes, with JSON the runtime reads and
writes as serde_json does, a `u64` to the digit
([ADR 0077](../../decisions/0077-serde-json.md)):

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub order_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

pub fn to_json(order: &Order) -> String {
    serde_json::to_string(order).expect("an order is JSON")
}

pub fn from_json(text: &str) -> Result<Order, serde_json::Error> {
    serde_json::from_str(text)
}
```

```js
import { $debugJsonError, $fromJson, $json, $toJson, $unwrapOk } from "@rust-js/runtime";

function orderSerialize_serialize(order, json) {
  json.beginObject();
  json.key("orderId");
  json.int(order.orderId);
  if (order.note != null) {
    json.key("note");
    json.string(order.note);
  }
  json.endObject();
}

function orderDeserialize_deserialize(json) {
  return json.struct(
    "struct Order",
    [
      ["orderId", $json.u64],
      ["note", $json.option($json.string)],
    ],
    ([orderId, note]) => ({ orderId, note }),
  );
}

export function toJson(order) {
  return $unwrapOk(
    $toJson(order, orderSerialize_serialize, false),
    "an order is JSON",
    $debugJsonError,
  );
}

export function fromJson(text) {
  return $fromJson(text, orderDeserialize_deserialize);
}
```

serde's common attributes work, `rename`, `rename_all`, `tag`, `content`,
`untagged`, `flatten`, `default`, `skip` and the rest, as do
`serde_json::Value` and `json!`
([the semantics](../semantics.md#the-javascript-boundary)); what's refused
is in [the limitations](../limitations.md#serde).

## Rust's `std`

`std` isn't a crate an app adds: rust-js writes each of its types as the
JS a person would, a `Vec` an array, a `HashMap` a `Map`, a `String` a
string, and its methods as their JS, with Rust's meaning
([the semantics](../semantics.md)).

[`docs/std-coverage.txt`](../../std-coverage.txt) lists each stable
inherent method of std's data structures and text, `+` where rust-js
compiles it, `-` where it refuses it, under a count of each type's,
`# Vec 42 of 48`: `Vec`, `VecDeque`, `HashMap`, `BTreeMap`, `Option`,
`Result`, `Rc`, `Cell`, `RefCell`, `Mutex`, `String`, `str`, slices,
`char`, `Iterator`'s provided methods and its adapters, and the rest. The
count only grows: a method compiled once and refused later fails rust-js's
tests ([ADR 0314](../../decisions/0314-std-data-structures.md)). Of what
isn't a data structure, what's refused, a thread, a file, standard input,
is [the limitations](../limitations.md).

## Find a binding

A binding is named for the JS it calls, so its name is guessable from
the JS: `addEventListener` is `add_event_listener`, `new Event(..)` is
`event::new(..)`, `JSON.parse` is `json::parse`.

- **In the crate's source:** each npm package holds its crate,
  `node_modules/@rust-js/webapi/src/lib.rs`, each binding with its JS name
  in its `link_name` and a link to MDN: search it for the JS name,
  `"getElementById"`. rust-analyzer's completion and hover, and `cargo
  doc`, show the same.
- **In the crate's coverage list:** rust-js's repository holds, for each
  crate, the list of what it binds, by TypeScript's names, each held by a
  test so a binding can't go missing:
  [`builtins/coverage.txt`](../../../builtins/coverage.txt),
  [`webapi/coverage.txt`](../../../webapi/coverage.txt),
  [`node/coverage.txt`](../../../node/coverage.txt) (`+` bound, `-` not
  yet), [`react/coverage.txt`](../../../react/coverage.txt) and
  [`next/coverage.txt`](../../../next/coverage.txt) (`x` not bound by
  design: TypeScript's own types, class components, old names).
  `grep '^Document.getElementById' webapi/coverage.txt`.

## When a binding is missing

A JS API that one of these crates should bind and doesn't, a browser
API, a Node module, an ES built-in, a React or Next.js export, or one
that types a value more narrowly or loosely than TypeScript does, is a
gap in rust-js: report it, or add it to the crate, where everyone gets it
([CONTRIBUTING.md](../../../CONTRIBUTING.md)). webapi's, react's and
next's fonts are generated, so a change to them is a change to their
generator, `webapi/generate.ts`, `react/generate.ts`; `js`'s and `node`'s
are written by hand. Don't declare a local stand-in for it in an app: a
second binding of one API is a second way to call it, which the crate's
won't know of.

A JavaScript library from npm is another matter: its binding is a crate
of its own, written as [Write bindings](../interop/bindings.md) says, and
shared as an npm package ([ADR 0118](../../decisions/0118-bindings-on-npm-only.md)).
A crate from crates.io is [Crates from crates.io](crates-io.md).
