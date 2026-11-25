// How `bun run push` (scripts/push.ts) resolves a rebase's conflict of a
// file two changes may each change at once: a blessed baseline made again,
// a ratchet by the lower count, an ADR by both amendments, a line set as a
// set; any other stops the push.
import { expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { changedWhileChecked, untrackedRefusal, movedWhilePushed, againEnv, appendedBoth, compileGate, gateRebase, givenRemade, beside, FAST_TESTS, fastAgain, givenBefore, givenRound, lineSet, carryOn, roundSaid, withoutRemade, loadedModules, lowerRatchets, mergedLineSet, phases, portRegen, ratchets, rebase, regenerated, remakeable, mutantsAgain, roundRefused, sanitySkipped, staleModules, stdGenerated } from "../scripts/push";
import { mergedChanges } from "../scripts/merge-changes";
import { fixture, root } from "./support";

// A rebase's conflict in a blessed baseline alone is resolved by making it
// again from the rebased tree (`bun run push`, scripts/push.ts), where it
// stopped the second of two pushes that each blessed one (2 of katniss agent's 5
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
  expect(remakeable(["docs/readability.txt", "crates/rust-js-lower/src/analysis.rs"])).toBe(false);
  expect(remakeable(["crates/rust-js-lower/src/analysis.rs"])).toBe(false);
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
  expect(remakeable(["test/architecture-names.test.ts", "crates/rust-js-lower/src/analysis.rs"])).toBe(false);
  // WORD_WALKS lowered by two changes at once, d8eb74bd1's and another's:
  // the lower; anything else in the test's conflict, stopped.
  expect(remakeable(["test/architecture-names.test.ts", "test/sig-binders.txt"])).toBe(true);
  expect(lowerRatchets(conflict(["const WORD_WALKS = 161;"], ["const WORD_WALKS = 137;"]))).toBe(["# a ratchet", "const WORD_WALKS = 137;", "src/z.rs\t1", ""].join("\n"));
  expect(lowerRatchets(conflict(["const WORD_WALKS = 161;", "  const free = [\"mentions_in\"];"], ["const WORD_WALKS = 137;"]))).toBeUndefined();
  for (const file of ratchets) expect([file, existsSync(join(root, file))]).toEqual([file, true]);
});

// Two pushes amending one ADR at once each append to its end: katniss agent's
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

// And by git itself, its driver `changes-log` (.gitattributes, named by
// `bun install`): a relay's cherry-pick, where nearly every one stopped
// on these lines, a minute by hand each (#2587), keeps both; a conflict
// in the table still stops it, as git's own.
test("a cherry-pick's conflict in TESTS.md's `Changed on` lines is merged by git's driver; one in its table stops it", () => {
  expect(readFileSync(join(root, ".gitattributes"), "utf8")).toContain("\nTESTS.md merge=changes-log\n");
  const prepare = (JSON.parse(readFileSync(join(root, "package.json"), "utf8")) as { scripts: Record<string, string> }).scripts.prepare!;
  expect(prepare).toContain('git config merge.changes-log.driver \"bun scripts/merge-changes.ts %O %A %B\"');
  const dir = fixture("push-tests-md-driver");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const env = { ...process.env, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@example.com", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@example.com" };
  const git = (...args: string[]) => spawnSync("git", args, { cwd: dir, env, encoding: "utf8" });
  const commit = (text: string, message: string) => {
    writeFileSync(join(dir, "TESTS.md"), text);
    git("add", "TESTS.md");
    expect(git("commit", "-q", "-m", message).status).toBe(0);
  };
  git("init", "-q", "-b", "main");
  git("config", "merge.changes-log.driver", `"${process.execPath}" "${join(root, "scripts/merge-changes.ts")}" %O %A %B`);
  writeFileSync(join(dir, ".gitattributes"), "TESTS.md merge=changes-log\n");
  git("add", ".gitattributes");
  const head = "# Tests at a glance\n\n";
  const table = "| Suite | Tests |\n| corpus | 2 |\n";
  commit(`${head}Changed on 1: a\n\n${table}`, "base");
  git("checkout", "-q", "-b", "relay");
  commit(`${head}Changed on 2: the relay's\n\nChanged on 1: a\n\n${table}`, "relay");
  git("checkout", "-q", "main");
  commit(`${head}Changed on 2: main's\n\nChanged on 1: a\n\n${table}`, "main");
  expect(git("cherry-pick", "relay").status).toBe(0);
  expect(readFileSync(join(dir, "TESTS.md"), "utf8")).toBe(`${head}Changed on 2: the relay's\n\nChanged on 2: main's\n\nChanged on 1: a\n\n${table}`);
  git("checkout", "-q", "-b", "counted", "relay");
  commit(`${head}Changed on 2: the relay's\n\nChanged on 1: a\n\n| Suite | Tests |\n| corpus | 3 |\n`, "counted");
  git("checkout", "-q", "main");
  commit(`${head}Changed on 3: main's\n\nChanged on 2: the relay's\n\nChanged on 2: main's\n\nChanged on 1: a\n\n| Suite | Tests |\n| corpus | 4 |\n`, "main again");
  expect(git("cherry-pick", "counted").status).not.toBe(0);
  expect(readFileSync(join(dir, "TESTS.md"), "utf8")).toContain("<<<<<<< ");
  git("cherry-pick", "--abort");
});

// Two changes that each took a line off a known list conflict where the
// lines are neighbours: worker-02 agent's #2054 and worker-03 agent's #2056 off the
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

// The React regen beside a round's check runs the round's own link of the
// compiler built, which it's given, so it builds none: a build in the
// check relinked the checkout's as the port's `cargo check` ran it, and
// the push failed, "could not execute .. rust-js (No such file)"
// (#2317). A relink is the old file removed and a new one written: here
// mid-run, and what runs beside still runs the build the round made.
test("a round's React regen runs the round's own compiler, which a relink mid-port leaves", async () => {
  const dir = fixture("push-own-compiler");
  const built = (n: number) => {
    const compiler = join(dir, "target", "debug", "rust-js");
    mkdirSync(join(dir, "target", "debug"), { recursive: true });
    rmSync(compiler, { force: true });
    writeFileSync(compiler, `#!/bin/sh\necho build ${n}\n`);
    chmodSync(compiler, 0o755);
  };
  built(1);
  const { cmd, env } = portRegen(dir);
  expect(cmd).toEqual(["bun", "scripts/react-in-rust.ts", "regen"]);
  expect(env.RUST_JS_COMPILER).not.toBe(join(dir, "target", "debug", "rust-js"));
  const started = join(dir, "started");
  const log = join(dir, "regen.log");
  const it = beside(["sh", "-c", `touch "${started}"; sleep 0.3; "$RUST_JS_COMPILER"`], log, env);
  for (let i = 0; i < 250 && !existsSync(started); i++) await Bun.sleep(20);
  built(2);
  expect(await it.done).toBe(0);
  expect(readFileSync(log, "utf8")).toBe("build 1\n");
  // The next round's is of the build it made.
  expect(spawnSync(portRegen(dir).env.RUST_JS_COMPILER!).stdout.toString()).toBe("build 2\n");
});

// A run's own compiler is own-compiler.ts's alone: check:fast's, a push
// round's and the mutation runner's are its links, and no other script
// links the checkout's build by hand. A run gone without exiting leaves its
// folder, which the next run of its kind removes.
test("a run's own compiler is own-compiler.ts's, and a dead run's folder goes", () => {
  const linking = readdirSync(join(root, "scripts"))
    .filter((file) => file.endsWith(".ts") && readFileSync(join(root, "scripts", file), "utf8").match(/linkSync\(join\(root, "target", "debug", "rust-js"\)/))
    .sort();
  expect(linking).toEqual(["own-compiler.ts"]);
  for (const file of ["checkFast.ts", "push.ts", "mutations.ts"]) expect(readFileSync(join(root, "scripts", file), "utf8")).toContain('from "./own-compiler"');
  const dir = fixture("push-own-folder");
  const dead = join(dir, "target", "push", "999999999");
  mkdirSync(dead, { recursive: true });
  mkdirSync(join(dir, "target", "debug"), { recursive: true });
  writeFileSync(join(dir, "target", "debug", "rust-js"), "");
  expect(portRegen(dir).env.RUST_JS_COMPILER).toBe(join(dir, "target", "push", String(process.pid), "rust-js"));
  expect(existsSync(dead)).toBe(false);
});

// What would refuse a push minutes in, or at its end, refuses it first, in
// seconds (`fast`): push-guard's `undone`, the rule git's hook runs as it
// pushes, after the whole check; and the mutation catalog's tests of a
// `find` its code moved past or a test renamed, which check:fast reaches
// minutes in.
test("a push's fast phase runs first, push-guard's rule and the catalog's quick tests", () => {
  expect(phases.slice(0, 5)).toEqual(["turn", "rebase", "gate", "fast", "build"]);
  const source = readFileSync(join(root, "scripts/push.ts"), "utf8");
  // Its wait for the machine's turn the line's first phase (#2516), the
  // line's writer's (`turnPhase`, test/leases.test.ts), and carried to push
  // run again, which takes none of its own (#2517).
  expect(source).toContain("phases: [...before.phases, ...turnPhase(), ...timed]");
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
// change (worker-03 agent's health check, 2026-11-20: a push's second round 230 s).
// The mutants type-check again where they touch a Rust source, a mutation
// list or the toolchain; the sanity pass checks nothing where no test is
// reached by both the change and them, and says why.
test("a round past a rebase type-checks the mutants and checks the interaction only where the incoming commits can change them", () => {
  expect(mutantsAgain(undefined)).toBe(true);
  expect(mutantsAgain(["docs/guide/README.md", "test/corpus/x.rs", "crates/rust-js-program/src/runtime/iter.js"])).toBe(false);
  for (const file of ["crates/rust-js-lower/src/iterators.rs", "scripts/mutations/runtime/iter.ts", "Cargo.lock", "rust-toolchain.toml"]) expect([file, mutantsAgain(["docs/x.md", file])]).toEqual([file, true]);
  expect(sanitySkipped("none")).toBe("incoming reach nothing this change reaches");
  expect(sanitySkipped("none, map 57283d2aa 3 behind")).toBe("incoming reach nothing this change reaches");
  expect(sanitySkipped("some (3 files, 2 cases), map 57283d2aa 3 behind")).toBeUndefined();
  expect(sanitySkipped("capped 56 of 334 files in 45 s")).toBeUndefined();
});

// A round past a rebase runs the fast phase's tests again only where the
// commits it brought touch what they read: the catalog, a file a mutation
// names, the compiler's sources, the owners' table or the tests (#2258).
test("a round past a rebase runs the fast tests again only where the incoming commits touch what they read", () => {
  const named = new Set(["crates/rust-js-program/src/runtime/iter.js", "tooling/cli.js"]);
  expect(fastAgain(undefined, named)).toBe(true);
  expect(fastAgain([], named)).toBe(false);
  expect(fastAgain(["docs/guide/README.md", "test/corpus/x.rs", "test/corpus/x.js", "TESTS.md"], named)).toBe(false);
  for (const file of ["crates/rust-js-lower/src/iterators.rs", "crates/rust-js-program/src/runtime/iter.js", "tooling/cli.js", "scripts/mutations/runtime/iter.ts", "scripts/mutations.ts", "docs/architecture.md", "scripts/owners.ts", ...FAST_TESTS])
    expect([file, fastAgain([file], named)]).toEqual([file, true]);
});

// std/'s generated files, its runtime, compiled modules, maps, manifest
// and entry, are made again on a rebase's conflict, never merged by hand;
// its crate, README and package.json are written by hand (#2259).
test("a rebase's conflict in std/'s generated files is made again, one in what's written by hand stops", () => {
  for (const file of ["std/runtime.js", "std/iter.js", "std/iter.js.map", "std/manifest.json", "std/index.d.ts", "std/index.js"]) expect([file, stdGenerated(file)]).toEqual([file, true]);
  for (const file of ["std/lib.rs", "std/README.md", "std/package.json", "crates/rust-js-program/src/runtime/flat_map.js", "std/sub/x.js"]) expect([file, stdGenerated(file)]).toEqual([file, false]);
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

// A rebase that changes a module the push has loaded runs push again from
// the rebased tree: its lazily imported, rebased modules met the loaded
// one's old exports, "Export named 'lockHeld' not found" (#2275). One not
// loaded yet loads as rebased; the install's manifests always count.
test("a rebase that changes what push has loaded runs it again, its line its own", () => {
  const loaded = loadedModules();
  expect(loaded).toContain("scripts/push.ts");
  expect(loaded).toContain("scripts/leases.ts");
  expect(staleModules(["scripts/leases.ts", "scripts/disk.ts", "src/lib.rs"], ["scripts/push.ts", "scripts/leases.ts"])).toEqual(["scripts/leases.ts"]);
  expect(staleModules(["scripts/disk.ts", "test/corpus/a.rs"], ["scripts/push.ts", "scripts/leases.ts"])).toEqual([]);
  expect(staleModules(["bun.lock", "package.json"], [])).toEqual(["bun.lock", "package.json"]);
  // The run again times itself, writing the one line.
  expect(againEnv({ RUST_JS_TIMED_BY: "push", RUST_JS_TIMED_OTHERS: "1", RUST_JS_AGENT: "a" })).toEqual({ RUST_JS_AGENT: "a" });
  // Asked right after each rebase, before anything it runs reads a module.
  const source = readFileSync(join(root, "scripts/push.ts"), "utf8");
  const at = source.indexOf('      const remade = [...new Set([...rebase(), ...carried])];\n      carried = [];\n      phase("rebase");\n');
  expect(at).toBeGreaterThan(0);
  expect(source.indexOf("staleModules(", at)).toBeLessThan(source.indexOf("fastAgain(since, named)", at));
});

// Push run again from the rebased tree carries on from the round before it
// (#2286): it checked everything again, 34 minutes of checks for one push.
// Only from a round that's its own: the old base an ancestor of the new,
// the tree clean, the branch's commits patch for patch those checked, as
// a rebase rewrites them; a forged or stale round is a full check.
test("push run again carries on from its round only where the round is its own", () => {
  const dir = fixture("push-round");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  // Every call of this repository's own, never one above it (#2279).
  const env = { ...process.env, GIT_DIR: join(dir, ".git"), GIT_WORK_TREE: dir, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@example.com", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@example.com" };
  const git = (...args: string[]) => {
    const p = spawnSync("git", args, { cwd: dir, env, encoding: "utf8" });
    return { code: p.status ?? 1, out: `${p.stdout}${p.stderr}` };
  };
  const ok = (...args: string[]) => {
    const done = git(...args);
    expect(done.code, done.out).toBe(0);
    return done.out.trim();
  };
  ok("init", "-q", "-b", "main");
  writeFileSync(join(dir, "a.txt"), "a\n");
  ok("add", "a.txt");
  ok("commit", "-q", "-m", "base");
  const b1 = ok("rev-parse", "HEAD");
  ok("switch", "-q", "-c", "branch");
  writeFileSync(join(dir, "b.txt"), "the change\n");
  ok("add", "b.txt");
  ok("commit", "-q", "-m", "change");
  const h1 = ok("rev-parse", "HEAD");
  ok("switch", "-q", "main");
  writeFileSync(join(dir, "c.txt"), "incoming\n");
  ok("add", "c.txt");
  ok("commit", "-q", "-m", "incoming");
  const b2 = ok("rev-parse", "HEAD");
  ok("switch", "-q", "branch");
  ok("rebase", "-q", "main");
  expect(ok("rev-parse", "HEAD")).not.toBe(h1);
  // Its own round: carried on from, its sanity pass of what came in.
  expect(roundRefused({ checked: h1, base: b1 }, b2, git)).toBeUndefined();
  // Forged: a commit it didn't check.
  expect(roundRefused({ checked: b2, base: b1 }, b2, git)).toEqual({ why: "its commits aren't patch for patch those its round checked", full: false });
  // Stale: a base past this one.
  expect(roundRefused({ checked: h1, base: b2 }, b1, git)).toMatchObject({ full: true, why: expect.stringContaining("isn't an ancestor") });
  // A change in the tree.
  writeFileSync(join(dir, "b.txt"), "changed since\n");
  expect(roundRefused({ checked: h1, base: b1 }, b2, git)).toEqual({ why: "its tree has a change", full: true });
  ok("checkout", "--", "b.txt");
  // The round given through the environment, and none where none passed.
  const round = { checked: h1, base: b1, react: b1 };
  expect(givenRound(againEnv({ RUST_JS_TIMED_BY: "push" }, round))).toEqual(round);
  expect(givenRound(againEnv({ RUST_JS_PUSH_ROUND: "x:y:" }))).toBeUndefined();
  // Taken up right after the rebase, before the selection asks the rebase's
  // commits since its base, so its round is a sanity pass.
  const source = readFileSync(join(root, "scripts/push.ts"), "utf8");
  const taken = source.indexOf("({ checked, base: checkedBase, react: reactBase, said: carry } = carryOn(given, refused));");
  const rebased = source.indexOf('      const remade = [...new Set([...rebase(), ...carried])];\n      carried = [];\n      phase("rebase");\n');
  expect(rebased).toBeGreaterThan(0);
  expect(taken).toBeGreaterThan(rebased);
  expect(taken).toBeLessThan(source.indexOf("const since = checkedBase"));
});

// A rebase that resolved a file mechanically, std/'s generated files taken
// as main's and a ratchet's lower count, rewrote the change's patches by
// itself, and push run again checked everything again: "its commits aren't
// patch for patch those its round checked" (#2484). The patches leave those
// files out, push's own writes, which their own phases make and check
// again; any other file's difference is still refused, and that refusal is
// the sanity pass a round past a rebase runs, with nothing of the round's
// own work reused, not a full check.
test("push run again carries on past a rebase's mechanical merge, and a refused carry is a sanity pass", () => {
  const dir = fixture("push-round-remade");
  rmSync(dir, { recursive: true, force: true });
  for (const sub of ["std", "test", "src"]) mkdirSync(join(dir, sub), { recursive: true });
  const env = { ...process.env, GIT_DIR: join(dir, ".git"), GIT_WORK_TREE: dir, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@example.com", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@example.com" };
  const git = (...args: string[]) => {
    const p = spawnSync("git", args, { cwd: dir, env, encoding: "utf8" });
    return { code: p.status ?? 1, out: `${p.stdout}${p.stderr}` };
  };
  const ok = (...args: string[]) => {
    const done = git(...args);
    expect(done.code, done.out).toBe(0);
    return done.out.trim();
  };
  const commit = (files: Record<string, string>, message: string) => {
    for (const [file, text] of Object.entries(files)) writeFileSync(join(dir, file), text);
    ok("add", "-A");
    ok("commit", "-q", "-m", message);
    return ok("rev-parse", "HEAD");
  };
  ok("init", "-q", "-b", "main");
  const b1 = commit({ "src/a.rs": "fn a() {\n    1;\n}\n", "std/runtime.js": "const one = 1;\n", "test/sig-binders.txt": "src/a.rs\t3\n" }, "base");
  // The change: a function's body, a helper of std/'s and a ratchet lowered.
  ok("switch", "-q", "-c", "change");
  const changed = "fn a() {\n    2;\n}\n";
  const h1 = commit({ "src/a.rs": changed, "std/runtime.js": "const one = 1;\nconst two = 2;\n", "test/sig-binders.txt": "src/a.rs\t2\n" }, "change");
  // `main` moves on, std/ and the ratchet each its own way.
  ok("switch", "-q", "main");
  const b2 = commit({ "std/runtime.js": "const one = 1;\nconst three = 3;\n", "test/sig-binders.txt": "src/a.rs\t1\n" }, "incoming");
  // The change rebased as push resolves it: std/ as main's, to be made
  // again, and the ratchet's lower count, which is main's.
  ok("switch", "-q", "-c", "rebased");
  commit({ "src/a.rs": changed }, "change");
  expect(roundRefused({ checked: h1, base: b1 }, b2, git)).toBeUndefined();
  // And after push's own write: std/ made again, amended in.
  writeFileSync(join(dir, "std/runtime.js"), "const one = 1;\nconst three = 3;\nconst two = 2;\n");
  ok("commit", "-q", "-a", "--amend", "--no-edit");
  expect(roundRefused({ checked: h1, base: b1 }, b2, git)).toBeUndefined();
  // The sections left out are those files' alone, whole.
  const log = withoutRemade(git("log", "-p", "--no-renames", "--format=commit %H", `${b1}..${h1}`).out);
  expect(log).toContain("diff --git a/src/a.rs b/src/a.rs");
  expect(log).toContain(`commit ${h1}`);
  expect(log).not.toContain("std/runtime.js");
  expect(log).not.toContain("sig-binders");
  // A hand-written file's resolution differs: refused, but not whole.
  ok("switch", "-q", "-c", "edited", b2);
  commit({ "src/a.rs": "fn a() {\n    3;\n}\n" }, "change");
  const refused = roundRefused({ checked: h1, base: b1 }, b2, git);
  expect(refused).toEqual({ why: "its commits aren't patch for patch those its round checked", full: false });
  // Carried: the round's commit, base and React gate, so its fast tests,
  // mutants and port are reused where the rebase's commits reach nothing
  // they read.
  const round = { checked: h1, base: b1, react: b1 };
  expect(carryOn(round, undefined)).toEqual({ checked: h1, base: b1, react: b1, said: "carried" });
  // Refused for its patches: the sanity pass over what came in since the
  // commit checked, `--sanity=<checked>`, each phase of the round run again.
  expect(carryOn(round, refused)).toEqual({ checked: h1, base: "", said: "refused (its commits aren't patch for patch those its round checked)" });
  // Refused whole: a full check.
  expect(carryOn(round, { why: "its tree has a change", full: true })).toEqual({ checked: "", base: "", said: "refused (its tree has a change)" });
  // Each round past a rebase says how it came by the one before.
  expect(roundSaid("check", undefined, "all (x)")).toBe("check all (x)");
  expect(roundSaid("sanity", "carried", "some (3 files, 2 cases)")).toBe("sanity carried some (3 files, 2 cases)");
  expect(roundSaid("check", "refused (its tree has a change)", "all (x)")).toBe("check refused (its tree has a change) all (x)");
  const source = readFileSync(join(root, "scripts/push.ts"), "utf8");
  expect(source).toContain("rounds.push(roundSaid(name, carry, selection));");
});

// What a rebase resolved as main's is carried into push run again from the
// rebased tree, whose own rebase finds nothing left to resolve: std/ made
// again there from the rebased tree, not main's text kept, which dropped a
// helper the change added, `$provided` (#2417).
test("push run again makes again what its rebase resolved as main's", () => {
  const remade = ["std/runtime.js", "std/iter.js", "docs/readability.txt"];
  expect(givenRemade(againEnv({}, undefined, undefined, remade))).toEqual(remade);
  // Only what the run before it said: none carried from further back.
  expect(givenRemade(againEnv({ RUST_JS_PUSH_REMADE: "std/runtime.js" }))).toEqual([]);
  expect(givenRemade({})).toEqual([]);
  // And push carries it: its first round makes them again with its own
  // rebase's, and the run again is given them.
  const source = readFileSync(join(root, "scripts/push.ts"), "utf8");
  expect(source).toContain("const remade = [...new Set([...rebase(), ...carried])];");
  expect(source).toContain("againEnv(process.env, round, took, remade)");
});

// Push run again writes the one line of the push, its parent's time and
// phases in it: 0851a9307's said 740 s of some 2,100 s, the parent's two
// checks on no line.
test("push run again counts the time and phases of the push before it", () => {
  const before = { seconds: 612.4, phases: [["rebase", 3], ["check", 595]] as [string, number][] };
  expect(givenBefore(againEnv({ RUST_JS_TIMED_BY: "push" }, undefined, before))).toEqual(before);
  expect(givenBefore(againEnv({ RUST_JS_PUSH_BEFORE: JSON.stringify(before) }))).toEqual({ seconds: 0, phases: [] });
  expect(givenBefore({ RUST_JS_PUSH_BEFORE: "torn" })).toEqual({ seconds: 0, phases: [] });
  const source = readFileSync(join(root, "scripts/push.ts"), "utf8");
  expect(source).toContain("started: started - before.seconds * 1000, phases: [...before.phases, ...timed]");
});

// A push the server refused because `main` moved while it was pushed, in
// git's pre-push hook's minutes, is rebased and checked again, as a move
// seen before it is: its sanity pass, not a whole check (#2338). Both facts,
// the server's refusal of the ref and `main` moved: a hook's refusal that
// meets a move by chance is refused, as is a refusal with `main` where it was.
test("a push refused as main moved under it is checked again; any other refusal is refused", () => {
  const lock = "To github.com:o/r.git\n ! [remote rejected]     HEAD -> main (cannot lock ref 'refs/heads/main': is at 2cd281abf but expected 66a035ec4)\nerror: failed to push some refs";
  const behind = "To github.com:o/r.git\n ! [rejected]        HEAD -> main (fetch first)\nerror: failed to push some refs";
  const stale = " ! [rejected]        HEAD -> main (non-fast-forward)";
  const hook = "push: the guard refused: a file no commit changes\nerror: failed to push some refs to 'github.com:o/r.git'";
  expect(movedWhilePushed(1, lock, "66a035ec4", "2cd281abf")).toBe(true);
  expect(movedWhilePushed(1, behind, "66a035ec4", "2cd281abf")).toBe(true);
  expect(movedWhilePushed(1, stale, "66a035ec4", "2cd281abf")).toBe(true);
  expect(movedWhilePushed(1, hook, "66a035ec4", "2cd281abf")).toBe(false);
  expect(movedWhilePushed(1, lock, "66a035ec4", "66a035ec4")).toBe(false);
  expect(movedWhilePushed(0, lock, "66a035ec4", "2cd281abf")).toBe(false);
});

// A push pushes its commits, but its check saw the tree: a file written
// while it ran, though put back, or one made meanwhile, was what it built
// and tested, so it's refused (#2415).
test("a push refuses a tree written while it was checked, though put back", () => {
  const settled = { at: 1000, untracked: new Set(["notes.txt"]) };
  const tracked = (at: number) => new Map([["src/a.rs", 900], ["crates/b.rs", at]]);
  // Untouched since it settled: nothing.
  expect(changedWhileChecked(settled, tracked(900), new Map([["notes.txt", 950]]))).toEqual([]);
  // A tracked file written since, its content as it was or not.
  expect(changedWhileChecked(settled, tracked(1001), new Map([["notes.txt", 950]]))).toEqual(["crates/b.rs"]);
  // An untracked file made since, and one there before but written since.
  expect(changedWhileChecked(settled, tracked(900), new Map([["notes.txt", 950], ["test/corpus/new_case.rs", 1200]]))).toEqual(["test/corpus/new_case.rs"]);
  expect(changedWhileChecked(settled, tracked(900), new Map([["notes.txt", 1100]]))).toEqual(["notes.txt"]);
  // One gone since is no change of this kind: `git status` says a tracked
  // one's, and an untracked one left builds nothing.
  expect(changedWhileChecked(settled, tracked(900), new Map())).toEqual([]);
  // Push refuses with it before `git push`, the round's tree settled before
  // its build.
  const push = readFileSync(join(root, "scripts/push.ts"), "utf8");
  const settles = push.indexOf("settled = { at: Date.now()");
  expect(settles).toBeGreaterThan(-1);
  expect(settles).toBeLessThan(push.indexOf('run("build", ["cargo", "build", "--quiet"]);'));
  expect(push.indexOf("the tree changed while it was checked; nothing pushed")).toBeLessThan(push.indexOf('git("push", "-q", "origin", "HEAD:main")'));
  // Its own write of std/ after the build, `remadeStd`'s amend of what a
  // rebase resolved as main's, settles the tree again: no refusal of it,
  // where #2447's red fix was refused twice of its own `std/runtime.js`.
  const remakes = push.indexOf("remadeStd()) settled = { at: Date.now()");
  expect(remakes).toBeGreaterThan(push.indexOf('run("build", ["cargo", "build", "--quiet"]);'));
  const resettled = { ...settled, at: 1500 };
  expect(changedWhileChecked(settled, new Map([["std/runtime.js", 1400]]), new Map([["notes.txt", 950]]))).toEqual(["std/runtime.js"]);
  expect(changedWhileChecked(resettled, new Map([["std/runtime.js", 1400]]), new Map([["notes.txt", 950]]))).toEqual([]);
});

// An untracked file git doesn't ignore, there as a push starts, would be
// read by its check and not pushed: refused, naming where scratch goes
// (#2415).
test("a push refuses to start with an untracked file its check would read", () => {
  expect(untrackedRefusal([])).toBeUndefined();
  expect(untrackedRefusal(["test/corpus/new_case.rs", "notes.txt"])).toBe(
    "untracked test/corpus/new_case.rs, notes.txt would be tested but not pushed; commit it, delete it, or move it under target/",
  );
  const push = readFileSync(join(root, "scripts/push.ts"), "utf8");
  expect(push.indexOf("untrackedRefusal([...filesWritten(untrackedArgs).keys()])")).toBeLessThan(push.indexOf("const remade = [...new Set([...rebase(), ...carried])];"));
});


// `--skip-check` without its reason is refused before anything runs: the
// rule (DEVELOPMENT.md's "Pushing") asks each skip to name whether the
// selector over-selects or who said so for that push (Tu's rule, 2026-11-24).
test("a push that skips its check is refused without its reason", () => {
  for (const arg of ["--skip-check", "--skip-check=", "--skip-check=  "]) {
    const p = Bun.spawnSync([process.execPath, join(root, "scripts/push.ts"), arg], { cwd: root, stdout: "pipe", stderr: "pipe", timeout: 30_000 });
    expect([arg, p.exitCode]).toEqual([arg, 2]);
    expect(p.stderr.toString()).toContain("--skip-check needs its reason, naming which it is");
  }
  const rule = readFileSync(join(root, "DEVELOPMENT.md"), "utf8");
  expect(rule).toContain("`--skip-check=<why>` is allowed only (a) where the selector is known to");
});

// A rebase's semantic conflict, the change removing what an incoming commit
// newly calls, which git merges cleanly, is found by a compile right after
// the rebase, not minutes later by a full check or CI: 567 s on the Mac
// mini (#2484). Each compile only where both sides touch its language, the
// one way a new conflict comes in.
test("a rebase that brought commits is compiled before its tests, where both sides touch the language", () => {
  expect(compileGate(["scripts/a.ts"], ["test/b.test.ts"])).toEqual(["typecheck"]);
  expect(compileGate(["crates/x/src/a.rs"], ["Cargo.lock"])).toEqual(["clippy"]);
  expect(compileGate(["scripts/a.ts", "crates/x/src/a.rs"], ["crates/y/src/b.rs", "test/c.mjs"])).toEqual(["typecheck", "clippy"]);
  expect(compileGate(["scripts/a.ts"], ["crates/y/src/b.rs"])).toEqual([]);
  expect(compileGate(["docs/a.md"], ["scripts/b.ts"])).toEqual([]);
  expect(compileGate(["crates/x/src/a.rs"], [])).toEqual([]);

  const dir = fixture("push-gate");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const env = { ...process.env, GIT_DIR: join(dir, ".git"), GIT_WORK_TREE: dir, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@example.com", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@example.com" };
  const git = (...args: string[]) => {
    const p = spawnSync("git", args, { cwd: dir, env, encoding: "utf8" });
    return { code: p.status ?? 1, out: `${p.stdout}${p.stderr}` };
  };
  const ok = (...args: string[]) => {
    const done = git(...args);
    expect(done.code, done.out).toBe(0);
    return done.out.trim();
  };
  const commit = (file: string, text: string, message: string) => {
    writeFileSync(join(dir, file), text);
    ok("add", file);
    ok("commit", "-q", "-m", message);
  };
  ok("init", "-q", "-b", "main");
  commit("lib.ts", "export function old(): number {\n  return 1;\n}\n", "base");
  // `main` moves on: a commit that newly calls `old`.
  ok("switch", "-q", "-c", "incoming");
  commit("user.ts", 'import { old } from "./lib";\nexport const n = old();\n', "call old");
  const mainIs = ok("rev-parse", "HEAD");
  // The change, from the base, renames it: each side compiles alone.
  ok("switch", "-q", "main");
  const mainWas = ok("rev-parse", "HEAD");
  ok("switch", "-q", "-c", "change");
  commit("lib.ts", "export function renamed(): number {\n  return 1;\n}\n", "rename old");
  // A compile that refuses an import of what isn't exported, as the gate's
  // own `typecheck` does: Bun's, which a mutation's copy of the checkout
  // runs as a worktree does, with no node_modules (test/type-checked.ts
  // alone runs TypeScript's compiler).
  const typecheck = { typecheck: [process.execPath, "build", join(dir, "user.ts"), "--outdir", join(dir, "out")], clippy: ["false"] };
  ok("rebase", "-q", mainIs);
  const refused = gateRebase(mainWas, mainIs, git, typecheck, dir);
  expect(refused.ran).toEqual(["typecheck"]);
  expect(refused.refused).toContain("a semantic conflict with the commits the rebase brought");
  expect(refused.refused).toContain("call old");
  // The change that keeps what the incoming commit calls passes.
  commit("lib.ts", "export function renamed(): number {\n  return 1;\n}\nexport const old = renamed;\n", "keep old");
  expect(gateRebase(mainWas, mainIs, git, typecheck, dir)).toEqual({ ran: ["typecheck"] });
  // Nothing brought, nothing run.
  expect(gateRebase(mainIs, mainIs, git, typecheck, dir)).toEqual({ ran: [] });
});
