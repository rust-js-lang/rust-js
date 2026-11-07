// The same input gives the same output (ADR 1003, #866): a corpus case
// compiled again, in a process of its own, gives the same JS, source map
// and, built as a library, manifest, byte for byte. The corpus's snapshots,
// the verified runs' keys and every library's manifest assume it; an
// analysis whose answer hangs on the order a map is walked in, which may
// differ from one process to the next, breaks it.
//
//   case.rs ─┬─ rust-js ──► JS, map, manifest ─┐
//            └─ rust-js ──► JS, map, manifest ─┴─► byte for byte the same?
//
//   bun test test/determinism.test.ts                                  # a few cases, each compiled 8 times
//   DETERMINISM=corpus RUST_JS_CASES=1/6 bun test test/determinism.test.ts  # the corpus, a sixth of it, each twice

import { beforeAll, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { cases, corpus, directives } from "./corpus-cases";
import { compileJsAsync } from "./programs";
import { buildCompiler, caseShard, fixture } from "./support";

beforeAll(() => {
  buildCompiler();
});

// Of a run of a few, the cases whose JS was once two from one compile to
// the next, each compiled so often that two forms, each half the time,
// show: ranges' fixpoint walked its callees in a `HashSet`'s order (#866).
const few = ["iterator_closes_of_quiet_items.rs", "ranges_told_by_many_callers.rs"];
const which = process.env.DETERMINISM;
if (which !== undefined && which !== "corpus") throw new Error(`DETERMINISM=${which}: \`corpus\`, or unset for a few cases`);
const chosen = which === "corpus" ? caseShard(cases) : few;
const times = which === "corpus" ? 2 : 8;

test("the cases a run of a few names are the corpus's", () => {
  for (const name of few) expect(cases).toContain(name);
});

// Each compile the compiler's own, never one kept (test/compiles.ts): the
// net is of the compiler, which a kept compile would answer for.
const keepNone = { RUST_JS_COMPILE_CACHE_MB: "0" };

/** What a compile wrote, each file's text by its name, or how it failed. */
async function compiled(file: string, dir: string, edition: string, library: boolean): Promise<Record<string, string>> {
  const result = await compileJsAsync(file, dir, edition, library, keepNone);
  if (!("js" in result)) return { failed: `${result.kind}: ${result.error}` };
  const written: Record<string, string> = {};
  for (const path of [result.js, `${result.js}.map`, result.js.replace(/\.js$/, ".manifest.json")]) {
    if (existsSync(path)) written[path.slice(dir.length)] = readFileSync(path, "utf8");
  }
  return written;
}

for (const name of chosen) {
  const file = join(corpus, name);
  const want = directives(readFileSync(file, "utf8"));
  if (typeof want === "string" || want.kind === "compile-fail") continue;
  test(name, async () => {
    // Each compile in the same folder, one after another, as the paths
    // it's given are part of what it writes: as a program and as a library.
    const dir = fixture(`determinism-${name.replace(/\.rs$/, "")}`);
    const runs = [];
    for (let i = 0; i < times; i++) {
      runs.push({
        program: await compiled(file, dir, want.edition, false),
        library: await compiled(file, dir, want.edition, true),
      });
    }
    for (const run of runs.slice(1)) expect(run).toEqual(runs[0]);
  });
}
