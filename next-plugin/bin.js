#!/usr/bin/env node
// `rust-js-next compile`: an app's Rust's JS, written beside it, for what
// reads it before Next.js runs. `next dev` and `next build` compile it
// themselves, through the app's config, `withRustJs` (ADR 0380).
import { compile } from "./index.js";

const [command, ...args] = process.argv.slice(2);
if (command !== "compile" || args.length > 0) {
  process.stderr.write("usage: rust-js-next compile\n`next dev` and `next build` compile an app's Rust through its config: `withRustJs(nextConfig)`\n");
  process.exit(2);
}
try {
  await compile({ app: process.cwd() });
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : error}\n`);
  process.exit(1);
}
