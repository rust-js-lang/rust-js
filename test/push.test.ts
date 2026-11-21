// How `bun run push` (scripts/push.ts) resolves a rebase's conflict of a
// file two changes may each change at once: a blessed baseline made again,
// a ratchet by the lower count, an ADR by both amendments, a line set as a
// set; any other stops the push.
import { expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { appendedBoth, beside, FAST_TESTS, fastAgain, lineSet, lowerRatchets, mergedChanges, mergedLineSet, phases, ratchets, rebase, regenerated, remakeable, mutantsAgain, sanitySkipped, stdGenerated } from "../scripts/push";
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
// lines are neighbours: m1-lisa's #2054 and m6-lily's #2056 off the
// metamorphic net's list stopped a push (2026-11-18). A line set's
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
  expect(known.length).toBeGreaterThan(3);
  for (const name of [...known, "rustc-known-failures.txt", "rustc-native-failures.txt", "rustc-out-of-scope.txt"]) expect([name, lineSet(`test/${name}`) && existsSync(join(root, "test", name))]).toEqual([name, true]);
  // And each issue's file of a known list kept as a directory (#2260).
  for (const list of ["metamorphic", "iterator-matrix", "neighbours"]) {
    const files = readdirSync(join(root, "test", "known", list)).filter((name) => name.endsWith(".txt"));
    expect(files.length).toBeGreaterThan(1);
    for (const name of files) expect([name, lineSet(`test/known/${list}/${name}`)]).toEqual([name, true]);
  }
  expect(remakeable(["test/known/metamorphic/1022.txt", "test/sig-binders.txt"])).toBe(true);
  expect(remakeable(["test/metamorphic.test.ts"])).toBe(false);
  expect(lineSet("docs/readability.txt")).toBe(false);
});

test("what runs beside a round's check, stopped, leaves nothing it started running", async () => {
  // A tree as the React regen's is, bun, node, cargo and rust-js: here a
  // shell and the two it starts. Killing only the shell left the rest.
  const it = beside(["sh", "-c", "sleep 300 & sleep 300 & wait"], join(root, "target", "push-beside.test.log"));
  const running = () => spawnSync("pgrep", ["-g", String(it.group)]).stdout.toString().split("\n").filter(Boolean);
  for (let i = 0; i < 250 && running().length < 3; i++) await Bun.sleep(20);
  expect(running().length).toBe(3);
  it.stop();
  // Ended by the stop: what it runs sleeps 300 s, so its end within the
  // test's time is the stop's. Not by its status, which a shell killed as
  // its `wait` ends may give as 0, under load (#2233): what's asked is the
  // tree, each process the group started gone.
  await it.done;
  for (let i = 0; i < 250 && running().length > 0; i++) await Bun.sleep(20);
  expect(running()).toEqual([]);
});

// What would refuse a push minutes in, or at its end, refuses it first, in
// seconds (`fast`): push-guard's `undone`, the rule git's hook runs as it
// pushes, after the whole check; and the mutation catalog's tests of a
// `find` its code moved past or a test renamed, which check:fast reaches
// minutes in.
test("a push's fast phase runs first, push-guard's rule and the catalog's quick tests", () => {
  expect(phases.slice(0, 3)).toEqual(["rebase", "fast", "build"]);
  const source = readFileSync(join(root, "scripts/push.ts"), "utf8");
  const main = source.slice(source.indexOf("async function main"));
  const at = (text: string) => main.indexOf(text);
  expect(at('undone(process.cwd(), base, "HEAD")')).toBeGreaterThan(0);
  expect(at('undone(process.cwd(), base, "HEAD")')).toBeLessThan(at('run("build"'));
  const fast = at('run("fast", ');
  expect(fast).toBeGreaterThan(0);
  expect(main.slice(fast, main.indexOf("\n", fast))).toContain("...FAST_TESTS]");
  expect(fast).toBeLessThan(at('run("build"'));
  // The mutation catalog's test file, whole: none of its tests picked; and the owners' table's.
  expect(FAST_TESTS).toEqual(["test/mutations.test.ts", "test/architecture-owners.test.ts"]);
});

// A round past a rebase spends again only what the commits it brought can
// change (m6-lily's health check, 2026-11-20: a push's second round 230 s).
// The mutants type-check again where they touch a Rust source, a mutation
// list or the toolchain; the sanity pass checks nothing where no test is
// reached by both the change and them, and says why.
test("a round past a rebase type-checks the mutants and checks the interaction only where the incoming commits can change them", () => {
  expect(mutantsAgain(undefined)).toBe(true);
  expect(mutantsAgain(["docs/guide/README.md", "test/corpus/x.rs", "src/runtime/iter.js"])).toBe(false);
  for (const file of ["src/lower/iterators.rs", "scripts/mutations/runtime/iter.ts", "Cargo.lock", "rust-toolchain.toml"]) expect([file, mutantsAgain(["docs/x.md", file])]).toEqual([file, true]);
  expect(sanitySkipped("none")).toBe("incoming reach nothing this change reaches");
  expect(sanitySkipped("none, map 57283d2aa 3 behind")).toBe("incoming reach nothing this change reaches");
  expect(sanitySkipped("some (3 files, 2 cases), map 57283d2aa 3 behind")).toBeUndefined();
  expect(sanitySkipped("capped 56 of 334 files in 45 s")).toBeUndefined();
});

// A round past a rebase runs the fast phase's tests again only where the
// commits it brought touch what they read: the catalog, a file a mutation
// names, the compiler's sources, the owners' table or the tests (#2258).
test("a round past a rebase runs the fast tests again only where the incoming commits touch what they read", () => {
  const named = new Set(["src/runtime/iter.js", "tooling/cli.js"]);
  expect(fastAgain(undefined, named)).toBe(true);
  expect(fastAgain([], named)).toBe(false);
  expect(fastAgain(["docs/guide/README.md", "test/corpus/x.rs", "test/corpus/x.js", "TESTS.md"], named)).toBe(false);
  for (const file of ["src/lower/iterators.rs", "src/runtime/iter.js", "tooling/cli.js", "scripts/mutations/runtime/iter.ts", "scripts/mutations.ts", "docs/architecture.md", "scripts/owners.ts", ...FAST_TESTS])
    expect([file, fastAgain([file], named)]).toEqual([file, true]);
});

// std/'s generated files, its runtime, compiled modules, maps, manifest
// and entry, are made again on a rebase's conflict, never merged by hand;
// its crate, README and package.json are written by hand (#2259).
test("a rebase's conflict in std/'s generated files is made again, one in what's written by hand stops", () => {
  for (const file of ["std/runtime.js", "std/iter.js", "std/iter.js.map", "std/manifest.json", "std/index.d.ts", "std/index.js"]) expect([file, stdGenerated(file)]).toEqual([file, true]);
  for (const file of ["std/lib.rs", "std/README.md", "std/package.json", "src/runtime/flat_map.js", "std/sub/x.js"]) expect([file, stdGenerated(file)]).toEqual([file, false]);
  expect(remakeable(["std/runtime.js", "std/iter.js", "docs/readability.txt"])).toBe(true);
  expect(remakeable(["std/runtime.js", "std/lib.rs"])).toBe(false);
});

// A constructed conflict: main and the change each write std/runtime.js
// and std/iter.js, and the change a file of its own; the rebase takes
// main's std/, which the round makes again after its build, and goes on,
// the change's own file kept. One in std/lib.rs stops it, aborted (#2259).
test("a rebase with std/'s generated files in conflict goes on, main's taken; one in std/lib.rs is aborted", () => {
  const dir = fixture("push-std-conflict");
  const env = { ...process.env, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@example.com", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@example.com" };
  const git = (cwd: string, ...args: string[]) => {
    const p = spawnSync("git", args, { cwd, env, encoding: "utf8" });
    if (p.status !== 0) throw new Error(`git ${args.join(" ")}: ${p.stderr}`);
    return p.stdout;
  };
  const write = (cwd: string, file: string, text: string) => {
    spawnSync("mkdir", ["-p", join(cwd, file, "..")]);
    writeFileSync(join(cwd, file), text);
  };
  const was = process.cwd();
  // `rebase()` commits as the process's environment says: CI's runner
  // has no identity of its own, and its `rebase --continue` failed there.
  const identity = ["GIT_AUTHOR_NAME", "GIT_AUTHOR_EMAIL", "GIT_COMMITTER_NAME", "GIT_COMMITTER_EMAIL"] as const;
  const before = identity.map((key) => process.env[key]);
  for (const key of identity) process.env[key] = env[key];
  const setUp = (name: string, generated: string[], [base, ours, theirs] = ["base\n", "main\n", "change\n"]) => {
    const origin = join(dir, `${name}.git`);
    const main = join(dir, `${name}-main`);
    const work = join(dir, `${name}-work`);
    git(dir, "init", "-q", "--bare", "-b", "main", origin);
    git(dir, "clone", "-q", origin, main);
    for (const file of generated) write(main, file, base);
    write(main, ".gitattributes", readFileSync(join(root, ".gitattributes"), "utf8"));
    git(main, "add", "-A");
    git(main, "commit", "-q", "-m", "base");
    git(main, "push", "-q", "origin", "HEAD:main");
    git(dir, "clone", "-q", origin, work);
    for (const file of generated) write(main, file, ours);
    git(main, "commit", "-q", "-am", "main");
    git(main, "push", "-q", "origin", "HEAD:main");
    for (const file of generated) write(work, file, theirs);
    write(work, "own.txt", "own\n");
    git(work, "add", "-A");
    git(work, "commit", "-q", "-m", "change");
    return work;
  };
  try {
    const work = setUp("generated", ["std/runtime.js", "std/iter.js"]);
    process.chdir(work);
    expect(rebase().sort()).toEqual(["std/iter.js", "std/runtime.js"]);
    expect(existsSync(join(work, ".git", "rebase-merge"))).toBe(false);
    expect(readFileSync(join(work, "std/runtime.js"), "utf8")).toBe("main\n");
    expect(readFileSync(join(work, "own.txt"), "utf8")).toBe("own\n");
    expect(git(work, "log", "--format=%s").trim().split("\n")).toEqual(["change", "main", "base"]);
    // Each side a line of its own, which a line merge would join into a
    // std/ no compiler wrote: .gitattributes' `-merge` makes it a conflict.
    const apart = setUp("apart", ["std/runtime.js"], ["a\nb\nc\n", "A\nb\nc\n", "a\nb\nC\n"]);
    process.chdir(apart);
    expect(rebase()).toEqual(["std/runtime.js"]);
    expect(readFileSync(join(apart, "std/runtime.js"), "utf8")).toBe("A\nb\nc\n");
    const crate = setUp("crate", ["std/lib.rs"]);
    process.chdir(crate);
    expect(() => rebase()).toThrow("the rebase stopped on a conflict, aborted: std/lib.rs");
    expect(existsSync(join(crate, ".git", "rebase-merge"))).toBe(false);
  } finally {
    process.chdir(was);
    identity.forEach((key, at) => (before[at] === undefined ? delete process.env[key] : (process.env[key] = before[at])));
  }
});

// A rebase that stops with no conflict to resolve, a step git can't
// commit, is refused and aborted, never returned as done: it left a rebase
// stopped, and the push pushed `origin/main` as it was (2026-11-21).
test("a rebase that stops with no conflict to resolve is refused and aborted", () => {
  const dir = fixture("push-rebase-stopped");
  const env = { ...process.env, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@example.com", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@example.com" };
  const git = (cwd: string, ...args: string[]) => {
    const p = spawnSync("git", args, { cwd, env, encoding: "utf8" });
    if (p.status !== 0) throw new Error(`git ${args.join(" ")}: ${p.stderr}`);
    return p.stdout;
  };
  const origin = join(dir, "origin.git");
  const main = join(dir, "main");
  const work = join(dir, "work");
  git(dir, "init", "-q", "--bare", "-b", "main", origin);
  git(dir, "clone", "-q", origin, main);
  writeFileSync(join(main, "a.txt"), "a\n");
  git(main, "add", "-A");
  git(main, "commit", "-q", "-m", "a");
  git(main, "push", "-q", "origin", "HEAD:main");
  git(dir, "clone", "-q", origin, work);
  writeFileSync(join(main, "b.txt"), "main\n");
  git(main, "add", "-A");
  git(main, "commit", "-q", "-m", "b");
  git(main, "push", "-q", "origin", "HEAD:main");
  writeFileSync(join(work, "c.txt"), "change\n");
  git(work, "add", "-A");
  git(work, "commit", "-q", "-m", "c");
  // A file of the working tree in the way of the incoming one, untracked.
  writeFileSync(join(work, "b.txt"), "in the way\n");
  const was = process.cwd();
  try {
    process.chdir(work);
    expect(() => rebase()).toThrow("the rebase stopped with no conflict to resolve, aborted");
    expect(existsSync(join(work, ".git", "rebase-merge"))).toBe(false);
    expect(git(work, "log", "--format=%s").trim().split("\n")).toEqual(["c", "a"]);
  } finally {
    process.chdir(was);
  }
});
