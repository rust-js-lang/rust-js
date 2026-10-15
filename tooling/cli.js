#!/usr/bin/env node
// `rust-js`, the one way an app uses rust-js (ADR 0412): `rust-js build`
// compiles the app's Rust in the directory it runs in, once, to JS and
// `.d.ts` beside it, of a workspace's `--package <name>`; and `rust-js
// build --check` says whether what's there is what it compiles to, as CI
// asks before a build. The compiler is this command's, not the app's to run.
import { buildApp } from "./build.js";

const usage = (said) => {
  process.stderr.write(`rust-js: ${said}\nusage: rust-js build [--package <name>] [--check]\n`);
  process.exit(2);
};
const [command, ...rest] = process.argv.slice(2);
if (command !== "build") usage(command === undefined ? "no command" : `unknown command ${command}`);
let check = false, packageName;
for (let i = 0; i < rest.length; i++) {
  if (rest[i] === "--check") check = true;
  else if (rest[i] === "--package") {
    packageName = rest[++i];
    if (!packageName || packageName.startsWith("-")) usage("--package needs the package's name");
  } else usage(`unknown argument ${rest[i]}`);
}
try {
  await buildApp({ app: process.cwd(), packageName, check });
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
}
