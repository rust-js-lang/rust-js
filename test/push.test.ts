// How `bun run push` (scripts/push.ts) resolves a rebase's conflict of a
// file two changes may each change at once: a blessed baseline made again,
// a ratchet by the lower count, an ADR by both amendments, a line set as a
// set; any other stops the push.
import { expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { appendedBoth, lineSet, lowerRatchets, mergedChanges, mergedLineSet, ratchets, regenerated, remakeable } from "../scripts/push";
import { fixture, root } from "./support";

// A rebase's conflict in a blessed baseline alone is resolved by making it
// again from the rebased tree (`bun run push`, scripts/push.ts), where it
// stopped the second of two pushes that each blessed one (2 of m3-max's 5
// failed pushes of 2026-11-17); one in hand-written code still stops it.
// The baselines are those .gitattributes merges as `merge=binary`, each
// with the test whose bless makes it.
test("a rebase's conflict in a blessed baseline alone is made again, any other stops it", () => {
  const binary = readFileSync(join(root, ".gitattributes"), "utf8").split("\n").flatMap((line) => {
    const m = line.match(/^(\S+) merge=binary$/);
    return m ? [m[1]!] : [];
  });
  expect(Object.keys(regenerated).sort()).toEqual(binary.sort());
  for (const [file, test] of Object.entries(regenerated)) {
    const source = readFileSync(join(root, test), "utf8");
    expect([file, source.includes(file) && /\bBLESS\b/.test(source)]).toEqual([file, true]);
  }
  expect(remakeable(["docs/readability.txt"])).toBe(true);
  expect(remakeable(["docs/readability.txt", "react/coverage.txt"])).toBe(true);
  expect(remakeable(["docs/readability.txt", "src/lower/analysis.rs"])).toBe(false);
  expect(remakeable(["src/lower/analysis.rs"])).toBe(false);
});

// A ratchet two changes lowered at once conflicts on rebase, and `bun run
// push` resolves it by the lower count, which holds of the rebased tree as
// each change only removed what's counted: KIN_PARSES, lowered 15 to 14 by
// one change and to 10 by others, cost a hand rebase (2026-11-17).
test("a rebase's conflict in a ratchet is resolved by the lower count, anything else in it stops", () => {
  const conflict = (ours: string[], theirs: string[], base?: string[]) =>
    ["# a ratchet", "<<<<<<< HEAD", ...ours, ...(base ? ["||||||| base", ...base] : []), "=======", ...theirs, ">>>>>>> theirs", "src/z.rs\t1", ""].join("\n");
  expect(lowerRatchets(conflict(["src/a.rs\t3", "src/b.rs\t5"], ["src/a.rs\t4", "src/b.rs\t2"]))).toBe(["# a ratchet", "src/a.rs\t3", "src/b.rs\t2", "src/z.rs\t1", ""].join("\n"));
  // A key one side alone names is kept as it says; a base section is read past.
  expect(lowerRatchets(conflict(["src/a.rs\t3"], ["src/a.rs\t2", "src/c.rs\t1"], ["src/a.rs\t9"]))).toBe(["# a ratchet", "src/a.rs\t2", "src/c.rs\t1", "src/z.rs\t1", ""].join("\n"));
  expect(lowerRatchets(conflict(["const KIN_PARSES = 10;"], ["const KIN_PARSES = 14;"]))).toBe(["# a ratchet", "const KIN_PARSES = 10;", "src/z.rs\t1", ""].join("\n"));
  expect(lowerRatchets(conflict(["const KIN_PARSES = 10;"], ["const KIN_PARSES = 14;", "test(\"x\", () => {});"]))).toBeUndefined();
  expect(remakeable(["test/sig-binders.txt", "docs/readability.txt"])).toBe(true);
  expect(remakeable(["test/architecture-names.test.ts", "src/lower/analysis.rs"])).toBe(false);
  // WORD_WALKS lowered by two changes at once, d8eb74bd1's and another's:
  // the lower; anything else in the test's conflict, stopped.
  expect(remakeable(["test/architecture-names.test.ts", "test/sig-binders.txt"])).toBe(true);
  expect(lowerRatchets(conflict(["const WORD_WALKS = 161;"], ["const WORD_WALKS = 137;"]))).toBe(["# a ratchet", "const WORD_WALKS = 137;", "src/z.rs\t1", ""].join("\n"));
  expect(lowerRatchets(conflict(["const WORD_WALKS = 161;", "  const free = [\"mentions_in\"];"], ["const WORD_WALKS = 137;"]))).toBeUndefined();
  for (const file of ratchets) expect([file, existsSync(join(root, file))]).toEqual([file, true]);
});

// Two pushes amending one ADR at once each append to its end: m3-max's
// stopped twice on ADR 1150 (2026-11-18). Both kept, main's first, each
// whole; a change to the text they share still stops.
test("a rebase's conflict in an ADR both sides only appended to keeps both amendments, any other stops", () => {
  const base = "# ADR 1150\n\nThe decision.\n";
  const ours = `${base}\n### Amended: one (m3-max)\n\nIts text.\n`;
  const theirs = `${base}\n### Amended: another (ioCP)\n\nIts text.\n`;
  expect(appendedBoth(base, ours, theirs)).toBe(`${base}\n### Amended: one (m3-max)\n\nIts text.\n\n### Amended: another (ioCP)\n\nIts text.\n`);
  // One with no newline at its end is ended before the other's.
  expect(appendedBoth(base, `${base}X`, `${base}Y\n`)).toBe(`${base}X\nY\n`);
  expect(appendedBoth(base, ours.replace("The decision.", "The decision, amended."), theirs)).toBeUndefined();
  expect(appendedBoth(base, ours, theirs.replace("# ADR 1150", "# ADR 1150, renamed"))).toBeUndefined();
  expect(remakeable(["docs/decisions/1150-names-are-ids.md", "test/sig-binders.txt"])).toBe(true);
  expect(remakeable(["docs/decisions/README.md"])).toBe(false);
  expect(remakeable(["docs/guide/README.md"])).toBe(false);
});

// TESTS.md's top, a `Changed on ..` line each change adds, conflicts on
// almost every two pushes at once (a dozen PR merges by hand, 2026-11-18):
// merged as git merges it (`git merge-file --diff3`), each conflict of
// those lines alone keeping both sides' new lines, the replayed change's
// first, newest; one in the table still stops.
test("a rebase's conflict in TESTS.md's `Changed on` lines keeps both sides' new lines, newest first", () => {
  const dir = fixture("push-tests-md");
  const merged = (base: string[], ours: string[], theirs: string[]) => {
    const files = Object.entries({ ours, base, theirs }).map(([name, lines]) => {
      writeFileSync(join(dir, name), [...lines, ""].join("\n"));
      return join(dir, name);
    });
    const p = spawnSync("git", ["merge-file", "-p", "--diff3", ...files], { encoding: "utf8" });
    expect(p.status).toBeGreaterThan(0);
    return mergedChanges(p.stdout)?.split("\n").slice(0, -1);
  };
  const head = ["# Tests at a glance", ""];
  const base = [...head, "Changed on 1: a", "", "| Suite | Tests |", "| corpus | 2 |"];
  // Two pushes each added its line.
  expect(merged(base, [...head, "Changed on 2: main's", "", ...base.slice(2)], [...head, "Changed on 2: mine", "", ...base.slice(2)])).toEqual([
    ...head,
    "Changed on 2: mine",
    "",
    "Changed on 2: main's",
    "",
    ...base.slice(2),
  ]);
  // Main rewrote a line, a count raised, that the replayed change kept and
  // added its own before: its own, then main's rewrite, never the old line.
  expect(merged(base, [...head, "Changed on 1: a, raised", "", ...base.slice(4)], [...head, "Changed on 2: mine", "", ...base.slice(2)])).toEqual([
    ...head,
    "Changed on 2: mine",
    "",
    "Changed on 1: a, raised",
    "",
    ...base.slice(4),
  ]);
  // A conflict in the table is its author's to merge.
  expect(merged(base, [...base.slice(0, 5), "| corpus | 3 |"], [...base.slice(0, 5), "| corpus | 4 |"])).toBeUndefined();
  expect(remakeable(["TESTS.md"])).toBe(true);
});

// Two changes that each took a line off a known list conflict where the
// lines are neighbours: m1-lisa's #2054 and m6-lily's #2056 off
// test/metamorphic-known.txt stopped a push (2026-11-18). A line set's
// conflict is merged as a set, of git's own merge (`git merge-file
// --diff3`, as the rebase's stages give it): what either side took off is
// gone, what either added is kept; a line each side changed still stops.
test("a rebase's conflict in a line set keeps what neither side took off and what either added, a line both changed stops", () => {
  const dir = fixture("push-line-sets");
  const merged = (base: string[], ours: string[], theirs: string[]) => {
    const files = Object.entries({ ours, base, theirs }).map(([name, lines]) => {
      writeFileSync(join(dir, name), [...lines, ""].join("\n"));
      return join(dir, name);
    });
    const p = spawnSync("git", ["merge-file", "-p", "--diff3", ...files], { encoding: "utf8" });
    expect(p.status).toBeGreaterThan(0);
    const text = mergedLineSet(p.stdout);
    return text?.split("\n").slice(0, -1);
  };
  const base = ["// the list", "", "#2054 P0: one", "plain std generic a.rs", "", "#2056 P0: another", "plain loud-drop b.rs", "", "#1022 P2: a third", "plain std c.rs"];
  // Each took its issue's lines off.
  expect(merged(base, [...base.slice(0, 2), ...base.slice(5)], [...base.slice(0, 5), ...base.slice(8)])).toEqual(["// the list", "", "#1022 P2: a third", "plain std c.rs"]);
  // One took a line off, the other added one beside it.
  expect(merged(base, base.filter((line) => line !== "plain std generic a.rs"), base.toSpliced(4, 0, "std generic a.rs"))).toEqual(base.toSpliced(3, 1, "std generic a.rs"));
  // Both took the same line off, one adding another; both added one line, kept once.
  expect(merged(base, base.toSpliced(3, 1), base.toSpliced(3, 1, "plain field e.rs"))).toEqual(base.toSpliced(3, 1, "plain field e.rs"));
  expect(merged(base, base.toSpliced(4, 0, "plain fact d.rs"), base.toSpliced(4, 0, "plain fact d.rs", "plain field e.rs"))).toEqual(base.toSpliced(4, 0, "plain fact d.rs", "plain field e.rs"));
  // A line each side changed: no set says which is the list's.
  expect(merged(base, base.toSpliced(3, 1, "plain a.rs"), base.toSpliced(3, 1, "std a.rs"))).toBeUndefined();
  // Git's markers without a base section can't be merged as a set.
  expect(mergedLineSet(["<<<<<<< ours", "a", "=======", "b", ">>>>>>> theirs", ""].join("\n"))).toBeUndefined();
  // Every known list is a line set, and the rustc suite's lists.
  const known = readdirSync(join(root, "test")).filter((name) => name.endsWith("-known.txt"));
  expect(known.length).toBeGreaterThan(5);
  for (const name of [...known, "rustc-known-failures.txt", "rustc-native-failures.txt", "rustc-out-of-scope.txt"]) expect([name, lineSet(`test/${name}`) && existsSync(join(root, "test", name))]).toEqual([name, true]);
  expect(remakeable(["test/metamorphic-known.txt", "test/sig-binders.txt"])).toBe(true);
  expect(remakeable(["test/metamorphic.test.ts"])).toBe(false);
  expect(lineSet("docs/readability.txt")).toBe(false);
});
