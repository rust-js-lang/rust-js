// What the tests keep between runs, each a directory per thing, holding an
// entry for each set of inputs it was made from: a native program's builds
// (`nativeBinary`, test/support.ts), one for each set of files it read;
// and a corpus case's verified runs (test/verified-runs.ts). Each use marks
// its entry used; a run prunes them when it starts, where none runs:
// what's unused for a week, then the least recently used past each one's
// cap, and below the disk's floor until it's back (test/bounded.ts).

import { readdirSync, rmSync, statSync, utimesSync } from "node:fs";
import { join } from "node:path";

import { bytesOf, evict, freeBytes, type Kept } from "../tooling/bounded.js";
import { floorBytes, machineStore } from "./bounded";
import { checkoutTarget, target } from "./packages-of";

const day = 24 * 60 * 60;

/** Where native programs' builds are kept: the machine's, every
 * worktree's, as a build is the same in any checkout of the same sources
 * (`nativeBinary`, #1047). */
export const nativeCache = join(machineStore, "native-cache");

/** Where runs verified by what they ran are kept (ADR 0567): the
 * machine's, every worktree's, as a run's key is what it ran, not where
 * (#1047); but a target's own given one, `RUST_JS_TARGET`, a mutant's,
 * so none of a mutant's runs is skipped for one the compiler verified (ADR
 * 0093), and each test the test map records, which shares the machine's
 * (scripts/testMap.ts). */
export const machineVerifiedRuns = join(machineStore, "verified-runs");
export const verifiedRuns = target === checkoutTarget ? machineVerifiedRuns : join(target, "corpus-cache");

/** The stores of what the tests keep, and the most each keeps: a
 * `check:fast --all` from nothing keeps about 1 GB of native programs,
 * which a change to the compiler keeps, and the metamorphic net's every
 * rewrite of every case about 7 GB more, the machine's (#1047); 8 MB of
 * their wrappers, the checkout's whichever target the tests run in (#875);
 * and the corpus's verified runs, about 8 MB for each build of it, a week
 * of builds about 500 MB (#914), and the net's about 100 MB more. */
export const keptStores = [
  { dir: nativeCache, cap: 10 * 2 ** 30 },
  { dir: join(checkoutTarget, "native-sources"), cap: 64 * 2 ** 20 },
  { dir: verifiedRuns, cap: 512 * 2 ** 20 },
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
  const evicted = evict(kept, { cap, free, floor: floorBytes });
  for (const program of programs) {
    try {
      if (readdirSync(join(cache, program)).length === 0) rmSync(join(cache, program), { recursive: true, force: true });
    } catch {}
  }
  return { removed: removed + evicted.removed, size: evicted.size };
}

/** The checkout's stores before they were the machine's (#1047): native
 * programs' builds and verified runs, which nothing reads now. */
export const retiredStores = [join(checkoutTarget, "native-cache"), join(checkoutTarget, "corpus-cache")];

/** Each store of `keptStores` pruned, each to its cap, and each retired
 * store removed whole. */
export function pruneKeptStores() {
  for (const dir of retiredStores) if (dir !== verifiedRuns) rmSync(dir, { recursive: true, force: true });
  for (const { dir, cap } of keptStores) pruneKept(dir, { cap });
}

/** Whether a store may keep what it's about to write: not below the disk's
 * floor (test/bounded.ts), where what's made is used and not kept. */
export function mayKeep(dir: string, free = () => freeBytes(dir)): boolean {
  return free() >= floorBytes;
}
