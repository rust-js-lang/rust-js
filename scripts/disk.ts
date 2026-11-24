// What rust-js development keeps on the machine's disk, every place of it,
// and each held to its cap (#1213, ADR 1118): the main checkout's `.cache/`,
// the machine's stores, which every worktree shares; each worktree's
// `target/`, whose stores the tests and scripts keep; the temp root the
// tests' tools write to; and what Rust's, Cargo's, Bun's and Playwright's own
// tools keep for it outside the checkout, which their own commands bound.
//
//   bun run disk            each store, its size, its cap and its oldest
//                           entry, and the total against the budget
//   bun run disk --prune    each held to its cap, and the tools' own
//                           stores by their own commands
//   bun run disk --prune --inside --quiet
//                           `check:fast`'s, before its jobs: the stores
//                           inside the checkouts held to their caps, and a
//                           warning where the last measure of them is past
//                           90% of the budget, naming the largest holders
//                           where pruning couldn't take it under; a new
//                           measure made apart, `--measure`, which holds
//                           them to that 90% together, the least recently
//                           used of all their stale entries first (#2163),
//                           never the working set, what was used since this
//                           checkout's last check or in the last day, as
//                           walking `target/` takes 3.5 s and removing
//                           gigabytes a minute

import { spawn, spawnSync } from "node:child_process";
import { closeSync, existsSync, lstatSync, mkdirSync, openSync, readdirSync, readFileSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { bytesOf, evict, freeBytes, type Kept } from "../tooling/bounded.js";
import { cargoHome, checkout, disk, floorBytes, machineStore, mainCheckoutOf, tempRoot } from "../test/bounded";
import { checkLogsCap, checkLogsStore } from "./checkLogs";
import { leasedIn, lockState } from "./leases";
import { worktreeRoots } from "./strays";
import { sanityRunsCap, sanityRunsStore } from "./select";
import { jobTimesCap, jobTimesStore } from "./jobTimes";
import { compileCache, capBytes as compileCap, prune as pruneCompiles } from "../test/compiles";
import { holdKept, keptAppsCap, keptBuilds, keptStores, lastCheckMark, pruneKeptStores, removeClaimed, removeWhole, rustlantisCap, rustlantisStore, workingBytes, workingSetSince } from "../test/kept";
import { boundCrateProjects, crateProjectsCap, crateProjectsKept } from "./crate-corpus";
import { boundCrater, craterCap, craterKept } from "./crater";
import { bound as boundPorts, ports, portsCap } from "./react-in-rust";
import { keptMaps, mapStore, mapsCap, pruneMaps } from "./testMap";

const GiB = 2 ** 30;

/** The most every store inside the checkouts keeps together, `.cache/`,
 * the worktrees' `target/`s and the temp root: their caps and a build of
 * the compiler and its tests' fixtures, about 6 GiB, with room for a second
 * worktree's (#1213), sized by the disk as the native builds' cap is
 * (`diskProfile`, #2505). `check:fast` warns past 90% of it. */
export const budgetBytes = disk.budget;

/** A store: where it is, the most it keeps, and how it's held to that,
 * given the disk's free bytes. A store of things each made once and used
 * again whole, a native build, a verified run, a log, keeps its working
 * set, what the machine's runs used since this checkout's last check or in
 * the last day, and `past` bytes beside it, so its cap is what's in use,
 * measured, and that (`holdKept`, #2483). One of builds, a Cargo target
 * each entry, which grows as it's built again in, is held to `cap` whole,
 * the least recently used first, till Cargo's own units bound it (#2483's
 * third item); a build, which a build replaces, has neither, held only by
 * the budget. */
export type Store = {
  name: string;
  dir: string;
  past?: number;
  cap?: number;
  bound?: (free: () => number) => void;
  outside?: true;
  /** Of a store that's some of `dir`'s entries, which. */
  entries?: (name: string) => boolean;
  /** Of a store whose entries are each its own, removed apart as its
   * `bound` removes them: each, with when it was last used. What holds the
   * stores inside the checkouts to the budget together evicts from these,
   * the least recently used of all first (#2163). */
  kept?: () => Kept[];
  /** How one of `kept`'s is removed, where a run may be using it: whether
   * it removed it. Whole, where the store says nothing (#2562). */
  remove?: (entry: Kept) => boolean;
};

/** Each entry of `dir`, a thing made once and kept, held to its working
 * set, what was changed since `since`, and `past` bytes beside it, the
 * least recently changed first, and nothing kept below the disk's floor
 * (`holdKept`): a store of things with no bound of its own. */
export function holdEntries(dir: string, past: number, free: () => number, since = workingSetSince()) {
  return holdKept(entriesOf(dir), { past, free, since });
}

/** Each entry of `dir`, a build's, held to `cap` whole, the least recently
 * changed first, and nothing kept below the disk's floor: a store of
 * builds with no bound of its own, `target/mutants`. */
export function boundEntries(dir: string, cap: number, free: () => number) {
  return evict(entriesOf(dir), { cap, free, floor: floorBytes });
}

/** Each entry of `dir`, kept, with when it last changed. */
export function entriesOf(dir: string): Kept[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    // A link is its own, as `bytesOf` takes it. One gone since it was
    // listed, another checkout's run's test removing its own directory
    // from the temp root they share, is no longer kept (#1473).
    try {
      return [{ path, bytes: bytesOf(path), used: lstatSync(path).mtimeMs }];
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return [];
      throw error;
    }
  });
}

/** What the stores inside the checkouts are held to together: the 90% of
 * the budget past which `check:fast` warns, so a warning is what pruning
 * can't reach, a build. Each store's cap alone let three worktrees' stores
 * sum past the budget, which no prune took a byte from (#2163). */
export const heldBytes = 0.9 * budgetBytes;

/** This checkout's check marked started now, `workingSetSince`'s mark. */
function markCheckStarted(mark = lastCheckMark) {
  mkdirSync(join(mark, ".."), { recursive: true });
  closeSync(openSync(mark, "a"));
  const now = new Date();
  utimesSync(mark, now, now);
}

/** `stores`, inside the checkouts, which take `total` bytes together, held
 * to `held`: past it, the entries their `kept` gives that are stale, unused
 * since `since`, the least recently used of all of them first, removed
 * until the total is under it or none is left, and nothing kept below the
 * disk's floor. The working set, what was used since `since`, is never
 * removed to meet the budget: where the rest can't fit without it, the
 * stale go and what's left is said, with its holders (#2163's regression:
 * idle worktrees' builds past the budget made every check's measure remove
 * every corpus run it was about to reuse). Only the disk's floor takes from
 * it, the least recently used first. A build, and a store whose entries
 * another holds or an index counts, keep what they have, and count. What
 * it removed, and the total left. */
export function holdToBudget(
  stores: Store[],
  total: number,
  { held = heldBytes, free = () => freeBytes(checkout), since = workingSetSince() }: { held?: number; free?: () => number; since?: number } = {},
) {
  if (total <= held) return { removed: 0, size: total };
  const inside = stores.filter((store) => !store.outside);
  const listed = inside.map((store) => ({ entries: store.kept?.() ?? [], remove: store.remove ?? removeWhole }));
  const kept = listed.flatMap(({ entries }) => entries);
  // Each removed as its store removes it: a crate's project and the
  // Crater's work only under its claim, as a run using it holds it (#2562).
  const removers = new Map(listed.flatMap(({ entries, remove }) => entries.map((entry) => [entry, remove] as const)));
  const remove = (entry: Kept) => removers.get(entry)!(entry);
  const rest = total - kept.reduce((n, entry) => n + entry.bytes, 0);
  const left = holdKept(kept, { past: held - rest - workingBytes(kept, since), free, since, remove });
  return { removed: left.removed, size: rest + left.size };
}

/** The repo's exact Rust toolchains, `1.100.0` (ADR 0109): what its
 * `rust-toolchain.toml` and the scripts that pin one name. */
export function pinnedToolchains(root = checkout): Set<string> {
  const files = [
    join(root, "rust-toolchain.toml"),
    ...readdirSync(join(root, "scripts")).filter((f) => f.endsWith(".ts")).map((f) => join(root, "scripts", f)),
  ];
  const pins = new Set<string>();
  for (const file of files) {
    for (const [pin] of readFileSync(file, "utf8").matchAll(/\b(?:1\.\d+\.\d+|nightly-\d{4}-\d{2}-\d{2})\b/g)) pins.add(pin);
  }
  // Only a toolchain file's channel is one for certain; a script's are
  // versions it names, a crate's among them, whose toolchain no one installs.
  const channel = /channel\s*=\s*"([^"]+)"/.exec(readFileSync(join(root, "rust-toolchain.toml"), "utf8"))?.[1];
  if (channel) pins.add(channel);
  return pins;
}

/** Of rustup's toolchains, `names` as `rustup toolchain list` gives them,
 * those of an exact release or dated nightly that `pins` doesn't name: an
 * old pin, which development installed and nothing uses now. A floating
 * channel's, `stable` or `nightly`, is the user's own, never one. */
export function unpinnedToolchains(names: string[], pins: Set<string>): string[] {
  return names
    .map((line) => line.split(/\s/)[0])
    .filter((name) => {
      const version = /^(\d+\.\d+\.\d+|nightly-\d{4}-\d{2}-\d{2})-/.exec(name)?.[1];
      return version !== undefined && !pins.has(version);
    });
}

/** Cargo's home, its registry's downloads and sources: the crater reads
 * a crate's source there. */
export { cargoHome };
const rustupHome = process.env.RUSTUP_HOME ?? join(homedir(), ".rustup");
const bunCache = process.env.BUN_INSTALL_CACHE_DIR ?? join(process.env.BUN_INSTALL ?? join(homedir(), ".bun"), "install", "cache");
/** Cargo's downloads, its registry's crates and sources and its git
 * checkouts, which its own GC holds: 2 GiB. */
export const cargoCap = 2 * GiB;
/** Bun's install cache, every worktree's packages: 4 GiB. */
export const bunCap = 4 * GiB;
/** The temp root: a tool's leftovers, a day of Vitest's module caches. */
export const tempCap = 2 * GiB;
/** The mutations' worktree and its build. */
export const mutantsCap = 4 * GiB;

/** sccache's cache, every worktree's dependencies' builds (DEVELOPMENT.md,
 * "A compile cache shared by the worktrees"), where it keeps it unless
 * `SCCACHE_DIR` says, and its config, which its server reads as it starts,
 * unless `SCCACHE_CONF` says. */
const darwin = process.platform === "darwin";
export const sccacheDir = process.env.SCCACHE_DIR ?? (darwin ? join(homedir(), "Library", "Caches", "Mozilla.sccache") : join(homedir(), ".cache", "sccache"));
export const sccacheConfig = process.env.SCCACHE_CONF ?? (darwin ? join(homedir(), "Library", "Application Support", "Mozilla.sccache", "config") : join(homedir(), ".config", "sccache", "config"));

/** The first line of an sccache config this writes, by which it knows one
 * it wrote from one a person did. */
export const sccacheWritten = "# Written by rust-js's `bun run disk --prune` (scripts/disk.ts): sccache's cap, sized by the disk (#2505).";

/** sccache's config held to `cap`, of `had`, the config's text where there's
 * one: `text` to write, where there's none or it's one this wrote with
 * another cap; neither where it's as it should be; `own`, a person's
 * config, which stays as they wrote it. sccache's own default is 10 GiB on
 * any disk, more than a 256 GB Mac spares and less than a 1 TB one does. */
export function sccacheConfigText(had: string | undefined, cap: number): { text?: string; own?: true } {
  const size = `size = ${Math.floor(cap)}`;
  if (had !== undefined && !had.startsWith(sccacheWritten)) return { own: true };
  if (had !== undefined && had.split("\n").includes(size)) return {};
  return { text: `${sccacheWritten}\n[cache.disk]\n${size}\n` };
}

/** sccache's cap, `disk.sccache`, written to its config where sccache is
 * used here (its cache is there): its server takes it as it next starts,
 * past its 10 idle minutes, and holds its cache to it then, the least
 * recently used going first. `SCCACHE_CACHE_SIZE` in a server's
 * environment wins over it. What it did. */
export function sizeSccache(cap = disk.sccache, dir = sccacheDir, config = sccacheConfig): "unused" | "own" | "as it was" | "written" {
  if (!existsSync(dir)) return "unused";
  const { text, own } = sccacheConfigText(existsSync(config) ? readFileSync(config, "utf8") : undefined, cap);
  if (own) return "own";
  if (text === undefined) return "as it was";
  mkdirSync(join(config, ".."), { recursive: true });
  writeFileSync(config, text);
  return "written";
}

/** Every store development keeps on this machine, the checkouts' and the
 * tools' own, each with its cap and how it's held to it. */
export function stores(): Store[] {
  const main = mainCheckoutOf(checkout);
  const worktrees = worktreeRoots(checkout);
  const inChecks: Store[] = [
    ...keptStores
      .filter(({ dir }) => dir.startsWith(machineStore))
      // Held by `pruneKeptStores`, with the kept apps.
      .map(({ dir, past }) => ({ name: `.cache/${dir.slice(machineStore.length + 1)}`, dir, past, kept: () => keptBuilds(dir) })),
    // Held by their count, a map a commit of `main`'s, each superseded by
    // the next: its cap is what that many of the newest one's size take.
    { name: ".cache/test-maps", dir: mapStore, cap: mapsCap(mapStore), bound: (free) => void pruneMaps(mapStore, keptMaps, free) },
    ...[
      { name: ".cache/tasks", dir: join(machineStore, "tasks"), past: 16 * 2 ** 20 },
      { name: ".cache/job-times", dir: jobTimesStore, past: jobTimesCap },
      { name: ".cache/check-logs", dir: checkLogsStore, past: checkLogsCap },
      { name: ".cache/sanity-runs", dir: sanityRunsStore, past: sanityRunsCap },
      { name: ".cache/rustlantis", dir: rustlantisStore, past: rustlantisCap },
    ].map(({ name, dir, past }): Store => ({ name, dir, past, bound: (free) => void holdEntries(dir, past, free), kept: () => entriesOf(dir) })),
    { name: "the temp root", dir: tempRoot, past: tempCap, bound: (free) => void holdEntries(tempRoot, tempCap, free) },
  ];
  // What rustc writes of a long type's name, or of its own crash, where it
  // runs, the checkout's root: kept a day, for a crash's reader. The loop
  // writes neither there now (ADR 1118, amended, #1466); these are an
  // older commit's runs', or a `rustc` run by hand's.
  const rustcNotes = (name: string) => /\.long-type-\d+\.txt$|^rustc-ice-.*\.txt$/.test(name);
  inChecks.push({
    name: "rustc's notes at the root",
    dir: checkout,
    cap: 0,
    entries: rustcNotes,
    bound: () => {
      for (const name of readdirSync(checkout).filter(rustcNotes)) {
        const seen = statSync(join(checkout, name), { throwIfNoEntry: false });
        if (seen !== undefined && Date.now() - seen.mtimeMs > 86_400_000) rmSync(join(checkout, name), { force: true });
      }
    },
  });
  // Each worktree's `target/`: its stores, each of their own, and its build.
  for (const tree of worktrees.length ? worktrees : [main]) {
    const at = (path: string) => join(tree, "target", path);
    const own = tree === checkout;
    // Another checkout's stores, while a run there holds a lease, are its
    // to bound: one it's using, a crate's project mid-build, isn't taken
    // from under it (scripts/leases.ts). Counted all the same.
    const bounded = own || !leasedIn(tree);
    const label = (path: string) => (tree === main ? `target/${path}` : `${resolve(tree).split("/").pop()}/target/${path}`);
    inChecks.push(
      { name: label("crate-corpus"), dir: at("crate-corpus"), cap: crateProjectsCap, bound: bounded ? (free) => void boundCrateProjects({ dir: at("crate-corpus"), free }) : undefined, kept: bounded ? () => crateProjectsKept(at("crate-corpus")) : undefined, remove: removeClaimed },
      { name: label("crater"), dir: at("crater"), cap: craterCap, bound: bounded ? (free) => void boundCrater({ dir: at("crater"), free }) : undefined, kept: bounded ? () => craterKept(at("crater")) : undefined, remove: removeClaimed },
      { name: label("kept-apps"), dir: at("kept-apps"), cap: keptAppsCap },
      { name: label("compile-cache"), dir: at("compile-cache"), cap: compileCap(), bound: own ? (free) => void pruneCompiles({ root: compileCache, free }) : undefined },
      mutantsStore(tree, label("mutants"), bounded),
      { name: label("native-sources"), dir: at("native-sources"), past: 64 * 2 ** 20 },
      // The npm crates' packs, one key's, which their packer keeps alone
      // (scripts/package-npm-crates.ts): under 1 MiB.
      { name: label("npm-crates"), dir: at("npm-crates"), past: 16 * 2 ** 20 },
      { name: label("the build"), dir: join(tree, "target"), entries: (name) => !storedIn.has(name) },
    );
  }
  const outside: Store[] = [
    { name: "rustup's toolchains", dir: join(rustupHome, "toolchains"), outside: true, bound: () => {
      const listed = spawnSync("rustup", ["toolchain", "list"], { encoding: "utf8" }).stdout.split("\n").filter(Boolean);
      for (const name of unpinnedToolchains(listed, pinnedToolchains())) spawnSync("rustup", ["toolchain", "uninstall", name], { stdio: "inherit" });
    } },
    { name: "Cargo's downloads", dir: join(cargoHome, "registry"), cap: cargoCap, outside: true, bound: () => {
      // Cargo's own GC, its sizes unstable: on the repo's toolchain, which
      // its bootstrap lets take them (ADR 0109).
      const pinned = /channel\s*=\s*"([^"]+)"/.exec(readFileSync(join(checkout, "rust-toolchain.toml"), "utf8"))?.[1];
      spawnSync("cargo", [`+${pinned}`, "-Zgc", "clean", "gc", `--max-download-size=${cargoCap / GiB}GiB`], { cwd: tempRoot, env: { ...process.env, RUSTC_BOOTSTRAP: "1" }, stdio: "inherit" });
    } },
    { name: "Bun's install cache", dir: bunCache, cap: bunCap, outside: true, bound: () => {
      if (bytesOf(bunCache) > bunCap) spawnSync("bun", ["pm", "cache", "rm"], { cwd: checkout, stdio: "inherit" });
    } },
    // Held by its own server, to the cap `sizeSccache` writes it.
    { name: "sccache's cache", dir: sccacheDir, cap: disk.sccache, outside: true },
    { name: "Playwright's browsers", dir: process.env.PLAYWRIGHT_BROWSERS_PATH ?? join(homedir(), "Library", "Caches", "ms-playwright"), outside: true },
    { name: "the React port's checkouts", dir: ports, cap: portsCap, outside: true, entries: (name) => name.startsWith("react-in-rust-"), bound: (free) => void boundPorts({ free }) },
  ];
  return [...inChecks, ...outside];
}

/** The lock a checkout's mutation run holds while it builds in
 * `target/mutants` (scripts/mutations.ts). */
export const mutantsLock = (tree: string) => join(tree, "target", "mutants.lock");

/** A checkout's mutants' store, `target/mutants`, held to its cap where
 * `bounded`, but never while a mutation run there holds its
 * `mutants.lock`: that run reads the crate's snapshot it took there, and
 * builds and tests in it, throughout. A retirement's each name's check:fast
 * pruned it past the cap, its snapshot the least used entry, and the next
 * name's build read a snapshot gone (ENOENT, #2272). The run stops itself
 * at the disk's floor instead (`build`'s `belowFloor`, scripts/mutations.ts). */
export function mutantsStore(tree: string, name: string, bounded: boolean): Store {
  const dir = join(tree, "target", "mutants");
  const prunable = bounded && lockState(mutantsLock(tree)) !== "held";
  return { name, dir, cap: mutantsCap, bound: prunable ? (free) => void boundEntries(dir, mutantsCap, free) : undefined, kept: prunable ? () => entriesOf(dir) : undefined };
}

/** A worktree's `target/` entries that are stores of their own. */
const storedIn = new Set(["crate-corpus", "crater", "kept-apps", "compile-cache", "mutants", "native-sources", "npm-crates"]);

/** What `store` takes of the disk: its entries', or its directory's. */
export function bytesOfStore(store: Store): number {
  if (!existsSync(store.dir)) return 0;
  if (!store.entries) return bytesOf(store.dir);
  return readdirSync(store.dir).filter(store.entries).reduce((n, name) => n + bytesOf(join(store.dir, name)), 0);
}

/** What of `store` is in use, its working set since `since`, and its cap:
 * of a store of things, that and what it keeps past it (`holdKept`); of
 * one of builds, its `cap`, what's in use unsaid. */
export function capOf(store: Store, since: number): { inUse?: number; cap?: number } {
  if (store.past === undefined) return { cap: store.cap };
  const inUse = workingBytes(store.kept?.() ?? entriesOf(store.dir), since);
  return { inUse, cap: inUse + store.past };
}

/** The last measure of the stores inside the checkouts, `--measure`'s. */
const measured = join(machineStore, "disk.json");

const size = (n: number) => (n >= GiB ? `${(n / GiB).toFixed(1)} GiB` : `${Math.round(n / 2 ** 20)} MiB`);

/** What's said where the stores inside the checkouts are past what
 * they're held to with every entry `holdToBudget` may take gone: the
 * largest of what's left, a build mostly, each with its age, so a stale
 * worktree's is seen and someone removes it (#2163). */
export function overBudget(stores: Store[], left: number, sizeOf = bytesOfStore, ageOf = oldest): string {
  const largest = stores
    .filter((store) => !store.outside)
    .map((store) => ({ store, bytes: sizeOf(store) }))
    .sort((a, b) => b.bytes - a.bytes)
    .slice(0, 3)
    .map(({ store, bytes }) => `${store.name} ${size(bytes)}, its oldest ${ageOf(store)}`);
  return `⚠ what development keeps inside the checkouts is ${size(left)} with every store pruned, past the ${size(heldBytes)} it's held to: the largest are ${largest.join("; ")}. A worktree no one uses goes with \`git worktree remove\` (AGENTS.md)`;
}

/** The least recently changed entry of `store`, its age in days. */
function oldest(store: Store): string {
  try {
    const times = readdirSync(store.dir)
      .filter(store.entries ?? (() => true))
      .flatMap((name) => statSync(join(store.dir, name), { throwIfNoEntry: false })?.mtimeMs ?? []);
    return times.length ? `${Math.floor((Date.now() - Math.min(...times)) / 86_400_000)} d` : "-";
  } catch {
    return "-";
  }
}

if (import.meta.main) {
  const prune = process.argv.includes("--prune");
  const measure = process.argv.includes("--measure");
  const inside = process.argv.includes("--inside") || measure;
  const quiet = process.argv.includes("--quiet");
  const all = stores().filter((store) => !inside || !store.outside);
  if (prune) {
    pruneKeptStores();
    for (const store of all) store.bound?.(() => freeBytes(store.dir));
    // sccache's cap, outside the checkouts, written however the prune is
    // asked, check:fast's too, as it reads no store's entries.
    const sized = sizeSccache();
    if (sized === "written") console.log(`sccache's cap written, ${size(disk.sccache)}: its server takes it as it next starts`);
    if (sized === "own" && !quiet) console.log(`sccache's config, ${sccacheConfig}, is a person's own: its cap left as it says, where ${size(disk.sccache)} is this disk's`);
  }
  if (quiet) {
    // The last measure, made apart, as it walks `target/` and may remove
    // gigabytes: past what the stores are held to with all they may lose
    // gone, it names the largest holders (#2163).
    const last = existsSync(measured) ? (JSON.parse(readFileSync(measured, "utf8")) as { total: number; over?: string }) : { total: 0 };
    if (last.over !== undefined) console.log(last.over);
    else if (last.total > heldBytes) console.log(`⚠ what development keeps inside the checkouts was ${size(last.total)}, past 90% of its ${size(budgetBytes)} budget: \`bun run disk\` says where`);
    // The working set from this checkout's last check, read before this
    // one marks its start, so what that check used stays.
    const since = workingSetSince();
    markCheckStarted();
    spawn("bun", [import.meta.path, "--measure", `--since=${since}`], { cwd: checkout, detached: true, stdio: "ignore" }).unref();
    process.exit(0);
  }
  let total = all.filter((store) => !store.outside).reduce((n, store) => n + bytesOfStore(store), 0);
  let over: string | undefined;
  if (prune || measure) {
    const given = process.argv.find((arg) => arg.startsWith("--since="));
    total = holdToBudget(all, total, given === undefined ? {} : { since: Number(given.slice("--since=".length)) }).size;
    if (total > heldBytes) over = overBudget(all, total);
  }
  // A checkout that has kept nothing yet has no `.cache/`.
  mkdirSync(machineStore, { recursive: true });
  writeFileSync(measured, JSON.stringify({ total, at: new Date().toISOString(), ...(over === undefined ? {} : { over }) }));
  if (measure) process.exit(0);
  const sized = all.map((store) => ({ store, bytes: bytesOfStore(store) }));
  const since = workingSetSince();
  const rows = sized.map(({ store, bytes }) => {
    const { inUse, cap } = capOf(store, since);
    return [store.name, size(bytes), inUse === undefined ? "-" : size(inUse), cap === undefined ? (store.outside ? "-" : "a build") : size(cap), oldest(store), store.outside ? "outside" : ""];
  });
  for (const row of [["store", "size", "in use", "cap", "oldest", ""], ...rows]) console.log(row.map((cell, i) => (i ? cell.padStart(10) : cell.padEnd(32))).join(""));
  console.log(`\ninside the checkouts: ${size(sized.filter(({ store }) => !store.outside).reduce((n, { bytes }) => n + bytes, 0))} of the ${size(budgetBytes)} budget; the disk has ${size(freeBytes(checkout))} free, its floor ${size(floorBytes)}, of its ${size(disk.volume)}`);
  if (over !== undefined) console.log(over);
}
