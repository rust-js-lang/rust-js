# Crates that use wasm-bindgen

rust-js builds every crate for `wasm32-unknown-unknown`, so a crate takes
its wasm32 path, and many crates' call wasm-bindgen, js-sys or web-sys:
chrono's `wasmbind`, on by default, for the local timezone; uuid's and
getrandom's `js` for randomness. rust-js has its own `wasm-bindgen`,
`js-sys` and `web-sys`, which your dependencies use in place of
crates.io's, which rust-js can't compile
([ADR 1112](../../decisions/1112-wasm-bindgen-for-dependencies.md)).

You write nothing for it. Where Cargo's graph has one of them, `rust-js
build` names rust-js's in the patch it writes in `.cargo/config.toml`,
under the line `# rust-js's own, which the app's dependencies use in place
of crates.io's:`, and drops them when none is left, so Cargo never warns
of an unused patch.

Their API grows from what crates use, crate by crate:
`tooling/compat/covered.txt` lists the crates they serve: getrandom 0.3,
getrandom 0.1 with its `wasm-bindgen` feature, which rand 0.7 and
quickcheck 0.9 use, chrono 0.4, whose default `wasmbind` reads the local time zone through
`js_sys::Date`, so `Local::now()` is your user's local time, and uuid 1 with
its `js` feature. They're compiled to JS as your crates.io dependencies are,
vendored under `rust-js/crates/`. Where wasm-bindgen behaves differently on
wasm32 and natively, they behave as it does natively: `unwrap_throw()` of a
`None` is a Rust panic, as `expect` is, which `catch_unwind` catches. Where any crate of your graph that uses wasm-bindgen isn't one of
them, chrono with its default `wasmbind` say, your build keeps crates.io's,
as it did before, and such a crate is its types alone, a call into it
refused.

Of wasm-bindgen's API, an `extern "C"` block's `#[wasm_bindgen]` binds a
function by its `js_namespace`, `js_name` and `catch`; a method of an
imported type, `method`, its first parameter its receiver, a property's
read, `getter`, and a static of a class, `static_method_of` and
`js_class`, `structural` binding each the same. An imported type, `type
NodeCrypto;`, is a JS value of it, owned, `From` and `Into` a `JsValue`
and one by `Deref`, as upstream's. A `static` is read where it's used;
CommonJS's `module`, which an ES module has none of, is given its
`require`, `process.getBuiltinModule("module").createRequire(
import.meta.url)`, read only where it's used, so a browser's bundle
imports nothing of Node's. A number, a `bool` or `&str` is passed as it
is; a `JsValue`, an imported type's or js-sys's `Uint8Array` as the value
itself; a slice of bytes as a `Uint8Array`, made once, and of a `&mut`
one, what JS wrote copied back once. What it doesn't bind yet is a compile
error naming it
([ADR 1112](../../decisions/1112-wasm-bindgen-for-dependencies.md),
amended).

## Your own code binds JS with `js` and `webapi`

A crate you write that depends on `wasm-bindgen`, `js-sys` or `web-sys` is
refused, whether or not the patch names them:

```text
rust-js: app depends on wasm-bindgen, which rust-js gives an app's dependencies alone, in place of crates.io's: an app's own Rust binds JS with the js and webapi crates (docs/guide/interop/bindings.md)
```

They exist for crates written for wasm-bindgen. Your code has one way to
bind JS, [the binding crates](README.md) and
[your own bindings](../interop/bindings.md).

A crate as Cargo packaged it is a dependency's source wherever it is:
`cargo package` keeps the manifest it was given as `Cargo.toml.orig`
beside the one it writes. So each crate from crates.io has one, unpacked,
vendored with `cargo vendor`, or copied for a `[patch]`, and no crate you
write does. That file decides it; there's no setting to opt out.

## A dependency pinned to another version

rust-js's crates are at crates.io's newest release of each minor:
wasm-bindgen 0.2.130, js-sys and web-sys 0.3.107. So a dependency's
`wasm-bindgen = "0.2"` takes them. One that pins an exact version gets
crates.io's from Cargo, and is refused before the build:

```text
rust-js: pins 1.0.0 asks for wasm-bindgen =0.2.100, and rust-js's own wasm-bindgen, which a dependency uses in place of crates.io's, is 0.2.130, so Cargo takes crates.io's, which rust-js can't compile: use a release of pins that takes wasm-bindgen 0.2.130
```
