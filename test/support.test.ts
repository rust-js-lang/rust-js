// The tests' own helpers (ADR 0104): a native program kept for the next
// run is the program its sources make now, and the compiler is built once
// for a run, with no one left waiting on no one.

import { underLoad } from "../scripts/timing";
import { expect, test } from "bun:test";
import { existsSync, lutimesSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, symlinkSync, utimesSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative } from "node:path";

import { bytesOf, volumeBytes } from "../tooling/bounded.js";
import { disk, diskProfile, floorBytes, mainCheckoutOf } from "./bounded";
import { boundEntries, budgetBytes, entriesOf, holdToBudget, overBudget, pinnedToolchains, sccacheConfigText, sccacheWritten, sizeSccache, unpinnedToolchains, type Store } from "../scripts/disk";
import { adoptRetiredStores, holdKept, indexedSize, keptAppsCap, keptStores, mayKeep, nativeCache, nativeSources, noteKept, pruneKept, pruneKeptApps, pruneKeptStores, retiredStores, verifiedRuns, walkDue, workingSetMs, workingSetSince } from "./kept";
import { generatedTexts, record, runKey, verified } from "./verified-runs";
import { runNative } from "./programs";
import { boundCrateProjects } from "../scripts/crate-corpus";
import { boundCrater } from "../scripts/crater";
import { pruneMaps } from "../scripts/testMap";
import { keptSerdeFlags } from "./serde-kept";
import { runSync } from "./child";
import { cargoApp } from "./cargo-app";
import { cargoArtifacts, contentDirectory, expectSnapshot, fixture, knownText, nativeBinary, nativeCrates, once, portable, root, run, seeded, writeKnown } from "./support";

// Found in review: a file the case includes was changed, and the kept
// binary, of the file before, answered for it.
test("a kept native program is built again when a file it includes changes", () => {
  const dir = fixture("native-transitive");
  writeFileSync(join(dir, "cases.rs"), 'pub fn value() -> u32 {\n    include!("value.rs")\n}\n');
  const build = () => {
    const built = nativeBinary('include!("cases.rs");\nfn main() {\n    println!("{}", value());\n}\n', dir, ["--edition=2024"]);
    if ("error" in built) throw new Error(built.error);
    return built;
  };
  const answer = () => run([build().binary]).trim();
  writeFileSync(join(dir, "value.rs"), "1\n");
  expect(answer()).toBe("1");
  // A value of this run's, so its build is new, and counted in the store's
  // index (#2498), unless the store's walk took its count meanwhile.
  const value = String(Date.now() % 2 ** 31);
  const began = Date.now() / 1000;
  writeFileSync(join(dir, "value.rs"), `${value}\n`);
  const built = build();
  expect(built.built).toBe(true);
  expect(run([built.binary]).trim()).toBe(value);
  // Not kept below the disk's floor, so not counted.
  expect(!built.kept || notedSince(nativeCache, dirname(built.binary), began)).toBe(true);
});

// A native program's sources, in a directory named by them, are counted in
// their store's index as it's made (#2498).
test("a native program's sources directory is counted in its store's index", () => {
  const began = Date.now() / 1000;
  expect(notedSince(nativeSources, contentDirectory(`// ${crypto.randomUUID()}`), began)).toBe(true);
});

/** Whether `entry` was counted in `store`'s index as it was kept (#2498):
 * named in what's kept since its last walk, or a walk since `began`, a
 * machine's store's, which may have taken that count. */
function notedSince(store: string, entry: string, began: number): boolean {
  let log = "";
  try {
    log = readFileSync(join(store, ".kept"), "utf8");
  } catch {}
  return log.includes(`\t${relative(store, entry)}\n`) || (indexedSize(store)?.walked ?? 0) >= began;
}

// A program of several crates is kept as one built from one is (#882): found
// again, as it was, and built again, or another kept build of those inputs
// found, where a crate's text, or a file one includes, changes.
test("a kept native program of several crates is built again when a crate changes", () => {
  const dir = fixture("native-crates");
  writeFileSync(join(dir, "dep.rs"), 'pub fn value() -> u32 {\n    include!("value.rs")\n}\n');
  writeFileSync(join(dir, "app.rs"), "pub fn main() {\n    println!(\"{}\", dep::value() + 1);\n}\n");
  const answer = () => {
    const built = nativeCrates(
      [
        { name: "dep", file: join(dir, "dep.rs"), uses: [] },
        { name: "app", file: join(dir, "app.rs"), uses: ["dep"] },
      ],
      "fn main() {\n    app::main();\n}\n",
      ["--edition=2024"],
    );
    if ("error" in built) throw new Error(built.error);
    return { said: run([built.binary]).trim(), built: built.built };
  };
  writeFileSync(join(dir, "value.rs"), "1\n");
  const first = answer();
  expect(first.said).toBe("2");
  expect(answer()).toEqual({ said: "2", built: false });
  // Of another's inputs, a build of them, kept by an earlier run or made now.
  writeFileSync(join(dir, "value.rs"), "2\n");
  expect(answer().said).toBe("3");
  writeFileSync(join(dir, "app.rs"), "pub fn main() {\n    println!(\"{}\", dep::value() * 10);\n}\n");
  expect(answer().said).toBe("20");
});

// Found in review: the case was copied away from the files beside it.
test("a native case reads the files beside it, as the JS one does", () => {
  const dir = fixture("native-beside");
  const source = join(dir, "case.rs");
  writeFileSync(source, 'fn main() {\n    println!("{} {}", include!("value.rs"), include_str!("value.txt").trim());\n}\n');
  writeFileSync(join(dir, "value.rs"), "7\n");
  writeFileSync(join(dir, "value.txt"), "seven\n");
  const native = runNative(source, fixture("native-beside-run"), "2024", "host", [], { free: () => floorBytes });
  if (typeof native === "string") throw new Error(native);
  expect(native.stdout).toBe("7 seven\n");
});

test("a kept native program is built again when a variable it reads changes", () => {
  const dir = fixture("native-env");
  const source = 'fn main() {\n    println!("{}", env!("RUST_JS_TEST_VALUE"));\n}\n';
  const answer = (value: string) => {
    process.env.RUST_JS_TEST_VALUE = value;
    try {
      const built = nativeBinary(source, dir, ["--edition=2024"]);
      if ("error" in built) throw new Error(built.error);
      return run([built.binary]).trim();
    } finally {
      delete process.env.RUST_JS_TEST_VALUE;
    }
  };
  expect(answer("one")).toBe("one");
  expect(answer("two")).toBe("two");
});

// Found in review: two tests' wrappers were the same text, `include!("cases.rs")`,
// in two directories, and the second was given the first's binary.
test("the same wrapper in two directories is two programs", () => {
  const answer = (value: string) => {
    const dir = fixture("native-same-wrapper");
    writeFileSync(join(dir, "cases.rs"), `pub fn value() -> u32 {\n    ${value}\n}\n`);
    const built = nativeBinary('include!("cases.rs");\nfn main() {\n    println!("{}", value());\n}\n', dir, ["--edition=2024"]);
    if ("error" in built) throw new Error(built.error);
    return run([built.binary]).trim();
  };
  expect(answer("1")).toBe("1");
  expect(answer("2")).toBe("2");
});

// Found in review: a newer build replaced the one another test file had
// been given, and was about to run.
test("a binary given out stays when a newer one of its program is kept", () => {
  const dir = fixture("native-kept");
  const build = (value: string) => {
    writeFileSync(join(dir, "value.rs"), `${value}\n`);
    const built = nativeBinary('fn main() {\n    println!("{}", include!("value.rs"));\n}\n', dir, ["--edition=2024"]);
    if ("error" in built) throw new Error(built.error);
    return built.binary;
  };
  const first = build("1");
  const second = build("2");
  expect(second).not.toBe(first);
  expect(run([first]).trim()).toBe("1");
  expect(run([second]).trim()).toBe("2");
});

// Found measuring #875: a mutant's tests, and each test the test map
// records, run in a target of their own (`RUST_JS_TARGET`), and each built
// every native program it ran again, kept by where its wrapper was there:
// the map's 3,642 at each recording, 2 GB left in a store held to 3 GB. A
// native program is the checkout's whichever target asks; one built by
// another toolchain is another.
test("a native program is kept once for any target, and built again by another toolchain", () => {
  const source = 'fn main() {\n    println!("kept for any target");\n}\n';
  const flags = ["--edition=2024"];
  const support = JSON.stringify(join(import.meta.dir, "support.ts"));
  const builtIn = (env: Record<string, string>) => {
    const script = `import { contentDirectory, nativeBinary } from ${support};
const source = ${JSON.stringify(source)};
console.log(JSON.stringify(nativeBinary(source, contentDirectory(source), ${JSON.stringify(flags)})));`;
    const p = Bun.spawnSync([process.execPath, "-e", script], { cwd: root, env: { ...process.env, ...env }, stdout: "pipe", stderr: "pipe" });
    if (p.exitCode !== 0) throw new Error(p.stderr.toString());
    const built = JSON.parse(p.stdout.toString().trim().split("\n").at(-1)!);
    if ("error" in built) throw new Error(built.error);
    return built.binary as string;
  };
  const here = builtIn({});
  expect(builtIn({ RUST_JS_TARGET: fixture("another-target") })).toBe(here);
  // A rustc that says it's another, building as this one does.
  const bin = fixture("another-rustc");
  const rustc = Bun.which("rustc")!;
  writeFileSync(join(bin, "rustc"), `#!/bin/sh\nif [ "$1" = "-vV" ]; then ${rustc} -vV; echo "another: 1"; else exec ${rustc} "$@"; fi\n`, { mode: 0o755 });
  const other = builtIn({ PATH: `${bin}:${process.env.PATH}` });
  expect(other).not.toBe(here);
  expect(run([other]).trim()).toBe("kept for any target");
});

// And for any worktree of the machine (#1047): what it was built from is
// said from the checkout's root, `<root>/`, its key and the files it read,
// which another worktree finds in its own checkout.
test("a native program is kept by what it was built from, said from the checkout's root", () => {
  expect(portable(`include!("${root}/target/x/variant.rs");`)).toBe('include!("<root>/target/x/variant.rs");');
  expect(portable(`--remap-path-prefix=${root}/=`)).toBe("--remap-path-prefix=<root>/=");
  // Another path that begins as the root does isn't in it.
  expect(portable(`${root}-other/x.rs`)).toBe(`${root}-other/x.rs`);
  // The wrapper's place is named so too: another worktree's is this one's.
  expect(contentDirectory(`include!("${root}/x.rs");`)).toBe(contentDirectory('include!("<root>/x.rs");'));
  const dir = fixture("native-portable");
  writeFileSync(join(dir, "read.rs"), 'fn main() {\n    println!("read from the checkout");\n}\n');
  const source = `include!(${JSON.stringify(join(dir, "read.rs"))});\n`;
  const built = nativeBinary(source, contentDirectory(source), ["--edition=2024"]);
  if ("error" in built) throw new Error(built.error);
  expect(dirname(dirname(built.binary)).startsWith(nativeCache)).toBe(true);
  const { files } = JSON.parse(readFileSync(join(dirname(built.binary), "inputs.json"), "utf8")) as { files: [string, string][] };
  expect(files.map(([file]) => file)).toContain(`<root>/${relative(root, join(dir, "read.rs"))}`);
  expect(files.every(([file]) => !file.startsWith(root))).toBe(true);
  // Its wrapper, whose text names the checkout, is the key's, not an input.
  expect(files.some(([file]) => file.endsWith("/native.rs"))).toBe(false);
  // Found again, and built again once what it read changes, to what no
  // run before built, as the store is the machine's.
  expect(nativeBinary(source, contentDirectory(source), ["--edition=2024"])).toEqual({ binary: built.binary, built: false, kept: true });
  const changed = `changed ${process.pid} ${Date.now()}`;
  writeFileSync(join(dir, "read.rs"), `fn main() {\n    println!("${changed}");\n}\n`);
  const again = nativeBinary(source, contentDirectory(source), ["--edition=2024"]);
  if ("error" in again) throw new Error(again.error);
  expect(again.built).toBe(true);
  expect(run([again.binary]).trim()).toBe(changed);
});

// The compiler's build, once for a run: one process does it, those at the
// same time wait for it, and none takes it over.
test("work claimed once runs once, however many ask at the same time", async () => {
  const dir = join(fixture("once"), "claim");
  const counter = join(dirname(dir), "count");
  const script = `import { appendFileSync } from "node:fs";
import { once } from ${JSON.stringify(join(import.meta.dir, "support.ts"))};
once(${JSON.stringify(dir)}, () => { appendFileSync(${JSON.stringify(counter)}, "ran\\n"); Bun.sleepSync(500); });
`;
  const runs = [0, 1, 2].map(() => Bun.spawn(["bun", "-e", script], { stderr: "pipe" }));
  expect(await Promise.all(runs.map((p) => p.exited))).toEqual([0, 0, 0]);
  expect(readFileSync(counter, "utf8")).toBe("ran\n");
});

// Found in review: a lock made, and its maker stopped before it said who it
// was, and every run after waited for it. A claim is a run's own, and one
// whose worker has ended is an error, not a wait, and not taken over.
test("work claimed by a process that has ended is an error, not a wait", () => {
  const dir = join(fixture("once-ended"), "claim");
  mkdirSync(dir);
  writeFileSync(join(dir, "pid"), "999999");
  expect(() => once(dir, () => "ran")).toThrow("ended before it finished");
});

// Found in review: two test files building the same program shared its
// source, and one's rustc read it as the other rewrote it, empty. It's
// replaced whole, a new file moved into place, never rewritten in place.
test("a native program's source is replaced whole, not rewritten in place", () => {
  const dir = fixture("native-whole");
  const source = 'fn main() {\n    println!("whole");\n}\n';
  const built = nativeBinary(source, dir, ["--edition=2024"]);
  if ("error" in built) throw new Error(built.error);
  const before = statSync(join(dir, "native.rs")).ino;
  nativeBinary(source, dir, ["--edition=2024"]);
  expect(statSync(join(dir, "native.rs")).ino).not.toBe(before);
  expect(readFileSync(join(dir, "native.rs"), "utf8")).toBe(source);
});

// Found in review: a file that never built the compiler ran the formatter,
// which built it itself, and Cargo, linking it again, took it away from the
// files running beside it. Each file is given it before it runs: this one
// builds nothing.
test("every test file is given the compiler before its tests run", () => {
  expect(process.env.RUST_JS_COMPILER).toBeTruthy();
});

// The kept native programs are pruned when a run starts, where none is
// running: what no test has used for a week, and a build a crash left half
// made. Each use marks one used, so a program in use stays however old it is.
test("a kept cache keeps what was used this week, and drops the rest", () => {
  const cache = fixture("native-cache");
  const now = Date.now() / 1000;
  const day = 24 * 60 * 60;
  const entry = (name: string, kept: string, age: number) => {
    const dir = join(cache, name, kept);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "native"), "");
    utimesSync(dir, now - age * day, now - age * day);
    utimesSync(join(cache, name), now - age * day, now - age * day);
  };
  entry("fresh", "a", 1);
  entry("stale", "a", 8);
  entry("mixed", "old", 9);
  entry("mixed", "new", 2);
  entry("crashed", ".building-1-2", 2);
  // A disk with room, so only age decides, however full this machine's is
  // (#963).
  pruneKept(cache, { now, days: 7, free: () => Number.POSITIVE_INFINITY });
  expect(["fresh/a", "stale", "mixed/old", "mixed/new", "crashed"].map((p) => existsSync(join(cache, p)))).toEqual([true, false, false, true, false]);
});

// And each store is held to its cap (#914): the least recently used go
// first, until what's left is under it, and below the disk's floor until
// it's back or nothing's left; a program with no build left goes too.
test("a kept cache is held to its cap, the least recently used first, and to the disk's floor", () => {
  const cache = fixture("native-capped");
  const now = Date.now() / 1000;
  const entry = (name: string, kept: string, used: number) => {
    const dir = join(cache, name, kept);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "native"), Buffer.alloc(64 * 1024));
    utimesSync(dir, now - used, now - used);
    return dir;
  };
  const oldest = entry("p", "a", 400);
  const older = entry("q", "a", 300);
  const newer = entry("p", "b", 200);
  const newest = entry("r", "a", 100);
  const each = bytesOf(newest);
  expect(each).toBeGreaterThanOrEqual(64 * 1024);
  // Each build stale here: a store's working set is the next test's.
  const since = Date.now();
  expect(pruneKept(cache, { now, cap: 2 * each, free: () => Number.POSITIVE_INFINITY, since })).toEqual({ removed: 2, size: 2 * each });
  expect([oldest, older, newer, newest].map(existsSync)).toEqual([false, false, true, true]);
  expect(existsSync(join(cache, "q"))).toBe(false);
  // Under the floor: pruned until it's back, here once one more is gone.
  expect(pruneKept(cache, { now, cap: 2 * each, free: () => (existsSync(newer) ? floorBytes - 1 : floorBytes), since })).toEqual({ removed: 1, size: each });
  expect([newer, newest].map(existsSync)).toEqual([false, true]);
  // Each store's cap.
  expect(keptStores.map(({ dir, cap }) => [basename(dir), cap / 2 ** 20])).toEqual([
    ["native-cache", disk.nativeCache / 2 ** 20],
    ["native-sources", 64],
    [basename(verifiedRuns), 512],
    ["std-reach", 64],
    ["native-outcomes", 256],
  ]);
});

// A store's cap never takes its working set, what the machine's runs used
// since this checkout's last check or in the last day: native-cache's 10 GiB
// sat below the 11.5 GiB the Mac mini's checks used, so a prune evicted
// builds in use, and the next check's every corpus part stopped at its
// limit, a 1,018 s re-warm (#2344). The least a store keeps is what's
// measured in use; past its cap only the stale go, and the disk's floor
// takes from the rest, the least recently used first.
test("a kept store past its cap keeps its working set, and only stale builds go", () => {
  const cache = fixture("native-working-set");
  const now = Date.now() / 1000;
  const hour = 3600;
  const entry = (name: string, ago: number) => {
    const dir = join(cache, name, "build");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "native"), Buffer.alloc(64 * 1024));
    utimesSync(dir, now - ago, now - ago);
    return dir;
  };
  const used = entry("used an hour ago", hour);
  const usedLater = entry("used a minute ago", 60);
  const stale = entry("stale", 3 * 24 * hour);
  const staler = entry("staler", 4 * 24 * hour);
  const each = bytesOf(used);
  const since = Date.now() - workingSetMs;
  // Its cap one build, its working set two: the stale go, the working set stays.
  expect(pruneKept(cache, { now, cap: each, free: () => Number.POSITIVE_INFINITY, since })).toEqual({ removed: 2, size: 2 * each });
  expect([used, usedLater, stale, staler].map(existsSync)).toEqual([true, true, false, false]);
  // The disk at its floor takes from it, the least recently used first.
  // Below it till one's gone: asked again before the walk, whether it's due.
  expect(pruneKept(cache, { now, cap: each, free: () => (existsSync(used) ? floorBytes - 1 : floorBytes), since })).toEqual({ removed: 1, size: each });
  expect([used, usedLater].map(existsSync)).toEqual([false, true]);
});

// A store's walk lists and sizes every entry: std-reach's 109k took 3.4 s,
// the five stores' 9 s quiet and 14–24 s of a core beside a check's jobs
// (#2498). Each walk is kept in the store's index, with what's kept since,
// and the next is made only where it's due: a day on, so what's a week
// unused goes within a day of it; with the disk below its floor; or past
// its cap, an hour on, as a working set past it is kept by a walk anyway.
test("a store is walked again only once its index says a walk is due", () => {
  const cache = fixture("native-indexed");
  const now = Date.now() / 1000;
  const day = 24 * 3600;
  const entry = (name: string, ago: number) => {
    const dir = join(cache, name, "build");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "native"), Buffer.alloc(64 * 1024));
    utimesSync(dir, now - ago, now - ago);
    return dir;
  };
  const used = entry("used", 60);
  const each = bytesOf(used);
  const room = () => Number.POSITIVE_INFINITY;
  const since = Date.now() - workingSetMs;
  const cap = 3 * each;
  expect(pruneKept(cache, { now, cap, free: room, since })).toEqual({ removed: 0, size: each });
  expect(indexedSize(cache)).toEqual({ walked: now, bytes: each });
  // A week unused, made where no walk saw it: it's not listed till one's due.
  const stale = entry("stale", 8 * day);
  expect(pruneKept(cache, { now: now + 60, cap, free: room, since })).toEqual({ removed: 0, size: each });
  expect(existsSync(stale)).toBe(true);
  // A day on, it's walked, and the week-old entry goes.
  expect(pruneKept(cache, { now: now + day, cap, free: room, since })).toEqual({ removed: 1, size: each });
  expect(existsSync(stale)).toBe(false);
  expect(existsSync(join(cache, ".kept"))).toBe(false);
  // What's kept since counts: past the cap, it's walked an hour after the
  // last walk, not before.
  for (const name of ["a", "b", "c"]) noteKept(cache, entry(name, 0));
  expect(indexedSize(cache)).toEqual({ walked: now + day, bytes: 4 * each });
  expect(walkDue(cache, { cap, free: room, now: now + day + 60 })).toBe(false);
  expect(walkDue(cache, { cap: 4 * each, free: room, now: now + day + 3600 })).toBe(false);
  expect(walkDue(cache, { cap, free: room, now: now + day + 3600 })).toBe(true);
  // Below the disk's floor, at once.
  expect(walkDue(cache, { cap: Number.POSITIVE_INFINITY, free: () => floorBytes - 1, now: now + day + 60 })).toBe(true);
  expect(walkDue(cache, { cap: Number.POSITIVE_INFINITY, free: room, now: now + day + 60 })).toBe(false);
});

// A check's prune runs beside its jobs (#2498), so a build its walk listed
// stale may be used by a job before it's removed: looked at again as it
// goes, it's the working set's, and stays; one still stale goes.
test("a kept entry used after the prune listed it stays", () => {
  const cache = fixture("kept-used-since-listed");
  const day = 24 * 3600 * 1000;
  const entry = (name: string) => {
    const dir = join(cache, name);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "native"), Buffer.alloc(64 * 1024));
    const at = (Date.now() - 3 * day) / 1000;
    utimesSync(dir, at, at);
    return { path: dir, bytes: bytesOf(dir), used: Date.now() - 3 * day };
  };
  const listed = [entry("used since"), entry("still stale")];
  // A job's use of the first, `markUsed`'s, after the walk read its time.
  const now = new Date();
  utimesSync(listed[0]!.path, now, now);
  const since = Date.now() - day;
  expect(holdKept(listed, { cap: 0, free: () => Number.POSITIVE_INFINITY, since })).toEqual({ removed: 1, size: listed[0]!.bytes });
  expect(listed.map(({ path }) => existsSync(path))).toEqual([true, false]);
});

// The kept apps (#882) are held to their cap, the least recently used first,
// and to the disk's floor; one a live process claims stays, whatever its age.
test("kept apps are held to their cap, but one a live process holds", () => {
  const base = fixture("kept-apps-pruned");
  const now = Date.now() / 1000;
  const app = (name: string, used: number, owner?: number) => {
    const dir = join(base, "xscope", name);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "built"), Buffer.alloc(64 * 1024));
    utimesSync(dir, now - used, now - used);
    if (owner !== undefined) writeFileSync(`${dir}.owner`, String(owner));
    return dir;
  };
  const held = app("held_0", 500, process.pid);
  const oldest = app("old_0", 400, 99999999);
  const newer = app("new_0", 100);
  const each = bytesOf(newer);
  expect(pruneKeptApps({ base, cap: each, free: () => Number.POSITIVE_INFINITY })).toEqual({ removed: 1, size: each });
  expect([held, oldest, newer].map(existsSync)).toEqual([true, false, true]);
  expect(keptAppsCap).toBe(4 * 2 ** 30);
});

// A kept app no process held as the prune listed them, which one claims
// before it's removed, as the pilot's was as another test's process ended
// below the disk's floor: it stays, its claim the claimant's (#2446).
test("a kept app claimed after the prune listed it, before it's removed, stays", () => {
  const base = fixture("kept-apps-claimed-mid-prune");
  const dir = join(base, "xscope", "pilot_0");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), "[workspace]\n");
  let asked = 0;
  // Asked first as it's about to remove the app: the claim's taken then.
  const free = () => {
    if (asked++ === 0) writeFileSync(`${dir}.owner`, String(process.pid));
    return floorBytes - 1;
  };
  expect(pruneKeptApps({ base, cap: Number.POSITIVE_INFINITY, free })).toEqual({ removed: 0, size: bytesOf(dir) });
  expect([existsSync(join(dir, "Cargo.toml")), readFileSync(`${dir}.owner`, "utf8")]).toEqual([true, String(process.pid)]);
  // Its claim given up, it goes, and its claim the prune took with it.
  rmSync(`${dir}.owner`);
  expect(pruneKeptApps({ base, cap: Number.POSITIVE_INFINITY, free: () => floorBytes - 1 }).removed).toBe(1);
  expect([existsSync(dir), existsSync(`${dir}.owner`)]).toEqual([false, false]);
});

// A claim whose process is gone, taken over by several processes at once:
// one takes it, each taking one over one at a time (#2446). Each waits for
// one moment to start, so they race, and lives till every one has said,
// so a winner's claim is never one whose process is gone by then.
test("a claim whose process is gone is taken over by one of the processes racing for it", async () => {
  const base = fixture("claims-raced");
  const kept = join(import.meta.dir, "kept.ts");
  for (let round = 0; round < 12; round++) {
    const owner = join(base, `app_${round}.owner`);
    writeFileSync(owner, "99999999");
    const at = Date.now() + 800;
    const racer = `import { takeClaim } from ${JSON.stringify(kept)};\nwhile (Date.now() < ${at});\nconsole.log(takeClaim(${JSON.stringify(owner)}) ? process.pid : "-");\nawait Bun.stdin.text();`;
    const racers = Array.from({ length: 12 }, () => Bun.spawn([process.execPath, "-e", racer], { stdin: "pipe", stdout: "pipe" }));
    const said = async (p: (typeof racers)[number]) => {
      const reader = p.stdout.getReader();
      let text = "";
      while (!text.includes("\n")) {
        const { value, done } = await reader.read();
        if (done) break;
        text += new TextDecoder().decode(value);
      }
      return text.trim();
    };
    const won = (await Promise.all(racers.map(said))).filter((pid) => pid !== "-");
    for (const p of racers) p.stdin.end();
    await Promise.all(racers.map((p) => p.exited));
    expect(won.length).toBe(1);
    expect(readFileSync(owner, "utf8")).toBe(won[0]);
  }
}, underLoad(60_000));

// Nothing's kept below the disk's floor, a native program's build or a
// verified run: what's made is used, not kept (#1047). And the checkout's
// stores from before they were the machine's go as the stores are pruned.
test("a store keeps nothing below the disk's floor, and a retired store is removed", () => {
  const dir = fixture("floor");
  expect(mayKeep(dir, () => floorBytes)).toBe(true);
  expect(mayKeep(dir, () => floorBytes - 1)).toBe(false);
  for (const retired of retiredStores) mkdirSync(join(retired, "left"), { recursive: true });
  // Of a store of its own, not the machine's: pruning theirs read each of
  // some 40,000 entries' sizes, 44 s alone on the Mac mini and 142.8 s
  // under a push's load, past its limit, failing the Mac mini's pushes
  // (#2498). Pruning them is the prune's tests' and every check's.
  const store = join(dir, "store");
  mkdirSync(join(store, "program", "build"), { recursive: true });
  pruneKeptStores(() => floorBytes, [{ dir: store, cap: Number.POSITIVE_INFINITY }]);
  expect(retiredStores.map(existsSync)).toEqual([false, false]);
  expect(existsSync(join(store, "program", "build"))).toBe(true);
}, underLoad(60_000));

// Each fixture a test claims is removed once its file's tests end
// (test/setup.ts), as Bun's test runner never emits `exit`; and one a
// process gone left, killed before it could, by the next process's first
// claim (#914): one `check:fast --all` left 7.6 GB of them.
test("a fixture whose process is gone is removed by the next process to claim one", () => {
  const mine = fixture("claimed");
  const base = dirname(dirname(mine));
  const left = join(base, "xgone", "left_0");
  mkdirSync(join(left, "deep"), { recursive: true });
  writeFileSync(join(left, "deep", "file"), "x");
  // A pid no process has: past the system's highest.
  writeFileSync(`${left}.owner`, "99999999");
  const held = join(base, "xheld", "held_0");
  mkdirSync(held, { recursive: true });
  writeFileSync(`${held}.owner`, String(process.pid));
  const p = Bun.spawnSync([process.execPath, "-e", "import { fixture } from './test/support.ts'; console.log(fixture('other'));"], { cwd: root, stdout: "pipe", stderr: "pipe" });
  expect(p.exitCode).toBe(0);
  const other = p.stdout.toString().trim();
  expect([existsSync(left), existsSync(`${left}.owner`), existsSync(held), existsSync(mine)]).toEqual([false, false, true, true]);
  // A script's process removes its own as it exits.
  expect([existsSync(other), existsSync(`${other}.owner`)]).toEqual([false, false]);
  rmSync(join(base, "xheld"), { recursive: true, force: true });
});

// The apps' Cargo targets made from the run's seed (#1005): their binding
// crates checked once a run, not by each app. Which holds only while their
// key is the same in every app (#892): another, and each app would check
// them again, unseen but for the time, so this build is held to finding
// them done, its `webapi` the seed's file, without rustc's incremental
// state a build of it writes.
test("an app's Cargo target made from the run's seed finds the binding crates checked", () => {
  const dir = fixture("seeded-app");
  cargoApp(dir, "pub fn answer() -> u32 {\n    42\n}\n", { react: true });
  const made = seeded(join(dir, "target"));
  expect(() => seeded(made)).toThrow("a seed is a new target's");
  const profile = join(made, "wasm32-unknown-unknown", "debug");
  const webapi = cargoArtifacts(profile).filter((file) => /^libwebapi-[0-9a-f]+\.rmeta$/.test(basename(file)));
  expect(webapi.length).toBe(1);
  const before = statSync(webapi[0]).mtimeMs;
  const built = runSync([process.execPath, join(root, "tooling", "cli.js"), "build"], dir, 600_000, { CARGO_TARGET_DIR: made });
  expect([built.code, built.stderr]).toEqual([0, ""]);
  expect(cargoArtifacts(profile).filter((file) => /^libapp-[0-9a-f]+\.rmeta$/.test(basename(file))).length).toBe(1);
  expect(statSync(webapi[0]).mtimeMs).toBe(before);
  const incremental = join(made, "wasm32-unknown-unknown", "debug", "incremental");
  expect(readdirSync(incremental).filter((name) => /^(webapi|react|js)-/.test(name))).toEqual([]);
}, 600_000);

// A corpus case's runs are kept by what they ran (ADR 0567): a key of each
// input whole, so no two sets of inputs share one; verified once recorded,
// in the cache given, and marked used then.
test("a verified run's key is of each input whole, and found once recorded", () => {
  expect(runKey(["ab", "c"])).not.toBe(runKey(["a", "bc"]));
  expect(runKey(["a", "b"])).not.toBe(runKey(["b", "a"]));
  expect(runKey(["a", "b"])).toBe(runKey(["a", Buffer.from("b")]));
  const cache = fixture("verified-runs");
  expect(verified("k", cache)).toBe(false);
  record("k", "case.rs", cache, () => floorBytes);
  expect(verified("k", cache)).toBe(true);
  expect(readFileSync(join(cache, "k", "passed"), "utf8")).toBe("case.rs\n");
  // The JS as written, its header's path the run's folder's, is one text.
  const dir = fixture("generated-texts");
  mkdirSync(join(dir, "library"));
  writeFileSync(join(dir, "case.js"), `// Generated by rust-js from ${dir}/lib.rs. Do not edit.\nexport const n = 1;\n`);
  writeFileSync(join(dir, "library", "case.js"), "other\n");
  expect(generatedTexts(dir, ["library"])).toEqual(["case.js\0// Generated by rust-js. Do not edit.\nexport const n = 1;\n"]);
});

// React's port keeps a directory for each checkout in the system's
// temporary one (scripts/react-in-rust.ts), bounded as each run starts
// (#914): one whose checkout is gone goes; then, of those no run used in
// the last hour, the least recently used, past the cap with the run's own,
// or below the disk's floor. Seven orphans once took 8 GB.
test("React's port's directories are bounded: a gone checkout's removed, then the least recently used past the cap", async () => {
  const { bound } = await import("../scripts/react-in-rust");
  const dir = fixture("ports");
  const checkout = fixture("a-checkout");
  const hours = (n: number) => Date.now() / 1000 - n * 60 * 60;
  const port = (name: string, { linked, used }: { linked?: string; used: number }) => {
    const at = join(dir, `react-in-rust-${name}`);
    mkdirSync(at);
    writeFileSync(join(at, "build"), Buffer.alloc(64 * 1024));
    if (linked !== undefined) symlinkSync(linked, join(at, "rust-js"));
    writeFileSync(join(at, "used"), "");
    utimesSync(join(at, "used"), used, used);
    utimesSync(at, used, used);
    return at;
  };
  const older = port("aaaaaaaa", { linked: checkout, used: hours(2) });
  const gone = port("bbbbbbbb", { linked: join(checkout, "gone"), used: hours(0) });
  const unlinked = port("cccccccc", { used: hours(2) });
  const busy = port("dddddddd", { linked: checkout, used: hours(0) });
  const oldest = port("eeeeeeee", { linked: checkout, used: hours(3) });
  const own = port("ffffffff", { linked: checkout, used: hours(0) });
  const other = join(dir, "not-a-port");
  mkdirSync(other);
  const each = bytesOf(older);
  // The run's own and one more fit; the one in use stays however far past.
  const removed = bound({ dir, own, cap: 2 * each + each / 2, free: () => Number.POSITIVE_INFINITY });
  expect(removed.sort()).toEqual([gone, unlinked, oldest].sort());
  expect([older, busy, own, other].map(existsSync)).toEqual([true, true, true, true]);
  // Below the floor, each one not in use goes, and never the run's own.
  expect(bound({ dir, own, free: () => floorBytes - 1 })).toEqual([older]);
  expect([busy, own].map(existsSync)).toEqual([true, true]);
});

// The crate corpus's projects are a store (#1213): each build of the
// compiler adds a copy of every crate's dependencies, so they're held to a
// cap, the least recently measured crate first, and to the disk's floor.
test("the crate corpus's projects keep the most recently measured under their cap and above the floor", () => {
  const dir = fixture("crate-projects");
  const made = (name: string, age: number) => {
    mkdirSync(join(dir, name, "target"), { recursive: true });
    writeFileSync(join(dir, name, "target", "lib.rlib"), Buffer.alloc(64 * 1024));
    writeFileSync(join(dir, name, "used"), "");
    const when = Date.now() / 1000 - age;
    utimesSync(join(dir, name, "used"), when, when);
  };
  made("oldest", 300);
  made("older", 200);
  made("newest", 100);
  const each = (statSync(join(dir, "newest", "target", "lib.rlib")).blocks ?? 128) * 512;
  // Room for two of the three: the least recently measured goes.
  boundCrateProjects({ dir, cap: 2.5 * each, free: () => floorBytes });
  expect(readdirSync(dir).sort()).toEqual(["newest", "older"]);
  // Below the floor, another goes, whatever the cap.
  boundCrateProjects({ dir, cap: Number.POSITIVE_INFINITY, free: () => (existsSync(join(dir, "older")) ? floorBytes - 1 : floorBytes) });
  expect(readdirSync(dir)).toEqual(["newest"]);
});

// One root of what development keeps (#1213): a worktree finds the main
// checkout's `.cache/` through its `.git` file and the common dir's
// `commondir`, as `git rev-parse --git-common-dir` does.
test("a worktree's stores are the main checkout's", () => {
  const dir = fixture("main-checkout");
  mkdirSync(join(dir, "main", ".git", "worktrees", "w"), { recursive: true });
  writeFileSync(join(dir, "main", ".git", "worktrees", "w", "commondir"), "../..\n");
  mkdirSync(join(dir, "w"));
  writeFileSync(join(dir, "w", ".git"), `gitdir: ${join(dir, "main", ".git", "worktrees", "w")}\n`);
  expect(mainCheckoutOf(join(dir, "w"))).toBe(join(dir, "main"));
  expect(mainCheckoutOf(join(dir, "main"))).toBe(join(dir, "main"));
});

// Rust's toolchains development installed and nothing pins now go; a
// floating channel's are the user's own (#1213).
test("an old pin's toolchain is one to remove, a floating channel's never", () => {
  const listed = ["stable-aarch64-apple-darwin (default)", "nightly-aarch64-apple-darwin", "1.98.1-aarch64-apple-darwin", "1.100.0-aarch64-apple-darwin (active)", "nightly-2026-03-25-x86_64-unknown-linux-gnu"];
  expect(unpinnedToolchains(listed, new Set(["1.100.0"]))).toEqual(["1.98.1-aarch64-apple-darwin", "nightly-2026-03-25-x86_64-unknown-linux-gnu"]);
  expect(pinnedToolchains().has("1.100.0")).toBe(true);
});

// A store with no bound of its own, the temp root's, is held by its
// entries, the least recently changed first, and to the disk's floor.
test("a store's entries are held to its cap and the floor", () => {
  const dir = fixture("entries");
  for (const [name, age] of [["old", 300], ["new", 100]] as const) {
    writeFileSync(join(dir, name), Buffer.alloc(64 * 1024));
    const when = Date.now() / 1000 - age;
    utimesSync(join(dir, name), when, when);
  }
  const each = (statSync(join(dir, "new")).blocks ?? 128) * 512;
  boundEntries(dir, 1.5 * each, () => floorBytes);
  expect(readdirSync(dir)).toEqual(["new"]);
  boundEntries(dir, Number.POSITIVE_INFINITY, () => floorBytes - 1);
  expect(readdirSync(dir)).toEqual([]);
});

// The stores inside the checkouts are held to the budget together, not
// each to its cap alone: three worktrees' stores, each under its cap,
// summed to 37.9 of 40 GiB and no prune took a byte (#2163). Past it, the
// least recently used of all their entries go first; a build stays, and
// what's left past it is said with the largest holders.
test("the stores inside the checkouts are held to the budget together, the least recently used of all first", () => {
  const [a, b, build, tools] = ["budget-a", "budget-b", "budget-build", "budget-tools"].map((name) => fixture(name));
  const entry = (dir: string, name: string, age: number) => {
    writeFileSync(join(dir, name), Buffer.alloc(64 * 1024));
    const when = Date.now() / 1000 - age;
    utimesSync(join(dir, name), when, when);
  };
  entry(a, "oldest", 400);
  entry(b, "older", 300);
  entry(a, "newer", 200);
  entry(b, "newest", 100);
  entry(build, "binary", 500);
  entry(tools, "a tool's own", 600);
  const each = bytesOf(join(a, "oldest"));
  const stores: Store[] = [
    { name: "a", dir: a, cap: Number.POSITIVE_INFINITY, kept: () => entriesOf(a) },
    { name: "b", dir: b, cap: Number.POSITIVE_INFINITY, kept: () => entriesOf(b) },
    { name: "the build", dir: build },
    { name: "outside", dir: tools, outside: true, kept: () => entriesOf(tools) },
  ];
  const free = () => floorBytes;
  // Each entry stale here: the working set is the next test's.
  const since = Date.now();
  // Under what they're held to: nothing goes.
  expect(holdToBudget(stores, 5 * each, { held: 5 * each, free, since })).toEqual({ removed: 0, size: 5 * each });
  // Past it: the two oldest of both stores, never the build, however old.
  expect(holdToBudget(stores, 5 * each, { held: 3 * each, free, since })).toEqual({ removed: 2, size: 3 * each });
  expect([readdirSync(a), readdirSync(b), readdirSync(build), readdirSync(tools)]).toEqual([["newer"], ["newest"], ["binary"], ["a tool's own"]]);
  // Held below what the build takes: every stale entry goes, and the build is what's left.
  expect(holdToBudget(stores, 3 * each, { held: each / 2, free, since })).toEqual({ removed: 2, size: each });
  expect(readdirSync(build)).toEqual(["binary"]);
  const said = overBudget(stores, each, (store) => ({ "the build": 6 * 2 ** 30, outside: 7 * 2 ** 30 })[store.name] ?? 0, () => "3 d");
  expect(said).toContain("the largest are the build 6.0 GiB, its oldest 3 d;");
  expect(said).not.toContain("outside");
});

// A checkout past the budget with every stale entry gone keeps its working
// set, what was used since its last check or in the last day: idle
// worktrees' builds took lily agent's total to 100 of 40 GiB, so each check's
// measure removed every corpus run and cache it was about to reuse, and
// check:fast went from ~300 s to 630-809 s (#2163's regression). Only the
// disk's floor takes from the working set; what's left is said, with its
// holders.
test("a checkout past the budget keeps its working set, and only stale entries go", () => {
  const [kept, build, mark] = ["working-set-kept", "working-set-build", "working-set-mark"].map((name) => fixture(name));
  const entry = (name: string, ago: number) => {
    writeFileSync(join(kept, name), Buffer.alloc(64 * 1024));
    const when = (Date.now() - ago) / 1000;
    utimesSync(join(kept, name), when, when);
  };
  const hour = 3600 * 1000;
  entry("used by the last check", 30 * hour);
  entry("used an hour ago", hour);
  entry("stale", 3 * workingSetMs);
  entry("staler", 4 * workingSetMs);
  writeFileSync(join(build, "idle worktree's build"), Buffer.alloc(4 * 64 * 1024));
  const each = bytesOf(join(kept, "stale"));
  const stores: Store[] = [
    { name: "kept runs", dir: kept, cap: Number.POSITIVE_INFINITY, kept: () => entriesOf(kept) },
    { name: "an idle build", dir: build },
  ];
  // This checkout's last check started 31 hours ago: what it used then is
  // in the working set, past the day.
  const last = join(mark, "last-check");
  writeFileSync(last, "");
  const started = (Date.now() - 31 * hour) / 1000;
  utimesSync(last, started, started);
  const since = workingSetSince(Date.now(), last);
  expect(since).toBeLessThan(Date.now() - workingSetMs);
  expect(workingSetSince(Date.now(), join(mark, "never checked"))).toBeLessThanOrEqual(Date.now() - workingSetMs);
  // The build alone is past what it's held to: the stale go, the working set stays.
  const total = 4 * each + bytesOf(build);
  const held = 2 * each;
  const result = holdToBudget(stores, total, { held, free: () => floorBytes, since });
  expect(result).toEqual({ removed: 2, size: 2 * each + bytesOf(build) });
  expect(readdirSync(kept).sort()).toEqual(["used an hour ago", "used by the last check"]);
  expect(result.size).toBeGreaterThan(held);
  // The disk at its floor takes from it, the least recently used first.
  let free = floorBytes - 1;
  const floored = holdToBudget(stores, result.size, { held, free: () => free++, since });
  expect(floored.removed).toBe(1);
  expect(readdirSync(kept)).toEqual(["used an hour ago"]);
});

// An entry gone since it was listed is no longer kept: two checkouts' runs
// share the temp root, and one removes a test's own directory there as the
// other prunes it, which ended a `check:fast` before it checked anything
// (#1473). A dangling link is listed and has nothing to `stat`, the same
// failure, made here: it's its own, as `bytesOf` takes it, and goes first.
test("a store's entry with nothing behind it is held as its own, not the end of the prune", () => {
  const dir = fixture("entries-gone");
  symlinkSync(join(dir, "removed-by-another-run"), join(dir, "left"));
  writeFileSync(join(dir, "kept"), Buffer.alloc(64 * 1024));
  expect(boundEntries(dir, Number.POSITIVE_INFINITY, () => floorBytes).removed).toBe(0);
  expect(readdirSync(dir).sort()).toEqual(["kept", "left"]);
  const when = Date.now() / 1000 - 300;
  lutimesSync(join(dir, "left"), when, when);
  expect(boundEntries(dir, 1, () => floorBytes).removed).toBe(2);
  expect(readdirSync(dir)).toEqual([]);
});

// The machine's stores were `~/.cache/rust-js`'s: each moved to the main
// checkout's `.cache/` once, merged with what's kept there already, and the
// old place removed, so no machine keeps an orphan (#1213); a task's log in
// both keeps its start, written before the move (ldAC's).
test("the machine's stores where they were are moved where they are, once", () => {
  const dir = fixture("retired-stores");
  const retired = join(dir, "home-cache");
  const current = join(dir, "checkout-cache");
  mkdirSync(join(retired, "native-cache", "p"), { recursive: true });
  writeFileSync(join(retired, "native-cache", "p", "build"), "old");
  mkdirSync(join(retired, "test-maps"), { recursive: true });
  writeFileSync(join(retired, "test-maps", "m.json"), "{}");
  mkdirSync(join(current, "test-maps"), { recursive: true });
  writeFileSync(join(current, "test-maps", "kept.json"), "{}");
  mkdirSync(join(retired, "tasks"), { recursive: true });
  writeFileSync(join(retired, "tasks", "main.jsonl"), '{"kind":"start","at":1}\n');
  mkdirSync(join(current, "tasks"), { recursive: true });
  writeFileSync(join(current, "tasks", "main.jsonl"), '{"kind":"run","at":2}\n');
  adoptRetiredStores(retired, current);
  expect(existsSync(retired)).toBe(false);
  expect(readFileSync(join(current, "native-cache", "p", "build"), "utf8")).toBe("old");
  expect(readdirSync(join(current, "test-maps")).sort()).toEqual(["kept.json", "m.json"]);
  expect(readFileSync(join(current, "tasks", "main.jsonl"), "utf8")).toBe('{"kind":"start","at":1}\n{"kind":"run","at":2}\n');
  adoptRetiredStores(retired, current);
  expect(readdirSync(current).sort()).toEqual(["native-cache", "tasks", "test-maps"]);
});

// A store's sweep lists its entries and then reads each one's time, while
// another run's sweep of the same store removes or renames them: crates.test.ts
// failed on `ENOENT … stat '…/target/serde-kept/<hash>'` (2026-11-17). An
// entry gone since it was listed, here a link whose target is gone, which
// `statSync` reads as such, is skipped by each sweep, not thrown on.
test("a store's sweep skips an entry gone since it was listed", () => {
  const dir = fixture("sweep-vanished");
  const gone = (store: string, name: string) => {
    mkdirSync(store, { recursive: true });
    symlinkSync(join(dir, "nothing-here"), join(store, name));
  };
  const serdeStore = join(dir, "serde-kept");
  gone(serdeStore, "0123456789abcdef01234567");
  const built = join(dir, "built.rmeta");
  writeFileSync(built, "serde's");
  expect(keptSerdeFlags(["--extern", `serde=${built}`], serdeStore)).toHaveLength(2);
  const kept = join(dir, "kept");
  gone(join(kept, "program"), "build");
  expect(pruneKept(kept, { free: () => Number.POSITIVE_INFINITY }).removed).toBe(0);
  const maps = join(dir, "maps");
  gone(maps, "map.json");
  expect(() => pruneMaps(maps, 1, () => Number.POSITIVE_INFINITY)).not.toThrow();
  const crater = join(dir, "crater");
  gone(crater, "a-crate");
  expect(() => boundCrater({ dir: crater, cap: 1, free: () => Number.POSITIVE_INFINITY })).not.toThrow();
});

// #2260: a known list kept as a directory, a file of each issue, read as
// one list and written back the same: an issue whose rows are all taken
// off loses its file, and the README keeps what comes before the headings.
test("a known list kept as a directory is read as one list and written back the same", () => {
  const dir = fixture("known-directory");
  const text = "// the list\n// generator 1a\n\n#7 a bug\nplain a.rs\n\n#difference 2 decided\nstd b.rs\n";
  writeKnown("metamorphic", text, dir);
  expect(readdirSync(dir).sort()).toEqual(["7.txt", "README.txt", "difference-2.txt"]);
  expect(readFileSync(join(dir, "7.txt"), "utf8")).toBe("#7 a bug\nplain a.rs\n");
  expect(readFileSync(join(dir, "README.txt"), "utf8")).toBe("// the list\n// generator 1a\n");
  const read = knownText("metamorphic", dir);
  expect(read).toBe("// the list\n// generator 1a\n\n#7 a bug\nplain a.rs\n\n#difference 2 decided\nstd b.rs\n");
  writeKnown("metamorphic", read, dir);
  expect(knownText("metamorphic", dir)).toBe(read);
  writeKnown("metamorphic", "// the list\n\n#7 a bug\n\n#difference 2 decided\nstd b.rs\n", dir);
  expect(readdirSync(dir).sort()).toEqual(["README.txt", "difference-2.txt"]);
});

// A bless replaces a snapshot's generated files and nothing else of its
// folder: the playground's snapshot is the JS its app commits beside its
// Rust, `wasm/web/rust`, whose sources a bless that removed the whole folder
// would have deleted. A generated file it no longer writes goes.
test("a bless replaces a snapshot's generated files, keeping the folder's others", () => {
  const [actual, snapshot] = [fixture("bless-actual"), fixture("bless-snapshot")];
  writeFileSync(join(actual, "lib.js"), "new\n");
  writeFileSync(join(snapshot, "lib.js"), "old\n");
  writeFileSync(join(snapshot, "gone.jsx"), "stale\n");
  writeFileSync(join(snapshot, "lib.rs"), "pub fn f() {}\n");
  const was = process.env.BLESS;
  process.env.BLESS = "1";
  try {
    expectSnapshot(actual, snapshot);
  } finally {
    if (was === undefined) delete process.env.BLESS;
    else process.env.BLESS = was;
  }
  expect(readdirSync(snapshot).sort()).toEqual(["lib.js", "lib.rs"]);
  expect(readFileSync(join(snapshot, "lib.js"), "utf8")).toBe("new\n");
});

// The floor and the caps are the disk's, as a run's width is the cores'
// (#2505): a fixed 40 GiB floor was 18% of a 256 GB Mac's 228 GiB volume,
// so its stores kept nothing whenever it had under 40 free, and the native
// builds' 10 GiB and sccache's were the same on it as on a 1 TB one. A 256
// GB Mac keeps a working loop; a 1 TB one keeps more; none past the 40 GiB
// floor that was enough everywhere.
test("the disk's floor and its stores' caps are sized by the volume", () => {
  const GiB = 2 ** 30;
  const of = (gib: number) => {
    const { floor, nativeCache, sccache, budget } = diskProfile(gib * GiB);
    return [floor, nativeCache, sccache, budget].map((bytes) => Math.round((bytes / GiB) * 10) / 10);
  };
  // The 256 GB Macs, the 512 GB Mac mini, a 1 TB and a 2 TB one.
  expect(of(228)).toEqual([20, 9.1, 9.1, 39.1]);
  expect(of(460)).toEqual([36.8, 18.4, 18.4, 48.4]);
  expect(of(931)).toEqual([40, 37.2, 37.2, 67.2]);
  expect(of(1862)).toEqual([40, 74.5, 74.5, 104.5]);
  // A small disk keeps its least.
  expect(of(100)).toEqual([20, 8, 8, 38]);
  // This machine's, of the volume the checkout is on, is what the stores use.
  expect(disk).toEqual(diskProfile(volumeBytes(root)));
  expect(floorBytes).toBe(disk.floor);
  expect(keptStores.find((store) => store.dir === nativeCache)?.cap).toBe(disk.nativeCache);
  expect(budgetBytes).toBe(disk.budget);
});

// sccache's cap is written to its config, which its server reads as it
// starts: the one way to give it a cap of the disk's, its own default 10
// GiB on every disk. A person's own config is theirs; one this wrote is
// written again only where the cap moved; none where sccache isn't used.
test("sccache's cap is the disk's, written to its config but never over a person's own", () => {
  const dir = fixture("sccache-sized");
  const cache = join(dir, "cache");
  const config = join(dir, "config", "config");
  expect(sizeSccache(9 * 2 ** 30, cache, config)).toBe("unused");
  expect(existsSync(config)).toBe(false);
  mkdirSync(cache, { recursive: true });
  expect(sizeSccache(9 * 2 ** 30, cache, config)).toBe("written");
  expect(readFileSync(config, "utf8")).toBe(`${sccacheWritten}\n[cache.disk]\nsize = ${9 * 2 ** 30}\n`);
  expect(sizeSccache(9 * 2 ** 30, cache, config)).toBe("as it was");
  expect(sizeSccache(18 * 2 ** 30, cache, config)).toBe("written");
  expect(readFileSync(config, "utf8")).toContain(`size = ${18 * 2 ** 30}\n`);
  writeFileSync(config, "[cache.disk]\nsize = 1\n");
  expect(sizeSccache(18 * 2 ** 30, cache, config)).toBe("own");
  expect(readFileSync(config, "utf8")).toBe("[cache.disk]\nsize = 1\n");
  expect(sccacheConfigText(undefined, 1.5)).toEqual({ text: `${sccacheWritten}\n[cache.disk]\nsize = 1\n` });
});
