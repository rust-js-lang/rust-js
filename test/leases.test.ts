// What concurrent runs share of the machine (scripts/leases.ts): each run's
// share by the live leases, a lease of a process gone not counted; locks
// whose owner is gone taken; the job times written by two at once, each
// one's kept; and a check's pool held to its share beside another run's.
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { readJobTimes } from "../scripts/jobTimes";
import { acquire, fairShare, holdLease, leasedIn, sharers, turnLock, liveLeases, liveWaiters, lockState, nextWaiter, ownMemory, ownPart, ownShare, runLimit, withLock } from "../scripts/leases";
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
  expect(holdLease("check", { store: none, env: { RUST_JS_LEASED: "1" } })).toBeUndefined();
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
  // anything meanwhile (#2418). Of a clock of its own, whose time is its
  // sleeps': the machine's, loaded, overslept the whole wait in one sleep,
  // and said no tick (#2603).
  const said: string[] = [];
  let time = 0;
  const slept: number[] = [];
  const clock = { now: () => time, sleep: (ms: number) => { slept.push(ms); time += ms; } };
  expect(acquire(lock, { wait: 200, waiting: "waiting for the lock", every: 40, say: (line) => said.push(line), clock })).toBeUndefined();
  expect(said).toEqual(["waiting for the lock", ...Array(4).fill(`waiting for the lock: 0 min so far, held by process ${process.pid}`)]);
  expect(slept.length).toBe(9);
  expect(time).toBe(225);
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

// A lock's release lets go what this process holds, once: a holder's
// release ran twice, as its signal's handler and again as it exited, and
// the second removed the lock the next holder had just taken, so two runs
// held the machine's turn at once (#2613).
test("a lock's release lets it go once, and only while it names this process", () => {
  const lock = join(fixture("leases-release"), "lock");
  rmSync(lock, { recursive: true, force: true });
  // Another's since: left as it is.
  const first = acquire(lock)!;
  writeFileSync(join(lock, "owner"), String(process.ppid));
  first();
  expect(readFileSync(join(lock, "owner"), "utf8")).toBe(String(process.ppid));
  rmSync(lock, { recursive: true, force: true });
  // Called again once this process has taken it anew, as a run that let
  // the turn go to a push takes it again: spent, it removes nothing.
  const used = acquire(lock)!;
  used();
  expect(lockState(lock)).toBe("absent");
  const again = acquire(lock)!;
  used();
  expect(lockState(lock)).toBe("held");
  again();
  expect(lockState(lock)).toBe("absent");
});

// Two waiters find the lock let go, or its holder gone, in one instant:
// one takes it, and the other waits on what the first made, never removing
// it. Two that found a stale turn at once each removed it, the second what
// the first had made of it meanwhile, and two runs held the machine's turn
// in one second (#2613). Here the other's whole take runs as the first asks
// what the lock is.
test("of two waiters that find a lock let go or stale in one instant, one takes it and the other never removes it", async () => {
  const lock = join(fixture("leases-lock-race"), "lock");
  for (const found of ["let go", "stale"]) {
    rmSync(lock, { recursive: true, force: true });
    const holder = living();
    try {
      mkdirSync(lock, { recursive: true });
      writeFileSync(join(lock, "owner"), String(found === "stale" ? await ended() : holder.pid));
      let other: (() => void) | undefined;
      const taken = acquire(lock, { wait: 100 }, (at) => {
        if (found === "let go" && other === undefined) rmSync(at, { recursive: true, force: true });
        const state = lockState(at);
        other ??= acquire(at, { wait: 100 });
        return state;
      });
      expect([found, other !== undefined, taken]).toEqual([found, true, undefined]);
      expect(readFileSync(join(lock, "owner"), "utf8")).toBe(String(process.pid));
      other!();
      expect(lockState(lock)).toBe("absent");
    } finally {
      holder.kill();
    }
  }
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
 * turn (`yieldBetween`); `check`, check:fast's pool, each batch a job of
 * its own, a process, on one core (#2585). Of none, it ends when told. */
function turnFixture(name: string) {
  const dir = fixture(name);
  rmSync(dir, { recursive: true, force: true });
  const store = join(dir, "leases");
  const marks = join(dir, "marks");
  mkdirSync(marks, { recursive: true });
  const child = join(dir, "run.ts");
  // A check's job, a process: its batch, as `batch` below.
  const job = join(dir, "job.ts");
  writeFileSync(
    job,
    `import { existsSync, writeFileSync } from "node:fs";
const [name, marks, i] = process.argv.slice(2);
writeFileSync(marks + "/" + name + ".started" + i, String(Date.now()));
while (!existsSync(marks + "/" + name + ".go" + i)) await Bun.sleep(20);
writeFileSync(marks + "/" + name + ".batch" + i, String(Date.now()));
`,
  );
  writeFileSync(
    child,
    `import { existsSync, writeFileSync } from "node:fs";
import { holdLease, runYielding, yieldBetween, yieldToPush } from ${JSON.stringify(join(root, "scripts/leases.ts"))};
const [name, kind, store, marks, batches, form] = process.argv.slice(2);
holdLease(kind, { store, every: 50, say: () => writeFileSync(marks + "/" + name + ".waited", "") });
writeFileSync(marks + "/" + name + ".held", String(Date.now()));
const batch = async (i: number) => {
  writeFileSync(marks + "/" + name + ".started" + i, String(Date.now()));
  while (!existsSync(marks + "/" + name + ".go" + i)) await Bun.sleep(20);
  writeFileSync(marks + "/" + name + ".batch" + i, String(Date.now()));
};
const items = Array.from({ length: Number(batches) }, (_, i) => i + 1);
if (form === "pool") await runYielding([...items], 1, batch, { poll: 20 });
else if (form === "check") {
  const { run } = await import(${JSON.stringify(join(root, "scripts/checkFast.ts"))});
  const { checkLogRun } = await import(${JSON.stringify(join(root, "scripts/checkLogs.ts"))});
  const jobs = items.map((i) => ({ name: "job " + i, cmd: [process.execPath, ${JSON.stringify(job)}, name, marks, String(i)], env: {}, timeout: 600, weight: 1, memory: 0 }));
  await run(jobs, "unused", 1, { began: Date.now(), samples: [] }, checkLogRun({ store: marks + "/logs" }), marks + "/times", store);
}
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
// its names. And check:fast's pool, between its jobs: three pushes waited
// about 1,600 s each behind a whole check (#2585).
for (const form of ["direct", "pool", "names", "check"]) {
  test(`a run of three batches lets a push have the turn between two, and finishes its third after it (${form})`, async () => {
    const { store, start, until, mark, at, has } = turnFixture(`leases-yield-${form}`);
    const run = start("run", "mutations", 3, form);
    expect(await until("run.held")).toBe(true);
    mark("run.go1");
    expect(await until("run.batch1")).toBe(true);
    // Its second batch under way, a push begins to wait: it waits for it.
    // Under way as the run says it began it, past where the run lets the
    // turn go: a push that began waiting before that point, its own start
    // quicker than the run's step past its first batch, had the turn then,
    // in the pool's form under a whole `check:fast --all`.
    expect(await until("run.started2")).toBe(true);
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
  // And a sweep of mutation-reach.ts' between its cases: one of 36
  // survivors held the turn some 50 min, a push waiting behind it (#2585).
  // Loaded, as a mutant of it is judged by a test that runs it.
  expect(typeof (await import("../scripts/mutation-reach")).said).toBe("function");
  expect(readFileSync(join(root, "scripts/mutation-reach.ts"), "utf8")).toContain("  await runYielding([...cases], Math.max(1, cores), one);");
});

/** A run of `kind` named `name` in a fixture's `store`, given `limit`
 * seconds of its run (none of 0): holds its lease, marks when it has it,
 * then as `mode` says: `end`, ends when told; `spawn`, starts a process
 * that sleeps, marks its pid, and never ends; `yield`, lets the turn go to a
 * waiting push on its `go` mark, marks when it has it again, and ends when
 * told; `line`, writes its line to the machine's log, `log`, of a step `x`
 * of 1 s, and ends. A run's limit (#2517). */
function limitFixture(name: string) {
  const dir = fixture(name);
  rmSync(dir, { recursive: true, force: true });
  const store = join(dir, "leases");
  const marks = join(dir, "marks");
  const log = join(dir, "times.log");
  mkdirSync(marks, { recursive: true });
  const child = join(dir, "run.ts");
  writeFileSync(
    child,
    `import { existsSync, writeFileSync } from "node:fs";
import { holdLease, yieldToPush } from ${JSON.stringify(join(root, "scripts/leases.ts"))};
import { devTimes } from ${JSON.stringify(join(root, "scripts/devTimes.ts"))};
const [name, kind, store, marks, limit, mode] = process.argv.slice(2);
const mark = (what: string) => writeFileSync(marks + "/" + name + "." + what, String(Date.now()));
const until = async (what: string) => { while (!existsSync(marks + "/" + name + "." + what)) await Bun.sleep(20); };
holdLease(kind, { store, every: 50, say: () => mark("waited"), limit: Number(limit) || undefined });
mark("held");
if (mode === "spawn") {
  mark("child." + Bun.spawn(["sleep", "30"], { stdio: ["ignore", "ignore", "ignore"] }).pid);
  for (;;) await Bun.sleep(20);
}
if (mode === "yield") {
  await until("go");
  while (!yieldToPush()) await Bun.sleep(20);
  mark("again");
}
if (mode === "line") devTimes(kind, { started: 0, phases: [["x", 1]], ok: true });
else await until("end");
`,
  );
  const env = { ...process.env, RUST_JS_LEASED: "", RUST_JS_LANE: "", RUST_JS_DEV_TIMES: log, RUST_JS_MACHINE: "m", RUST_JS_AGENT: "a" };
  const start = (name: string, kind: string, { limit = 0, mode = "end", lane = false, leased = "" } = {}) =>
    Bun.spawn(["bun", child, name, kind, store, marks, String(limit), mode], {
      cwd: root,
      stdout: "inherit",
      stderr: "pipe",
      env: { ...env, RUST_JS_LANE: lane ? "1" : "", RUST_JS_LEASED: leased },
    });
  const has = (file: string) => existsSync(join(marks, file));
  const until = async (file: string) => {
    for (let i = 0; i < 400 && !has(file); i++) await Bun.sleep(25);
    return has(file);
  };
  const mark = (file: string) => writeFileSync(join(marks, file), "");
  return { store, marks, log, start, until, mark, has };
}

/** What `run` said on its stderr, once it's ended. */
const saidBy = (run: { stderr: ReadableStream<Uint8Array> }) => new Response(run.stderr).text();

// A heavy command's limit counted its wait for the machine's turn: a
// one-mutant run's `timeout 900` killed it still waiting, and the same run
// waited 24 minutes more (#2517). `--limit=<s>` is armed once the turn is
// taken, so a run that waits past it, for the turn or as a lane for no run
// at all, runs its whole limit after.
test("a run's limit counts its run, not its wait for the machine's turn", async () => {
  const { start, until, mark } = limitFixture("leases-limit-wait");
  const first = start("first", "check");
  expect(await until("first.held")).toBe(true);
  const limited = start("limited", "check", { limit: 2 });
  const lane = start("lane", "check", { limit: 2, lane: true });
  expect(await until("limited.waited")).toBe(true);
  expect(await until("lane.waited")).toBe(true);
  await Bun.sleep(3_500);
  mark("first.end");
  expect(await first.exited).toBe(0);
  expect(await until("limited.held")).toBe(true);
  mark("limited.end");
  expect(await limited.exited).toBe(0);
  expect(await until("lane.held")).toBe(true);
  mark("lane.end");
  expect(await lane.exited).toBe(0);
  expect(await saidBy(limited)).not.toContain("past its limit");
  expect(await saidBy(lane)).not.toContain("past its limit");
}, underLoad(60_000));

// Past its limit once it has the turn, a run is stopped, with what it
// started, and says it was its run, not its wait; its turn and lease are
// let go for the next.
test("a run past its limit once it has the turn is stopped, with what it started, and says it was its run", async () => {
  const { store, marks, start, until } = limitFixture("leases-limit-run");
  const run = start("run", "mutations", { limit: 1, mode: "spawn" });
  expect(await until("run.held")).toBe(true);
  const spawned = () => readdirSync(marks).find((file) => file.startsWith("run.child."));
  for (let i = 0; i < 200 && spawned() === undefined; i++) await Bun.sleep(25);
  const child = Number(spawned()!.slice("run.child.".length));
  const held = Date.now();
  // Stopped within its limit and the watcher's grace, or not at all: then
  // killed here, with its child, so a run never stopped fails, not hangs.
  const ended = await Promise.race([run.exited, Bun.sleep(15_000).then(() => "never")]);
  let gone = false;
  try {
    expect(ended).not.toBe("never");
    expect(ended).not.toBe(0);
    expect(Date.now() - held).toBeGreaterThanOrEqual(500);
    expect(await saidBy(run)).toMatch(/^mutations: past its limit, \d+ s of its run against --limit=1, its 0 s waiting for the machine's turn not counted: stopped, with what it started \(#2517\)\n$/);
    for (let i = 0; i < 200 && !gone; i++) {
      try {
        process.kill(child, 0);
        await Bun.sleep(25);
      } catch {
        gone = true;
      }
    }
  } finally {
    run.kill("SIGKILL");
    try {
      process.kill(child, "SIGKILL");
    } catch {}
  }
  expect(gone).toBe(true);
  expect(liveLeases(store)).toEqual([]);
  expect(lockState(turnLock(store))).not.toBe("held");
}, underLoad(60_000));

// A long run lets the turn go to a push between its batches (#2516): its
// wait for the turn again isn't its run either.
test("a run's wait for the turn again, after it let it go to a push, isn't counted against its limit", async () => {
  const { start, until, mark } = limitFixture("leases-limit-yield");
  const run = start("run", "mutations", { limit: 2, mode: "yield" });
  expect(await until("run.held")).toBe(true);
  const push = start("push", "push");
  expect(await until("push.waited")).toBe(true);
  mark("run.go");
  expect(await until("push.held")).toBe(true);
  await Bun.sleep(3_500);
  mark("push.end");
  expect(await push.exited).toBe(0);
  expect(await until("run.again")).toBe(true);
  mark("run.end");
  expect(await run.exited).toBe(0);
  expect(await saidBy(run)).not.toContain("past its limit");
}, underLoad(60_000));

// A run that let the turn go to a push and is told to end as it waits for
// it again lets go only what it holds: the push's turn is left to it, and
// none waiting after takes it meanwhile (#2613). Its handler runs once it
// has the turn again, its wait being the process's own, and lets that go.
test("a run told to end while it waits for the turn it let go to a push leaves the push's turn alone", async () => {
  const { store, start, until, mark, has } = limitFixture("leases-yield-ended");
  const run = start("run", "mutations", { mode: "yield" });
  expect(await until("run.held")).toBe(true);
  const push = start("push", "push");
  expect(await until("push.waited")).toBe(true);
  mark("run.go");
  expect(await until("push.held")).toBe(true);
  const later = start("later", "check");
  expect(await until("later.waited")).toBe(true);
  run.kill("SIGTERM");
  await Bun.sleep(500);
  expect(readFileSync(join(turnLock(store), "owner"), "utf8")).toBe(String(push.pid));
  expect(has("later.held")).toBe(false);
  mark("push.end");
  expect([await push.exited, await run.exited]).toEqual([0, 143]);
  expect(await until("later.held")).toBe(true);
  expect(readFileSync(join(turnLock(store), "owner"), "utf8")).toBe(String(later.pid));
  mark("later.end");
  expect(await later.exited).toBe(0);
  expect(liveLeases(store)).toEqual([]);
  expect(lockState(turnLock(store))).toBe("absent");
}, underLoad(60_000));

// A run's line says its wait apart, `turn <s>`, first (#2517), where it
// took the turn itself: a run in its holder's lease, push run again in its
// first's, says none, where it said `turn 0s` a second time.
test("a run's line says its wait for the turn first, and a run in its holder's lease says none", async () => {
  const { log, start, until, mark } = limitFixture("leases-limit-line");
  const first = start("first", "check");
  expect(await until("first.held")).toBe(true);
  const waiting = start("waiting", "check", { mode: "line" });
  expect(await until("waiting.waited")).toBe(true);
  await Bun.sleep(2_000);
  mark("first.end");
  expect([await first.exited, await waiting.exited]).toEqual([0, 0]);
  const leased = start("leased", "push", { mode: "line", leased: String(process.pid) });
  expect(await leased.exited).toBe(0);
  const [waited, inLease] = readFileSync(log, "utf8").trim().split("\n").map((line) => line.split(" | ")[6]);
  expect(waited).toMatch(/^turn [2-9]s, x 1s$/);
  expect(inLease).toBe("x 1s");
}, underLoad(60_000));

test("a run's limit is one `--limit=<seconds>`, refused otherwise, and the mutation runner takes it", async () => {
  expect([runLimit([]), runLimit(["--all", "--limit=600"]), runLimit(["--limit=1.5"])]).toEqual([undefined, 600, 1.5]);
  for (const args of [["--limit"], ["--limit="], ["--limit=0"], ["--limit=10m"], ["--limit=5", "--limit=6"]]) expect(() => runLimit(args)).toThrow("--limit=<seconds>");
  const { refusedFlags } = await import("../scripts/mutations");
  expect(refusedFlags(["--limit=600"])).toEqual([]);
});
