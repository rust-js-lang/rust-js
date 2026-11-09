// Loaded before each test file of `bun test --parallel` (bunfig.toml), and
// once before all of them one after another.
import { afterAll, setDefaultTimeout } from "bun:test";

import { underLoad } from "../scripts/timing";
import { asNode } from "./node-surface";
import { buildCompiler, releaseFixtures } from "./support";

// The code rust-js writes runs in Node and browsers, not Bun: Bun's own
// built-ins, an iterator's `join` say, taken away (`node-surface.ts`).
asNode();

// A test here runs compilers, browsers and bundlers, which take longer than
// Bun's five seconds when the files run side by side. One after another,
// Bun sets each file's back to five seconds, so `bun run test:serial` gives
// `--timeout` the same minute; a bare `bun test` has Bun's five, which a
// test fits alone. On a loaded machine it's longer, as check:fast's jobs'
// are (`underLoad`): at a load of 43 on 12 cores, a tidy stage's test
// passed its minute and failed, where alone it takes seconds.
setDefaultTimeout(underLoad(60_000));

// The printer of declarations the tests' compiles run, named, as `rust-js
// build` names its own (ADR 0412): the checkout's, which Node would find
// from a fixture in it, so a compile that prints declarations says what it
// read and is kept (ADR 1099, amended). A test of a crate's own printer
// takes it out.
process.env.RUST_JS_TYPESCRIPT ??= Bun.resolveSync("@rust-js/typescript", import.meta.dir);

// The compiler, before any file's tests. `bun run test` builds it before
// Bun starts and gives it to every process from the start, which is what
// reaches a child a file spawns; a bare `bun test` has it built here, by
// whichever file is first (ADR 0104).
buildCompiler();

// Each fixture its tests claimed removed once they've run: Bun's test
// runner never emits `exit`, where a script's process removes its own, and
// each run left all of its fixtures, 7.6 GB a `check:fast --all` (#914).
afterAll(releaseFixtures);
