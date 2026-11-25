// How a child process ended, for every program the tests build or run
// (ADR 0088): its exit code, the signal that stopped it, whether it ran out
// of time or printed more than it may, and what it printed. A run is judged
// on all of it, never on what it printed alone. A compile of rust-js's is
// kept by its inputs, and given again (`test/compiles.ts`, ADR 1099).

import { dirname, resolve } from "node:path";
import { keptBuildsAsync, keptCompile, keptCompileAsync } from "./compiles";
import { compiler } from "./packages-of";
import { workerJob } from "./workers";

export type Exit = {
  code: number | null;
  signal: string | null;
  timedOut: boolean;
  overflowed: boolean;
  /** Completion observed this far past the deadline; a queued exit is not
   * proof the child itself ran late. */
  observedOverdueMs?: number;
  /** Stopped where it and what it started held more memory than `run`'s
   * `memoryCap` (#1097). */
  overgrown?: boolean;
  stdout: string;
  stderr: string;
  /** What it printed as it printed it: `stdout` and `stderr` read it as
   * UTF-8, where a byte that isn't is U+FFFD, as U+FFFD itself is. */
  bytes: { stdout: Buffer; stderr: Buffer };
};

type Printed = Pick<Exit, "stdout" | "stderr" | "bytes">;

const observationPrefix = "warning: child observed ";

/** The child owner's diagnostic lines, passed through by check:fast with
 * the job name. Formatting and recognition share this one authority. */
export function childWarnings(output: string): string[] {
  return output.split("\n").filter(line => line.startsWith(observationPrefix));
}


/** What `a` printed to `stream`, to show beside what `b` did: its text, or
 * its bytes, in hex, where they read as the same text as `b`'s. */
export const printed = (a: Printed, b: Printed, stream: "stdout" | "stderr") =>
  a[stream] === b[stream] ? `${a.bytes[stream].toString("hex")} (as bytes)\n` : a[stream];

/** `printed`, cut to the lines around where it first differs from `b`'s,
 * where it's long: a report of two whole long outputs, a boundary matrix's
 * (ADR 0182), would be megabytes, which a test runner takes minutes to diff. */
export const excerpt = (a: Printed, b: Printed, stream: "stdout" | "stderr", limit = 4000) => {
  const text = printed(a, b, stream);
  if (text.length <= limit) return text;
  if (a[stream] === b[stream]) {
    let at = 0;
    while (at < a.bytes[stream].length && a.bytes[stream][at] === b.bytes[stream][at]) at++;
    const from = Math.max(0, at - 16);
    return `[bytes ${from} to ${at + 16} of ${a.bytes[stream].length}, from the first that differs, ${at}] ${a.bytes[stream].subarray(from, at + 16).toString("hex")} (as bytes)\n`;
  }
  const ours = a[stream].split("\n");
  const theirs = b[stream].split("\n");
  let at = 0;
  while (at < ours.length && ours[at] === theirs[at]) at++;
  const from = Math.max(0, at - 2);
  return `[lines ${from + 1} to ${Math.min(at + 3, ours.length)} of ${ours.length}, from the first that differs, ${at + 1}]\n${ours.slice(from, at + 3).join("\n")}\n`;
};

// What a process may print before it's stopped: far more than any test's.
const maxBuffer = 16 * 1024 * 1024;
// How long output is still read after a process is stopped, if something it
// started, outside its group, holds it open.
const grace = 1000;

/** A stopped process's signal, by its name: Bun's types since 1.4.3 allow
 * its number too, which is said as it is. */
const signalName = (signal: string | number | null | undefined) => (signal == null ? null : String(signal));

/** Stop `pid` and what it started: each process runs as the leader of a
 * group of its own, and the group is what's killed, by `signal`. */
export function stopGroup(pid: number, signal: NodeJS.Signals = "SIGKILL") {
  try {
    process.kill(-pid, signal);
  } catch {
    // The group has ended already.
  }
}

/** `cmd` run so it goes with this process (#1444). A command runs as the
 * leader of a group of its own, so a deadline stops what it started too
 * (`stopGroup`), but then a SIGKILL of this process, or of its group, a
 * `timeout -s KILL` or an agent's stopped task, never reaches it: a
 * compile that never ended ran on for two hours, reparented to launchd.
 * A watcher in its group, its output elsewhere, so it holds no pipe of
 * ours open, asks each second whether this process is alive, and once it
 * isn't, kills the group; the command itself is `exec`'d, the group's
 * leader, its pid, exit code and signal its own. The watcher goes with the
 * group, as `stopGroup` stops it once the command ends. */
export function guarded(cmd: string[]): string[] {
  const watch = '( while kill -0 "$0" 2>/dev/null; do sleep 1; done; kill -9 -$$ ) </dev/null >/dev/null 2>&1 & exec "$@"';
  return ["/bin/sh", "-c", watch, String(process.pid), ...cmd];
}

/** `cmd`, a compile of a crate's file by the compiler, given its file's
 * directory left out of the files it names, as the corpus leaves the
 * checkout's (ADR 0427): a panic's location in its JS is `lib.rs:6:5`, the
 * same on every machine and in every fixture (#2292). Not where it says
 * where it's left out itself, nor of rustc's own run, `--rustc`. */
export function located(cmd: string[], cwd: string): string[] {
  const file = cmd[0] === compiler ? cmd.find((arg) => arg.endsWith(".rs")) : undefined;
  // Not rustc's own run of a crate, `--rustc`, which writes no JS.
  if (file === undefined || cmd.includes("--rustc") || cmd.some((arg) => arg.startsWith("--remap-path-prefix"))) return cmd;
  const remap = `--remap-path-prefix=${dirname(resolve(cwd, file))}/=`;
  const rustc = cmd.indexOf("--");
  return rustc < 0 ? [...cmd, "--", remap] : [...cmd.slice(0, rustc + 1), remap, ...cmd.slice(rustc + 1)];
}

/** `cmd`, stopped after `timeout` ms or `maxBuffer` bytes of output, with
 * this process's environment and `env`, which a change to `process.env`
 * wouldn't give it; a setting that's `undefined` in `env` is left out. */
export function runSync(cmd: string[], cwd: string, timeout: number, env: Record<string, string | undefined> = {}): Exit {
  cmd = located(cmd, cwd);
  return keptCompile(cmd, cwd, env, (cmd) => spawnedSync(cmd, cwd, timeout, env));
}

/** `cmds`, a case's builds, each its own compile's command, through the
 * cache: those not kept compiled as one job of the worker this process
 * keeps (`test/workers.ts`, #2301, ADR 1182), in one rustc session; a build
 * it ran apart, or each of a job it gave no answer of, as its own compile.
 * Each `timeout` a build, the job's theirs together. */
export function runBuilds(cmds: string[][], cwd: string, timeout: number, env: Record<string, string | undefined> = {}): Promise<Exit[]> {
  return keptBuildsAsync(cmds, cwd, env, (given) => workerJob(given, cwd, timeout, env), (cmd) => spawned(cmd, cwd, timeout, env));
}

/** `runSync`'s run of `cmd`, a compile kept or not (`test/compiles.ts`). */
export function spawnedSync(cmd: string[], cwd: string, timeout: number, env: Record<string, string | undefined>): Exit {
  const p = Bun.spawnSync(guarded(cmd), {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
    timeout,
    killSignal: "SIGKILL",
    maxBuffer,
    detached: true,
    env: { ...process.env, ...env },
  });
  // What it started and left running goes with it.
  stopGroup(p.pid);
  return {
    code: p.exitCode,
    signal: signalName(p.signalCode),
    // `spawnSync` returns only once its output closes, which what it
    // started may hold open past the deadline after it ended by itself.
    // Stopped at the deadline, it has a signal; without one, it had ended.
    timedOut: (p.exitedDueToTimeout ?? false) && p.signalCode != null,
    overflowed: p.exitedDueToMaxBuffer ?? false,
    stdout: p.stdout.toString(),
    stderr: p.stderr.toString(),
    bytes: { stdout: p.stdout, stderr: p.stderr },
  };
}

/** What `ownPeak` says of a command: how it ended, what it said on
 * stderr, its peak memory in bytes, and the process that spawned it. */
export type Peak = { code: number | null; stderr: string; maxRSS: number; spawner: number };

/** `cmd`'s peak memory, its own, run in `cwd` for at most `timeout` ms.
 * It's spawned by a Bun of its own, never this process: Linux keeps a
 * process's high-water mark of memory across `exec`, its spawner's under
 * `posix_spawn`, so a child of a test worker that has grown is said to
 * have held at least the worker's memory: a compile of 0.27 GB was said
 * to hold 1.2 GB in CI's test job (8b4a77c53's check), and under 1 GiB in
 * the same job's second run of it. */
export function ownPeak(cmd: string[], cwd: string, timeout: number): Peak {
  const measure = `const [cmd, cwd, timeout] = JSON.parse(process.argv.at(-1));
const p = Bun.spawnSync(cmd, { cwd, stdout: "ignore", stderr: "pipe", timeout });
process.stdout.write(JSON.stringify({ code: p.exitCode, stderr: p.stderr.toString(), maxRSS: p.resourceUsage.maxRSS, spawner: process.pid }));`;
  const p = Bun.spawnSync([process.execPath, "-e", measure, JSON.stringify([cmd, cwd, timeout])], {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
    timeout: timeout + 10_000,
  });
  if (p.exitCode !== 0) throw new Error(`measuring ${cmd.join(" ")}: ${p.stderr.toString()}`);
  return JSON.parse(p.stdout.toString()) as Peak;
}

/** `cmd`, as `runSync` runs it, without blocking the others running; and
 * stopped, `overgrown`, where it and every process it started, its own
 * group's or one of its own, hold more than `memoryCap` bytes together
 * (#1097). */
export function run(cmd: string[], cwd: string, timeout: number, env: Record<string, string | undefined> = {}, memoryCap?: number): Promise<Exit> {
  cmd = located(cmd, cwd);
  return keptCompileAsync(cmd, cwd, env, (cmd) => spawned(cmd, cwd, timeout, env, memoryCap));
}

/** `pid` and each process it started, in turn, the memory each holds, in
 * bytes, and what it runs, as `ps` says. */
export function processTree(pid: number): { pid: number; bytes: number; command: string }[] {
  const rows = Bun.spawnSync(["ps", "-eo", "pid=,ppid=,rss=,args="]).stdout.toString().trim().split("\n");
  const children = new Map<number, { pid: number; bytes: number; command: string }[]>();
  let own: { pid: number; bytes: number; command: string } | undefined;
  for (const row of rows) {
    const fields = /^\s*(\d+)\s+(\d+)\s+(\d+)\s?(.*)$/.exec(row);
    if (fields === null) continue;
    const [id, parent, kb] = fields.slice(1, 4).map(Number);
    const at = { pid: id, bytes: kb * 1024, command: fields[4] };
    if (id === pid) own = at;
    children.set(parent, [...(children.get(parent) ?? []), at]);
  }
  const tree = own ? [own] : [];
  for (let i = 0; i < tree.length; i++) tree.push(...(children.get(tree[i].pid) ?? []));
  return tree;
}

/** How long a `footprint` may take, in ms: hundreds of its usual 10 ms,
 * a hang past it (`footprints`). */
export const footprintLimit = 5_000;

/** What each of `pids` holds, in bytes, on macOS: its physical footprint,
 * which counts the pages the memory compressor holds of it, as `ps`'
 * `rss` doesn't, the measure the system's memory pressure takes and
 * test/memory-watch.rs's `proc_pid_rusage` reads of a native run (#1515).
 * Under memory pressure the compressor takes most of what a process holds,
 * 400 MB of ones seen as 10 MB resident, so a watch of `rss` let a variant
 * past its share (#1499). One `footprint` of them all, by pid alone, a
 * process gone by then left out. Elsewhere, none: `rss` is it.
 * `footprint` answers in about 10 ms, but now and then hangs for minutes,
 * which stalled a check's or mutation run's watch of its parts with it:
 * stopped past `footprintLimit`, what it said by then is read, and each
 * pid it didn't answer for is measured by its `rss` (`treeBytes`). */
export function footprints(pids: number[]): Map<number, number> {
  if (process.platform !== "darwin" || pids.length === 0) return new Map();
  return askedOf(["footprint"], pids);
}

/** What `tool`, a `footprint` command, says each of `pids` holds, asked
 * once in `footprintLimit`; none of a command that hung before in this
 * process. Apart from `footprints`' platform, so a test's stand-in for one
 * that hangs runs on every one, Linux's CI's too (#2329). */
export function askedOf(tool: string[], pids: number[]): Map<number, number> {
  const held = new Map<number, number>();
  if (hung.has(tool.join(" "))) return held;
  const args = pids.flatMap((pid) => ["--pid", String(pid)]);
  const cmd = [...tool, ...args, "-f", "bytes", "--noCategories"];
  const p = Bun.spawnSync(cmd, { timeout: footprintLimit, killSignal: "SIGKILL" });
  for (const [, pid, bytes] of p.stdout.toString().matchAll(/\[(\d+)\]: .*Footprint: (\d+) B/g)) held.set(Number(pid), Number(bytes));
  // Stopped past its limit: not asked again in this process, its `rss`
  // read instead. One that hung did so for every process, while what it
  // waited on, a debugger's authorization never given, stayed, and each
  // job of a run waited its limit out, the corpus's 15 minutes past 45
  // (#2329).
  if (p.exitedDueToTimeout) {
    hung.add(tool.join(" "));
    console.error(`${tool[0]} hung past ${footprintLimit / 1000} s: this run reads what each process holds by its rss from now on (#2329)`);
  }
  return held;
}

/** Each `footprint` command that hung in this process, `askedOf`'s
 * `tool`'s words, never asked again. */
const hung = new Set<string>();

/** What `tree`, a process and what it started (`processTree`), holds
 * together, in bytes: each its footprint (`footprints`), or its resident
 * set where none is read, off macOS or of one gone. The one measure of
 * what a run holds, which a memory cap stops it by and a part's peak is,
 * memory-watch's of a native run (#1515). */
export function treeBytes(tree: { pid: number; bytes: number }[], held: Map<number, number> = footprints(tree.map(({ pid }) => pid))): number {
  return tree.reduce((sum, { pid, bytes }) => sum + (held.get(pid) ?? bytes), 0);
}

/** `run`'s run of `cmd`, a compile kept or not (`test/compiles.ts`). */
async function spawned(cmd: string[], cwd: string, timeout: number, env: Record<string, string | undefined>, memoryCap?: number): Promise<Exit> {
  const began = performance.now();
  const p = Bun.spawn(guarded(cmd), { cwd, stdout: "pipe", stderr: "pipe", detached: true, env: { ...process.env, ...env } });
  let timedOut = false;
  let exited = false;
  let observedMs = 0;
  let overflowed = false;
  let overgrown = false;
  const readers = [p.stdout.getReader(), p.stderr.getReader()];
  let cut: Timer | undefined;
  const stop = () => {
    stopGroup(p.pid);
    cut ??= setTimeout(() => readers.forEach((reader) => reader.cancel()), grace);
  };
  const timer = setTimeout(() => {
    // A blocked parent can queue both this deadline and a completed exit.
    // Drain pending I/O before deciding that the child is still running.
    setTimeout(() => {
      if (exited) return;
      timedOut = true;
      stop();
    }, 0);
  }, timeout);
  // Each second, what it and what it started hold: past the cap, each is
  // stopped, a process another started in a group of its own too.
  const watching =
    memoryCap === undefined
      ? undefined
      : setInterval(() => {
          const tree = processTree(p.pid);
          if (treeBytes(tree) <= memoryCap) return;
          overgrown = true;
          for (const { pid } of tree.reverse()) {
            try {
              process.kill(pid, "SIGKILL");
            } catch {}
          }
          stop();
        }, 1000);
  const read = async (reader: ReadableStreamDefaultReader<Uint8Array>) => {
    const chunks: Uint8Array[] = [];
    let size = 0;
    for (let next = await reader.read(); !next.done; next = await reader.read()) {
      if (size + next.value.length > maxBuffer) {
        overflowed = true;
        stop();
        break;
      }
      chunks.push(next.value);
      size += next.value.length;
    }
    return Buffer.concat(chunks);
  };
  // Its end is seen when it comes, not when its output closes, which what
  // it started may hold open: that goes with it then.
  void p.exited.then(() => {
    exited = true;
    observedMs = performance.now() - began;
    clearTimeout(timer);
    clearInterval(watching);
    stop();
  });
  const [stdout, stderr] = await Promise.all(readers.map(read));
  await p.exited;
  clearTimeout(cut);
  const observedOverdueMs = !timedOut && observedMs > timeout ? Math.ceil(observedMs - timeout) : undefined;
  if (observedOverdueMs !== undefined) console.warn(observationPrefix + observedOverdueMs + " ms past its " + timeout + " ms deadline (" + cmd.slice(0,2).join(" ") + "): a blocked parent or a slow child");
  return {
    code: p.exitCode,
    signal: signalName(p.signalCode),
    timedOut,
    overflowed,
    ...(overgrown ? { overgrown } : {}),
    ...(observedOverdueMs === undefined ? {} : { observedOverdueMs }),
    stdout: stdout.toString(),
    stderr: stderr.toString(),
    bytes: { stdout, stderr },
  };
}

/** Why a process didn't end as a program does, or nothing if it did: it
 * ran out of time, printed too much, or was stopped by a signal. */
export function stopped(exit: Exit, timeout: number): string | undefined {
  if (exit.timedOut) return `didn't finish in ${timeout / 1000}s`;
  if (exit.overflowed) return `printed more than ${maxBuffer / 1024 / 1024} MB`;
  if (exit.overgrown) return "held more memory than it's given";
  if (exit.signal) return `killed by ${exit.signal}`;
  return undefined;
}

const errors = (stderr: string) =>
  stderr.split("\n").filter((line) => /^error(\[E\d+\])?:/.test(line) && !line.startsWith("error: aborting due to"));
const crashLine = (stderr: string) =>
  stderr.split("\n").find((line) => /^thread '[^']*'( \(\d+\))? panicked at|internal compiler error|unexpectedly panicked/.test(line));

/** How a compile that didn't succeed failed. rust-js rejects a program by
 * exiting 1 with errors of its own, each `error: rust-js ..`; anything
 * else crashed, whatever it said first: a panic, an internal compiler
 * error, a signal, running out of time, another exit code, an error of
 * rustc's that native Rust didn't give, or one of rust-js's that says it's
 * a bug of rust-js's, a tree `js::validate` refuses (ADR 1143) or a module
 * it can't read back, `const 3 = ..`, which is no refusal of the
 * program's. */
export function compileFailure(exit: Exit, timeout: number): { kind: "rejected" | "crashed"; reason: string } {
  const why = stopped(exit, timeout);
  if (why) return { kind: "crashed", reason: why };
  const crash = crashLine(exit.stderr);
  if (crash) return { kind: "crashed", reason: crash };
  const all = errors(exit.stderr);
  const first = all[0] ?? exit.stderr.trim().split("\n")[0] ?? "no output";
  if (exit.code !== 1) return { kind: "crashed", reason: `exited ${exit.code}: ${first}` };
  const other = all.find((line) => !line.startsWith("error: rust-js"));
  if (all.length === 0 || other) return { kind: "crashed", reason: other ?? first };
  const bug = all.find((line) => line.endsWith("a bug of rust-js's"));
  if (bug) return { kind: "crashed", reason: bug };
  return { kind: "rejected", reason: first };
}
