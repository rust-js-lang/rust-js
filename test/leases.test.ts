// What concurrent runs share of the machine (scripts/leases.ts): each run's
// share by the live leases, a lease of a process gone not counted; locks
// whose owner is gone taken; the job times written by two at once, each
// one's kept; and a check's pool held to its share beside another run's.
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { readJobTimes } from "../scripts/jobTimes";
import { acquire, fairShare, holdLease, leasedIn, liveLeases, lockState, ownShare, withLock } from "../scripts/leases";
import { mutantsLock, mutantsStore } from "../scripts/disk";
import { underLoad } from "../scripts/timing";
import { fixture, root } from "./support";

/** A process that lives until killed, to hold a lease in a test. */
function living(): { pid: number; kill: () => void } {
  const p = Bun.spawn(["sleep", "120"]);
  return { pid: p.pid, kill: () => p.kill("SIGKILL") };
}

/** A pid no process has: one that lived, and ended. */
async function ended(): Promise<number> {
  const p = Bun.spawn(["true"]);
  await p.exited;
  return p.pid;
}

test("a run's share is the machine divided by the live leases, and a lease of a process gone counts for none and goes", async () => {
  const store = join(fixture("leases-share"), "leases");
  rmSync(store, { recursive: true, force: true });
  mkdirSync(store, { recursive: true });
  const other = living();
  try {
    writeFileSync(join(store, `${other.pid}.json`), JSON.stringify({ pid: other.pid, checkout: "/elsewhere", kind: "check" }));
    const gone = await ended();
    writeFileSync(join(store, `${gone}.json`), JSON.stringify({ pid: gone, checkout: "/gone", kind: "check" }));
    writeFileSync(join(store, `${process.pid}.json`), JSON.stringify({ pid: process.pid, checkout: root, kind: "check" }));
    const live = liveLeases(store);
    expect(live.map((lease) => lease.pid).sort()).toEqual([other.pid, process.pid].sort());
    expect(existsSync(join(store, `${gone}.json`))).toBe(false);
    expect(fairShare(14, live.length)).toBe(7);
    expect(fairShare(14, 0)).toBe(14);
    expect([leasedIn("/elsewhere", store), leasedIn("/gone", store)]).toEqual([true, false]);
  } finally {
    other.kill();
  }
  // What a pool started takes the whole it's given, the pool's share
  // holding it; and holds no lease of its own.
  expect(ownShare(14, { RUST_JS_LEASED: "1" })).toBe(14);
  const none = join(fixture("leases-held"), "leases");
  rmSync(none, { recursive: true, force: true });
  holdLease("check", { store: none, env: { RUST_JS_LEASED: "1" } });
  expect(existsSync(none)).toBe(false);
});

test("a lock is one holder's at a time, and one whose owner is gone, or never named itself, is taken", async () => {
  const dir = fixture("leases-locks");
  const lock = join(dir, "lock");
  rmSync(lock, { recursive: true, force: true });
  // Held: another waits, and past its wait gives up.
  const release = acquire(lock);
  expect(release).toBeDefined();
  expect(acquire(lock, { wait: 100 })).toBeUndefined();
  // And says it waits as it starts to and again each `every`, whose process
  // holds it: its wait sleeps the process, so no timer of the run's says
  // anything meanwhile (#2418).
  const said: string[] = [];
  expect(acquire(lock, { wait: 200, waiting: "waiting for the lock", every: 40, say: (line) => said.push(line) })).toBeUndefined();
  expect(said[0]).toBe("waiting for the lock");
  expect(said[1]).toBe(`waiting for the lock: 0 min so far, held by process ${process.pid}`);
  release!();
  expect(withLock(lock, () => "ran")).toBe("ran");
  expect(existsSync(lock)).toBe(false);
  // Its owner gone: taken at once.
  mkdirSync(lock);
  writeFileSync(join(lock, "owner"), String(await ended()));
  expect(withLock(lock, () => "taken", { wait: 100 })).toBe("taken");
  // Made 6 s ago and no owner named: its maker gone between.
  mkdirSync(lock);
  const past = (Date.now() - 6_000) / 1000;
  utimesSync(lock, past, past);
  expect(withLock(lock, () => "taken", { wait: 100 })).toBe("taken");
});

// Two checks ending together each read the job times, added their own and
// wrote what they had: the first's were lost. Under the store's lock, each
// writer's runs are kept.
test("the job times two runs record at once keep each one's", async () => {
  const store = join(fixture("leases-job-times"), "times");
  rmSync(store, { recursive: true, force: true });
  const writer = (who: string) =>
    Bun.spawn(
      [
        "bun",
        "-e",
        `import { recordJobTimes } from ${JSON.stringify(join(root, "scripts/jobTimes.ts"))};
for (let i = 0; i < 40; i++) recordJobTimes([{ name: "${who} " + i, seconds: 1, cores: 1 }], { store: ${JSON.stringify(store)}, free: () => Infinity });`,
      ],
      { stdout: "ignore", stderr: "inherit" },
    );
  const [a, b] = [writer("a"), writer("b")];
  expect([await a.exited, await b.exited]).toEqual([0, 0]);
  const names = Object.keys(readJobTimes(store));
  expect(names.filter((name) => name.startsWith("a ")).length).toBe(40);
  expect(names.filter((name) => name.startsWith("b ")).length).toBe(40);
}, underLoad(60_000));

// Beside another run's live lease, a pool of 4 cores runs jobs of a core
// each two at a time, not four.
test("check:fast's pool keeps to its share of the cores beside another run's lease", async () => {
  const dir = fixture("leases-pool");
  const leases = join(dir, "leases");
  const marks = join(dir, "marks");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(leases, { recursive: true });
  mkdirSync(marks, { recursive: true });
  const other = living();
  try {
    writeFileSync(join(leases, `${other.pid}.json`), JSON.stringify({ pid: other.pid, checkout: "/elsewhere", kind: "check" }));
    const child = join(dir, "pool.ts");
    const mark = (i: number) => ["sh", "-c", `date +%s%N > ${join(marks, `${i}.start`)}; sleep 0.6; date +%s%N > ${join(marks, `${i}.end`)}`];
    writeFileSync(
      child,
      `import { run } from ${JSON.stringify(join(root, "scripts/checkFast.ts"))};
import { checkLogRun } from ${JSON.stringify(join(root, "scripts/checkLogs.ts"))};
const job = (name: string, cmd: string[]) => ({ name, cmd, env: {}, timeout: 120, weight: 1, memory: 0, size: 0 });
await run(${JSON.stringify([0, 1, 2, 3])}.map((i) => job("a core " + i, ${JSON.stringify([0, 1, 2, 3].map(mark))}[i])), "unused", 4, { began: Date.now(), samples: [] }, checkLogRun({ store: ${JSON.stringify(join(dir, "logs"))} }), ${JSON.stringify(join(dir, "times"))}, ${JSON.stringify(leases)});
`,
    );
    const p = Bun.spawn(["bun", child], { cwd: root, stdout: "ignore", stderr: "inherit", env: { ...process.env, RUST_JS_LEASED: "" } });
    expect(await p.exited).toBe(0);
  } finally {
    other.kill();
  }
  const spans = [0, 1, 2, 3].map((i) => ["start", "end"].map((end) => Number(readFileSync(join(marks, `${i}.${end}`), "utf8"))));
  const most = Math.max(...spans.map(([start]) => spans.filter(([s, e]) => s <= start && start < e).length));
  expect(most).toBe(2);
}, underLoad(60_000));

// A lock is held while its owner lives, or for 5 s past its making before
// it names one; `acquire` waits on it, and the pruner leaves a mutation
// run's store alone by it (#2272).
test("a lock is held while its owner lives, and a run's mutants store isn't pruned while its lock is", () => {
  const tree = fixture("leases-lock-held");
  const lock = mutantsLock(tree);
  rmSync(lock, { recursive: true, force: true });
  expect(lockState(lock)).toBe("absent");
  const free = mutantsStore(tree, "mutants", true);
  expect(free.bound).toBeDefined();
  expect(free.kept).toBeDefined();
  const release = acquire(lock)!;
  expect(lockState(lock)).toBe("held");
  // Its run's snapshot, which a check:fast's pruning evicted from under a
  // retirement: neither bounded nor offered to `holdToBudget` now.
  const running = mutantsStore(tree, "mutants", true);
  expect(running.bound).toBeUndefined();
  expect(running.kept).toBeUndefined();
  release();
  expect(lockState(lock)).toBe("absent");
  // One whose owner is gone isn't held; one made just now naming none is.
  mkdirSync(lock, { recursive: true });
  expect(lockState(lock)).toBe("held");
  writeFileSync(join(lock, "owner"), "999999999");
  expect(lockState(lock)).toBe("stale");
  rmSync(lock, { recursive: true, force: true });
  // Another checkout's, leased, stays unbounded as before.
  expect(mutantsStore(tree, "another checkout's mutants", false).bound).toBeUndefined();
});

// A lock gone between `mkdir` refusing it and its state asked, its holder
// done, is made again, never removed: in that instant another run may
// make it, and removing that one let both hold it, two job-time writers
// each writing what it read (#2283). Here the state is asked as the lock
// is absent, and another live process makes it right then: it's left
// as it is, and waited on.
test("a lock found absent is made again, never removing what another run made then", () => {
  const dir = fixture("leases-lock-absent");
  const lock = join(dir, "lock");
  rmSync(lock, { recursive: true, force: true });
  mkdirSync(lock, { recursive: true });
  let asked = 0;
  const released = acquire(lock, { wait: 100 }, (at) => {
    asked++;
    if (asked === 1) {
      // Its holder done, and another run, alive, takes it at once.
      rmSync(at, { recursive: true, force: true });
      const state = lockState(at);
      mkdirSync(at);
      writeFileSync(join(at, "owner"), String(process.ppid));
      return state;
    }
    return lockState(at);
  });
  expect(released).toBeUndefined();
  expect(readFileSync(join(lock, "owner"), "utf8")).toBe(String(process.ppid));
  expect(lockState(join(dir, "never-made"))).toBe("absent");
  rmSync(lock, { recursive: true, force: true });
});
