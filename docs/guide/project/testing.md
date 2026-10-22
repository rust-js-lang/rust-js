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

```
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

The compiler itself compiles a crate's `#[test]`s to JS too, with
rustc's `--test` and its harness left out ([ADR 0026](../../decisions/0026-testing.md)). It's how rust-js
tests its own examples, from a checkout. `rust-js build` doesn't compile
tests yet, and the compiler run alone doesn't find the crates Cargo builds,
so an app's Rust tests are `cargo test`'s; this is what the JS is.

```bash
./target/debug/rust-js --test src/lib.rs -o out/lib.js
```

Beside each module's JS, it writes `out/lib.test.js`, which calls the
runner's global `test()` for each `#[test]`:

```js
// out/lib.test.js
import * as tests from "./shapes/tests.js";

test("shapes::tests::a_unit_circle_has_area_pi", tests.aUnitCircleHasAreaPi);
```

```js
// out/shapes/tests.js
import { area } from "../shapes.js";
import { $assertFailed, $debugF64 } from "@rust-js/runtime";

export function aUnitCircleHasAreaPi() {
  const value = area(1);
  const value$1 = 3.141592653589793;
  if (value !== value$1) {
    $assertFailed("Eq", $debugF64(value), $debugF64(value$1));
  }
}
```

```bash
bun test out/lib.test.js
# 1 pass
```

Any runner with a global `test()` runs it: `bun test`, with happy-dom
preloaded for a DOM, and Vitest with `globals: true`.

- **A panic is a thrown `Error`** with Rust's message. `assert_eq!` shows
  both sides, as Rust does: ``assertion `left == right` failed``, then
  `left:` and `right:`.
- **`#[should_panic(expected = "..")]` passes on a panic whose message has
  it.** Anything else thrown, a `TypeError`, fails the test: it's the JS
  going wrong, never a panic.
- **`#[ignore]` is `test.skip`**, and a test that returns a `Result` fails
  on its `Err`, as libtest's does.

### In real browsers

happy-dom has no layout. A test that needs one runs in Chromium, through
Playwright Test or Vitest's browser mode, both on Bun ([ADR 0027](../../decisions/0027-real-browser-tests.md)): rust-js's
own `browser/` does this for its examples' tests. Mark a test that needs a
real browser, and turn it on with `--cfg=browser` there:

```rust
#[test]
#[cfg_attr(not(browser), ignore = "needs a real browser")]
fn the_count_sits_between_the_buttons() {
    // ..
}
```
