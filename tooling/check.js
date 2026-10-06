#!/usr/bin/env node
// The check an editor runs (ADR 0222): an app's `cargo check` through
// rust-js, as its dev server's, for rust-analyzer, which a plain rustc can't
// check JSX for, in the app's `.vscode/settings.json`:
//
//   "rust-analyzer.check.overrideCommand": ["./node_modules/.bin/rust-js-check"]
//
//   rust-js-check [app-dir]
import { join, resolve } from "node:path";
import { createNativeBuilder, findCompiler } from "./build.js";
import { writePatch } from "./patch.js";

const app = resolve(process.argv[2] ?? ".");
try {
  // Its crates where its npm packages have them, as the dev server's.
  writePatch(app);
  const builder = createNativeBuilder({ root: app, rustJs: resolve(app, process.env.RUST_JS_COMPILER ?? findCompiler(app)) });
  process.exitCode = await builder.editorCheck({
    manifestPath: join(app, "Cargo.toml"),
    // Not the dev server's, whose Turbopack or Vite doesn't watch it either.
    targetDir: process.env.CARGO_TARGET_DIR ?? join(app, "node_modules/.cache/rust-js/editor"),
  });
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : error}\n`);
  process.exitCode = 1;
}
