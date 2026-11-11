// The loop's timings (scripts/timing.ts): how a step's time is said
// against its usual, and when it's slower.
import { expect, test } from "bun:test";
import { realpathSync } from "node:fs";

import { keptJobs, keptRuns, readJobTimes, recordJobTimes } from "../scripts/jobTimes";
import { elapsedSeconds, said, strays, type Stray } from "../scripts/strays";
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
