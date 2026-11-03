# Tests in real browsers

rust-js compiles a crate's `#[test]` functions with `rust-js --test`, into a
`.test.js` beside each module's JS that has tests, each importing `test` from
the crate's runner (ADRs 0026, 1076). Node's test runner runs them in
happy-dom; this folder runs the same files in **Chromium**, through
Playwright, on Bun. See
[ADR 0027](../docs/decisions/0027-real-browser-tests.md).

```bash
bun run setup                                  # once: the tools, and the browsers

# Compile the tests for a browser: `--cfg browser` turns on the ones that need one.
./target/debug/rust-js --test examples/counter.rs -o out/counter.js --manifest out/manifest.json -- --extern webapi=target/libwebapi.rmeta --cfg=browser

# Then Playwright Test, whose pages give the files their runner's `test`:
RUST_JS_TESTS=out/manifest.json bunx --bun playwright test -c browser/playwright.config.ts
```

`RUST_JS_TESTS` takes several manifests, separated by spaces. Vitest's
browser mode runs files written for Vitest, of a crate whose `Cargo.toml`
says so:

```toml
[package.metadata.rust-js.test]
runner = "vitest"
```

```bash
RUST_JS_TESTS=out/manifest.json bunx --bun vitest run -c browser/vitest.config.ts
```

| File | What it is |
|---|---|
| `playwright.config.ts` | Playwright Test: Chromium |
| `rust-tests.spec.ts` | Runs each Rust test in a fresh page |
| `vitest.config.ts` | Vitest's browser mode, with the Playwright provider |

Mark a test that only makes sense in a real browser, such as one that needs
layout, which happy-dom doesn't do:

```rust
#[test]
#[cfg_attr(not(browser), ignore = "needs a real browser")]
fn the_count_sits_between_the_buttons() { .. }
```
