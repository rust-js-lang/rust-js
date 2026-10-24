# rust-js documentation

**Rust in. Readable JavaScript out.** rust-js compiles a Rust crate to ES
modules a JavaScript developer would have written by hand, so a web app's
frontend can be written in the Rust its server is.

This guide is how to write a JavaScript app in rust-js, as rust-js is
today. It's the one place that says what to write: where a design decision
(`docs/decisions/`) changed something, this guide says what's current, and
links the decision for why.

## Tutorials

Start here.

- [Getting started with rust-js and Vite](tutorial/vite.md)
- [Build UIs with React](tutorial/react.md)
- [Use a JavaScript library from Rust](tutorial/js-library.md)

## Rust for JavaScript developers

What you write in Rust for what you'd write in JavaScript, and the
JavaScript rust-js makes of it.

- [Basics: values, functions, control flow](for-js-developers/basics.md)
- [Collections: arrays, maps, sets, strings](for-js-developers/collections.md)
- [Advanced: closures, errors, async, traits](for-js-developers/advanced.md)

## Project setup

- [Building the application](project/building.md)
- [Dependencies: crates and npm packages](project/dependencies.md)
- [Emitting modules](project/modules.md)
- [JavaScript environments: browsers, Node, Vite, Next.js](project/environments.md)
- [Testing](project/testing.md)
- [Compile errors](project/errors.md)

## JavaScript interoperability

- [JavaScript types: what each Rust type is in JS](interop/types.md)
- [Write bindings for JavaScript APIs](interop/bindings.md)
- [Access to the JavaScript global scope](interop/global-scope.md)
- [Export Rust APIs to JavaScript](interop/export-to-javascript.md)
- [Structs, enums and traits as JavaScript sees them](interop/objects.md)
- [Promises and async](interop/async.md)
- [Errors, panics and exceptions](interop/exceptions.md)

## React

- [Components and props](react/components.md)
- [JSX with `jsx!`](react/jsx.md)
- [Hooks](react/hooks.md)
- [Context, refs, memo and the rest of React's API](react/api.md)
- [Next.js](react/nextjs.md)

## Libraries

- [The crates rust-js ships](libraries/README.md): `js`, `webapi`, `node`,
  `react`, `next`, and Rust's `std`
- [Crates from crates.io](libraries/crates-io.md)

## Reference

- [Semantics of rust-js: how Rust behaves in JavaScript](semantics.md)
- [Known limitations: what rust-js refuses](limitations.md)
- [Internals: how the compiler works](../architecture.md), and
  [why each decision was made](../decisions/README.md)

## How these pages are written

For people and for AI coding agents alike, each page:

- **Says the one way.** rust-js has exactly one way to do each job (one
  attribute, one binding form, one output for each construct). A page shows
  that way, never alternatives; where an old way was removed, it's not
  mentioned, except as the error that names the new one.
- **Shows Rust and the JavaScript it becomes**, side by side, each example
  compiled by the compiler as it is: the JS shown is what it writes.
- **Is current.** Where a decision record and this guide disagree, the
  guide is right, or it's a bug in the guide: decisions record why, as
  they were made, and a later one may change an earlier one.
- **Links the decision** that explains a rule, `([ADR 0597](../decisions/0597-whether-the-host-has-a-global-is-is-defined.md))`, for why, never
  for what.
- **Is kept true with the code.** A change to rust-js that changes what a
  program writes, or what it becomes, changes the page that says so in the
  same commit (CONTRIBUTING.md).

## Examples

No JavaScript on these pages is written by hand. `bun run docs` compiles
each example and writes the fences after it; `bun run docs --check`, which
the tests run, fails where a page isn't what the compiler makes.

- **A ```` ```rust ```` fence is an example**, compiled as `src/lib.rs` with
  the crates rust-js ships. The fences right after it are written from it:
  `js` or `jsx`, its JavaScript; `ts`, its declarations; a fence with no
  language, what the compiler says of it, which then must fail.
- **A line `# ..` is compiled but isn't the example's**, as rustdoc's: the
  `use` or the type an example needs, which an earlier one showed.
- **Several files are a run of fences**, each headed by its path, `//
  src/lib.rs`, `// src/greeting.rs`; each output fence after them is headed
  by the module it's of, `// src/greeting.js`.
- **```` ```rust excerpt ```` isn't compiled**, and no output fence follows
  it: a part of an example shown whole elsewhere.
