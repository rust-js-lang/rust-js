// How long each of check:fast's jobs took, and the cores it kept busy, the
// machine's, kept for every checkout of it (#1364): the pool starts the
// longest first by it. A worktree's own `target/` kept them alone, so a new
// worktree, every agent's task, started its jobs in their list's order, the
// corpus's parts and every test file first and the iterators' slices, five
// cores each, last, its last minute's cores 55% busy (2026-11-11).
//
// One file in the main checkout's `.cache/job-times/`, each job's last
// `keptRuns` runs, at most `keptJobs` jobs, the least recently run dropped
// first; `scripts/disk.ts` holds the store to `jobTimesCap` besides.
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { freeBytes } from "../tooling/bounded.js";
import { floorBytes, machineStore } from "../test/bounded";

/** Where the machine keeps its jobs' times. */
export const jobTimesStore = join(machineStore, "job-times");

/** The most the store holds, as `scripts/disk.ts` bounds it. */
export const jobTimesCap = 2 ** 20;

/** The most jobs kept, and the most runs of each. */
export const keptJobs = 1000;
export const keptRuns = 5;

/** A job's last runs: the seconds each took and the cores each kept busy,
 * and when it last ran, in ms. */
export type JobTimes = { seconds: number[]; cores: number[]; at: number };

const file = (store: string) => join(store, "times.json");

/** Each job's last runs, by its name, as the machine kept them. */
export function readJobTimes(store = jobTimesStore): Record<string, JobTimes> {
  try {
    return JSON.parse(readFileSync(file(store), "utf8"));
  } catch {
    return {};
  }
}

/** `runs` kept with what the machine has, each job's last `keptRuns`, and at
 * most `keptJobs` jobs, the least recently run dropped; written whole, so a
 * reader never sees half of it; nothing written below the disk's floor,
 * `free` its free bytes. */
export function recordJobTimes(
  runs: { name: string; seconds: number; cores: number }[],
  { store = jobTimesStore, now = Date.now(), free = () => freeBytes(machineStore) }: { store?: string; now?: number; free?: () => number } = {},
) {
  if (runs.length === 0 || free() < floorBytes) return;
  const kept = readJobTimes(store);
  for (const { name, seconds, cores } of runs) {
    const was = kept[name] ?? { seconds: [], cores: [], at: now };
    kept[name] = {
      seconds: [...was.seconds, Math.round(seconds * 10) / 10].slice(-keptRuns),
      cores: [...was.cores, Math.round(cores * 100) / 100].slice(-keptRuns),
      at: now,
    };
  }
  const newest = Object.entries(kept)
    .sort((a, b) => b[1].at - a[1].at)
    .slice(0, keptJobs);
  if (!existsSync(store)) mkdirSync(store, { recursive: true });
  const written = `${file(store)}.${process.pid}`;
  writeFileSync(written, JSON.stringify(Object.fromEntries(newest)));
  renameSync(written, file(store));
}
