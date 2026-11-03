# Testing

You test a rust-js app at two levels: its Rust logic natively, with
`cargo test`, and the JavaScript rust-js wrote, with a JS test runner, as
any JS module is tested.

## Test shared logic natively

Code that calls no JavaScript is ordinary Rust, and `cargo test` runs its
`#[test]`s natively, as it would a server's:

```rust
// src/shapes.rs
pub fn area(radius: f64) -> f64 {
    std::f64::consts::PI * radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_unit_circle_has_area_pi() {
        assert_eq!(area(1.0), std::f64::consts::PI);
    }
}
```

`cargo test` says:

```text
$ cargo test
running 1 test
test shapes::tests::a_unit_circle_has_area_pi ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

- **Keep it in a crate of its own**, a workspace's `models`, which the
  client and the native server both depend on, and run `cargo test -p
  models`. See [Building](building.md#a-cargo-workspace).
- **A crate that touches a JS global doesn't link natively.** A binding of
  a global, `webapi::document` say, has nothing behind it outside
  JavaScript, so `cargo test` of that crate fails at link time:

  ```
  error: linking with `cc` failed: exit status: 1
    = note: Undefined symbols for architecture arm64:
              "_document", referenced from: ..
  ```

  A call to a bound function links, and panics if it's run: its body is
  `unreachable!()`.
- **Native isn't JavaScript.** A native test runs on your machine's
  target: `usize` is 64 bits there and 32 in JS, and integer overflow
  panics in a debug build where JS wraps. What differs is
  [Semantics](../semantics.md#differences-from-native-rust-in-one-list).

## Test the JavaScript

The JS rust-js writes is ES modules. Import them in a test, with the
runner your app already has:

```js
// test/area.test.js
import { test } from "node:test";
import assert from "node:assert/strict";
import { area, totalArea } from "../src/lib.js";

test("a unit circle's area is π", () => {
  assert.equal(area(1), Math.PI);
});

test("areas add up", () => {
  assert.equal(totalArea([1, 2]), 5 * Math.PI);
});
```

```bash
node --test test/area.test.js
```

`bun test` and Vitest run the same file. Test React components as you
test any, with Testing Library and a DOM: the JSX rust-js writes is plain
React. Run `rust-js build` first, or test in CI after `rust-js build
--check`, so the JS is the Rust's.

## Rust tests compiled to JavaScript

`rust-js test` runs a crate's `#[test]`s as JavaScript, as `cargo test`
runs them natively ([ADR 0916](../../decisions/0916-rust-js-test-runs-a-crates-tests-as-js.md),
[ADR 1076](../../decisions/1076-each-modules-tests-are-its-test-file-for-the-crates-runner.md)):
its library's and its integration tests', `tests/*.rs`. Cargo builds them,
with rust-js as its wrapper, each dependency compiled to JS as `rust-js
build` compiles it, and the crate's test runner runs them, Node's by
default. It writes nothing beside the Rust; what follows `--` is the
runner's.

```bash
rust-js test
rust-js test --package models -- --test-name-pattern=quadruples
```

### The runner

The crate's `Cargo.toml` says which runner its tests are for, the one
place it's said:

```toml
[package.metadata.rust-js.test]
runner = "vitest"
```

| `runner` | Its test files import | `rust-js test` runs |
|---|---|---|
| `"node"`, the default | `import { test } from "node:test";` | `node --test` |
| `"bun"` | `import { test } from "bun:test";` | `bun test` |
| `"vitest"` | `import { test } from "vitest";` | `vitest run` |
| `"jest"` | `import { test } from "@jest/globals";` | `jest`, with `NODE_OPTIONS=--experimental-vm-modules` for its ES modules |

Each but Node's and bun's is the app's own, in its `devDependencies`.
Another runner is refused, with the four named.

### Each module's tests are its `.test.js`

Beside each module's JS that has tests is its `.test.js`, which every
runner finds by its name. A module's `#[cfg(test)] mod tests`, inline or
in its own file, `src/shapes/tests.rs`, is that file: its helpers and its
tests, each registered under its Rust path.

```rust test
// src/lib.rs
pub mod shapes;
```

```rust
// src/shapes.rs
pub fn area(radius: f64) -> f64 {
    assert!(radius >= 0.0, "negative radius");
    std::f64::consts::PI * radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit() -> f64 {
        area(1.0)
    }

    #[test]
    fn a_unit_circle_has_area_pi() {
        assert_eq!(unit(), std::f64::consts::PI);
    }

    #[test]
    #[should_panic(expected = "negative")]
    fn a_negative_radius_panics() {
        area(-1.0);
    }

    #[test]
    #[ignore]
    fn slow() {}
}
```

```js
// src/shapes.test.js
import { test } from "node:test";
import { area } from "./shapes.js";
import { $assertFailed, $debugF64, $shouldPanic } from "@rust-js/runtime";

function unit() {
  return area(1);
}

function aUnitCircleHasAreaPi() {
  const value = unit();
  const PI = Math.PI;
  if (value !== PI) {
    $assertFailed("Eq", $debugF64(value), $debugF64(PI));
  }
}

function aNegativeRadiusPanics() {
  area(-1);
}

function slow() {}

test("shapes::tests::a_unit_circle_has_area_pi", aUnitCircleHasAreaPi);

test("shapes::tests::a_negative_radius_panics", () =>
  $shouldPanic(aNegativeRadiusPanics, "negative"));

test.skip("shapes::tests::slow", slow);
```

The rule, one for every module:

- **A module's test file is its JS's path, `.test.js`**: `shapes.js`'s
  `shapes.test.js`, the root's `lib.test.js`.
- **A module of tests**, one declared `#[cfg(test)]` that has `#[test]`s
  of its own, is its parent's test file, no file of its own.
- **Each `#[test]` is registered in its module's test file**, or, inside a
  module of tests, in that module's parent's: a `mod nested` in `mod tests`
  is its own file, `shapes/tests/nested.js`, whose tests `shapes.test.js`
  registers.
- **A `#[test]` written in any other module** stays in its module's JS,
  and its module's test file imports and registers it.
- **A module with two modules of tests**, `mod tests` and `mod proptests`,
  has each as its own file, `tests.js` and `proptests.js`, and its test
  file registers theirs.
- **A `#[cfg(test)]` module of no `#[test]`s**, the tests' fixtures, is a
  module as any other.
- **An integration test, `tests/quadruples.rs`**, a crate of its own, is
  `quadruples.js`, and its tests `quadruples.test.js`.

What the tests reach of a module's private items, `use super::*`, is
exported in a test build.

- **A panic is a thrown `Error`** with Rust's message. `assert_eq!` shows
  both sides, as Rust does: ``assertion `left == right` failed``, then
  `left:` and `right:`.
- **`#[should_panic(expected = "..")]` passes on a panic whose message has
  it.** Anything else thrown, a `TypeError`, fails the test: it's the JS
  going wrong, never a panic.
- **`#[ignore]` is `test.skip`**, and a test that returns a `Result` fails
  on its `Err`, as libtest's does.

The compiler writes them alone too, of a crate of no dependency, for the
runner of the nearest `Cargo.toml` above it, and its manifest names each
test file and the runner:

```bash
./target/debug/rust-js --test src/lib.rs -o out/lib.js --manifest out/manifest.json
node --test out/shapes.test.js
```

### A DOM

A test of the DOM needs one, which is the runner's own setup, as for any
JS test; rust-js adds none. With happy-dom:

| Runner | What to set up |
|---|---|
| Node | a module that registers it, `rust-js test -- --import ./test/dom.js` |
| bun | `preload = ["./test/dom.js"]` under `bunfig.toml`'s `[test]` |
| Vitest | `test: { environment: "happy-dom" }` in `vitest.config.js` |
| Jest | `testEnvironment: "@happy-dom/jest-environment"` in its config |

```js
// test/dom.js
import { GlobalRegistrator } from "@happy-dom/global-registrator";

GlobalRegistrator.register();
```

happy-dom keeps one `document` for a test file: each test empties
`document.body` first.

### In real browsers

happy-dom has no layout. A test that needs one runs in Chromium, through
Playwright Test or Vitest's browser mode, both on Bun ([ADR 0027](../../decisions/0027-real-browser-tests.md)): rust-js's
own `browser/` does this for its examples' tests, given the manifests of
their builds. Playwright's page gives the files their runner's `test`;
Vitest's browser mode runs files written for it, `runner = "vitest"`.
Mark a test that needs a real browser, and turn it on with `--cfg=browser`
there:

```rust
#[test]
#[cfg_attr(not(browser), ignore = "needs a real browser")]
fn the_count_sits_between_the_buttons() {
    // ..
}
```
