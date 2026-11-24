// The worker a test process compiles its jobs in, `rust-js --worker`
// (#2301 step 2, ADR 1182): started at its first job and kept, so the
// compiler's start, loading rustc's libraries, is paid once a test process,
// not once a job. Its jobs are one at a time, in the order they're asked.
// One that gives no answer by its deadline, or ends it, is stopped, and its
// builds are compiled apart; the next job starts another. And one that
// holds more than a test's share of the machine's memory
// (`testShareBytes`), as a large case leaves it, macOS keeping what it
// freed, is replaced before its next job: twelve parts each keeping 1.1 to
// 1.7 GB would be 13 to 20 GB on a 16 GB machine.

import type { Subprocess } from "bun";

import { testShareBytes } from "../scripts/shard";
import { guarded, processTree, stopGroup, treeBytes, type Exit } from "./child";

type Answered = { builds: ({ code: number; stdout: string; stderr: string } | { apart: string })[] } | { error: string };

/** The worker kept: its process, what's read of its stdout and not yet a
 * line, and what it was started as, so a job of another compiler, place or
 * environment starts its own. */
type Kept = {
  proc: Subprocess<"pipe", "pipe", "pipe">;
  reader: ReadableStreamDefaultReader<Uint8Array>;
  buffered: string;
  key: string;
  /** The end of what it said on its own stderr, outside any build's, at
   * most `saidMost` characters: a panic's message as it dies, and read
   * till it ends. */
  said: { text: string; read: Promise<void> };
  /** What it held when last measured, every `measureEvery` ms, and the
   * timer that measures it. */
  held: number;
  watch: ReturnType<typeof setInterval>;
};

/** How often a kept worker's memory is measured, in ms: measuring takes
 * about 20 ms (`treeBytes`), too long to ask after each job. */
const measureEvery = 2000;
/** The most of a worker's own stderr kept: a panic's message and its
 * backtrace's first frames. */
const saidMost = 4000;
let kept: Kept | undefined;
/** The pid of the worker that answered the last job. */
let answeredBy: number | undefined;
/** The job before, which the next waits for. */
let queue: Promise<unknown> = Promise.resolve();
/** Why each build of the last job was compiled apart, where it was: the
 * worker's own word, or how it gave no answer, which the nightly's run of
 * every case had to say: 45309220022 ran 4 builds of each of two parts
 * apart, and its failure named none. */
let apartWhy: (string | undefined)[] = [];

function stop() {
  if (kept === undefined) return;
  clearInterval(kept.watch);
  stopGroup(kept.proc.pid);
  kept = undefined;
}

/** What the kept worker holds, measured now, as its timer measures it. */
export function measureKept(): number | undefined {
  if (kept !== undefined) kept.held = treeBytes(processTree(kept.proc.pid));
  return kept?.held;
}

function start(program: string, cwd: string, env: Record<string, string | undefined>, key: string): Kept {
  const proc = Bun.spawn(guarded([program, "--worker"]), {
    cwd,
    env: { ...process.env, ...env },
    stdin: "pipe",
    stdout: "pipe",
    // Its own output outside a build's, a crash's, is no build's: a job
    // it ends gets each build compiled apart, which says it, and what it
    // said as it died, as a panic's message is all that names what killed
    // it: 4 builds of two cases ran apart in each of two nightly parts,
    // "it ended (101)" and no more (#2448).
    stderr: "pipe",
    detached: true,
  });
  const said = { text: "", read: Promise.resolve() };
  said.read = (async () => {
    const decoder = new TextDecoder();
    for await (const chunk of proc.stderr) said.text = (said.text + decoder.decode(chunk, { stream: true })).slice(-saidMost);
  })().catch(() => {});
  // It neither keeps this process alive nor outlives it: its stdin closes
  // as this process ends, and its watcher stops it (`guarded`).
  proc.unref();
  const watch = setInterval(measureKept, measureEvery);
  watch.unref();
  return { proc, reader: proc.stdout.getReader(), buffered: "", key, said, held: 0, watch };
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
export function workerJob(
  cmds: string[][],
  cwd: string,
  timeout: number,
  env: Record<string, string | undefined>,
  share = testShareBytes(),
): Promise<(Exit | undefined)[]> {
  const run = queue.then(() => answered(cmds, cwd, timeout, env, share));
  queue = run.catch(() => undefined);
  return run;
}

async function answered(cmds: string[][], cwd: string, timeout: number, env: Record<string, string | undefined>, share: number): Promise<(Exit | undefined)[]> {
  const program = cmds[0][0];
  const key = JSON.stringify([program, cwd, env]);
  if (kept?.key !== key || kept.held > share) stop();
  kept ??= start(program, cwd, env, key);
  const worker = kept;
  const none = cmds.map(() => undefined);
  const noneBecause = (why: string) => {
    apartWhy = cmds.map(() => why);
    return none;
  };
  try {
    worker.proc.stdin.write(`${JSON.stringify({ builds: cmds.map((cmd) => cmd.slice(1)) })}\n`);
    worker.proc.stdin.flush();
  } catch {
    stop();
    return noneBecause("its stdin was closed");
  }
  const text = await line(worker, Date.now() + timeout * cmds.length);
  answeredBy = worker.proc.pid;
  if (text === undefined) {
    // Ended, or late: which, as its exit says a moment after.
    await Promise.race([worker.proc.exited, Bun.sleep(1000)]);
    const ended = worker.proc.exitCode ?? worker.proc.signalCode;
    if (ended !== null) await Promise.race([worker.said.read, Bun.sleep(1000)]);
    stop();
    const said = worker.said.text.trim();
    return noneBecause(ended === null ? `it gave no answer in ${timeout * cmds.length} ms` : `it ended (${ended})${said ? `, saying:\n${said}` : ""}`);
  }
  let answer: Answered;
  try {
    answer = JSON.parse(text);
  } catch {
    stop();
    return noneBecause(`its answer wasn't JSON: ${text.slice(0, 200)}`);
  }
  if ("error" in answer) throw new Error(`the worker refused a job: ${answer.error}`);
  if (answer.builds.length !== cmds.length) return noneBecause(`it answered ${answer.builds.length} builds of ${cmds.length}`);
  apartWhy = answer.builds.map((build) => ("apart" in build ? build.apart : undefined));
  return answer.builds.map((build) => {
    if (!("code" in build)) return undefined;
    const bytes = { stdout: Buffer.from(build.stdout), stderr: Buffer.from(build.stderr) };
    return { code: build.code, signal: null, timedOut: false, overflowed: false, stdout: build.stdout, stderr: build.stderr, bytes };
  });
}

/** The pid of the worker the last job went to: a test's, of whether jobs
 * share one. */
export const workerPid = (): number | undefined => answeredBy;

/** Why each build of the last job ran apart, `undefined` of one answered. */
export const workerApart = (): (string | undefined)[] => apartWhy;
