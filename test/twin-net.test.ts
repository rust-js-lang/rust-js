// The twin net's reading of the one catalog of the runtime's models
// (models.rs, which `rust-js --std-models` prints; ADR 1202, #1205): each a
// helper of the runtime's, and what a run's helpers make of it. The net
// itself runs as `bun scripts/iteratorSlice.ts run twins`.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { blessedCatalog, catalog, catalogFile, helpersCommand, helpersOf, importedHelpers, pittedHelpers, sharedHelpers, twinVerdict, unboundModels } from "./twin-net";
import { buildCompiler, fixture } from "./support";

beforeAll(buildCompiler, 600_000);

test("each helper the catalog names is one the runtime declares", () => {
  const runtime = readFileSync(join(import.meta.dir, "..", "std", "runtime.js"), "utf8");
  const declared = new Set([...runtime.matchAll(/^(?:export )?(?:async )?(?:function\*? |class |const |let )(\$[A-Za-z0-9_$]*)/gm)].map((m) => m[1]));
  expect([...pittedHelpers(), ...Object.keys(sharedHelpers())].filter((helper) => !declared.has(helper))).toEqual([]);

  // A form the lowering writes is named by the row of `$stdSites` it reads.
  const sites = readFileSync(join(import.meta.dir, "..", "crates", "rust-js-program", "src", "runtime", "std_sites.js"), "utf8");
  const rows = new Set(Object.keys(JSON.parse(sites.slice(sites.indexOf("{"), sites.lastIndexOf("}") + 1))));
  const forms = catalog().models.filter((entry) => entry.kind === "form").map((entry) => entry.helper);
  expect(forms.length).toBeGreaterThan(0);
  expect(forms.filter((form) => !rows.has(form))).toEqual([]);
});

// A model `outside` the twin net's tables, which none reaches yet, isn't
// taken off models.rs where no case imports it; glue nothing imports is.
test("a whole run that imports no model outside the tables keeps it", () => {
  const { unshare } = twinVerdict({ pitted: new Set(), unpitted: new Set() }, true);
  expect(unshare).not.toContain("$durationAdd");
  expect(unshare.filter((helper) => sharedHelpers()[helper] === "outside")).toEqual([]);
  expect(unshare.length).toBeGreaterThan(0);
});

// #2513's measure: how many models stand for no std item yet, which each
// step of Campaign 5 binds as it touches them; never more than this.
test("how many models stand for no std item yet", () => {
  const unbound = unboundModels();
  console.log(`${unbound.length} models stand for no std item yet`);
  expect(unbound.length).toBeLessThanOrEqual(207);
});

// Of the runtime's exports, those models.rs doesn't say are glue or a
// model yet, which each step of Campaign 5 catalogs as it touches them;
// never more than this, so a helper added to the runtime is catalogued
// with it (ADR 1202, amended).
test("how many runtime exports the catalog doesn't name yet", () => {
  const runtime = readFileSync(join(import.meta.dir, "..", "std", "runtime.js"), "utf8");
  const exported = [...runtime.matchAll(/^export (?:async )?(?:function\*? |class |const |let )(\$[A-Za-z0-9_$]*)/gm)].map((m) => m[1]);
  const named = new Set(catalog().models.filter((entry) => entry.kind !== "form").map((entry) => entry.helper));
  const unnamed = exported.filter((helper) => !named.has(helper));
  console.log(`${unnamed.length} runtime exports the catalog doesn't name yet`);
  expect(unnamed.length).toBeLessThanOrEqual(1183);
});

test("a run's helpers are pitted where only the plain build imports them, and the catalog says why of the rest", () => {
  const dir = join(fixture("twin-net-helpers"), "runs");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "matrix-0.json"), JSON.stringify({ judged: 2, of: 3, plain: ["$lazyZip", "$foldDropping", "$new"], std: ["$foldDropping", "$new", "$advanceSlice"] }));
  writeFileSync(join(dir, "matrix-1.json"), JSON.stringify({ judged: 1, of: 1, plain: ["$new", "iterator_map"], std: [] }));
  const helpers = helpersOf(dir);
  // In the order the batches' names give, `helpersOf`'s, on every system.
  expect([[...helpers.pitted], [...helpers.unpitted], helpers.judged, helpers.of]).toEqual([["$lazyZip", "$new"], ["$foldDropping"], 3, 4]);
  // Of a slice: `$foldDropping` is unproven, `$lazyZip` pitted, and what
  // the catalog doesn't name is said.
  expect(twinVerdict(helpers, false)).toEqual({ gaps: [], drift: [], pit: [], unshare: [] });
});

// A run as the catalog says, but for what each test changes.
const asListed = () => ({ pitted: new Set(pittedHelpers()), unpitted: new Set(Object.keys(sharedHelpers())) });

test("a gap fails a run: a helper the catalog doesn't name, and of a whole run one it says is pitted that no case pits now", () => {
  const run = asListed();
  run.unpitted.add("$unlisted");
  expect(twinVerdict(run, false).gaps).toEqual(["$unlisted: both builds import it and models.rs doesn't say why std's build has no body of its own in its place"]);
  // Coverage lost: `$lazyZip` both builds import now, `$lent` none; a
  // slice may see either so, a whole run says it.
  run.pitted.delete("$lazyZip");
  run.unpitted.add("$lazyZip");
  run.pitted.delete("$lent");
  expect(twinVerdict(run, false).gaps).toHaveLength(1);
  const whole = twinVerdict(run, true);
  expect(whole.gaps).toEqual([
    "$lazyZip: models.rs says it's `Pitted` and both builds import it now, so no case pits it against std's body: coverage lost: pit it again, or make it `Unproven` saying why",
    "$lent: models.rs says it's `Pitted` and no case imports it now: coverage lost: give a table a case that reaches it, or take it off models.rs where nothing makes it",
    "$unlisted: both builds import it and models.rs doesn't say why std's build has no body of its own in its place",
  ]);
  expect(whole.drift).toEqual([]);
});

test("drift fails no run, and its bless writes the catalog as the run found it", () => {
  // `$foldDropping`, unproven, is pitted now; `$newlyPitted` is pitted and
  // the catalog doesn't name it; `$extend`, unproven, nothing imports now.
  const run = asListed();
  run.unpitted.delete("$foldDropping");
  run.pitted.add("$foldDropping");
  run.pitted.add("$newlyPitted");
  run.unpitted.delete("$extend");
  const verdict = twinVerdict(run, true);
  expect(verdict.gaps).toEqual([]);
  expect(verdict.drift).toEqual([
    "$extend: no case imports it now: off models.rs",
    "$foldDropping: a case pits it against std's body now: `Pitted` in models.rs",
    "$newlyPitted: a case pits it against std's body: onto models.rs, `Pitted`",
  ]);
  // A slice says none of it.
  expect(twinVerdict(run, false)).toEqual({ gaps: [], drift: [], pit: [], unshare: [] });
  const source = readFileSync(catalogFile, "utf8");
  const blessed = blessedCatalog(source, verdict);
  const entry = (text: string, helper: string) => text.split("\n").find((line) => line.startsWith(`    entry("${helper}", `));
  expect(entry(blessed, "foldDropping")).toMatch(/^ {4}entry\("foldDropping", Model\(Pitted, &\[\]\), /);
  expect(entry(blessed, "newlyPitted")).toBe('    entry("newlyPitted", Model(Pitted, &[]), ""),');
  expect(entry(blessed, "extend")).toBeUndefined();
  // Nothing else moves: one line changed, one on, one off.
  expect(blessed.split("\n").filter((line) => !source.includes(line))).toHaveLength(2);
  expect(blessed.split("\n")).toHaveLength(source.split("\n").length);
  // In order: the new line where its name puts it.
  const names = blessed.split("\n").flatMap((line) => /^ {4}entry\("(\w+)"/.exec(line)?.[1] ?? []);
  expect(names).toEqual([...names].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0)));
  // Blessed again, what it wrote is what the run says.
  expect(blessedCatalog(blessed, verdict)).toBe(blessed);
});

test("`BLESS=1 bun test/twin-net.ts helpers` writes the drift of a whole run that passed, and of no other", () => {
  const at = fixture("twin-net-bless");
  const dir = join(at, "runs");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const pitted = pittedHelpers();
  const sharedNow = Object.keys(sharedHelpers()).filter((helper) => helper !== "$extend");
  writeFileSync(join(dir, "consumers-0.json"), JSON.stringify({ judged: 1, of: 1, plain: [...pitted, "$newlyPitted", ...sharedNow], std: sharedNow }));
  const source = readFileSync(catalogFile, "utf8");
  const file = join(at, "models.rs");
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
  expect(blessed).toContain('    entry("newlyPitted", Model(Pitted, &[]), ""),\n');
  expect(blessed).not.toMatch(/^ {4}entry\("extend", /m);
});

test("a module's helpers are what it imports of the runtime", () => {
  expect(importedHelpers(`import { $a, $b } from "@rust-js/std";\nimport { iterator_map } from "@rust-js/std/iter.js";\nconst x = 1;\n`)).toEqual(["$a", "$b"]);
});
