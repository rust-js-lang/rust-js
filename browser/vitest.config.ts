// Vitest's browser mode, with Playwright, for rust-js's `#[test]`s (ADR 0027):
//
//   RUST_JS_TESTS=out/manifest.json bunx --bun vitest run -c browser/vitest.config.ts
//
// Vitest runs each test file inside the browser page, the `test` each
// imports Vitest's own: they're written for it, by a crate whose
// `[package.metadata.rust-js.test]` says `runner = "vitest"` (ADR 1076).

import { readFileSync } from "node:fs";
import { relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

const root = fileURLToPath(new URL("..", import.meta.url));
const manifests = (process.env.RUST_JS_TESTS ?? "").split(/[\s,]+/).filter(Boolean);
if (manifests.length === 0 || manifests.some((path) => !path.endsWith(".json"))) {
  throw new Error("set RUST_JS_TESTS to the manifests `rust-js --test --manifest` wrote, which name each module's test files (ADR 1076)");
}
const files = manifests.flatMap((path) => {
  const { tests } = JSON.parse(readFileSync(resolve(path), "utf8"));
  if (tests.runner !== "vitest") {
    throw new Error(`${path}: its tests are for ${tests.runner}: Vitest runs those of \`[package.metadata.rust-js.test] runner = "vitest"\``);
  }
  // As its `include` globs are, from its root.
  return (tests.files as string[]).map((file) => relative(root, file));
});

export default defineConfig({
  root,
  test: {
    include: files,
    browser: {
      enabled: true,
      headless: true,
      provider: playwright(),
      instances: [{ browser: "chromium" }],
    },
  },
});
