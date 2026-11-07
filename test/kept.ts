// What the tests keep between runs, each a directory per thing, holding an
// entry for each set of inputs it was made from: a native program's builds
// (`nativeBinary`, test/support.ts), one for each set of files it read;
// and a corpus case's verified runs (test/verified-runs.ts). Each use marks
// its entry used; a run prunes them when it starts, where none runs:
// what's unused for a week, then the least recently used past each one's
// cap, and below the disk's floor until it's back (test/bounded.ts).

import { readdirSync, rmSync, statSync, utimesSync } from "node:fs";
import { join } from "node:path";

import { bytesOf, evict, freeBytes, type Kept } from "./bounded";
import { checkoutTarget, target } from "./packages-of";

const day = 24 * 60 * 60;

/** The stores of what the tests keep, and the most each keeps: a
 * `check:fast --all` from nothing keeps about 1 GB of native programs,
 * which a change to the compiler keeps, and 8 MB of their wrappers, each
 * the checkout's whichever target the tests run in (#875); and 8 MB of
 * verified runs, made again for each build of it, a week of builds about
 * 500 MB (#914). */
export const keptStores = [
  { dir: join(checkoutTarget, "native-cache"), cap: 3 * 2 ** 30 },
  { dir: join(checkoutTarget, "native-sources"), cap: 64 * 2 ** 20 },
  { dir: join(target, "corpus-cache"), cap: 256 * 2 ** 20 },
];

/** Mark `entry`, a kept one, used now: what's used stays. */
export function markUsed(entry: string) {
  const now = new Date();
  try {
    utimesSync(entry, now, now);
  } catch {}
}

/** Drop the entries in `cache` unused for `days`, and those a crash left
 * half made, `.building-*`, an hour on; then the least recently used,
 * until what's left is under `cap` and the disk's `free` space above its
 * floor; then a thing with none left. `now` is in seconds. What it
 * removed, and the size of what's left. */
export function pruneKept(
  cache: string,
  { now = Date.now() / 1000, days = 7, cap = Number.POSITIVE_INFINITY, free = () => freeBytes(cache) }: { now?: number; days?: number; cap?: number; free?: () => number } = {},
): { removed: number; size: number } {
  let programs: string[];
  try {
    programs = readdirSync(cache);
  } catch {
    return { removed: 0, size: 0 };
  }
  const kept: Kept[] = [];
  let removed = 0;
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
      const used = statSync(path).mtimeMs;
      const age = now - used / 1000;
      if (age > days * day || (build.startsWith(".building-") && age > 60 * 60)) {
        rmSync(path, { recursive: true, force: true });
        removed++;
      } else if (!build.startsWith(".building-")) {
        kept.push({ path, bytes: bytesOf(path), used });
      }
    }
  }
  const evicted = evict(kept, { cap, free });
  for (const program of programs) {
    try {
      if (readdirSync(join(cache, program)).length === 0) rmSync(join(cache, program), { recursive: true, force: true });
    } catch {}
  }
  return { removed: removed + evicted.removed, size: evicted.size };
}

/** Each store of `keptStores` pruned, each to its cap. */
export function pruneKeptStores() {
  for (const { dir, cap } of keptStores) pruneKept(dir, { cap });
}
