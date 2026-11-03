import { beforeAll, expect, test } from "bun:test";
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { stripVTControlCharacters } from "node:util";
import { root, target, run, buildCompiler, buildReact, compiler } from "./support";
import { counts } from "./rust-js-tests";

beforeAll(() => {
  buildCompiler();
  buildReact();
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  // Test mode (ADR 0026): the same programs with their `#[test]`s, and some
  // failing on purpose, for Node's test runner, the default (ADR 1076).
  const compile = (kind: string, rs: string, name: string, flags: string[]) =>
    run([compiler, "--test", rs, "-o", join(target, kind, name, `${name}.js`), "--manifest", join(target, kind, name, "manifest.json"), ...flags]);
  const tests = (rs: string, name: string, flags: string[] = []) => compile("rust-tests", rs, name, flags);
  tests("examples/counter.rs", "counter", withWeb);
  tests("examples/todo.rs", "todo", withWeb);
  tests("examples/countdown.rs", "countdown", withWeb);
  tests("test/asserts.rs", "asserts", withWeb);
  // For real browsers (ADR 0027): `--cfg browser` turns on tests that need
  // one. Playwright's pages give the files their runner's `test`; Vitest's
  // are for Vitest, of a crate that says so (ADR 1076), the file copied there.
  const forBrowser = (rs: string, name: string, flags: string[] = []) => {
    compile("browser-tests", rs, name, [...flags, "--cfg=browser"]);
    const crate = join(target, "vitest-crates", name);
    mkdirSync(crate, { recursive: true });
    writeFileSync(join(crate, "Cargo.toml"), '[package.metadata.rust-js.test]\nrunner = "vitest"\n');
    copyFileSync(join(root, rs), join(crate, `${name}.rs`));
    compile("vitest-tests", join(crate, `${name}.rs`), name, [...flags, "--cfg=browser"]);
  };
  forBrowser("examples/counter.rs", "counter", withWeb);
  forBrowser("examples/todo.rs", "todo", withWeb);
  forBrowser("test/asserts.rs", "asserts", withWeb);
}, 600_000);

/** Where `beforeAll` wrote `name`'s tests for `kind`, `rust-tests`,
 * `browser-tests` or `vitest-tests`: its manifest's test files, from the
 * root the runners run in, under `target`, its own where a mutation's run
 * gives it one, not a copy another run left. */
const built = (kind: string, name: string): string[] =>
  JSON.parse(readFileSync(join(target, kind, name, "manifest.json"), "utf8")).tests.files.map((file: string) => relative(root, file));
const manifest = (kind: string, name: string) => relative(root, join(target, kind, name, "manifest.json"));

// ADR 0026: `#[test]` functions, in Rust, compiled by `rust-js --test` and
// run by Node's test runner, the default (ADR 1076), in happy-dom's DOM:
// the runner's own setup, an `--import` of a module that registers it.
function rustTests(files: string[]): { exit: number; output: string } {
  const p = Bun.spawnSync(["node", "--test", "--import", "./test/happydom.js", ...files], {
    cwd: root,
    stderr: "pipe",
  });
  return { exit: p.exitCode ?? -1, output: p.stdout.toString() + p.stderr.toString() };
}

test("the example apps' own tests pass, in a DOM, by Node's test runner", () => {
  const apps = ["counter", "todo", "countdown"].flatMap((app) => built("rust-tests", app));
  const { exit, output } = rustTests(apps);
  expect([exit, counts("node", output)], output).toEqual([0, { pass: 8, fail: 0, skip: 1 }]);
});

test("a failing test fails the way Rust's would", () => {
  const { exit, output } = rustTests(built("rust-tests", "asserts"));
  expect(exit).toBe(1);
  expect(counts("node", output), output).toEqual({ pass: 5, fail: 7, skip: 1 });
  // A promise it rejected that no one handled fails the run, not the test,
  // as Vitest's does, and Node says which test left it.
  expect(output).toContain('Test "tests::rejects_a_promise"');
  expect(output).toContain('created the error "rejected later" and would have caused the test to fail');
  // `assert!` with a message; `assert_eq!` showing both sides, as Rust does.
  expect(output).toContain("Error: n was 3");
  expect(output).toContain("Error: assertion `left == right` failed\n    left: Point { x: 1, y: 2 }\n   right: Point { x: 1, y: 3 }");
  // `#[should_panic]`: the wrong message, and no panic at all.
  expect(output).toContain('panic message: "\\"something\\" happened"\n   expected substring: "nope"');
  expect(output).toContain("Error: test did not panic as expected");
  // A test returning an `Err`, as libtest shows it.
  expect(output).toContain('Error: Error: "no stock"');
  // A `TypeError` is the JS going wrong, whatever it says: never a panic.
  expect(output).toContain("TypeError");
  for (const name of ["fails_an_assert", "fails_an_assert_eq", "panics_with_the_wrong_message", "does_not_panic", "throws_a_type_error", "throws_a_type_error_with_the_message", "returns_an_err"]) {
    expect(output).toContain(`✖ tests::${name}`);
  }
});

// ADR 0027: the same tests in Chromium,
// through Playwright Test and through Vitest's browser mode, both on Bun.
function inBrowsers(runner: "playwright" | "vitest", manifests: string[]): { exit: number; output: string } {
  const command =
    runner === "playwright"
      ? ["bunx", "--bun", "playwright", "test", "-c", "browser/playwright.config.ts", "--reporter=line"]
      : ["bunx", "--bun", "vitest", "run", "-c", "browser/vitest.config.ts"];
  // A `bun test --parallel` worker says it's Jest's, which Playwright
  // refuses to run inside: the runners here are the browsers' own.
  const { JEST_WORKER_ID: _, ...env } = process.env;
  const p = Bun.spawnSync(command, { cwd: root, env: { ...env, RUST_JS_TESTS: manifests.join(" ") }, stderr: "pipe" });
  // CI reporters may color individual words and numbers in their summaries.
  return { exit: p.exitCode ?? -1, output: stripVTControlCharacters(p.stdout.toString() + p.stderr.toString()) };
}

test("in real browsers, with Playwright Test on Bun", () => {
  const { exit, output } = inBrowsers("playwright", [manifest("browser-tests", "counter"), manifest("browser-tests", "todo")]);
  // 8 tests, the layout one included, on Chromium.
  expect([exit, output.match(/(\d+) passed/)?.[1]], output).toEqual([0, "8"]);
  const failing = inBrowsers("playwright", [manifest("browser-tests", "asserts")]);
  expect([failing.exit, failing.output.match(/(\d+) failed/)?.[1], failing.output.match(/(\d+) skipped/)?.[1]]).toEqual([1, "8", "1"]);
  expect(failing.output).toContain("Error: assertion `left == right` failed\n      left: Point { x: 1, y: 2 }");
  // A promise it rejected that no one handled, found once it has run.
  expect(failing.output).toContain("rejected later");
}, 120_000);

test("in real browsers, with Vitest's browser mode", () => {
  const { exit, output } = inBrowsers("vitest", [manifest("vitest-tests", "counter"), manifest("vitest-tests", "todo")]);
  expect([exit, output.match(/Tests\s+(\d+) passed/)?.[1]], output).toEqual([0, "8"]);
  const failing = inBrowsers("vitest", [manifest("vitest-tests", "asserts")]);
  expect([failing.exit, failing.output.match(/Tests\s+(\d+) failed/)?.[1]]).toEqual([1, "7"]);
  // Vitest fails the run for that promise, not the test.
  expect(failing.output).toMatch(/Errors\s+1 error/);
  // Vitest follows the source map back into the Rust.
  expect(failing.output).toMatch(/failsAnAssert \S*asserts\.rs:/);
  // Files for another runner are refused, saying which they're for.
  const other = inBrowsers("vitest", [manifest("browser-tests", "counter")]);
  expect(other.output).toContain('its tests are for node: Vitest runs those of `[package.metadata.rust-js.test] runner = "vitest"`');
}, 120_000);
