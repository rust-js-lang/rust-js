// How a child process ended, and how a failed compile is told apart:
// rust-js's rejection, or a crash, whatever it said first.

import { expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";

import { compileFailure, footprintLimit, footprints, processTree, run, runSync, treeBytes, type Exit } from "./child";
import { execute, nativeCommand, nativeShare } from "./programs";
import { underLoad } from "../scripts/timing";
import { contentDirectory, fixture, nativeBinary, run as setUp } from "./support";

const exit = (code: number | null, stderr: string, more: Partial<Exit> = {}): Exit => ({
  code,
  signal: null,
  timedOut: false,
  overflowed: false,
  stdout: "",
  stderr,
  bytes: { stdout: Buffer.alloc(0), stderr: Buffer.from(stderr) },
  ...more,
});
const rejection = "error: rust-js does not support 128-bit integers yet\n --> case.rs:1:1\n\nerror: aborting due to 1 previous error\n";

test("rust-js's own errors, and exit 1, are a rejection", () => {
  expect(compileFailure(exit(1, rejection), 1000)).toEqual({ kind: "rejected", reason: "error: rust-js does not support 128-bit integers yet" });
  expect(compileFailure(exit(1, "error: rust-js: generated trait implementation name `x` collides\n"), 1000).kind).toBe("rejected");
});

test("a rejection followed by a crash is a crash", () => {
  const panicked = `${rejection}thread 'rustc' panicked at src/lower.rs:1:1:\nboom\n`;
  expect(compileFailure(exit(101, panicked), 1000)).toEqual({ kind: "crashed", reason: "thread 'rustc' panicked at src/lower.rs:1:1:" });
  expect(compileFailure(exit(1, panicked), 1000).kind).toBe("crashed");
  // As rustc says it, with the thread's number.
  const numbered = `${rejection}thread 'rustc' (21196015) panicked at src/lower/representation.rs:226:34:\n`;
  expect(compileFailure(exit(101, numbered), 1000).reason).toBe("thread 'rustc' (21196015) panicked at src/lower/representation.rs:226:34:");
  expect(compileFailure(exit(101, "error: internal compiler error: oops\n"), 1000).reason).toBe("error: internal compiler error: oops");
});

test("another exit code, rustc's own error, a signal or a deadline is a crash", () => {
  expect(compileFailure(exit(2, rejection), 1000)).toEqual({ kind: "crashed", reason: "exited 2: error: rust-js does not support 128-bit integers yet" });
  expect(compileFailure(exit(1, `${rejection}error[E0277]: the trait bound is not satisfied\n`), 1000)).toEqual({
    kind: "crashed",
    reason: "error[E0277]: the trait bound is not satisfied",
  });
  expect(compileFailure(exit(1, ""), 1000).kind).toBe("crashed");
  expect(compileFailure(exit(null, rejection, { signal: "SIGSEGV" }), 1000)).toEqual({ kind: "crashed", reason: "killed by SIGSEGV" });
  expect(compileFailure(exit(null, rejection, { signal: "SIGTERM", timedOut: true }), 2000)).toEqual({ kind: "crashed", reason: "didn't finish in 2s" });
});

test("a process that doesn't end, or prints without end, is stopped and says so", async () => {
  const slow = await run(["sh", "-c", "sleep 30"], ".", 200);
  expect([slow.timedOut, slow.signal]).toEqual([true, "SIGKILL"]);
  expect(runSync(["sh", "-c", "sleep 30"], ".", 200).timedOut).toBe(true);
  const loud = await run(["sh", "-c", "yes"], ".", 10_000);
  expect([loud.overflowed, loud.timedOut]).toEqual([true, false]);
  expect(runSync(["sh", "-c", "yes"], ".", 10_000).overflowed).toBe(true);
  const failed = await run(["sh", "-c", "echo out; echo err >&2; exit 3"], ".", 10_000);
  expect(failed).toEqual({ code: 3, signal: null, timedOut: false, overflowed: false, stdout: "out\n", stderr: "err\n", bytes: { stdout: Buffer.from("out\n"), stderr: Buffer.from("err\n") } });
});

// What a process started, holding its output open, is stopped with it, so
// a deadline is kept. Found in review: `sleep 3 & wait` took 3s of 100ms.
// The fixture must have started its descendant before the deadline stops
// it, or there's nothing to see stopped: under load `sh` took more than
// 100 ms to get there, and the test read a PID file never written (#1770).
// So that's its precondition, said where it isn't met: the async run is
// watched for the file, written whole by a rename, as it runs to its
// deadline; the sync one finds it once it returns.
test("a process's descendants are stopped with it, and its deadline kept", async () => {
  const pidFile = join(fixture("child-descendant"), "pid");
  const fixtureCmd = ["sh", "-c", `sleep 30 & echo $! > ${pidFile}.part && mv ${pidFile}.part ${pidFile}; wait`];
  const deadline = 1000;
  const unmet = `precondition not met: the fixture didn't start its descendant within the ${deadline} ms deadline`;
  const descendant = () => (existsSync(pidFile) ? Number(readFileSync(pidFile, "utf8")) : undefined);
  const alive = (pid: number) => {
    try {
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  };
  // Stopped, it's gone once whoever it's left to reaps it: at once alone,
  // past a fixed 50 ms on a machine at a load of 20 or more (#750). So it's
  // asked again until it's gone, for 5 s at most: still there then, it was
  // left running.
  const gone = async (pid: number) => {
    for (const until = Date.now() + 5000; alive(pid) && Date.now() < until; ) await Bun.sleep(20);
    return !alive(pid);
  };
  rmSync(pidFile, { force: true });
  const started = Date.now();
  let ended = false;
  const running = run(fixtureCmd, ".", deadline).finally(() => (ended = true));
  let pid = descendant();
  while (pid === undefined && !ended) {
    await Bun.sleep(10);
    pid = descendant();
  }
  const slow = await running;
  pid ??= descendant();
  expect(pid, unmet).toBeDefined();
  expect(slow.timedOut).toBe(true);
  expect(Date.now() - started).toBeLessThan(deadline + 2000);
  expect(await gone(pid!)).toBe(true);
  rmSync(pidFile, { force: true });
  const sync = runSync(fixtureCmd, ".", deadline);
  const syncPid = descendant();
  expect(syncPid, unmet).toBeDefined();
  expect(sync.timedOut).toBe(true);
  expect(await gone(syncPid!)).toBe(true);
}, underLoad(30_000));

// A process that ends, leaving what it started holding its output open,
// ended as it did; what it started goes with it. Found in review: it
// waited out its deadline, and said it had run out of time. `run` sees the
// end at once; `runSync` can't before the output closes, so only says it.
test("a process that ends is seen to, whatever it left running", async () => {
  const pidFile = join(fixture("child-left-running"), "pid");
  const started = Date.now();
  const done = await run(["sh", "-c", `sleep 30 & echo $! > ${pidFile}; echo done`], ".", 5000);
  expect(done).toEqual({ code: 0, signal: null, timedOut: false, overflowed: false, stdout: "done\n", stderr: "", bytes: { stdout: Buffer.from("done\n"), stderr: Buffer.alloc(0) } });
  expect(Date.now() - started).toBeLessThan(2000);
  const alive = (pid: number) => {
    try {
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  };
  await Bun.sleep(50);
  expect(alive(Number(readFileSync(pidFile, "utf8")))).toBe(false);
  const sync = runSync(["sh", "-c", `sleep 30 & echo $! > ${pidFile}; echo done`], ".", 500);
  expect([sync.code, sync.signal, sync.timedOut, sync.stdout]).toEqual([0, null, false, "done\n"]);
  await Bun.sleep(50);
  expect(alive(Number(readFileSync(pidFile, "utf8")))).toBe(false);
});

// A setting for a child is given to it, as `process.env` changed later
// isn't. Found by the mutation runner, whose tests never saw its compiler.
test("a process gets the environment it's given", async () => {
  expect(runSync(["sh", "-c", "echo $CHILD_SETTING"], ".", 10_000, { CHILD_SETTING: "given" }).stdout).toBe("given\n");
  expect((await run(["sh", "-c", "echo $CHILD_SETTING"], ".", 10_000, { CHILD_SETTING: "given" })).stdout).toBe("given\n");
});

// A setting given as `undefined` is left out, as qualification leaves out
// what would weaken its checks, such as `BLESS`.
test("a setting given as undefined isn't passed on", () => {
  process.env.CHILD_LEFT_OUT = "set";
  try {
    expect(runSync(["sh", "-c", "echo ${CHILD_LEFT_OUT:-none}"], ".", 10_000, { CHILD_LEFT_OUT: undefined }).stdout).toBe("none\n");
  } finally {
    delete process.env.CHILD_LEFT_OUT;
  }
});

// A command the tests set up with, as they load, keeps a deadline too, or
// a native program that doesn't end would stall them before any test's
// deadline applies. Found in review.
test("a setup command that doesn't end is stopped, and says so", () => {
  const started = Date.now();
  expect(() => setUp(["sh", "-c", "sleep 30"], 200)).toThrow("didn't finish in 0.2s");
  expect(Date.now() - started).toBeLessThan(2000);
});

// A run past its memory cap, it and what it started, is stopped and said
// so, `overgrown`: a mutant compiler that ran away took CI's runners down
// (#1097). The whole tree is counted, a child's in a group of its own too.
test("a run that holds more than its memory cap is stopped, its children too", async () => {
  const grow = `const a = []; for (;;) { a.push(new Array(1e6).fill(1)); await Bun.sleep(1); }`;
  const child = `Bun.spawn([process.execPath, "-e", ${JSON.stringify(grow)}], { detached: true }); await Bun.sleep(60000);`;
  const started = Date.now();
  const p = await run([process.execPath, "-e", child], ".", 60_000, {}, 300 * 2 ** 20);
  expect([p.overgrown, p.timedOut, Date.now() - started < 30_000]).toEqual([true, false, true]);
  expect(processTree(process.pid).length).toBeGreaterThan(0);
}, underLoad(60_000));

// What a run holds is each process's footprint, its resident set where
// none is read: under memory pressure macOS's compressor holds most of a
// process's pages, which `rss` leaves out, 400 MB of ones seen as 10 MB,
// so a memory cap by `rss` let a variant past its share (#1499); the
// footprint is what memory-watch reads of a native run too (#1515).
test("what a run holds counts what the memory compressor holds of it", async () => {
  expect(treeBytes([{ pid: 1, bytes: 10 << 20 }], new Map([[1, 400 << 20]]))).toBe(400 << 20);
  expect(treeBytes([{ pid: 1, bytes: 10 << 20 }, { pid: 2, bytes: 5 << 20 }], new Map([[2, 1 << 20]]))).toBe(11 << 20);
  if (process.platform !== "darwin") return;
  // A process holding 64 MB it wrote: its footprint says so, by pid.
  const hold = `const a = new Uint8Array(64 << 20).fill(1); await Bun.sleep(30000); a[0];`;
  const p = Bun.spawn([process.execPath, "-e", hold]);
  let held = 0;
  for (let i = 0; i < 50 && held < 64 << 20; i++) {
    await Bun.sleep(100);
    held = footprints([p.pid]).get(p.pid) ?? 0;
  }
  p.kill("SIGKILL");
  await p.exited;
  expect(held).toBeGreaterThanOrEqual(64 << 20);
  expect(footprints([])).toEqual(new Map());
}, underLoad(30_000));

// A `footprint` that hangs, as macOS's now and then does for minutes, is
// stopped past its limit, and what it didn't say is read of `rss`: it
// stalled every part's watch with it, a check's and a mutation run's.
test("a footprint that hangs is stopped past its limit", () => {
  if (process.platform !== "darwin") return;
  const started = Date.now();
  const hangs = [process.execPath, "-e", "await Bun.sleep(600000)"];
  expect(footprints([process.pid], hangs)).toEqual(new Map());
  expect(Date.now() - started).toBeLessThan(footprintLimit + 5_000);
}, underLoad(30_000));

// A run's command goes with the process that ran it: one SIGKILLed, by a
// `timeout -s KILL` or an agent's stopped task, left a compile running in
// its own group, reparented to launchd, for two hours (#1444).
test("a run's command is stopped when the process that ran it is killed", async () => {
  // A duration no other process sleeps, 30 s at most, so one this test
  // leaves, failing, ends by itself.
  const marker = `29.${process.pid % 1000}${Date.now() % 1000}`;
  const runner = `import { runSync } from ${JSON.stringify(import.meta.dir + "/child.ts")}; runSync(["sleep", "${marker}"], ".", 60_000);`;
  const left = () => Bun.spawnSync(["pgrep", "-f", `^sleep ${marker}$`]).stdout.toString().trim();
  const parent = Bun.spawn([process.execPath, "-e", runner]);
  for (let i = 0; i < 100 && left() === ""; i++) await Bun.sleep(50);
  expect(left()).not.toBe("");
  parent.kill("SIGKILL");
  await parent.exited;
  for (let i = 0; i < 60 && left() !== ""; i++) await Bun.sleep(100);
  expect(left()).toBe("");
}, underLoad(30_000));


// A native run on macOS, which keeps no memory limit an allocator heeds, is
// killed by its watchdog past its share of the machine, what the system
// compressed of it counted too, and said so (#1515): three runs that held
// 9 GB each froze a 16 GB Mac. The program touches every page it takes, as
// one that never ends does, and would print once it held twice its share.
test.skipIf(process.platform !== "darwin")("a native run past its memory share is stopped and said so", () => {
  const share = nativeShare();
  const source = `fn main() {
    let mut held: Vec<Vec<u8>> = Vec::new();
    while held.len() * (64 << 20) < ${2 * share} {
        held.push(vec![7; 64 << 20]);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    println!("held {}", held.len());
}
`;
  const built = nativeBinary(source, contentDirectory(source), ["--edition=2024", "-O"]);
  if ("error" in built) throw new Error(built.error);
  const outcome = join(fixture("native-memory-share"), "native.json");
  const started = Date.now();
  const ran = execute(nativeCommand(built.binary, outcome), outcome, 60_000);
  expect([ran.stopped, String(ran.outcome).startsWith("stopped past its memory share"), ran.stdout]).toEqual([true, true, ""]);
  expect(Date.now() - started).toBeLessThan(30_000);
}, underLoad(90_000));
