// A net's failure, given with its program reduced (#1088): each net, the
// metamorphic net's, the neighbour shuffle's, the memory net's and the
// fuzzer's, hands what goes wrong to `scripts/reduce.ts`, so the failure it
// reports comes with a small program that fails the same way, where an
// agent reduced each by hand.
//
// A test's failures are reduced together, once it has found them all, each
// a share of the test's cores and memory, and within what's left of the
// test's own timeout: a memory-net seed failing in both builds reduced one
// after the other, up to 300 s each, ran past its 600 s timeout (#1365).
//
//   NET_REDUCE=0          the command to reduce it, not reduced
//   NET_REDUCE_BUDGET=<s> how long each may take, 300 s unless it's given
import { spawn } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";

import { root, target } from "./support";
import { machine, memoryBudget } from "../scripts/shard";

/** A failing program to reduce, kept as it was found (`toReduce`): its
 * file, the build it goes wrong in and its edition. */
export type Reduction = { kept: string; build: string; edition: string };

/** `file`'s program, which goes wrong in `build`, to reduce: kept as it is
 * now, where the run's fixtures aren't, as a fixture may be written again,
 * and they go as the run ends. */
export function toReduce(file: string, build: string, edition = "2024"): Reduction {
  const kept = join(target, "reduce", "net", `${Bun.hash(readFileSync(file, "utf8")).toString(36)}.rs`);
  mkdirSync(join(kept, ".."), { recursive: true });
  copyFileSync(file, kept);
  return { kept, build, edition };
}

/** How long a reduction's last steps may run past its budget, a batch of
 * its variants checked and its program formatted, before it's stopped. */
const overrun = 60;

/** The least budget a reduction is started with: less, and it's the
 * command alone. */
const leastBudget = 15;

/** Each of `reductions`, reduced at once: what to append to each failure,
 * the reduced program or why there's none, or with `NET_REDUCE=0` the
 * command that reduces it. They share the cores and memory of the test's
 * part, `RUST_JS_CASES`'s, as each part may reduce at once (#1306), and each
 * ends before `deadline`, a time of `Date.now()`'s, by which the test must
 * end: its budget the less of `NET_REDUCE_BUDGET` and what's left. */
export async function reducedRepros(reductions: Reduction[], deadline: number): Promise<string[]> {
  if (reductions.length === 0) return [];
  const parts = Number((process.env.RUST_JS_CASES ?? "1/1").split("/")[1]) || 1;
  const share = parts * reductions.length;
  const jobs = Math.max(1, Math.floor(machine().cores / share));
  const memory = (memoryBudget() / share).toFixed(1);
  const asked = Number(process.env.NET_REDUCE_BUDGET ?? "300");
  const budget = Math.min(asked, Math.floor((deadline - Date.now()) / 1000) - overrun);
  return Promise.all(reductions.map((r) => reduced(r, budget, jobs, memory)));
}

/** A test's deadline: `timeout` ms after `started`, less a margin for what
 * it does after its reductions. */
export function deadlineOf(started: number, timeout: number): number {
  return started + timeout - 30_000;
}

async function reduced({ kept, build, edition }: Reduction, budget: number, jobs: number, memory: string): Promise<string> {
  const command = `bun scripts/reduce.ts ${relative(root, kept)} --build ${build} --edition ${edition}`;
  if (process.env.NET_REDUCE === "0") return `\nreduce it: ${command}`;
  if (budget < leastBudget) return `\nnot reduced (no time left in the test's timeout): ${command}`;
  const out = kept.replace(/\.rs$/, `.${build}.reduced.rs`);
  const args = ["scripts/reduce.ts", kept, "--build", build, "--edition", edition, "--budget", String(budget), "--jobs", String(jobs), "--memory", memory, "--out", out];
  const p = await ran("bun", args, (budget + overrun) * 1000);
  const said = p.stderr.trim().split("\n").pop() ?? "";
  if (p.status !== 0) return `\nnot reduced (${said || p.error || "the reducer failed"}): ${command}`;
  return `\n${said}:\n${p.stdout.trimEnd()}`;
}

/** `cmd args` run in the repository, stopped, its process group too, after
 * `timeout` ms: its exit status, `null` where it was stopped, and what it
 * printed. */
function ran(cmd: string, args: string[], timeout: number): Promise<{ status: number | null; stdout: string; stderr: string; error?: string }> {
  return new Promise((resolve) => {
    const child = spawn(cmd, args, { cwd: root, detached: true, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8").on("data", (chunk: string) => (stdout += chunk));
    child.stderr.setEncoding("utf8").on("data", (chunk: string) => (stderr += chunk));
    let stopped = false;
    const timer = setTimeout(() => {
      stopped = true;
      try {
        process.kill(-child.pid!, "SIGKILL");
      } catch {}
    }, timeout);
    child.on("error", (error) => {
      clearTimeout(timer);
      resolve({ status: null, stdout, stderr, error: error.message });
    });
    child.on("close", (status) => {
      clearTimeout(timer);
      resolve({ status: stopped ? null : status, stdout, stderr, error: stopped ? `stopped after ${timeout / 1000} s` : undefined });
    });
  });
}
