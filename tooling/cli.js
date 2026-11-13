#!/usr/bin/env node
// `rust-js`, the one way an app uses rust-js (ADR 0412): `rust-js build`
// compiles the app's Rust in the directory it runs in, once, to JS and
// `.d.ts` beside it: the package its `Cargo.toml` is of, or a workspace's
// members, all of them at once, or those `--package <name>` names, each
// given its own, as Cargo's `-p` is (ADR 0360, amended); and `rust-js
// build --check` says whether what's there is what it compiles to, as CI
// asks before a build. `rust-js clippy` lints it with clippy's lints, as
// rust-js reads it, its lint flags after `--`, `-- -D warnings`, as `cargo
// clippy`'s are (ADR 0888). `rust-js test` runs its library's `#[test]`s as
// JS, as `cargo test` runs them natively, by the test runner the crate names,
// its flags after `--` (ADRs 0916, 1076), and the crate's features
// `--features` names and `--no-default-features`, Cargo's own flags,
// given it as they are. `rust-js bindings ./site.module.css` prints the
// binding of a CSS Module's classes, run where the crate root's JS is, the
// form the compiler checks against the file, once for the app to keep
// (ADR 1133).
// The compiler is this command's, not the app's to run.
import { buildApp, clippyApp, printBindings, testApp } from "./build.js";

const usage = (said) => {
  process.stderr.write(`rust-js: ${said}\nusage: rust-js build [--package <name>]... [--check]\n       rust-js clippy [--package <name>]... [-- <lint flags>]\n       rust-js test [--package <name>]... [--features <features>] [--no-default-features] [-- <test runner flags>]\n       rust-js bindings <./x.module.css>\n`);
  process.exit(2);
};
const [command, ...rest] = process.argv.slice(2);
if (command !== "build" && command !== "clippy" && command !== "test" && command !== "bindings") usage(command === undefined ? "no command" : `unknown command ${command}`);
if (command === "bindings") {
  if (rest.length !== 1 || rest[0].startsWith("-")) usage("bindings needs the CSS Module, `./site.module.css`");
  try {
    process.exit(await printBindings({ app: process.cwd(), specifier: rest[0] }));
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exit(1);
  }
}
let check = false, lints = [];
const packageNames = [];
const features = [];
let noDefaultFeatures = false;
for (let i = 0; i < rest.length; i++) {
  if (rest[i] === "--check" && command === "build") check = true;
  else if (rest[i] === "--" && (command === "clippy" || command === "test")) {
    lints = rest.slice(i + 1);
    break;
  } else if (rest[i] === "--no-default-features" && command === "test") noDefaultFeatures = true;
  else if (rest[i] === "--features" && command === "test") {
    const list = rest[++i];
    if (!list || list.startsWith("-")) usage("--features needs the features, `v4,js`");
    features.push(...list.split(/[,\s]+/).filter(Boolean));
  } else if (rest[i] === "--package") {
    const name = rest[++i];
    if (!name || name.startsWith("-")) usage("--package needs the package's name");
    packageNames.push(name);
  } else usage(`unknown argument ${rest[i]}`);
}
try {
  if (command === "clippy") process.exitCode = await clippyApp({ app: process.cwd(), packageNames, lints });
  else if (command === "test") process.exitCode = await testApp({ app: process.cwd(), packageNames, features, noDefaultFeatures, args: lints });
  else await buildApp({ app: process.cwd(), packageNames, check });
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
}
