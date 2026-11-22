// What the tests keep between runs, each a directory per thing, holding an
// entry for each set of inputs it was made from: a native program's builds
// (`nativeBinary`, test/support.ts), one for each set of files it read;
// and a corpus case's verified runs (test/verified-runs.ts). Each use marks
// its entry used; a run prunes them when it starts, where none runs:
// what's unused for a week, then the least recently used past each one's
// cap, and below the disk's floor until it's back (test/bounded.ts).

import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import { bytesOf, evict, freeBytes, type Kept } from "../tooling/bounded.js";
import { checkout, floorBytes, machineStore, retiredMachineStore } from "./bounded";
import { checkoutTarget, target } from "./packages-of";

const day = 24 * 60 * 60;

/** Where native programs' builds are kept: the machine's, every
 * worktree's, as a build is the same in any checkout of the same sources
 * (`nativeBinary`, #1047). */
export const nativeCache = join(machineStore, "native-cache");

/** Where native programs' runs are kept by what they were built from
 * (test/native-outcomes.ts): the machine's, every worktree's, as their
 * builds are, and what CI's jobs restore and `main`'s save. */
export const nativeOutcomes = join(machineStore, "native-outcomes");

/** Where the Rustlantis net's generator is built (ADR 1125): the
 * machine's, one build of its pinned commit and patch, some 120 MB, an
 * earlier one removed as a build starts (test/rustlantis.ts). */
export const rustlantisStore = join(machineStore, "rustlantis");
export const rustlantisCap = 512 * 2 ** 20;

/** Where runs verified by what they ran are kept (ADR 0567): the
 * machine's, every worktree's, as a run's key is what it ran, not where
 * (#1047); but a target's own given one, `RUST_JS_TARGET`, a mutant's,
 * so none of a mutant's runs is skipped for one the compiler verified (ADR
 * 0093), and each test the test map records, which shares the machine's
 * (scripts/testMap.ts). */
export const machineVerifiedRuns = join(machineStore, "verified-runs");
export const verifiedRuns = target === checkoutTarget ? machineVerifiedRuns : join(target, "corpus-cache");

/** The stores of what the tests keep, and the most each keeps past its
 * working set (`holdKept`), which is what the machine's runs measure
 * themselves: a `check:fast --all` from nothing keeps about 1 GB of native
 * programs, which a change to the compiler keeps, and the metamorphic
 * net's every rewrite of every case about 7 GB more, the machine's
 * (#1047), 11.5 GiB of them in use on the Mac mini, past the cap, which
 * evicted them for a 1,018 s re-warm (#2344); 8 MB of their wrappers, the
 * checkout's whichever target the tests run in (#875); and the corpus's
 * verified runs, about 8 MB for each build of it, a week of builds about
 * 500 MB (#914), and the net's about 100 MB more. */
export const keptStores = [
  { dir: nativeCache, cap: 10 * 2 ** 30 },
  { dir: join(checkoutTarget, "native-sources"), cap: 64 * 2 ** 20 },
  { dir: verifiedRuns, cap: 512 * 2 ** 20 },
  { dir: nativeOutcomes, cap: 256 * 2 ** 20 },
];

/** The least a working set is: what was used in the last day. */
export const workingSetMs = 24 * 3600 * 1000;

/** Where a checkout's check marks when it last started: `check:fast`'s
 * prune, `--quiet`, touches it (scripts/disk.ts). */
export const lastCheckMark = join(checkout, "target", "last-check");

/** When the working set starts: this checkout's last check's start, or a
 * day before `now`, the earlier, so what every check of it reuses, a
 * corpus case's verified run, is in it however long ago that check was. */
export function workingSetSince(now = Date.now(), mark = lastCheckMark): number {
  const last = statSync(mark, { throwIfNoEntry: false })?.mtimeMs ?? now;
  return Math.min(last, now - workingSetMs);
}

/** `kept`, entries a store keeps, held to `cap` with `rest` bytes beside
 * them that nothing here removes: the stale, unused since `since`, the
 * least recently used first, until the whole is under it or none is left;
 * the working set, what was used since, never for the cap, so what the
 * machine's runs use is the least a store keeps, measured, not a constant
 * below it (#2163 of the budget, #2344 of a store's cap). Only the disk's
 * floor takes from it, the least recently used first. What it removed,
 * and the size of the entries left. The one rule of both. */
export function holdKept(kept: Kept[], { cap, free, since, rest = 0 }: { cap: number; free: () => number; since: number; rest?: number }): { removed: number; size: number } {
  const stale = kept.filter((entry) => entry.used < since);
  const working = kept.filter((entry) => entry.used >= since);
  const workingBytes = working.reduce((n, entry) => n + entry.bytes, 0);
  const old = evict(stale, { cap: Math.max(0, cap - rest - workingBytes), free, floor: floorBytes });
  const floored = evict(working, { cap: Number.POSITIVE_INFINITY, free, floor: floorBytes });
  return { removed: old.removed + floored.removed, size: old.size + floored.size };
}

/** Mark `entry`, a kept one, used now: what's used stays. */
export function markUsed(entry: string) {
  const now = new Date();
  try {
    utimesSync(entry, now, now);
  } catch {}
}

/** Drop the entries in `cache` unused for `days`, and those a crash left
 * half made, `.building-*`, an hour on; then held to `cap`, its working
 * set, used since `since` (ms), kept, and to the disk's `free` space above
 * its floor (`holdKept`); then a thing with none left. `now` is in
 * seconds. What it removed, and the size of what's left. */
export function pruneKept(
  cache: string,
  {
    now = Date.now() / 1000,
    days = 7,
    cap = Number.POSITIVE_INFINITY,
    free = () => freeBytes(cache),
    since = workingSetSince(),
  }: { now?: number; days?: number; cap?: number; free?: () => number; since?: number } = {},
): { removed: number; size: number } {
  let programs: string[];
  try {
    programs = readdirSync(cache);
  } catch {
    return { removed: 0, size: 0 };
  }
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
      // One another run's prune removed since it was listed is gone.
      const seen = statSync(path, { throwIfNoEntry: false });
      if (seen === undefined) continue;
      const used = seen.mtimeMs;
      const age = now - used / 1000;
      if (age > days * day || (build.startsWith(".building-") && age > 60 * 60)) {
        rmSync(path, { recursive: true, force: true });
        removed++;
      }
    }
  }
  const evicted = holdKept(keptBuilds(cache), { cap, free, since });
  for (const program of programs) {
    try {
      if (readdirSync(join(cache, program)).length === 0) rmSync(join(cache, program), { recursive: true, force: true });
    } catch {}
  }
  return { removed: removed + evicted.removed, size: evicted.size };
}

/** Each build `pruneKept` keeps in `cache`, `<program>/<build>`, with when
 * it was last used: what holding every store to the budget may evict
 * (scripts/disk.ts `holdToBudget`, #2163). One half made, `.building-*`,
 * isn't, as a run may be writing it. */
export function keptBuilds(cache: string): Kept[] {
  let programs: string[];
  try {
    programs = readdirSync(cache);
  } catch {
    return [];
  }
  return programs.flatMap((program) => {
    let builds: string[];
    try {
      builds = readdirSync(join(cache, program));
    } catch {
      return [];
    }
    return builds.flatMap((build) => {
      const path = join(cache, program, build);
      const seen = statSync(path, { throwIfNoEntry: false });
      return seen === undefined || build.startsWith(".building-") ? [] : [{ path, bytes: bytesOf(path), used: seen.mtimeMs }];
    });
  });
}

/** The checkout's stores before they were the machine's (#1047): native
 * programs' builds and verified runs, which nothing reads now. */
export const retiredStores = [join(checkoutTarget, "native-cache"), join(checkoutTarget, "corpus-cache")];

/** Each of the machine's stores where it was, `retired`, moved to where it
 * is, `current`, once (#1213), merged with what's there already: a
 * directory's entries each in turn; an append-only log, a task's `.jsonl`,
 * its old lines first and then the new ones it hasn't, so a task started
 * before the move keeps its start (#1213, ldAC's); any other file, the
 * newer. Then `retired` itself, left empty. The same disk, so a move is a
 * rename, and a store a test is reading goes on being read from where it's
 * moved to. */
export function adoptRetiredStores(retired = retiredMachineStore, current = machineStore) {
  if (!existsSync(retired)) return;
  mergeInto(retired, current);
  rmSync(retired, { recursive: true, force: true });
}

/** `from` merged into `to`, as `adoptRetiredStores` merges a store. */
function mergeInto(from: string, to: string) {
  if (!existsSync(to)) {
    mkdirSync(dirname(to), { recursive: true });
    try {
      renameSync(from, to);
      return;
    } catch {}
  }
  if (statSync(from).isDirectory() && statSync(to).isDirectory()) {
    for (const name of readdirSync(from)) mergeInto(join(from, name), join(to, name));
    return;
  }
  if (from.endsWith(".jsonl")) {
    const had = readFileSync(to, "utf8").split("\n").filter(Boolean);
    const old = readFileSync(from, "utf8").split("\n").filter(Boolean);
    const merged = [...old, ...had.filter((line) => !old.includes(line))];
    writeFileSync(to, `${merged.join("\n")}\n`);
  } else if (statSync(from).mtimeMs > statSync(to).mtimeMs) {
    rmSync(to, { recursive: true, force: true });
    renameSync(from, to);
  }
}

/** Each store of `keptStores` pruned, each to its cap and, where `free`
 * says the disk is below its floor, to above it, each retired store
 * removed whole, and the machine's stores where they were moved where they
 * are. */
export function pruneKeptStores(free?: () => number) {
  adoptRetiredStores();
  for (const dir of retiredStores) if (dir !== verifiedRuns) rmSync(dir, { recursive: true, force: true });
  for (const { dir, cap } of keptStores) pruneKept(dir, free ? { cap, free } : { cap });
  pruneKeptApps(free ? { free } : {});
}

/** Where apps kept between runs are (`KeptApp`, test/support.ts): the
 * worktree's, whose path dependencies name its own crates, so never
 * another's; `<test file>/<name>_<n>`, each claimed by `<name>_<n>.owner`. */
export const keptApps = join(target, "kept-apps");

/** The most the kept apps keep: the pilot's is 709 MB, its Cargo target
 * the most of it (2026-11-09). */
export const keptAppsCap = 4 * 2 ** 30;

/** The kept apps no process holds, the least recently used first, dropped
 * until what's left is under `cap` and the disk's `free` space above its
 * floor (`evict`, tooling/bounded.js); one a live process claims stays. */
export function pruneKeptApps({ base = keptApps, cap = keptAppsCap, free = () => freeBytes(base) }: { base?: string; cap?: number; free?: () => number } = {}): { removed: number; size: number } {
  const kept: Kept[] = [];
  const list = (dir: string) => {
    try {
      return readdirSync(dir);
    } catch {
      return [];
    }
  };
  const held = (dir: string) => {
    try {
      const pid = Number(readFileSync(`${dir}.owner`, "utf8"));
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  };
  for (const scope of list(base)) {
    for (const name of list(join(base, scope)).filter((entry) => !entry.endsWith(".owner"))) {
      const path = join(base, scope, name);
      if (!held(path)) kept.push({ path, bytes: bytesOf(path), used: statSync(path).mtimeMs });
    }
  }
  return evict(kept, { cap, free, floor: floorBytes });
}

/** Whether a store may keep what it's about to write: not below the disk's
 * floor (test/bounded.ts), where what's made is used and not kept. */
export function mayKeep(dir: string, free = () => freeBytes(dir)): boolean {
  return free() >= floorBytes;
}
