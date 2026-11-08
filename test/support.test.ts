// The tests' own helpers (ADR 0104): a native program kept for the next
// run is the program its sources make now, and the compiler is built once
// for a run, with no one left waiting on no one.

import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, symlinkSync, utimesSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";

import { bytesOf, floorBytes } from "./bounded";
import { keptStores, pruneKept } from "./kept";
import { generatedTexts, record, runKey, verified } from "./verified-runs";
import { runNative } from "./programs";
import { fixture, nativeBinary, once, root, run } from "./support";

// Found in review: a file the case includes was changed, and the kept
// binary, of the file before, answered for it.
test("a kept native program is built again when a file it includes changes", () => {
  const dir = fixture("native-transitive");
  writeFileSync(join(dir, "cases.rs"), 'pub fn value() -> u32 {\n    include!("value.rs")\n}\n');
  const answer = () => {
    const built = nativeBinary('include!("cases.rs");\nfn main() {\n    println!("{}", value());\n}\n', dir, ["--edition=2024"]);
    if ("error" in built) throw new Error(built.error);
    return run([built.binary]).trim();
  };
  writeFileSync(join(dir, "value.rs"), "1\n");
  expect(answer()).toBe("1");
  writeFileSync(join(dir, "value.rs"), "2\n");
  expect(answer()).toBe("2");
});

// Found in review: the case was copied away from the files beside it.
test("a native case reads the files beside it, as the JS one does", () => {
  const dir = fixture("native-beside");
  const source = join(dir, "case.rs");
  writeFileSync(source, 'fn main() {\n    println!("{} {}", include!("value.rs"), include_str!("value.txt").trim());\n}\n');
  writeFileSync(join(dir, "value.rs"), "7\n");
  writeFileSync(join(dir, "value.txt"), "seven\n");
  const native = runNative(source, fixture("native-beside-run"));
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
  expect(pruneKept(cache, { now, cap: 2 * each, free: () => Number.POSITIVE_INFINITY })).toEqual({ removed: 2, size: 2 * each });
  expect([oldest, older, newer, newest].map(existsSync)).toEqual([false, false, true, true]);
  expect(existsSync(join(cache, "q"))).toBe(false);
  // Under the floor: pruned until it's back, here once one more is gone.
  expect(pruneKept(cache, { now, cap: 2 * each, free: () => (existsSync(newer) ? floorBytes - 1 : floorBytes) })).toEqual({ removed: 1, size: each });
  expect([newer, newest].map(existsSync)).toEqual([false, true]);
  // Each store's cap.
  expect(keptStores.map(({ dir, cap }) => [basename(dir), cap / 2 ** 20])).toEqual([
    ["native-cache", 3072],
    ["native-sources", 64],
    ["corpus-cache", 256],
  ]);
});

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

// A corpus case's runs are kept by what they ran (ADR 0567): a key of each
// input whole, so no two sets of inputs share one; verified once recorded,
// in the cache given, and marked used then.
test("a verified run's key is of each input whole, and found once recorded", () => {
  expect(runKey(["ab", "c"])).not.toBe(runKey(["a", "bc"]));
  expect(runKey(["a", "b"])).not.toBe(runKey(["b", "a"]));
  expect(runKey(["a", "b"])).toBe(runKey(["a", Buffer.from("b")]));
  const cache = fixture("verified-runs");
  expect(verified("k", cache)).toBe(false);
  record("k", "case.rs", cache);
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
