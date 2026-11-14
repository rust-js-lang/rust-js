// How long each of check:fast's jobs took, and the cores it kept busy, the
// machine's, kept for every checkout of it (#1364): the pool starts the
// longest first by it. A worktree's own `target/` kept them alone, so a new
// worktree, every agent's task, started its jobs in their list's order, the
// corpus's parts and every test file first and the iterators' slices, five
// cores each, last, its last minute's cores 55% busy (2026-11-11).
//
// A job whose work the change sizes, the rows of the iterator tables it
// names, the corpus cases whose rewrites it runs, the mutations it adds,
// keeps each run's size too, and is expected to take its size at its runs'
// pace (`expected`): by its last time alone, 0.1 s after a run that named
// no rows, the rows a change named started last, at 4:41 of a check, and
// ran alone till 6:40 (#1364).
//
// One file in the main checkout's `.cache/job-times/`, each job's last
// `keptRuns` runs, at most `keptJobs` jobs, the least recently run dropped
// first; `scripts/disk.ts` holds the store to `jobTimesCap` besides.
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { freeBytes } from "../tooling/bounded.js";
import { floorBytes, machineStore } from "../test/bounded";
import { withLock } from "./leases";
import { median } from "./timing";

/** Where the machine keeps its jobs' times. */
export const jobTimesStore = join(machineStore, "job-times");

/** The most the store holds, as `scripts/disk.ts` bounds it. */
export const jobTimesCap = 2 ** 20;

/** The most jobs kept, and the most runs of each. */
export const keptJobs = 1000;
export const keptRuns = 5;

/** A job's last runs: the seconds each took, the cores each kept busy and,
 * of a job the change sizes, the units of work each ran, `null` of a run
 * kept unsized; and when it last ran, in ms. */
export type JobTimes = { seconds: number[]; cores: number[]; sizes?: (number | null)[]; at: number };

/** What a job of `size` units of work, or of no size where its work is the
 * same each run, may take, `longest`, by which the pool starts it, and
 * usually takes, `usual`, by which it's given time: of an unsized job, the
 * longest and the median of its last runs' seconds; of a sized one, its
 * size at the slowest and at the median pace of its runs of some size, and
 * nothing of a size of none. `Infinity` where no run says, so a job not
 * yet measured starts first. */
export function expected(times: JobTimes | undefined, size?: number): { longest: number; usual: number } {
  const unknown = { longest: Number.POSITIVE_INFINITY, usual: Number.POSITIVE_INFINITY };
  if (size === 0) return { longest: 0, usual: 0 };
  if (size === undefined) {
    if (times === undefined || times.seconds.length === 0) return unknown;
    return { longest: Math.max(...times.seconds), usual: median(times.seconds) };
  }
  const paces = (times?.seconds ?? []).flatMap((seconds, i) => {
    const ran = times?.sizes?.[i];
    return typeof ran === "number" && ran > 0 ? [seconds / ran] : [];
  });
  if (paces.length === 0) return unknown;
  return { longest: Math.max(...paces) * size, usual: median(paces) * size };
}

const file = (store: string) => join(store, "times.json");

/** Each job's last runs, by its name, as the machine kept them. */
export function readJobTimes(store = jobTimesStore): Record<string, JobTimes> {
  try {
    return JSON.parse(readFileSync(file(store), "utf8"));
  } catch {
    return {};
  }
}

/** `runs` kept with what the machine has, each job's last `keptRuns`, a
 * sized job's sizes beside its seconds, and at most `keptJobs` jobs, the
 * least recently run dropped; written whole, so a reader never sees half
 * of it; nothing written below the disk's floor, `free` its free bytes. */
export function recordJobTimes(
  runs: { name: string; seconds: number; cores: number; size?: number }[],
  { store = jobTimesStore, now = Date.now(), free = () => freeBytes(machineStore) }: { store?: string; now?: number; free?: () => number } = {},
) {
  if (runs.length === 0 || free() < floorBytes) return;
  // Read, changed and written again under the store's lock: two checks
  // ending together each wrote what it read, and the first's runs were lost.
  // Past its wait, not written, the times being a guide.
  if (!existsSync(store)) mkdirSync(store, { recursive: true });
  withLock(join(store, "lock"), () => recordUnder(runs, store, now));
}

/** `recordJobTimes`'s change of `store`, its lock held. */
function recordUnder(runs: { name: string; seconds: number; cores: number; size?: number }[], store: string, now: number) {
  const kept = readJobTimes(store);
  for (const { name, seconds, cores, size } of runs) {
    const was = kept[name] ?? { seconds: [], cores: [], at: now };
    const sizes = [...(was.sizes ?? was.seconds.map(() => null)), size ?? null].slice(-keptRuns);
    kept[name] = {
      seconds: [...was.seconds, Math.round(seconds * 10) / 10].slice(-keptRuns),
      cores: [...was.cores, Math.round(cores * 100) / 100].slice(-keptRuns),
      ...(sizes.some((ran) => ran !== null) ? { sizes } : {}),
      at: now,
    };
  }
  const newest = Object.entries(kept)
    .sort((a, b) => b[1].at - a[1].at)
    .slice(0, keptJobs);
  const written = `${file(store)}.${process.pid}`;
  writeFileSync(written, JSON.stringify(Object.fromEntries(newest)));
  renameSync(written, file(store));
}
