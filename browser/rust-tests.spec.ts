// rust-js's compiled `#[test]`s, run in real browsers by Playwright Test
// (ADR 0027). Playwright runs on Bun:
//
//   RUST_JS_TESTS=out/manifest.json bunx --bun playwright test -c browser/playwright.config.ts
//
// Each Rust test gets a fresh page, which loads its module's test file as an
// ES module, the `test` it imports from its runner's module (ADR 1076) one
// that only records, then runs that one test. The page is served from a
// made-up origin by `page.route`: no server.

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve, sep } from "node:path";

import { test } from "@playwright/test";

const ORIGIN = "http://rust-js.test";

const manifests = (process.env.RUST_JS_TESTS ?? "").split(/[\s,]+/).filter(Boolean);
if (manifests.length === 0 || manifests.some((path) => !path.endsWith(".json"))) {
  throw new Error("set RUST_JS_TESTS to the manifests `rust-js --test --manifest` wrote, which name each module's test files (ADR 1076)");
}

/** `@rust-js/runtime`, as the project installed it: what the modules import (ADR 0103). */
const runtime = createRequire(join(process.cwd(), "package.json")).resolve("@rust-js/runtime");
const RUNTIME = "/@rust-js/runtime.js";
/** The runner's module, as the page has it: a `test` that records. */
const RUNNER = "/@rust-js/test.js";

/** The page: its runner's `test`, as the test files import it, and a way to
 * run one of the tests of the file at `path`, from the build's directory. */
const page = (module: string, path: string) => `<!doctype html>
<meta charset="utf-8">
<body></body>
<script type="importmap">${JSON.stringify({ imports: { "@rust-js/runtime": RUNTIME, [module]: RUNNER } })}</script>
<script>
  globalThis.tests = new Map();
  globalThis.test = (name, f) => tests.set(name, f);
  test.skip = () => {};
</script>
<script type="module">
  const loaded = import(${JSON.stringify(`./${path}`)});
  // A panic in an event handler doesn't reach the test: the browser reports it.
  // Nor does a promise it rejected that no one handles, as a runner fails it.
  const escaped = [];
  addEventListener("error", (e) => escaped.push(e.message));
  addEventListener("unhandledrejection", (e) => escaped.push(e.reason instanceof Error ? e.reason.message : String(e.reason)));
  window.runRustTest = async (name) => {
    await loaded;
    try {
      await tests.get(name)();
    } catch (e) {
      return e instanceof Error ? e.message : String(e);
    }
    // What it left for later has run by then: its microtasks, as this task
    // ends, and the task that reports the rejections they left unhandled,
    // queued as they end, before the second of these.
    for (let i = 0; i < 2; i++) await new Promise((settled) => setTimeout(settled));
    return escaped.length ? escaped.join("\\n") : null;
  };
</script>`;

for (const manifestPath of manifests) {
  const manifest = JSON.parse(readFileSync(resolve(manifestPath), "utf8"));
  const { module, files } = manifest.tests as { module: string; files: string[] };
  // Every file the build wrote is below its output's directory.
  const dir = dirname(manifest.output);
  for (const path of files) {
    // The tests' names, as rust-js wrote them, by the name the file imports
    // its runner's `test` by: test("tests::adds", ..).
    const source = readFileSync(path, "utf8");
    const imported = new RegExp(`^import \\{ test(?: as ([\\w$]+))? \\} from ${JSON.stringify(module)};$`, "m").exec(source);
    const local = (imported?.[1] ?? "test").replace(/\$/g, "\\$");
    const found = [...source.matchAll(new RegExp(`^${local}(\\.skip)?\\(("(?:[^"\\\\]|\\\\.)*"), `, "gm"))];

    test.describe(relative(process.cwd(), path), () => {
      for (const [, skip, label] of found) {
        const name: string = JSON.parse(label);
        (skip ? test.skip : test)(name, async ({ page: tab }) => {
          await tab.route(`${ORIGIN}/**`, (route) => {
            const url = new URL(route.request().url());
            if (url.pathname === "/") {
              return route.fulfill({ contentType: "text/html", body: page(module, relative(dir, path)) });
            }
            if (url.pathname === RUNTIME) return route.fulfill({ path: runtime, contentType: "text/javascript" });
            if (url.pathname === RUNNER) {
              return route.fulfill({ contentType: "text/javascript", body: "export const test = globalThis.test;\n" });
            }
            // Only the build's files.
            const target = resolve(dir, `.${decodeURIComponent(url.pathname)}`);
            return target.startsWith(dir + sep) ? route.fulfill({ path: target }) : route.fulfill({ status: 404 });
          });
          await tab.goto(`${ORIGIN}/`);
          const failure = await tab.evaluate<string | null>(`window.runRustTest(${JSON.stringify(name)})`);
          if (failure !== null) throw new Error(failure);
        });
      }
    });
  }
}
