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

Their API grows from what crates use. Until a crate's part of it is there,
a dependency that calls it is refused as before.

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
