// What concurrent runs share of the machine (scripts/leases.ts): each run's
// share by the live leases, a lease of a process gone not counted; locks
// whose owner is gone taken; the job times written by two at once, each
// one's kept; and a check's pool held to its share beside another run's.
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { readJobTimes } from "../scripts/jobTimes";
import { acquire, fairShare, holdLease, leasedIn, liveLeases, ownShare, withLock } from "../scripts/leases";
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
