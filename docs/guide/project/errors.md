# Compile errors

When the Rust doesn't compile, `rust-js build` prints why, exits with
status 1, and writes nothing. Two kinds of error reach you,
in this order:

1. **rustc's.** Types, traits, borrows, lifetimes: the errors any Rust
   project has, with their codes, `error[E0308]`. rust-js checks nothing
   of these itself ([ADR 0006](../../decisions/0006-errors-and-unsupported-features.md)).
2. **rust-js's.** Once rustc has accepted the program, rust-js lowers it
   to JS, and refuses what it can't write. Its errors have no code.

So a file with both shows rustc's alone. Fix those; rust-js's come next.

## rustc's errors

```rust
pub fn total() -> i32 {
    "3"
}
```

```
error[E0308]: mismatched types
 --> src/lib.rs:2:5
  |
1 | pub fn total() -> i32 {
  |                   --- expected `i32` because of return type
2 |     "3"
  |     ^^^ expected `i32`, found `&str`

error: aborting due to 1 previous error

For more information about this error, try `rustc --explain E0308`.
```

They're the same errors `cargo check` gives, and your editor shows them
where they are, inside `jsx!` too, through `rust-js-check`
([Building](building.md#your-editor)).

## "rust-js does not support .. yet"

What rust-js can't compile is refused, with the place and what it is.
It's never compiled to JS that does something else ([ADR 0262](../../decisions/0262-when-rust-and-js-disagree.md)).

```rust
pub fn spawn() {
    std::thread::spawn(|| {});
}
```

```
error: rust-js does not support calling `std::thread::spawn` yet
 --> src/lib.rs:2:5
  |
2 |     std::thread::spawn(|| {});
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^

error: aborting due to 1 previous error
```

The words after "support" say what's refused:

| It says | It's refused |
| --- | --- |
| ``calling `std::fs::read_to_string` `` | a function of std's, or another crate's, that rust-js doesn't map |
| ``values of type `*const str` ``, ``statics of type `..` ``, ``constants of type `..` `` | a type wherever it's held |
| a feature, ``a `&mut` to a union's field`` | a construct of the language |
| a feature, then `: ` and why | a construct, with the reason it can't be: ``.. yet: React gives a component no dictionary`` |

### Addressable memory

JavaScript has no memory to address, so what reads or writes bytes by
where they are says so, and why ([ADR 0390](../../decisions/0390-addressable-memory-is-refused-as-such.md)):

```rust
pub fn address(value: &u32) -> usize {
    value as *const u32 as usize
}
```

```
error: rust-js does not support addressable memory yet: a raw pointer to this place
 --> src/lib.rs:2:5
  |
2 |     value as *const u32 as usize
  |     ^^^^^
  |
  = note: JavaScript has no memory to address: a raw pointer, `transmute` and `std::ptr` read or write bytes by where they are (ADR 0390)

error: aborting due to 1 previous error
```

### A crate rust-js couldn't compile

A crate from crates.io that rust-js couldn't compile is its types alone. A
call into it is refused, with a note of why its crate wasn't compiled:

```
error: rust-js does not support calling `once_cell::unsync::OnceCell::<T>::new` yet
 --> src/lib.rs:4:16
  |
4 |     let cell = OnceCell::new();
  |                ^^^^^^^^^^^^^^^
  |
  = note: its crate is its types alone: rust-js couldn't compile it (ADR 0415): rust-js does not support calling `std::thread::current` yet
```

See [Dependencies](dependencies.md#crates-from-cratesio).

## Common refusals and what to write

| Refused | What to write |
| --- | --- |
| `std::thread::spawn`, `std::thread::current` | JS has one thread: run work later with `js::spawn` of a boxed `async` block, or `js::set_timeout` |
| `std::fs`, `std::net` | Node's modules, from the `node` crate: `node::fs::read_file_sync`; in a browser, `webapi`'s `fetch` |
| a raw pointer, `transmute`, `std::ptr` reading by address | references, `Box`, `Rc` and indices: JS objects are values, not addresses |
| `#![feature(..)]` | rust-js runs a stable release: rustc refuses it, ``error[E0554]: `#![feature]` may not be used on the stable release channel`` |

What works in Node but not in a browser, `std::env::args` and
`std::process::exit`, compiles, and panics in a browser naming what needed
Node ([JavaScript environments](environments.md#browsers)).

Everything rust-js refuses today, by area, is
[Known limitations](../limitations.md).

## "rust-js: .." errors

An error that starts `rust-js:` is about the project, or about a form
rust-js has one way to write. Each says what to do.

A binding written another way than the one form:

```rust
unsafe extern "Rust" {
    #[link_name = "Math.max"]
    pub safe fn max(a: f64, b: f64) -> f64;
}
```

```
error: rust-js: a JS function is bound as an ordinary function, `#[cfg_attr(rust_js, rust_js::link_name = "Math.max")] fn max(..) { unreachable!() }`, not in an `extern` block (ADR 0373)
 --> src/lib.rs:3:5
  |
3 |     pub safe fn max(a: f64, b: f64) -> f64;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: aborting due to 1 previous error
```

And from `rust-js build` itself:

| It says | Do |
| --- | --- |
| ``rust-js: no Cargo.toml in ..: run `rust-js build` where the app's is`` | run it in the app's directory |
| ``rust-js: .. is a workspace's: name the app's package, `rust-js build --package <name>` `` | add `--package` |
| ``rust-js: the JS beside the Rust isn't what it compiles to; run `rust-js build`:`` | `--check` found stale files: build, and commit them |
| `the JS rust-js 0.0.4 writes imports @rust-js/runtime 0.0.4: install it` | install `@rust-js/runtime` and `@rust-js/std` at the compiler's version |
| `rust-js: no rust-js at ..: install rust-js ..` | install `@rust-js/native`, or set `RUST_JS_COMPILER` |
| `rust-js can't write the JS in source: .. would both be ..; rename the module` | two modules would be one file: rename one ([Emitting modules](modules.md#where-each-modules-file-goes)) |
| ``rust-js: the check `tsc` failed:`` | a check of the crate's `Cargo.toml` failed; its lines name the Rust ([Building](building.md#run-your-tools-on-what-it-writes)) |

A setting that's gone is an error that names the one to use:
`[package.metadata.rust-js.format]` names `.prettierrc`, and
`js::camel_case!` says to remove it.

## Reading one

- **The place is the Rust's**, file, line and column, as rustc gives it:
  what's underlined is what's refused.
- **Read the first error first.** One refusal can cause the next: a static
  of a refused type, then each read of it.
- **"yet" means it's on the way**: a refusal is a gap in rust-js, not a
  rule about your Rust. If idiomatic Rust is refused, that's rust-js's to
  fix; don't restructure your program around it.
