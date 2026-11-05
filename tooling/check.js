#!/usr/bin/env node
// The check an editor runs (ADR 0222): an app's `cargo check` through
// rust-js, as its dev server's, for rust-analyzer, which a plain rustc can't
// check JSX for, in the app's `.vscode/settings.json`:
//
//   "rust-analyzer.check.overrideCommand": ["./node_modules/.bin/rust-js-check"]
//
// A full-stack workspace names the packages rust-js compiles, `--package
// frontend`, as `rust-js build` does, and the rest of its members, native
// Rust, are checked by a plain `cargo check`, as rust-analyzer's own check
// would (ADR 1094).
//
//   rust-js-check [<app-dir>] [--package <name>]...
import { join, resolve } from "node:path";
import { cacheDirOf, createNativeBuilder, findCompiler } from "./build.js";
import { nativeCheck } from "./cargo.js";
import { writePatch } from "./patch.js";

const usage = (said) => {
  process.stderr.write(`rust-js-check: ${said}\nusage: rust-js-check [<app-dir>] [--package <name>]...\n`);
  process.exit(2);
};
let dir;
const packageNames = [];
const args = process.argv.slice(2);
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--package") {
    const name = args[++i];
    if (!name || name.startsWith("-")) usage("--package needs the package's name");
    packageNames.push(name);
  } else if (args[i].startsWith("-") || dir !== undefined) usage(`unknown argument ${args[i]}`);
  else dir = args[i];
}
const app = resolve(dir ?? ".");
try {
  // Its crates where its npm packages have them, as the dev server's.
  writePatch(app);
  const manifestPath = join(app, "Cargo.toml");
  const builder = createNativeBuilder({ root: app, rustJs: await findCompiler(app) });
  const rustJs = await builder.lintCheck({
    manifestPath,
    packageNames,
    // Not the dev server's, whose Turbopack or Vite doesn't watch it either.
    targetDir: process.env.CARGO_TARGET_DIR ?? join(cacheDirOf(app), "editor"),
  });
  // One after the other: each prints whole lines of JSON to one stdout.
  const native = packageNames.length > 0 ? await nativeCheck({ manifestPath, except: packageNames }) : 0;
  process.exitCode = rustJs || native;
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : error}\n`);
  process.exitCode = 1;
}
