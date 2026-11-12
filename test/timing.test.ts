// The loop's timings (scripts/timing.ts): how a step's time is said
// against its usual, and when it's slower.
import { expect, test } from "bun:test";
import { mkdirSync, realpathSync } from "node:fs";
import { join } from "node:path";

import { expected, keptJobs, keptRuns, readJobTimes, recordJobTimes } from "../scripts/jobTimes";
import { elapsedSeconds, said, strays, worktreeRoots, type Stray } from "../scripts/strays";
import { compared, median, shown, slower, type Timing } from "../scripts/timing";
import { floorBytes } from "./bounded";
import { fixture, root } from "./support";

const runs = (step: string, seconds: number[]): Timing[] =>
  seconds.map((s, i) => ({ step, seconds: s, at: `2026-10-11T00:0${i}:00Z`, commit: "abc" }));

test("a time reads as a person says it", () => {
  expect([shown(0.85), shown(4.24), shown(38.4), shown(200), shown(120)]).toEqual(["850 ms", "4.2 s", "38 s", "3 min 20 s", "2 min"]);
});

test("a step's usual is the middle of its last runs", () => {
  expect([median([3, 1, 2]), median([4, 1, 3, 2])]).toEqual([2, 2.5]);
});

test("slower is a quarter and five seconds over the usual, not noise", () => {
  expect([slower(46, 36), slower(40, 36), slower(6, 4), slower(30, 20)]).toEqual([true, false, false, true]);
});

test("a time is said against the step's last ten runs, once there are three", () => {
  expect(compared("check:fast", 38, runs("check:fast", [36, 37]))).toBe("check:fast 38 s");
  expect(compared("check:fast", 38, runs("check:fast", [36, 37, 35]))).toBe("check:fast 38 s (usual 36 s)");
  expect(compared("check:fast", 52, runs("check:fast", [36, 37, 35]))).toBe("check:fast 52 s (usual 36 s)  ⚠ slower than usual: why?");
  // Another step's runs aren't its usual.
  expect(compared("port build", 52, runs("check:fast", [36, 37, 35]))).toBe("port build 52 s");
});

// What the loop left running, parented by launchd and working in a
// worktree's `target/`, is named by `check:fast` and `bun run task start`:
// one held a core for 28 hours, unseen, every timing beside it (2026-11-09).
// A shell's background `sleep`, its shell gone, is one; `ps`'s time read
// as `[[dd-]hh:]mm:ss`.
test("a process the loop left running in a target is named, with how to stop it", async () => {
  expect([elapsedSeconds("05"), elapsedSeconds("01:02"), elapsedSeconds("03:00:01"), elapsedSeconds("1-04:31:12")]).toEqual([5, 62, 10_801, 102_672]);
  const dir = fixture("stray");
  const shell = Bun.spawnSync(["sh", "-c", "sleep 30 > /dev/null 2>&1 & echo $!"], { cwd: dir });
  const pid = Number(shell.stdout.toString().trim());
  try {
    let found: Stray[] = [];
    for (let i = 0; i < 50 && !found.some((s) => s.pid === pid); i++) {
      found = strays(root, 0);
      if (!found.some((s) => s.pid === pid)) await Bun.sleep(100);
    }
    const stray = found.find((s) => s.pid === pid);
    expect(stray?.cwd).toBe(realpathSync(dir));
    expect(said([stray!])).toContain(`kill ${pid}`);
    // An hour on, it's one; a minute in, it's not yet.
    expect(strays(root).some((s) => s.pid === pid)).toBe(false);
  } finally {
    process.kill(pid);
  }
  expect(said([])).toBeUndefined();
});

// A worktree beside the main checkout, `git worktree add ../x`, is one of
// the repository's too: what the loop left in its `target/` is named from
// any of them (#1449), where only the main checkout's were searched.
test("a stray in a worktree outside the main checkout is named", async () => {
  const dir = realpathSync(fixture("strays-repo"));
  const [main, outside] = [join(dir, "main"), join(dir, "outside")];
  mkdirSync(main);
  const git = (cwd: string, ...args: string[]) => Bun.spawnSync(["git", ...args], { cwd, stdout: "pipe", stderr: "pipe" });
  git(main, "init", "-q");
  git(main, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "start");
  expect(git(main, "worktree", "add", "-q", "--detach", outside).exitCode).toBe(0);
  expect(worktreeRoots(main)).toEqual([main, outside]);
  mkdirSync(join(outside, "target"));
  const shell = Bun.spawnSync(["sh", "-c", "sleep 30 > /dev/null 2>&1 & echo $!"], { cwd: join(outside, "target") });
  const pid = Number(shell.stdout.toString().trim());
  try {
    let found: Stray[] = [];
    for (let i = 0; i < 50 && !found.some((s) => s.pid === pid); i++) {
      found = strays(main, 0);
      if (!found.some((s) => s.pid === pid)) await Bun.sleep(100);
    }
    expect(found.find((s) => s.pid === pid)?.cwd).toBe(join(outside, "target"));
  } finally {
    process.kill(pid);
  }
});

// check:fast's pool starts the longest job first by each job's last runs on
// the machine, in any of its checkouts (#1364): a new worktree's own
// `target/` has none. Each job's last five runs are kept, of at most a
// thousand jobs, the least recently run dropped first, and nothing is
// written below the disk's floor.
test("the machine keeps each job's last runs, bounded, for every checkout's pool", () => {
  const store = fixture("job-times");
  const roomy = () => floorBytes + 2 ** 30;
  for (let i = 1; i <= keptRuns + 2; i++) recordJobTimes([{ name: "corpus 1/12", seconds: i, cores: 1 }], { store, now: i, free: roomy });
  expect(readJobTimes(store)["corpus 1/12"]).toEqual({ seconds: [3, 4, 5, 6, 7], cores: [1, 1, 1, 1, 1], at: keptRuns + 2 });
  const many = Array.from({ length: keptJobs + 5 }, (_, i) => ({ name: `test/f${i}.test.ts`, seconds: 1, cores: 1 }));
  recordJobTimes(many, { store, now: 100, free: roomy });
  const kept = readJobTimes(store);
  expect(Object.keys(kept).length).toBe(keptJobs);
  expect(kept["corpus 1/12"]).toBeUndefined();
  recordJobTimes([{ name: "below the floor", seconds: 1, cores: 1 }], { store, now: 200, free: () => floorBytes - 1 });
  expect(readJobTimes(store)["below the floor"]).toBeUndefined();
});

// A job whose work the change sizes, the rows of the iterator tables it
// names, is expected to take its size at its last runs' pace, not its last
// time (#1364): 0.1 s after a change that named none, the rows of the next
// one started last and ran alone for two minutes. An unsized job's is its
// last times'; one not yet measured, or of no run of some size, starts first.
test("a job the change sizes is expected to take its size at its runs' pace", () => {
  const store = fixture("job-times-sized");
  const roomy = () => floorBytes + 2 ** 30;
  const rows = "the rows the change names";
  recordJobTimes([{ name: rows, seconds: 0.1, cores: 1 }], { store, now: 1, free: roomy });
  expect(expected(readJobTimes(store)[rows], 9000).longest).toBe(Number.POSITIVE_INFINITY);
  recordJobTimes([{ name: rows, seconds: 0.1, cores: 1, size: 0 }], { store, now: 2, free: roomy });
  recordJobTimes([{ name: rows, seconds: 30, cores: 5, size: 3000 }], { store, now: 3, free: roomy });
  recordJobTimes([{ name: rows, seconds: 12, cores: 5, size: 600 }], { store, now: 4, free: roomy });
  const kept = readJobTimes(store)[rows];
  expect(kept.sizes).toEqual([null, 0, 3000, 600]);
  expect(expected(kept, 9000).longest).toBeCloseTo(180);
  expect(expected(kept, 9000).usual).toBeCloseTo(135);
  expect(expected(kept, 0)).toEqual({ longest: 0, usual: 0 });
  // Of none, it's nothing, measured or not: not started first.
  expect(expected(undefined, 0)).toEqual({ longest: 0, usual: 0 });
  expect(expected(kept)).toEqual({ longest: 30, usual: 6.05 });
  expect(expected(undefined).longest).toBe(Number.POSITIVE_INFINITY);
  recordJobTimes([{ name: "fmt", seconds: 8, cores: 1 }], { store, now: 5, free: roomy });
  expect(readJobTimes(store).fmt.sizes).toBeUndefined();
});
