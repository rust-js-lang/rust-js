import { beforeAll, expect, test } from "bun:test";
import { join, relative } from "node:path";
import { stripVTControlCharacters } from "node:util";
import { root, target, run, buildCompiler, buildReact, compiler } from "./support";

beforeAll(() => {
  buildCompiler();
  buildReact();
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  // Test mode (ADR 0026): the same programs with their `#[test]`s, and some failing on purpose.
  const tests = (rs: string, name: string, flags: string[] = []) =>
    run([compiler, "--test", rs, "-o", join(target, "rust-tests", name, `${name}.js`), ...flags]);
  tests("examples/counter.rs", "counter", withWeb);
  tests("examples/todo.rs", "todo", withWeb);
  tests("examples/countdown.rs", "countdown", withWeb);
  tests("test/asserts.rs", "asserts", withWeb);
  // For real browsers (ADR 0027): `--cfg browser` turns on tests that need one.
  const forBrowser = (rs: string, name: string, flags: string[] = []) =>
    run([compiler, "--test", rs, "-o", join(target, "browser-tests", name, `${name}.js`), ...flags, "--cfg=browser"]);
  forBrowser("examples/counter.rs", "counter", withWeb);
  forBrowser("examples/todo.rs", "todo", withWeb);
  forBrowser("test/asserts.rs", "asserts", withWeb);
}, 600_000);

/** Where `beforeAll` wrote `name`'s tests for `kind`, `rust-tests` or
 * `browser-tests`, from the root the runners run in: under `target`, its
 * own where a mutation's run gives it one, not a copy another run left. */
const built = (kind: string, name: string) => relative(root, join(target, kind, name, `${name}.test.js`));

// ADR 0026: `#[test]` functions, in Rust, compiled by `rust-js --test` and
// run by `bun test` in happy-dom's DOM.
function rustTests(files: string[]): { exit: number; output: string } {
  const p = Bun.spawnSync(["bun", "test", "--preload", "./test/happydom.ts", ...files.map((f) => `./${f}`)], {
    cwd: root,
    stderr: "pipe",
  });
  return { exit: p.exitCode ?? -1, output: p.stdout.toString() + p.stderr.toString() };
}

test("the example apps' own tests pass, in a DOM", () => {
  const apps = ["counter", "todo", "countdown"].map((app) => built("rust-tests", app));
  const { exit, output } = rustTests(apps);
  expect([exit, output.match(/(\d+) pass/)?.[1], output.match(/(\d+) fail/)?.[1]]).toEqual([0, "8", "0"]);
});

test("a failing test fails the way Rust's would", () => {
  const { exit, output } = rustTests([built("rust-tests", "asserts")]);
  expect(exit).toBe(1);
  expect([output.match(/(\d+) pass/)?.[1], output.match(/(\d+) skip/)?.[1], output.match(/(\d+) fail/)?.[1]]).toEqual(["4", "1", "8"]);
  // `assert!` with a message; `assert_eq!` showing both sides, as Rust does.
  expect(output).toContain("error: n was 3");
  expect(output).toContain("error: assertion `left == right` failed\n  left: Point { x: 1, y: 2 }\n right: Point { x: 1, y: 3 }");
  // `#[should_panic]`: the wrong message, and no panic at all.
  expect(output).toContain('panic message: "\\"something\\" happened"\n expected substring: "nope"');
  expect(output).toContain("error: test did not panic as expected");
  // A test returning an `Err`, as libtest shows it.
  expect(output).toContain('error: Error: "no stock"');
  // A `TypeError` is the JS going wrong, whatever it says: never a panic.
  expect(output).toContain("TypeError");
  for (const name of ["fails_an_assert", "fails_an_assert_eq", "panics_with_the_wrong_message", "does_not_panic", "throws_a_type_error", "throws_a_type_error_with_the_message", "rejects_a_promise", "returns_an_err"]) {
    expect(output).toContain(`(fail) tests::${name}`);
  }
});

// ADR 0027: the same tests in Chromium,
// through Playwright Test and through Vitest's browser mode, both on Bun.
const browserTests = [built("browser-tests", "counter"), built("browser-tests", "todo")];

function inBrowsers(runner: "playwright" | "vitest", files: string[]): { exit: number; output: string } {
  const command =
    runner === "playwright"
      ? ["bunx", "--bun", "playwright", "test", "-c", "browser/playwright.config.ts", "--reporter=line"]
      : ["bunx", "--bun", "vitest", "run", "-c", "browser/vitest.config.ts"];
  // A `bun test --parallel` worker says it's Jest's, which Playwright
  // refuses to run inside: the runners here are the browsers' own.
  const { JEST_WORKER_ID: _, ...env } = process.env;
  const p = Bun.spawnSync(command, { cwd: root, env: { ...env, RUST_JS_TESTS: files.join(" ") }, stderr: "pipe" });
  // CI reporters may color individual words and numbers in their summaries.
  return { exit: p.exitCode ?? -1, output: stripVTControlCharacters(p.stdout.toString() + p.stderr.toString()) };
}

test("in real browsers, with Playwright Test on Bun", () => {
  const { exit, output } = inBrowsers("playwright", browserTests);
  // 8 tests, the layout one included, on Chromium.
  expect([exit, output.match(/(\d+) passed/)?.[1]], output).toEqual([0, "8"]);
  const failing = inBrowsers("playwright", [built("browser-tests", "asserts")]);
  expect([failing.exit, failing.output.match(/(\d+) failed/)?.[1], failing.output.match(/(\d+) skipped/)?.[1]]).toEqual([1, "8", "1"]);
  expect(failing.output).toContain("Error: assertion `left == right` failed\n      left: Point { x: 1, y: 2 }");
  // A promise it rejected that no one handled, found once it has run.
  expect(failing.output).toContain("rejected later");
}, 120_000);

test("in real browsers, with Vitest's browser mode", () => {
  const { exit, output } = inBrowsers("vitest", browserTests);
  expect([exit, output.match(/Tests\s+(\d+) passed/)?.[1]], output).toEqual([0, "8"]);
  const failing = inBrowsers("vitest", [built("browser-tests", "asserts")]);
  expect([failing.exit, failing.output.match(/Tests\s+(\d+) failed/)?.[1]]).toEqual([1, "7"]);
  // Vitest fails the run for that promise, not the test.
  expect(failing.output).toMatch(/Errors\s+1 error/);
  // Vitest follows the source map back into the Rust.
  expect(failing.output).toContain("failsAnAssert test/asserts.rs:");
}, 120_000);
