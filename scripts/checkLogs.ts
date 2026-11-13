// What `check:fast` keeps of each job that fails: its whole output, a file
// each, in a directory of its run's, whose path the summary prints, so a
// flake under load keeps its evidence after the next run (#1622). Each is
// written the moment its job fails, not as the pool ends, so a check an
// outer limit stops keeps every failure it had (#1706). Inside the
// main checkout's `.cache/`, as the dev loop's other stores are (ADR 1118);
// `scripts/disk.ts` holds it to `checkLogsCap`, the oldest runs first.
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { machineStore } from "../test/bounded";

/** Where each run's failing jobs' output is kept. */
export const checkLogsStore = join(machineStore, "check-logs");

/** The most the runs' kept output takes together. */
export const checkLogsCap = 64 * 2 ** 20;

/** A run's kept output: `keep` writes a failing job's whole output to a
 * file named for it, at once, in the run's directory in `store`, made at
 * its first; `kept` is that directory, or none where nothing failed. */
export function checkLogRun({ store = checkLogsStore, now = new Date() }: { store?: string; now?: Date } = {}) {
  const dir = join(store, `${now.toISOString().replace(/[:.]/g, "-")}-${process.pid}`);
  let made = false;
  return {
    keep(name: string, log: string): string {
      if (!made) mkdirSync(dir, { recursive: true });
      made = true;
      writeFileSync(join(dir, `${name.replace(/[^\w.-]+/g, "-")}.log`), log);
      return dir;
    },
    kept: (): string | undefined => (made ? dir : undefined),
  };
}

/** A run's kept output, as one. */
export type CheckLogRun = ReturnType<typeof checkLogRun>;

/** Each of `failed`'s whole output in a file named for its job, in a new
 * directory of this run's in `store`: the directory, or none where nothing
 * failed. */
export function keepFailedLogs(
  failed: { name: string; log: string }[],
  options: { store?: string; now?: Date } = {},
): string | undefined {
  const run = checkLogRun(options);
  for (const { name, log } of failed) run.keep(name, log);
  return run.kept();
}
