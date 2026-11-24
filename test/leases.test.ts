// What concurrent runs share of the machine (scripts/leases.ts): each run's
// share by the live leases, a lease of a process gone not counted; locks
// whose owner is gone taken; the job times written by two at once, each
// one's kept; and a check's pool held to its share beside another run's.
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { readJobTimes } from "../scripts/jobTimes";
import { acquire, fairShare, holdLease, leasedIn, sharers, turnLock, liveLeases, liveWaiters, lockState, nextWaiter, ownMemory, ownPart, ownShare, withLock } from "../scripts/leases";
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
  // Its cores and its memory one part of the machine, `ownPart`'s, so
  // neither is sized for what other runs hold (#1767).
  expect([ownPart({}, 2), ownShare(14, {}, 2), ownMemory(30, {}, 2)]).toEqual([0.5, 7, 15]);
  expect([ownPart({ RUST_JS_LEASED: "1" }, 3), ownShare(14, { RUST_JS_LEASED: "1" }, 3), ownMemory(30, { RUST_JS_LEASED: "1" }, 3)]).toEqual([1, 14, 30]);
  expect([ownShare(14, {}, 3), ownMemory(30, {}, 3)]).toEqual([4, 10]);
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

// One heavy run at a time where agents share a machine (Tu's rule,
// 2026-11-24): a lease is taken with the machine's turn, so a second run
// waits for the first to end; a lane's run starts only where none holds a
// lease and takes no turn, so a run started after it goes on beside it, the
// lane's share not counted against it; and what a run starts is in its
// lease, not another sharer.
test("one heavy run at a time: a second waits for the first's turn, a lane only for no run at all", async () => {
  const dir = fixture("leases-turn");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const store = join(dir, "leases");
  const marks = join(dir, "marks");
  mkdirSync(marks, { recursive: true });
  // A run: holds its lease, marks when it has it, and ends when told.
  const child = join(dir, "run.ts");
  writeFileSync(
    child,
    `import { existsSync, writeFileSync } from "node:fs";
import { holdLease } from ${JSON.stringify(join(root, "scripts/leases.ts"))};
const [name, store, marks] = process.argv.slice(2);
holdLease(name, { store, every: 50, say: () => writeFileSync(marks + "/" + name + ".waited", "") });
writeFileSync(marks + "/" + name + ".held", String(Date.now()));
while (!existsSync(marks + "/" + name + ".end")) Bun.sleepSync(20);
`,
  );
  const env = { ...process.env, RUST_JS_LEASED: "", RUST_JS_LANE: "" };
  const start = (name: string, lane = false) => Bun.spawn(["bun", child, name, store, marks], { cwd: root, stdout: "inherit", stderr: "inherit", env: { ...env, RUST_JS_LANE: lane ? "1" : "" } });
  const until = async (file: string) => {
    for (let i = 0; i < 400 && !existsSync(join(marks, file)); i++) await Bun.sleep(25);
    return existsSync(join(marks, file));
  };
  const end = (name: string) => writeFileSync(join(marks, `${name}.end`), "");

  const first = start("first");
  expect(await until("first.held")).toBe(true);
  const second = start("second");
  // The second waits, and says so, while the first holds the turn.
  expect(await until("second.waited")).toBe(true);
  await Bun.sleep(300);
  expect(existsSync(join(marks, "second.held"))).toBe(false);
  // A lane waits too, while any run holds a lease.
  const lane = start("lane", true);
  expect(await until("lane.waited")).toBe(true);
  end("first");
  expect(await first.exited).toBe(0);
  expect(await until("second.held")).toBe(true);
  await Bun.sleep(300);
  expect(existsSync(join(marks, "lane.held"))).toBe(false);
  end("second");
  expect(await second.exited).toBe(0);
  // With no run, the lane goes, taking no turn.
  expect(await until("lane.held")).toBe(true);
  expect(lockState(turnLock(store))).toBe("absent");
  // A run started beside the lane goes at once, and the lane isn't one of
  // its sharers, though it is the lane's.
  const beside = start("beside");
  expect(await until("beside.held")).toBe(true);
  expect(existsSync(join(marks, "beside.waited"))).toBe(false);
  const leases = liveLeases(store);
  expect(leases.map((lease) => [lease.kind, lease.lane ?? false]).sort()).toEqual([["beside", false], ["lane", true]]);
  const lanePid = leases.find((lease) => lease.lane)!.pid;
  const runPid = leases.find((lease) => !lease.lane)!.pid;
  expect(sharers({ RUST_JS_LEASED: String(runPid) }, store)).toBe(1);
  expect(sharers({ RUST_JS_LEASED: String(lanePid), RUST_JS_LANE: "1" }, store)).toBe(2);
  end("beside");
  end("lane");
  expect([await beside.exited, await lane.exited]).toEqual([0, 0]);
  // Each let go of what it held.
  expect(liveLeases(store)).toEqual([]);
  expect(lockState(turnLock(store))).toBe("absent");
}, underLoad(60_000));

/** A run of `kind` named `name` in a fixture's `store`: holds its lease,
 * marks when it has it, and does `batches` batches, each on its `go<i>`
 * mark, as `form` runs them: `direct`, `yieldToPush` after each; `pool`,
 * a mutation run's pool (`runYielding`); `names`, a retirement's names in
 * turn (`yieldBetween`). Of none, it ends when told. */
function turnFixture(name: string) {
  const dir = fixture(name);
  rmSync(dir, { recursive: true, force: true });
  const store = join(dir, "leases");
  const marks = join(dir, "marks");
  mkdirSync(marks, { recursive: true });
  const child = join(dir, "run.ts");
  writeFileSync(
    child,
    `import { existsSync, writeFileSync } from "node:fs";
import { holdLease, runYielding, yieldBetween, yieldToPush } from ${JSON.stringify(join(root, "scripts/leases.ts"))};
const [name, kind, store, marks, batches, form] = process.argv.slice(2);
holdLease(kind, { store, every: 50, say: () => writeFileSync(marks + "/" + name + ".waited", "") });
writeFileSync(marks + "/" + name + ".held", String(Date.now()));
const batch = async (i: number) => {
  while (!existsSync(marks + "/" + name + ".go" + i)) await Bun.sleep(20);
  writeFileSync(marks + "/" + name + ".batch" + i, String(Date.now()));
};
const items = Array.from({ length: Number(batches) }, (_, i) => i + 1);
if (form === "pool") await runYielding([...items], 1, batch, { poll: 20 });
else if (form === "names") for (const [at, i] of items.entries()) {
  await yieldBetween(at, async () => {});
  await batch(i);
}
else for (const i of items) {
  await batch(i);
  yieldToPush();
}
while (items.length === 0 && !existsSync(marks + "/" + name + ".end")) Bun.sleepSync(20);
`,
  );
  const env = { ...process.env, RUST_JS_LEASED: "", RUST_JS_LANE: "" };
  const start = (name: string, kind: string, batches = 0, form = "direct") => Bun.spawn(["bun", child, name, kind, store, marks, String(batches), form], { cwd: root, stdout: "inherit", stderr: "inherit", env });
  const until = async (file: string) => {
    for (let i = 0; i < 400 && !existsSync(join(marks, file)); i++) await Bun.sleep(25);
    return existsSync(join(marks, file));
  };
  const mark = (file: string) => writeFileSync(join(marks, file), "");
  const at = (file: string) => Number(readFileSync(join(marks, file), "utf8"));
  return { store, start, until, mark, at, has: (file: string) => existsSync(join(marks, file)) };
}

// The turn has an order (#2516): a push's fix of a red `main` waited 34
// minutes behind a retirement, then behind the next, which polled first as
// the turn was let go. A push goes before any other waiter, whenever it
// began to wait; the rest go longest waiting first.
test("a push waiting goes before a run that waited longer, and the rest by how long each waited", async () => {
  const { store, start, until, mark, at, has } = turnFixture("leases-order");
  const first = start("first", "mutations");
  expect(await until("first.held")).toBe(true);
  const older = start("older", "mutations");
  expect(await until("older.waited")).toBe(true);
  const push = start("push", "push");
  expect(await until("push.waited")).toBe(true);
  mark("first.end");
  expect(await first.exited).toBe(0);
  expect(await until("push.held")).toBe(true);
  await Bun.sleep(300);
  expect(has("older.held")).toBe(false);
  mark("push.end");
  expect(await push.exited).toBe(0);
  expect(await until("older.held")).toBe(true);
  expect(at("older.held")).toBeGreaterThan(at("push.held"));
  mark("older.end");
  expect(await older.exited).toBe(0);
  expect(liveWaiters(store)).toEqual([]);
  expect(nextWaiter([{ pid: 3, kind: "check", since: 10 }, { pid: 2, kind: "mutations", since: 5 }])?.pid).toBe(2);
  expect(nextWaiter([{ pid: 3, kind: "check", since: 10 }, { pid: 2, kind: "push", since: 50 }])?.pid).toBe(2);
  expect(nextWaiter([])).toBeUndefined();
}, underLoad(60_000));

// A long run lets the turn go between its batches where a push waits, and
// waits for it again, next after the push (#2516): never mid-batch, so no
// work is lost, and none of it runs beside the push. So does a mutation
// run's pool, its batches running ending first, and a retirement between
// its names.
for (const form of ["direct", "pool", "names"]) {
  test(`a run of three batches lets a push have the turn between two, and finishes its third after it (${form})`, async () => {
    const { store, start, until, mark, at, has } = turnFixture(`leases-yield-${form}`);
    const run = start("run", "mutations", 3, form);
    expect(await until("run.held")).toBe(true);
    mark("run.go1");
    expect(await until("run.batch1")).toBe(true);
    // Its second batch under way, a push begins to wait: it waits for it.
    const push = start("push", "push");
    expect(await until("push.waited")).toBe(true);
    await Bun.sleep(300);
    expect(has("push.held")).toBe(false);
    // The second batch done, the run lets the push go; the third waits.
    mark("run.go2");
    expect(await until("run.batch2")).toBe(true);
    expect(await until("push.held")).toBe(true);
    expect(liveLeases(store).map((lease) => lease.kind)).toEqual(["push"]);
    mark("run.go3");
    await Bun.sleep(300);
    expect(has("run.batch3")).toBe(false);
    mark("push.end");
    expect(await push.exited).toBe(0);
    expect(await until("run.batch3")).toBe(true);
    expect(at("run.batch3")).toBeGreaterThan(at("push.held"));
    expect(await run.exited).toBe(0);
    expect(liveLeases(store)).toEqual([]);
    expect(lockState(turnLock(store))).toBe("absent");
  }, underLoad(60_000));
}

// The mutation runner's pool and its retirement go through these, so what
// they hold holds of them.
test("a mutation run's pool and a retirement's names let the turn go through leases' own", async () => {
  // The runner loaded, as a mutant of it is judged by a test that runs it.
  expect(typeof (await import("../scripts/mutations")).batches).toBe("function");
  const runner = readFileSync(join(root, "scripts/mutations.ts"), "utf8");
  expect(runner).toContain("  await runYielding(queue, jobs, one, { beside: () => Promise.all(buildsTesting) });");
  expect(runner).toContain("    await yieldBetween(at, () => judged);");
});
