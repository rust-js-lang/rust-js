// How `rust-js test` runs a crate's tests (ADR 1076): by the JS test runner
// they're written for, `[package.metadata.rust-js.test]`'s `runner`, as the
// compiler's manifest records it. The compiler owns which module each
// runner's `test` is imported from (`src/settings.rs`); this, the command
// each is run by. test/rust-js-test-runners.test.ts runs each.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

/**
 * The command that runs `files`, test files beside the JS rust-js wrote in
 * `targetDir`, by `runner`, `args` given it, from the app at `app`, which
 * has the runner installed, but Node's and bun's own. Each runner leaves
 * out what's in `node_modules`, where the target is by default, as it finds
 * tests: each is given the files themselves, and where to look.
 * @param {string} runner
 * @param {{ app: string, targetDir: string, files: string[], args: string[] }} options
 * @returns {{ command: string, args: string[], env: Record<string, string> }}
 */
export function runnerCommand(runner, { app, targetDir, files, args }) {
  switch (runner) {
    case "node":
      return { command: "node", args: ["--test", ...args, ...files], env: {} };
    case "bun":
      return { command: process.versions.bun ? process.execPath : "bun", args: ["test", ...args, ...files], env: {} };
    case "vitest":
      // Positional arguments filter what it finds in `--dir`.
      return { command: "node", args: [bin(app, runner, "vitest"), "run", "--dir", targetDir, ...args, ...files], env: {} };
    case "jest":
      // Its haste map, and the paths it tests, leave out `node_modules`
      // unless told; and ES modules, which rust-js writes, need Node's VM
      // modules (Jest's ECMAScript Modules guide).
      return {
        command: "node",
        args: [bin(app, runner, "jest"), "--roots", targetDir, "--haste", JSON.stringify({ retainAllFiles: true }),
          "--testPathIgnorePatterns", "(?!)", ...args, "--runTestsByPath", ...files],
        env: { NODE_OPTIONS: [process.env.NODE_OPTIONS, "--experimental-vm-modules"].filter(Boolean).join(" ") },
      };
    default:
      throw new Error(`rust-js: no test runner ${JSON.stringify(runner)}: \`[package.metadata.rust-js.test] runner\` is "node", "bun", "vitest" or "jest"`);
  }
}

/** The script of package `name`'s command, as the app at `app` has it installed. */
function bin(app, runner, name) {
  const require = createRequire(join(app, "package.json"));
  let manifest;
  try { manifest = require.resolve(`${name}/package.json`); }
  catch {
    throw new Error(`rust-js: the crate's tests are for ${runner}, which ${app} doesn't have installed: add it to its devDependencies`);
  }
  const { bin } = JSON.parse(readFileSync(manifest, "utf8"));
  return join(dirname(manifest), typeof bin === "string" ? bin : bin[name]);
}
