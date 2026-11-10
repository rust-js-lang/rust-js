// How a child process ended, for every program the tests build or run
// (ADR 0088): its exit code, the signal that stopped it, whether it ran out
// of time or printed more than it may, and what it printed. A run is judged
// on all of it, never on what it printed alone. A compile of rust-js's is
// kept by its inputs, and given again (`test/compiles.ts`, ADR 1099).

import { keptCompile, keptCompileAsync } from "./compiles";

export type Exit = {
  code: number | null;
  signal: string | null;
  timedOut: boolean;
  overflowed: boolean;
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

/** `cmd`, stopped after `timeout` ms or `maxBuffer` bytes of output, with
 * this process's environment and `env`, which a change to `process.env`
 * wouldn't give it; a setting that's `undefined` in `env` is left out. */
export function runSync(cmd: string[], cwd: string, timeout: number, env: Record<string, string | undefined> = {}): Exit {
  return keptCompile(cmd, cwd, env, (cmd) => spawnedSync(cmd, cwd, timeout, env));
}

/** `runSync`'s run of `cmd`, a compile kept or not (`test/compiles.ts`). */
function spawnedSync(cmd: string[], cwd: string, timeout: number, env: Record<string, string | undefined>): Exit {
  const p = Bun.spawnSync(cmd, {
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

/** `cmd`, as `runSync` runs it, without blocking the others running; and
 * stopped, `overgrown`, where it and every process it started, its own
 * group's or one of its own, hold more than `memoryCap` bytes together
 * (#1097). */
export function run(cmd: string[], cwd: string, timeout: number, env: Record<string, string | undefined> = {}, memoryCap?: number): Promise<Exit> {
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

/** `run`'s run of `cmd`, a compile kept or not (`test/compiles.ts`). */
async function spawned(cmd: string[], cwd: string, timeout: number, env: Record<string, string | undefined>, memoryCap?: number): Promise<Exit> {
  const p = Bun.spawn(cmd, { cwd, stdout: "pipe", stderr: "pipe", detached: true, env: { ...process.env, ...env } });
  let timedOut = false;
  let overflowed = false;
  let overgrown = false;
  const readers = [p.stdout.getReader(), p.stderr.getReader()];
  let cut: Timer | undefined;
  const stop = () => {
    stopGroup(p.pid);
    cut ??= setTimeout(() => readers.forEach((reader) => reader.cancel()), grace);
  };
  const timer = setTimeout(() => {
    timedOut = true;
    stop();
  }, timeout);
  // Each second, what it and what it started hold: past the cap, each is
  // stopped, a process another started in a group of its own too.
  const watching =
    memoryCap === undefined
      ? undefined
      : setInterval(() => {
          const tree = processTree(p.pid);
          if (tree.reduce((sum, at) => sum + at.bytes, 0) <= memoryCap) return;
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
    clearTimeout(timer);
    clearInterval(watching);
    stop();
  });
  const [stdout, stderr] = await Promise.all(readers.map(read));
  await p.exited;
  clearTimeout(cut);
  return {
    code: p.exitCode,
    signal: signalName(p.signalCode),
    timedOut,
    overflowed,
    ...(overgrown ? { overgrown } : {}),
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
 * error, a signal, running out of time, another exit code, or an error of
 * rustc's that native Rust didn't give. */
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
  return { kind: "rejected", reason: first };
}
