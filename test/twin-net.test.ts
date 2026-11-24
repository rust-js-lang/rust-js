// The twin net's list of the helpers it can't pit (test/twin-net.ts,
// #1205): each a helper of the runtime's, and what a run's helpers make of
// it. The net itself runs as `bun scripts/iteratorSlice.ts run twins`.

import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { blessedLists, helpersCommand, helpersOf, importedHelpers, pitted, shared, twinVerdict } from "./twin-net";
import { fixture } from "./support";

test("each helper `pitted` and `shared` name is one the runtime declares", () => {
  const runtime = readFileSync(join(import.meta.dir, "..", "std", "runtime.js"), "utf8");
  const declared = new Set([...runtime.matchAll(/^(?:export )?(?:async )?(?:function\*? |class |const |let )(\$[A-Za-z0-9_$]*)/gm)].map((m) => m[1]));
  expect([...pitted, ...Object.keys(shared)].filter((helper) => !declared.has(helper))).toEqual([]);
});

test("a run's helpers are pitted where only the plain build imports them, and `shared` says why of the rest", () => {
  const dir = join(fixture("twin-net-helpers"), "runs");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "matrix-0.json"), JSON.stringify({ judged: 2, of: 3, plain: ["$lazyZip", "$foldDropping", "$new"], std: ["$foldDropping", "$new", "$advanceSlice"] }));
  writeFileSync(join(dir, "matrix-1.json"), JSON.stringify({ judged: 1, of: 1, plain: ["$new", "iterator_map"], std: [] }));
  const helpers = helpersOf(dir);
  // In the order the batches' names give, `helpersOf`'s, on every system.
  expect([[...helpers.pitted], [...helpers.unpitted], helpers.judged, helpers.of]).toEqual([["$lazyZip", "$new"], ["$foldDropping"], 3, 4]);
  // Of a slice: `$foldDropping` is `shared`'s, `$lazyZip` `pitted`'s, and
  // what neither lists is said.
  expect(twinVerdict(helpers, false)).toEqual({ gaps: [], drift: [], pit: [], unshare: [] });
});

// A run as the lists say, but for what each test changes.
const asListed = () => ({ pitted: new Set(pitted), unpitted: new Set(Object.keys(shared)) });

test("a gap fails a run: a helper neither pitted nor listed, and of a whole run one `pitted` lists that no case pits now", () => {
  const run = asListed();
  run.unpitted.add("$unlisted");
  expect(twinVerdict(run, false).gaps).toEqual(["$unlisted: both builds import it and `shared` doesn't say why std's build has no body of its own in its place"]);
  // Coverage lost: `$lazyZip` both builds import now, `$lent` none; a
  // slice may see either so, a whole run says it.
  run.pitted.delete("$lazyZip");
  run.unpitted.add("$lazyZip");
  run.pitted.delete("$lent");
  expect(twinVerdict(run, false).gaps).toHaveLength(1);
  const whole = twinVerdict(run, true);
  expect(whole.gaps).toEqual([
    "$lazyZip: `pitted` lists it and both builds import it now, so no case pits it against std's body: coverage lost: pit it again, or move it to `shared` saying why",
    "$lent: `pitted` lists it and no case imports it now: coverage lost: give a table a case that reaches it, or take it off `pitted` where nothing makes it",
    "$unlisted: both builds import it and `shared` doesn't say why std's build has no body of its own in its place",
  ]);
  expect(whole.drift).toEqual([]);
});

test("drift fails no run, and its bless writes the lists as the run found them", () => {
  // `$foldDropping`, `shared`'s, is pitted now; `$newlyPitted` is pitted
  // and listed by neither; `$extend`, `shared`'s, nothing imports now.
  const run = asListed();
  run.unpitted.delete("$foldDropping");
  run.pitted.add("$foldDropping");
  run.pitted.add("$newlyPitted");
  run.unpitted.delete("$extend");
  const verdict = twinVerdict(run, true);
  expect(verdict.gaps).toEqual([]);
  expect(verdict.drift).toEqual([
    "$extend: no case imports it now: off `shared`",
    "$foldDropping: a case pits it against std's body now: from `shared` to `pitted`",
    "$newlyPitted: a case pits it against std's body: onto `pitted`",
  ]);
  // A slice says none of it.
  expect(twinVerdict(run, false)).toEqual({ gaps: [], drift: [], pit: [], unshare: [] });
  const source = readFileSync(join(import.meta.dir, "twin-net.ts"), "utf8");
  const blessed = blessedLists(source, verdict);
  const block = (text: string) => text.slice(text.indexOf("export const pitted"), text.indexOf("];\n", text.indexOf("export const pitted")));
  expect([...block(blessed).matchAll(/^ {2}"(\$\w+)",$/gm)].map((m) => m[1])).toEqual([...pitted, "$foldDropping", "$newlyPitted"].sort((a, b) => (a < b ? -1 : 1)));
  expect(blessed).not.toMatch(/^ {2}"\$extend": /m);
  expect(blessed).not.toMatch(/^ {2}"\$foldDropping": /m);
  // Nothing else moves: two lines on, two off.
  expect(blessed.split("\n").filter((line) => !source.includes(line))).toEqual(['  "$foldDropping",', '  "$newlyPitted",']);
  expect(blessed.split("\n")).toHaveLength(source.split("\n").length);
  // Blessed again, what it wrote is what the run says.
  expect(blessedLists(blessed, verdict)).toBe(blessed);
});

test("`BLESS=1 bun test/twin-net.ts helpers` writes the drift of a whole run that passed, and of no other", () => {
  const at = fixture("twin-net-bless");
  const dir = join(at, "runs");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const sharedNow = Object.keys(shared).filter((helper) => helper !== "$extend");
  writeFileSync(join(dir, "consumers-0.json"), JSON.stringify({ judged: 1, of: 1, plain: [...pitted, "$newlyPitted", ...sharedNow], std: sharedNow }));
  const source = readFileSync(join(import.meta.dir, "twin-net.ts"), "utf8");
  const file = join(at, "twin-net.ts");
  const ran = join(at, "run.json");
  const bless = (run: { whole: boolean; passed: boolean }) => {
    writeFileSync(file, source);
    writeFileSync(ran, JSON.stringify(run));
    return helpersCommand({ dir, ran, file, bless: true });
  };
  // A whole run that failed: its helpers may be some batches' alone.
  expect(() => bless({ whole: true, passed: false })).toThrow("only a whole run that passed is blessed");
  expect(readFileSync(file, "utf8")).toBe(source);
  // A slice says no drift, so writes none.
  expect(bless({ whole: false, passed: true })).toBe(true);
  expect(readFileSync(file, "utf8")).toBe(source);
  expect(bless({ whole: true, passed: true })).toBe(true);
  const blessed = readFileSync(file, "utf8");
  expect(blessed).toContain('  "$newlyPitted",\n');
  expect(blessed).not.toMatch(/^ {2}"\$extend": /m);
});

test("a module's helpers are what it imports of the runtime", () => {
  expect(importedHelpers(`import { $a, $b } from "@rust-js/std";\nimport { iterator_map } from "@rust-js/std/iter.js";\nconst x = 1;\n`)).toEqual(["$a", "$b"]);
});
