// What the tests keep between runs, each a directory per thing, holding an
// entry for each set of inputs it was made from: a native program's builds
// (`nativeBinary`, test/support.ts), one for each set of files it read;
// and a corpus case's verified runs (test/verified-runs.ts). Each use marks
// its entry used; a run prunes them when it starts, where none runs.

import { readdirSync, rmSync, statSync, utimesSync } from "node:fs";
import { join } from "node:path";

const day = 24 * 60 * 60;

/** Mark `entry`, a kept one, used now: what's used stays. */
export function markUsed(entry: string) {
  const now = new Date();
  try {
    utimesSync(entry, now, now);
  } catch {}
}

/** Drop the entries in `cache` unused for `days`, and those a crash left
 * half made, `.building-*`, an hour on; then a thing with none left. `now`
 * is in seconds. */
export function pruneKept(cache: string, now = Date.now() / 1000, days = 7) {
  let programs: string[];
  try {
    programs = readdirSync(cache);
  } catch {
    return;
  }
  for (const program of programs) {
    const dir = join(cache, program);
    let builds: string[];
    try {
      builds = readdirSync(dir);
    } catch {
      continue;
    }
    for (const build of builds) {
      const path = join(dir, build);
      const age = now - statSync(path).mtimeMs / 1000;
      if (age > days * day || (build.startsWith(".building-") && age > 60 * 60)) {
        rmSync(path, { recursive: true, force: true });
      }
    }
    if (readdirSync(dir).length === 0) rmSync(dir, { recursive: true, force: true });
  }
}
