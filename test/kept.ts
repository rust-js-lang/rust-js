// The kept native programs (`nativeBinary`, test/support.ts): a directory
// per program, holding a build of it for each set of files it read. Each use
// marks its build used; a run prunes them when it starts, where none runs.

import { readdirSync, rmSync, statSync, utimesSync } from "node:fs";
import { join } from "node:path";

const day = 24 * 60 * 60;

/** Mark `build`, a kept build of a program, used now: what's used stays. */
export function markUsed(build: string) {
  const now = new Date();
  try {
    utimesSync(build, now, now);
  } catch {}
}

/** Drop the builds in `cache` unused for `days`, and those a crash left half
 * made, `.building-*`, an hour on; then a program with none left. `now` is in
 * seconds. */
export function pruneNativeCache(cache: string, now = Date.now() / 1000, days = 7) {
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
