// The twin net's reading of the one catalog of the runtime's models
// (test/twin-net.ts, models.rs; ADR 1122, amended; ADR 1202): a run fails
// on a gap alone, a helper the catalog doesn't name or one it says is
// pitted that no case pits now, and the catalog's drift is blessed, not
// failed: the nightly's twin net was red four nights running to 2026-11-24
// of drift alone.
import type { Mutation } from "../mutations";

const gaps = ["test/twin-net.test.ts", "-t", "a gap fails a run"];
const drift = ["test/twin-net.test.ts", "-t", "drift fails no run"];
const blessed = ["test/twin-net.test.ts", "-t", "writes the drift of a whole run that passed"];

export const mutations: Mutation[] = [
  {
    name: "twin-unlisted-helper-passes",
    breaks: "a helper both builds import that neither list names passes, so a runtime copy no case pits against std's body goes unsaid",
    file: "test/twin-net.ts",
    find: "    .filter((helper) => shared[helper] === undefined && !listed.has(helper))",
    replace: "    .filter((helper) => shared[helper] === undefined && listed.has(helper))",
    tests: gaps,
  },
  {
    name: "twin-lost-coverage-passes",
    breaks: "a helper `pitted` names that no case pits now passes, so the net stops pitting a runtime copy against std's body unseen",
    file: "test/twin-net.ts",
    find: "  for (const helper of pitted.filter((helper) => !run.pitted.has(helper))) {",
    replace: "  for (const helper of pitted.filter((helper) => !run.pitted.has(helper) && run.unpitted.has(helper))) {",
    tests: gaps,
  },
  {
    name: "twin-slice-says-lost-coverage",
    breaks: "a slice, which pits a helper only where its cases reach it, says each `pitted` helper it didn't pit is lost, failing every push of the iterators' code",
    file: "test/twin-net.ts",
    find: "  if (!whole) return { gaps: gaps.sort(), drift: [], pit: [], unshare: [] };",
    replace: "",
    tests: gaps,
  },
  {
    name: "twin-drift-fails",
    breaks: "a list out of date with no gap fails the run, the nightly red of bookkeeping alone, fixed by hand each morning",
    file: "test/twin-net.ts",
    find: "  return { gaps: gaps.sort(), drift: drift.sort(), pit, unshare };",
    replace: "  return { gaps: [...gaps, ...drift].sort(), drift: drift.sort(), pit, unshare };",
    tests: drift,
  },
  {
    name: "twin-bless-keeps-unshared",
    breaks: "the bless leaves the catalog's line of what nothing imports now, so the next night says the same drift again",
    file: "test/twin-net.ts",
    find: "    return !(m && off.has(`$${m[1]}`));",
    replace: "    return true;",
    tests: drift,
  },
  {
    name: "twin-bless-pit-unwritten",
    breaks: "the bless leaves a helper a case pits now unproven in the catalog, so the next night says the same drift again",
    file: "test/twin-net.ts",
    find: "    lines[i] = `    entry(\"${helper.slice(1)}\", Model(Pitted, ${m[3] ?? \"&[]\"}), ${m[4]}),`;",
    replace: "    lines[i] = lines[i];",
    tests: drift,
  },
  {
    name: "twin-bless-of-a-failed-run",
    breaks: "a whole run that failed is blessed, its helpers maybe some batches' alone, taking off `shared` what the rest import",
    file: "test/twin-net.ts",
    find: "    if (!whole || !passed) throw new Error(",
    replace: "    if (!whole) throw new Error(",
    tests: blessed,
  },
  {
    name: "twin-outside-unshared",
    breaks: "a model outside the twin net's tables, which no case reaches yet, is drift where no case imports it, so blessing a whole run takes a site reader, `$durationAdd`, off models.rs",
    file: "test/twin-net.ts",
    find: '    .filter((helper) => run.pitted.has(helper) || (!run.unpitted.has(helper) && shared[helper] !== "outside"))\n',
    replace: "    .filter((helper) => run.pitted.has(helper) || !run.unpitted.has(helper))\n",
    tests: ["test/twin-net.test.ts", "-t", "a whole run that imports no model outside the tables keeps it"],
  },
  {
    name: "twin-form-shared",
    breaks: "a form the lowering writes is taken for a runtime helper both builds import, so the catalog names a helper the runtime doesn't declare",
    file: "test/twin-net.ts",
    find: '  return Object.fromEntries(catalog().models.filter((entry) => entry.kind !== "form" && !entry.pitted)',
    replace: "  return Object.fromEntries(catalog().models.filter((entry) => !entry.pitted)",
    tests: ["test/twin-net.test.ts", "-t", "each helper the catalog names is one the runtime declares"],
  },
];
