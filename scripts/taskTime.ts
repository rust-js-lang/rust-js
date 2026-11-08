// How long a task took, and how much of it went to running checks and
// tests: the return on what the loop's speed-ups cost (DEVELOPMENT.md,
// "Keeping the loop fast"). A task's log is kept per branch in
// ~/.cache/rust-js/tasks/, outside `target/`, so `cargo clean` keeps it.
//
//   bun run task start <name>        the task starts now
//   bun run task run <label> -- cmd  runs cmd, timed as a check (its exit kept)
//   bun run task report              the task's wall time, and its checks'
//
// Run every check or test of the task through `task run`: check:fast, the
// corpus, mutations, a port's regen, a matrix run.

import { appendFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { machineStore } from "../test/bounded";

type Entry = { kind: "start"; name: string; at: number } | { kind: "run"; label: string; at: number; seconds: number; code: number };

const branch = Bun.spawnSync(["git", "rev-parse", "--abbrev-ref", "HEAD"]).stdout.toString().trim() || "detached";
const dir = join(machineStore, "tasks");
const file = join(dir, `${branch.replaceAll("/", "_")}.jsonl`);

function entries(): Entry[] {
  if (!existsSync(file)) return [];
  return readFileSync(file, "utf8")
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line) as Entry);
}

/** The entries since the task's last start. */
function current(): Entry[] {
  const all = entries();
  let from = -1;
  all.forEach((e, i) => {
    if (e.kind === "start") from = i;
  });
  return from < 0 ? [] : all.slice(from);
}

function minutes(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  return m > 0 ? `${m} min ${s} s` : `${s} s`;
}

const [command, ...rest] = process.argv.slice(2);
mkdirSync(dir, { recursive: true });

if (command === "start") {
  const name = rest.join(" ") || branch;
  // Appended, not written over: a branch's earlier tasks keep their runs,
  // and `current()` reads from the last start.
  appendFileSync(file, JSON.stringify({ kind: "start", name, at: Date.now() } satisfies Entry) + "\n");
  console.log(`task "${name}" started, logged in ${file}`);
} else if (command === "run") {
  const split = rest.indexOf("--");
  if (split < 1) {
    console.error("usage: bun run task run <label> -- <command…>");
    process.exit(2);
  }
  const label = rest.slice(0, split).join(" ");
  const cmd = rest.slice(split + 1);
  const start = Date.now();
  const p = Bun.spawn(cmd, { stdout: "inherit", stderr: "inherit", stdin: "inherit" });
  const code = await p.exited;
  const seconds = (Date.now() - start) / 1000;
  appendFileSync(file, JSON.stringify({ kind: "run", label, at: start, seconds, code } satisfies Entry) + "\n");
  process.exit(code);
} else if (command === "report") {
  const task = current();
  const start = task[0];
  if (!start || start.kind !== "start") {
    console.error(`no task started on ${branch}: bun run task start <name>`);
    process.exit(1);
  }
  const runs = task.filter((e): e is Extract<Entry, { kind: "run" }> => e.kind === "run");
  const wall = (Date.now() - start.at) / 1000;
  // Runs can overlap: the time spent waiting on checks is their union.
  const spans = runs.map((r) => [r.at, r.at + r.seconds * 1000] as const).sort((a, b) => a[0] - b[0]);
  let checked = 0;
  let end = -Infinity;
  for (const [a, b] of spans) {
    if (b <= end) continue;
    checked += (b - Math.max(a, end)) / 1000;
    end = b;
  }
  const byLabel = new Map<string, { seconds: number; count: number; failed: number }>();
  for (const r of runs) {
    const t = byLabel.get(r.label) ?? { seconds: 0, count: 0, failed: 0 };
    t.seconds += r.seconds;
    t.count += 1;
    if (r.code !== 0) t.failed += 1;
    byLabel.set(r.label, t);
  }
  console.log(`task "${start.name}" (${branch})`);
  console.log(`  wall time        ${minutes(wall)}`);
  console.log(`  running checks   ${minutes(checked)} (${Math.round((100 * checked) / Math.max(wall, 1))}% of it)`);
  for (const [label, t] of [...byLabel].sort((a, b) => b[1].seconds - a[1].seconds)) {
    console.log(`    ${label}: ${t.count}× ${minutes(t.seconds)}${t.failed ? `, ${t.failed} failed` : ""}`);
  }
} else {
  console.error("usage: bun run task start <name> | run <label> -- <command…> | report");
  process.exit(2);
}
