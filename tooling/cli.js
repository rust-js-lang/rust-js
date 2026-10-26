#!/usr/bin/env node
// `rust-js`, the one way an app uses rust-js (ADR 0412): `rust-js build`
// compiles the app's Rust in the directory it runs in, once, to JS and
// `.d.ts` beside it, of a workspace's `--package <name>`; and `rust-js
// build --check` says whether what's there is what it compiles to, as CI
// asks before a build. `rust-js clippy` lints it with clippy's lints, as
// rust-js reads it, its lint flags after `--`, `-- -D warnings`, as `cargo
// clippy`'s are (ADR 0888). `rust-js test` runs its library's `#[test]`s as
// JS, as `cargo test` runs them natively, bun's flags after `--` (ADR 0914).
// The compiler is this command's, not the app's to run.
import { buildApp, clippyApp, testApp } from "./build.js";

const usage = (said) => {
  process.stderr.write(`rust-js: ${said}\nusage: rust-js build [--package <name>] [--check]\n       rust-js clippy [--package <name>] [-- <lint flags>]\n       rust-js test [--package <name>] [-- <bun test flags>]\n`);
  process.exit(2);
};
const [command, ...rest] = process.argv.slice(2);
if (command !== "build" && command !== "clippy" && command !== "test") usage(command === undefined ? "no command" : `unknown command ${command}`);
let check = false, packageName, lints = [];
for (let i = 0; i < rest.length; i++) {
  if (rest[i] === "--check" && command === "build") check = true;
  else if (rest[i] === "--" && (command === "clippy" || command === "test")) {
    lints = rest.slice(i + 1);
    break;
  } else if (rest[i] === "--package") {
    packageName = rest[++i];
    if (!packageName || packageName.startsWith("-")) usage("--package needs the package's name");
  } else usage(`unknown argument ${rest[i]}`);
}
try {
  if (command === "clippy") process.exitCode = await clippyApp({ app: process.cwd(), packageName, lints });
  else if (command === "test") process.exitCode = await testApp({ app: process.cwd(), packageName, args: lints });
  else await buildApp({ app: process.cwd(), packageName, check });
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
}
