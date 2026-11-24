// What the tests keep between runs, each a directory per thing, holding an
// entry for each set of inputs it was made from: a native program's builds
// (`nativeBinary`, test/support.ts), one for each set of files it read;
// and a corpus case's verified runs (test/verified-runs.ts). Each use marks
// its entry used; a run prunes them when it starts, where none runs:
// what's unused for a week, then the least recently used past each one's
// cap, and below the disk's floor until it's back (test/bounded.ts).

import { appendFileSync, existsSync, linkSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";

import { bytesOf, evict, freeBytes, type Kept } from "../tooling/bounded.js";
import { checkout, disk, floorBytes, machineStore, retiredMachineStore } from "./bounded";
import { checkoutTarget, target } from "./packages-of";

const day = 24 * 60 * 60;

/** Where native programs' builds are kept: the machine's, every
 * worktree's, as a build is the same in any checkout of the same sources
 * (`nativeBinary`, #1047). */
export const nativeCache = join(machineStore, "native-cache");

/** Where native programs' sources and wrappers are written, each in a
 * directory named by what's in it (`contentDirectory`, test/support.ts):
 * the checkout's, whichever target the tests run in (#875). */
export const nativeSources = join(checkoutTarget, "native-sources");

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

/** Where what of std a run's JS can run is kept, by std and the JS's files
 * (test/verified-runs.ts' `reached`, #2352): the machine's, as verified
 * runs are, but a target's own given one, a mutant's, so a mutant of how
 * it's found keeps none another run reads. */
export const stdReach = target === checkoutTarget ? join(machineStore, "std-reach") : join(target, "std-reach");

/** The stores of what the tests keep, and the most each keeps past its
 * working set, `past` (`holdKept`): each one's cap is what the machine's
 * runs use of it, measured, and that (#2483). A `check:fast --all` from
 * nothing keeps about 1 GB of native programs, which a change to the
 * compiler keeps, and the metamorphic net's every rewrite of every case
 * about 7 GB more, the machine's (#1047): 11.5 GiB of them in use on the
 * Mac mini, and 22 GiB on the M5 Pro (2026-11-25), each past the cap that
 * held them whole, 10 GiB, then 4% of the disk (#2344, #2505), so the
 * native builds keep the disk's past theirs in use (`diskProfile`: 4% of
 * it, at least 8 GiB), a night's net run again; 8 MB of their wrappers,
 * the checkout's whichever target the tests run in (#875); the corpus's
 * verified runs, about 8 MB for each build of it, a week of builds about
 * 500 MB (#914), and the net's about 100 MB more; and what of std a run's
 * JS reaches and the native runs, 438 and 400 MiB in use on the M5 Pro,
 * past the 64 and 256 that held them whole. */
export const keptStores = [
  { dir: nativeCache, past: disk.nativeCache },
  { dir: nativeSources, past: 64 * 2 ** 20 },
  { dir: verifiedRuns, past: 512 * 2 ** 20 },
  { dir: stdReach, past: 64 * 2 ** 20 },
  { dir: nativeOutcomes, past: 256 * 2 ** 20 },
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

/** `kept`, entries a store keeps, held to its working set and `past`
 * bytes beside it: the working set, what was used since `since`, is always
 * kept, so a store's cap is what the machine's runs use of it, measured,
 * and `past`, never a constant below it (#2163 of the budget, #2344 and
 * #2483 of a store's cap); the stale, unused since, go the least recently
 * used first until they're `past` or less, or none is left. Only the
 * disk's floor takes from the working set, the least recently used first.
 * Each goes by `remove`, where a store's entries may be another's to give,
 * which says whether it went; else whole. What it removed, the size of
 * the entries left, and of their working set. The one rule of every store
 * of things made and used again, and of the budget (scripts/disk.ts'
 * `holdToBudget`).
 *
 * A stale entry is looked at again as it goes: one a run used since it was
 * listed is the working set's, and stays. A check's prune runs beside its
 * jobs (scripts/checkFast.ts' `startup`, #2498), and every checkout's
 * beside the others' runs, so the store's walk, seconds of it, is no
 * longer the last word on what's in use. */
export function holdKept(
  kept: Kept[],
  { past, free, since, remove = removeWhole }: { past: number; free: () => number; since: number; remove?: (entry: Kept) => boolean },
): { removed: number; size: number; working: number } {
  const stale = kept.filter((entry) => entry.used < since);
  const working = kept.filter((entry) => entry.used >= since);
  const unused = (entry: Kept) => {
    const used = statSync(entry.path, { throwIfNoEntry: false })?.mtimeMs;
    if (used !== undefined && used >= since) return false;
    return remove(entry);
  };
  const old = evict(stale, { cap: Math.max(0, past), free, floor: floorBytes, remove: unused });
  const floored = evict(working, { cap: Number.POSITIVE_INFINITY, free, floor: floorBytes, remove });
  return { removed: old.removed + floored.removed, size: old.size + floored.size, working: floored.size };
}

/** How `holdKept` removes an entry where its store says nothing else:
 * whole. */
function removeWhole({ path }: Kept): boolean {
  rmSync(path, { recursive: true, force: true });
  return true;
}

/** The size of `kept`'s working set, what was used since `since`. */
export function workingBytes(kept: Kept[], since: number): number {
  return kept.filter((entry) => entry.used >= since).reduce((n, entry) => n + entry.bytes, 0);
}

/** Mark `entry`, a kept one, used now: what's used stays. */
export function markUsed(entry: string) {
  const now = new Date();
  try {
    utimesSync(entry, now, now);
  } catch {}
}

/** A store's index (#2498): beside it, `.walked.json`, when `pruneKept`
 * last walked it, the size of what it left and of its working set then;
 * and `.kept`, a line for
 * each entry kept in it since, its size and its path in the store, which
 * what keeps one appends (`noteKept`). Its size between walks is the two together: never less
 * than it is, as a removal by another checkout's prune only makes it
 * more. A walk takes `.kept` first, so an entry kept as it walks is
 * counted again, not lost. Derived from the walk alone, it's a cache of
 * it, walked again where it can't say (`walkDue`). */
const walkedOf = (store: string) => join(store, ".walked.json");
const keptLogOf = (store: string) => join(store, ".kept");

/** `entry`, just kept in `store`, counted in the store's size until its
 * next walk (`pruneKept`): what every store's writer calls once it has
 * kept one. */
export function noteKept(store: string, entry: string) {
  try {
    appendFileSync(keptLogOf(store), `${bytesOf(entry)}\t${relative(store, entry)}\n`);
  } catch {}
}

/** The size of what `noteKept` counted in `store` since its last walk. */
export function keptSince(store: string): number {
  let since = 0;
  try {
    for (const line of readFileSync(keptLogOf(store), "utf8").split("\n")) since += Number(line.split("\t")[0]) || 0;
  } catch {}
  return since;
}

/** `store`'s size by its index, and its working set's: its last walk's,
 * and what was kept since, in use as it's kept; none where it has no last
 * walk, or one that didn't count its working set. Its cap is that working
 * set and what it keeps past it (`holdKept`), which `bun run disk` says. */
export function indexedSize(store: string): { walked: number; bytes: number; working: number } | undefined {
  let last: { walked?: unknown; bytes?: unknown; working?: unknown };
  try {
    last = JSON.parse(readFileSync(walkedOf(store), "utf8"));
  } catch {
    return undefined;
  }
  if (typeof last.walked !== "number" || typeof last.bytes !== "number" || typeof last.working !== "number") return undefined;
  const since = keptSince(store);
  return { walked: last.walked, bytes: last.bytes + since, working: last.working + since };
}

/** Whether `store` is walked now, or its index says what a walk would
 * remove is nothing: walked where it has no index; a day after its last
 * walk, so what's unused for a week goes within a day of it; with the
 * disk below its floor; or with more of it stale than `past`, what it
 * keeps past its working set, an hour after its last walk. A working set,
 * however large, is in its cap, never a reason to walk: past a constant
 * cap, native-cache's 33,000 builds in use were walked every hour (#2483).
 * `now` is in seconds. */
export function walkDue(store: string, { past, free, now }: { past: number; free: () => number; now: number }): boolean {
  const index = indexedSize(store);
  if (index === undefined || free() < floorBytes) return true;
  const ago = now - index.walked;
  return ago < 0 || ago >= day || (index.bytes - index.working > past && ago >= 60 * 60);
}

/** Drop the entries in `cache` unused for `days`, and those a crash left
 * half made, `.building-*`, an hour on; then held to its working set,
 * used since `since` (ms), and `past` bytes beside it, and to the disk's
 * `free` space above
 * its floor (`holdKept`); then a thing with none left. Each walk is kept
 * in the store's index, which says when the next is due (`walkDue`): till
 * then, nothing's listed. `now` is in seconds. What it removed, and the
 * size of what's left. */
export function pruneKept(
  cache: string,
  {
    now = Date.now() / 1000,
    days = 7,
    past = Number.POSITIVE_INFINITY,
    free = () => freeBytes(cache),
    since = workingSetSince(),
  }: { now?: number; days?: number; past?: number; free?: () => number; since?: number } = {},
): { removed: number; size: number } {
  if (!walkDue(cache, { past, free, now })) return { removed: 0, size: indexedSize(cache)!.bytes };
  let programs: string[];
  try {
    programs = readdirSync(cache).filter((name) => !name.startsWith("."));
  } catch {
    return { removed: 0, size: 0 };
  }
  // What's kept as it walks is counted again after it, not lost.
  const taken = `${keptLogOf(cache)}.${process.pid}`;
  try {
    renameSync(keptLogOf(cache), taken);
  } catch {}
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
  const evicted = holdKept(keptBuilds(cache), { past, free, since });
  for (const program of programs) {
    try {
      if (readdirSync(join(cache, program)).length === 0) rmSync(join(cache, program), { recursive: true, force: true });
    } catch {}
  }
  const writing = `${walkedOf(cache)}.${process.pid}`;
  try {
    writeFileSync(writing, JSON.stringify({ walked: now, bytes: evicted.size, working: evicted.working }));
    renameSync(writing, walkedOf(cache));
  } catch {}
  rmSync(taken, { force: true });
  return { removed: removed + evicted.removed, size: evicted.size };
}

/** Each build `pruneKept` keeps in `cache`, `<program>/<build>`, with when
 * it was last used: what holding every store to the budget may evict
 * (scripts/disk.ts `holdToBudget`, #2163). One half made, `.building-*`,
 * isn't, as a run may be writing it. */
export function keptBuilds(cache: string): Kept[] {
  let programs: string[];
  try {
    programs = readdirSync(cache).filter((name) => !name.startsWith("."));
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

/** Each store of `stores`, `keptStores` unless a test gives its own,
 * pruned, each to its working set and what it keeps past it and, where
 * `free` says the disk is below its floor, to above it, each retired store
 * removed whole, and the machine's stores where they were moved where they
 * are. */
export function pruneKeptStores(free?: () => number, stores: { dir: string; past: number }[] = keptStores) {
  adoptRetiredStores();
  for (const dir of retiredStores) if (dir !== verifiedRuns) rmSync(dir, { recursive: true, force: true });
  for (const { dir, past } of stores) pruneKept(dir, free ? { past, free } : { past });
  pruneKeptApps(free ? { free } : {});
}

/** Whether `pid` is a live process's. */
export function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/** The pid a claim, `owner`, names, where it's there as it's read. */
function readOwner(owner: string): string | undefined {
  try {
    return readFileSync(owner, "utf8");
  } catch {
    return undefined;
  }
}

/** `owner`, a claim, made this process's: made where there's none, the
 * one link to it that succeeds being the one process's that takes it; or
 * taken over where its process is gone, one process at a time, each under
 * `<owner>.taking`, made by the one `mkdir` of it that succeeds, so what
 * it saw gone is what it replaces. Taken over by renaming whatever was
 * there by then, two racing for one both took it, the second taking the
 * first's new claim from it (#2446). Whether it's this process's. */
export function takeClaim(owner: string): boolean {
  const writing = `${owner}.${process.pid}`;
  writeFileSync(writing, String(process.pid));
  let got = false;
  try {
    linkSync(writing, owner);
    got = true;
  } catch {
    const taking = `${owner}.taking`;
    if (takeOver(taking)) {
      try {
        if (!alive(Number(readOwner(owner)))) {
          rmSync(owner, { force: true });
          linkSync(writing, owner);
          got = true;
        }
      } catch {
        // Made since by a link of another process's, where there was none.
      } finally {
        rmSync(taking, { recursive: true, force: true });
      }
    }
  }
  rmSync(writing, { force: true });
  return got;
}

/** Whether this process may take a claim over, `taking` made by it; one a
 * process that died taking one left, a few seconds old, removed for the
 * next to make. */
function takeOver(taking: string): boolean {
  try {
    mkdirSync(taking);
    return true;
  } catch {
    const made = statSync(taking, { throwIfNoEntry: false })?.mtimeMs;
    if (made !== undefined && Date.now() - made > 10_000) rmSync(taking, { recursive: true, force: true });
    return false;
  }
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
 * floor (`evict`, tooling/bounded.js); one a live process claims stays.
 * Each is claimed as it's removed, as a `KeptApp` claims it, so one a
 * process claims after it was listed, as another's ends, stays: it was
 * removed under the pilot's build as it began (#2446). */
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
    // Each `<name>_<n>`, not its claim nor a claim's makings beside it.
    for (const name of list(join(base, scope)).filter((entry) => /_\d+$/.test(entry))) {
      const path = join(base, scope, name);
      if (!held(path)) kept.push({ path, bytes: bytesOf(path), used: statSync(path).mtimeMs });
    }
  }
  const remove = ({ path }: Kept) => {
    const owner = `${path}.owner`;
    if (!takeClaim(owner)) return false;
    rmSync(path, { recursive: true, force: true });
    rmSync(owner, { force: true });
    return true;
  };
  return evict(kept, { cap, free, floor: floorBytes, remove });
}

/** Whether a store may keep what it's about to write: not below the disk's
 * floor (test/bounded.ts), where what's made is used and not kept. */
export function mayKeep(dir: string, free = () => freeBytes(dir)): boolean {
  return free() >= floorBytes;
}
