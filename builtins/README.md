# js: the JS language for rust-js

The `js` crate declares what JS has that Rust's `std` doesn't, as ReScript's
standard library does: its promises, errors and regular expressions, its byte
buffers, its JSON, and its global functions, its timers too, as ReScript's has
them. What the browser adds is the
[`webapi`](../webapi/README.md) crate's; what `std` has, rust-js maps itself.
It holds declarations only, so it's never compiled to JS. See
[ADR 0102](../docs/decisions/0102-js-and-webapi.md).

```rust
use js::{decode_uri_component, encode_uri_component, settle, spawn};

let query = encode_uri_component("a b&c");             // encodeURIComponent("a b&c")
let text = decode_uri_component("%E0%A4%A");           // Err: a URIError
spawn(Box::new(async move {                            // runs, unawaited
    match settle(webapi::window.fetch(&url)).await {
        Ok(response) => { .. }                         // what fetch fulfils with
        Err(error) => { .. }                           // a network error: not a throw
    }
}));
```

- A promise is a `Promise<T>`, to `.await` ([ADR 0029](../docs/decisions/0029-async-await.md));
  `settle(p)` makes its `.await` a `Result`, and one a binding declares as
  `Promise<Result<T, &JsError>>` is one already ([ADR 0035](../docs/decisions/0035-results-and-throwing-js.md)).
- `JsError` is what JS threw; `js_error::to_string(e)` is `String(e)`, and
  `e.is_error()` and `e.message()` are an `Error`'s.
- `RegExp` is there for what Rust would use `regex` for: `reg_exp::new(r"^\d+$", "")`,
  and `reg_exp::replace(text, pattern, "$1")`. A replacer closure, and `matchAll`,
  are a program's own bindings, typed for its pattern's groups.
- `json::stringify(text)` is a string's JSON, which is a JS string literal too.
  A Rust value's JSON is serde's ([ADR 0077](../docs/decisions/0077-serde-json.md)).
- `Unknown` is a JS value of any shape, as TypeScript's `unknown` is
  ([ADR 0225](../docs/decisions/0225-unknown-values.md)): what `json::parse(text)`
  gives, or `webapi`'s `r.json()`. `classify(value)` tells what it is,
  a `Kind` to `match`, by `typeof`; `get(value, key)` and `set(value, key, to)`
  are `value[key]`; `object::keys(value)` is `Object.keys`.
- `Json` is a JSON value, ReScript's `JSON.t`: `Json::parse(text)`, then
  `match`, a `null` the `None` of the `Option` holding it; its objects are
  `Dict<Option<Json>>`. A `Dict<T>` is a plain object of `T`s by name:
  `dict::get(d, key)`, `dict::entries(d)`, `dict::set(d, key, value)`.
- `StructuredClone` is what the browser's structured clone copies as it is:
  what `postMessage`, `pushState` and `structuredClone` take. A struct is one
  by `unsafe impl StructuredClone for Saved {}`, which vouches its fields are.
- `object::from_entries(entries)` is a JS object of keys and values, as an API
  taking a dictionary wants, and `object::is(a, b)` is `Object.is`: whether two
  JS objects are one.
- `set_timeout(f, ms)` and `set_interval(f, ms)` run a closure later, or every
  so often, and `clear_timeout` and `clear_interval` of what they gave stop it:
  every JS runtime has them, browsers', workers' and Node's.
- `number::to_fixed(x, digits)` is `x.toFixed(digits)`, JS's rounding: a tie
  away from zero, `2.5` to `"3"`, where `format!("{:.0}", 2.5)` is `"2"`.

```bash
builtins/build.sh -o "$PWD/target/libjs.rmeta"
./target/debug/rust-js app.rs -- --extern js=target/libjs.rmeta
```
