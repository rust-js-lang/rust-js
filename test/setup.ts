// Loaded before each test file of `bun test --parallel` (bunfig.toml), and
// once before all of them one after another.
import { setDefaultTimeout } from "bun:test";

import { asNode } from "./node-surface";
import { buildCompiler } from "./support";

// The code rust-js writes runs in Node and browsers, not Bun: Bun's own
// built-ins, an iterator's `join` say, taken away (`node-surface.ts`).
asNode();

// A test here runs compilers, browsers and bundlers, which take longer than
// Bun's five seconds when the files run side by side. One after another,
// Bun sets each file's back to five seconds, so `bun run test:serial` gives
// `--timeout` the same minute; a bare `bun test` has Bun's five, which a
// test fits alone.
setDefaultTimeout(60_000);

// The compiler, before any file's tests. `bun run test` builds it before
// Bun starts and gives it to every process from the start, which is what
// reaches a child a file spawns; a bare `bun test` has it built here, by
// whichever file is first (ADR 0104).
buildCompiler();
