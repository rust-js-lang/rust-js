// What `check:fast` keeps of each job that fails: its whole output, a file
// each, in a directory of its run's, whose path the summary prints, so a
// flake under load keeps its evidence after the next run (#1622). Inside the
// main checkout's `.cache/`, as the dev loop's other stores are (ADR 1118);
// `scripts/disk.ts` holds it to `checkLogsCap`, the oldest runs first.
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { machineStore } from "../test/bounded";

/** Where each run's failing jobs' output is kept. */
export const checkLogsStore = join(machineStore, "check-logs");

/** The most the runs' kept output takes together. */
export const checkLogsCap = 64 * 2 ** 20;

/** Each of `failed`'s whole output in a file named for its job, in a new
 * directory of this run's in `store`: the directory, or none where nothing
 * failed. */
export function keepFailedLogs(
  failed: { name: string; log: string }[],
  { store = checkLogsStore, now = new Date() }: { store?: string; now?: Date } = {},
): string | undefined {
  if (failed.length === 0) return undefined;
  const run = join(store, `${now.toISOString().replace(/[:.]/g, "-")}-${process.pid}`);
  mkdirSync(run, { recursive: true });
  for (const { name, log } of failed) writeFileSync(join(run, `${name.replace(/[^\w.-]+/g, "-")}.log`), log);
  return run;
}
