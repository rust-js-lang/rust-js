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
//                           used of all their entries first (#2163), as
//                           walking `target/` takes 3.5 s and removing
//                           gigabytes a minute

import { spawn, spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { bytesOf, evict, freeBytes, type Kept } from "../tooling/bounded.js";
import { checkout, floorBytes, machineStore, mainCheckoutOf, tempRoot } from "../test/bounded";
import { checkLogsCap, checkLogsStore } from "./checkLogs";
import { leasedIn } from "./leases";
import { worktreeRoots } from "./strays";
import { sanityRunsCap, sanityRunsStore } from "./select";
import { jobTimesCap, jobTimesStore } from "./jobTimes";
import { compileCache, capBytes as compileCap, prune as pruneCompiles } from "../test/compiles";
import { keptAppsCap, keptBuilds, keptStores, pruneKeptStores, rustlantisCap, rustlantisStore } from "../test/kept";
import { boundCrateProjects, crateProjectsCap, crateProjectsKept } from "./crate-corpus";
import { boundCrater, craterCap, craterKept } from "./crater";
import { bound as boundPorts, ports, portsCap } from "./react-in-rust";
import { keptMaps, mapStore, pruneMaps } from "./testMap";

const GiB = 2 ** 30;

/** The most every store inside the checkouts keeps together, `.cache/`,
 * the worktrees' `target/`s and the temp root: their caps, 32 GiB, and a
 * build of the compiler and its tests' fixtures, about 6 GiB, with room for
 * a second worktree's (#1213). `check:fast` warns past 90% of it. */
export const budgetBytes = 40 * GiB;

/** A store: where it is, the most it keeps (`undefined` of one that's a
 * build, which a build replaces, held only by the budget), and how it's held
 * to that, given the disk's free bytes. */
export type Store = {
  name: string;
  dir: string;
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
};

/** Each entry of `dir` held to `cap`, the least recently changed first, and
 * nothing kept below the disk's floor: a store with no bound of its own. */
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

/** `stores`, inside the checkouts, which take `total` bytes together, held
 * to `held`: past it, the entries their `kept` gives, the least recently
 * used of all of them first, removed until the total is under it or none
 * is left, and nothing kept below the disk's floor. A build, and a store
 * whose entries another holds or an index counts, keep what they have, and
 * count. What it removed, and the total left. */
export function holdToBudget(stores: Store[], total: number, { held = heldBytes, free = () => freeBytes(checkout) }: { held?: number; free?: () => number } = {}) {
  if (total <= held) return { removed: 0, size: total };
  const kept = stores.filter((store) => !store.outside).flatMap((store) => store.kept?.() ?? []);
  const theirs = kept.reduce((n, entry) => n + entry.bytes, 0);
  const rest = total - theirs;
  const { removed, size } = evict(kept, { cap: Math.max(0, held - rest), free, floor: floorBytes });
  return { removed, size: rest + size };
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
export const cargoHome = process.env.CARGO_HOME ?? join(homedir(), ".cargo");
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

/** Every store development keeps on this machine, the checkouts' and the
 * tools' own, each with its cap and how it's held to it. */
export function stores(): Store[] {
  const main = mainCheckoutOf(checkout);
  const worktrees = worktreeRoots(checkout);
  const inChecks: Store[] = [
    ...keptStores
      .filter(({ dir }) => dir.startsWith(machineStore))
      // Held by `pruneKeptStores`, with the kept apps.
      .map(({ dir, cap }) => ({ name: `.cache/${dir.slice(machineStore.length + 1)}`, dir, cap, kept: () => keptBuilds(dir) })),
    { name: ".cache/test-maps", dir: mapStore, cap: keptMaps * 64 * 2 ** 20, bound: (free) => void pruneMaps(mapStore, keptMaps, free) },
    { name: ".cache/tasks", dir: join(machineStore, "tasks"), cap: 16 * 2 ** 20, bound: (free) => void boundEntries(join(machineStore, "tasks"), 16 * 2 ** 20, free), kept: () => entriesOf(join(machineStore, "tasks")) },
    { name: ".cache/job-times", dir: jobTimesStore, cap: jobTimesCap, bound: (free) => void boundEntries(jobTimesStore, jobTimesCap, free), kept: () => entriesOf(jobTimesStore) },
    { name: ".cache/check-logs", dir: checkLogsStore, cap: checkLogsCap, bound: (free) => void boundEntries(checkLogsStore, checkLogsCap, free), kept: () => entriesOf(checkLogsStore) },
    { name: ".cache/sanity-runs", dir: sanityRunsStore, cap: sanityRunsCap, bound: (free) => void boundEntries(sanityRunsStore, sanityRunsCap, free), kept: () => entriesOf(sanityRunsStore) },
    { name: ".cache/rustlantis", dir: rustlantisStore, cap: rustlantisCap, bound: (free) => void boundEntries(rustlantisStore, rustlantisCap, free), kept: () => entriesOf(rustlantisStore) },
    { name: "the temp root", dir: tempRoot, cap: tempCap, bound: (free) => void boundEntries(tempRoot, tempCap, free) },
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
      { name: label("crate-corpus"), dir: at("crate-corpus"), cap: crateProjectsCap, bound: bounded ? (free) => void boundCrateProjects({ dir: at("crate-corpus"), free }) : undefined, kept: bounded ? () => crateProjectsKept(at("crate-corpus")) : undefined },
      { name: label("crater"), dir: at("crater"), cap: craterCap, bound: bounded ? (free) => void boundCrater({ dir: at("crater"), free }) : undefined, kept: bounded ? () => craterKept(at("crater")) : undefined },
      { name: label("kept-apps"), dir: at("kept-apps"), cap: keptAppsCap },
      { name: label("compile-cache"), dir: at("compile-cache"), cap: compileCap(), bound: own ? (free) => void pruneCompiles({ root: compileCache, free }) : undefined },
      { name: label("mutants"), dir: at("mutants"), cap: mutantsCap, bound: bounded ? (free) => void boundEntries(at("mutants"), mutantsCap, free) : undefined, kept: bounded ? () => entriesOf(at("mutants")) : undefined },
      { name: label("native-sources"), dir: at("native-sources"), cap: 64 * 2 ** 20 },
      // The npm crates' packs, one key's, which their packer keeps alone
      // (scripts/package-npm-crates.ts): under 1 MiB.
      { name: label("npm-crates"), dir: at("npm-crates"), cap: 16 * 2 ** 20 },
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
    { name: "Playwright's browsers", dir: process.env.PLAYWRIGHT_BROWSERS_PATH ?? join(homedir(), "Library", "Caches", "ms-playwright"), outside: true },
    { name: "the React port's checkouts", dir: ports, cap: portsCap, outside: true, entries: (name) => name.startsWith("react-in-rust-"), bound: (free) => void boundPorts({ free }) },
  ];
  return [...inChecks, ...outside];
}

/** A worktree's `target/` entries that are stores of their own. */
const storedIn = new Set(["crate-corpus", "crater", "kept-apps", "compile-cache", "mutants", "native-sources", "npm-crates"]);

/** What `store` takes of the disk: its entries', or its directory's. */
export function bytesOfStore(store: Store): number {
  if (!existsSync(store.dir)) return 0;
  if (!store.entries) return bytesOf(store.dir);
  return readdirSync(store.dir).filter(store.entries).reduce((n, name) => n + bytesOf(join(store.dir, name)), 0);
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
  }
  if (quiet) {
    // The last measure, made apart, as it walks `target/` and may remove
    // gigabytes: past what the stores are held to with all they may lose
    // gone, it names the largest holders (#2163).
    const last = existsSync(measured) ? (JSON.parse(readFileSync(measured, "utf8")) as { total: number; over?: string }) : { total: 0 };
    if (last.over !== undefined) console.log(last.over);
    else if (last.total > heldBytes) console.log(`⚠ what development keeps inside the checkouts was ${size(last.total)}, past 90% of its ${size(budgetBytes)} budget: \`bun run disk\` says where`);
    spawn("bun", [import.meta.path, "--measure"], { cwd: checkout, detached: true, stdio: "ignore" }).unref();
    process.exit(0);
  }
  let total = all.filter((store) => !store.outside).reduce((n, store) => n + bytesOfStore(store), 0);
  let over: string | undefined;
  if (prune || measure) {
    total = holdToBudget(all, total).size;
    if (total > heldBytes) over = overBudget(all, total);
  }
  // A checkout that has kept nothing yet has no `.cache/`.
  mkdirSync(machineStore, { recursive: true });
  writeFileSync(measured, JSON.stringify({ total, at: new Date().toISOString(), ...(over === undefined ? {} : { over }) }));
  if (measure) process.exit(0);
  const sized = all.map((store) => ({ store, bytes: bytesOfStore(store) }));
  const rows = sized.map(({ store, bytes }) => [store.name, size(bytes), store.cap === undefined ? (store.outside ? "-" : "a build") : size(store.cap), oldest(store), store.outside ? "outside" : ""]);
  for (const row of [["store", "size", "cap", "oldest", ""], ...rows]) console.log(row.map((cell, i) => (i ? cell.padStart(10) : cell.padEnd(32))).join(""));
  console.log(`\ninside the checkouts: ${size(sized.filter(({ store }) => !store.outside).reduce((n, { bytes }) => n + bytes, 0))} of the ${size(budgetBytes)} budget; the disk has ${size(freeBytes(checkout))} free, its floor ${size(floorBytes)}`);
  if (over !== undefined) console.log(over);
}
