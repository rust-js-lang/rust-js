#!/usr/bin/env node
// `rust-js-next dev|build [next's arguments]`: an app's scripts, in
// place of `next dev` and `next build` (ADR 0192); `rust-js-next compile`,
// its Rust's JS alone, for what its build reads first.
import { run } from "./index.js";

const [command, ...args] = process.argv.slice(2);
if (!["dev", "build", "compile"].includes(command)) {
  process.stderr.write("usage: rust-js-next dev|build [next's arguments] | compile\n");
  process.exit(2);
}
try {
  process.exit(await run({ app: process.cwd(), command, args, rustJs: process.env.RUST_JS_COMPILER }));
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : error}\n`);
  process.exit(1);
}
