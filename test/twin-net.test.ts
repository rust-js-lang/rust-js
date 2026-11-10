// The twin net's list of the helpers it can't pit (test/twin-net.ts,
// #1205): each a helper of the runtime's, and what a run's helpers make of
// it. The net itself runs as `bun scripts/iteratorSlice.ts run twins`.

import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { helpersOf, importedHelpers, pitted, shared, sharedProblems } from "./twin-net";
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
  expect([[...helpers.pitted], [...helpers.unpitted], helpers.judged, helpers.of]).toEqual([["$lazyZip", "$new"], ["$foldDropping"], 3, 4]);
  // Of a slice: `$foldDropping` is `shared`'s, `$lazyZip` `pitted`'s, and
  // what neither lists is said.
  expect(sharedProblems(helpers, false)).toEqual([]);
  expect(sharedProblems({ pitted: new Set(), unpitted: new Set(["$unlisted"]) }, false)).toEqual([
    "$unlisted: both builds import it and `shared` doesn't say why std's build has no body of its own in its place",
  ]);
  // Of a whole run, what each list says that the run doesn't.
  const whole = sharedProblems({ pitted: new Set(["$foldDropping", "$newlyPitted"]), unpitted: new Set() });
  expect(whole).toContain("$foldDropping: a case pits it against std's body now: take it off `shared`");
  expect(whole).toContain("$newlyPitted: a case pits it against std's body: add it to `pitted`");
  expect(whole).toContain("$lazyZip: no case pits it now: take it off `pitted`");
  expect(whole).toContain("$extend: no case imports it now: take it off `shared`");
});

test("a module's helpers are what it imports of the runtime", () => {
  expect(importedHelpers(`import { $a, $b } from "@rust-js/std";\nimport { iterator_map } from "@rust-js/std/iter.js";\nconst x = 1;\n`)).toEqual(["$a", "$b"]);
});
