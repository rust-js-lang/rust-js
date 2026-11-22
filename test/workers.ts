// The worker a test process compiles its jobs in, `rust-js --worker`
// (#2301 step 2, ADR 1182): started at its first job and kept, so the
// compiler's start, loading rustc's libraries, is paid once a test process,
// not once a job. Its jobs are one at a time, in the order they're asked.
// One that gives no answer by its deadline, or ends it, is stopped, and its
// builds are compiled apart; the next job starts another.

import type { Subprocess } from "bun";

import { guarded, stopGroup, type Exit } from "./child";

type Answered = { builds: ({ code: number; stdout: string; stderr: string } | { apart: string })[] } | { error: string };

/** The worker kept: its process, what's read of its stdout and not yet a
 * line, and what it was started as, so a job of another compiler, place or
 * environment starts its own. */
type Kept = {
  proc: Subprocess<"pipe", "pipe", "ignore">;
  reader: ReadableStreamDefaultReader<Uint8Array>;
  buffered: string;
  key: string;
};
let kept: Kept | undefined;
/** The pid of the worker that answered the last job. */
let answeredBy: number | undefined;
/** The job before, which the next waits for. */
let queue: Promise<unknown> = Promise.resolve();

function stop() {
  if (kept === undefined) return;
  stopGroup(kept.proc.pid);
  kept = undefined;
}

function start(program: string, cwd: string, env: Record<string, string | undefined>, key: string): Kept {
  const proc = Bun.spawn(guarded([program, "--worker"]), {
    cwd,
    env: { ...process.env, ...env },
    stdin: "pipe",
    stdout: "pipe",
    // Its own output outside a build's, a crash's, is no build's: a job
    // it ends gets each build compiled apart, which says it.
    stderr: "ignore",
    detached: true,
  });
  // It neither keeps this process alive nor outlives it: its stdin closes
  // as this process ends, and its watcher stops it (`guarded`).
  proc.unref();
  return { proc, reader: proc.stdout.getReader(), buffered: "", key };
}

/** The worker's next line, or `undefined` where it ended or `deadline` came first. */
async function line(worker: Kept, deadline: number): Promise<string | undefined> {
  const decoder = new TextDecoder();
  for (;;) {
    const at = worker.buffered.indexOf("\n");
    if (at >= 0) {
      const text = worker.buffered.slice(0, at);
      worker.buffered = worker.buffered.slice(at + 1);
      return text;
    }
    let timer: ReturnType<typeof setTimeout> | undefined;
    const late = new Promise<"late">((resolve) => {
      timer = setTimeout(() => resolve("late"), Math.max(0, deadline - Date.now()));
    });
    const read = await Promise.race([worker.reader.read(), late]);
    clearTimeout(timer);
    if (read === "late" || read.done) return undefined;
    worker.buffered += decoder.decode(read.value, { stream: true });
  }
}

/** Each build's answer of a job of `cmds`, each build's own compile's
 * command: its exit, as its own compile's, or `undefined` of one the
 * worker ran apart, or of each where it gave none. `timeout` a build. */
export function workerJob(cmds: string[][], cwd: string, timeout: number, env: Record<string, string | undefined>): Promise<(Exit | undefined)[]> {
  const run = queue.then(() => answered(cmds, cwd, timeout, env));
  queue = run.catch(() => undefined);
  return run;
}

async function answered(cmds: string[][], cwd: string, timeout: number, env: Record<string, string | undefined>): Promise<(Exit | undefined)[]> {
  const program = cmds[0][0];
  const key = JSON.stringify([program, cwd, env]);
  if (kept?.key !== key) stop();
  kept ??= start(program, cwd, env, key);
  const worker = kept;
  const none = cmds.map(() => undefined);
  try {
    worker.proc.stdin.write(`${JSON.stringify({ builds: cmds.map((cmd) => cmd.slice(1)) })}\n`);
    worker.proc.stdin.flush();
  } catch {
    stop();
    return none;
  }
  const text = await line(worker, Date.now() + timeout * cmds.length);
  answeredBy = worker.proc.pid;
  let answer: Answered | undefined;
  try {
    answer = text === undefined ? undefined : JSON.parse(text);
  } catch {
    answer = undefined;
  }
  if (answer === undefined) {
    stop();
    return none;
  }
  if ("error" in answer) throw new Error(`the worker refused a job: ${answer.error}`);
  if (answer.builds.length !== cmds.length) return none;
  return answer.builds.map((build) => {
    if (!("code" in build)) return undefined;
    const bytes = { stdout: Buffer.from(build.stdout), stderr: Buffer.from(build.stderr) };
    return { code: build.code, signal: null, timedOut: false, overflowed: false, stdout: build.stdout, stderr: build.stderr, bytes };
  });
}

/** The pid of the worker the last job went to: a test's, of whether jobs
 * share one. */
export const workerPid = (): number | undefined => answeredBy;
