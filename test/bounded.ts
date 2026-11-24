// What the loop keeps on disk, bounded (DEVELOPMENT.md, "every cache is
// bounded"): the disk's floor, below which the loop's stores keep nothing,
// and where every store is, the one owner of each path development writes
// (#1213): the machine's stores in the main checkout's `.cache/`, which
// every worktree shares; its scratch in the temp root. Each is held to its
// cap by tooling/bounded.js's `evict`, the one rule an app's cache is held
// by too, and `bun run disk` reports each.

import { existsSync, readFileSync, statSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { volumeBytes } from "../tooling/bounded.js";

/** The checkout this file is in. */
export const checkout = resolve(import.meta.dir, "..");

const GiB = 2 ** 30;

/** What the loop keeps on a disk of `volume` bytes, sized by the disk as
 * `machine()` sizes a run by its cores and memory (scripts/shard.ts), with
 * no list of machines (#2505, #2483):
 * - `floor`, below which the stores keep nothing: 8% of the volume, at
 *   least 20 GiB, room for a check's own writes beside a build and macOS's
 *   swap, and at most the 40 GiB it was on every disk, which no machine ran
 *   out above. A fixed 40 GiB was 18% of a 256 GB Mac's 228 GiB volume, so
 *   its stores kept nothing whenever it had under 40 free, their tests
 *   failing on it (#2505);
 * - `nativeCache`, what native programs' builds keep past their working set
 *   (test/kept.ts), and `sccache`, sccache's cap (scripts/disk.ts): 4% of
 *   the volume each, at least 8 GiB; the native builds' 10 GiB was the same
 *   on a 228 GiB disk as on a 1 TB one, below the Mac mini's 11.5 GiB in
 *   use (#2344);
 * - `budget`, what the stores inside the checkouts keep together
 *   (scripts/disk.ts): the native builds' cap and 30 GiB, the other stores'
 *   caps and a build of the compiler with its fixtures, the 40 GiB it was
 *   with the native builds at 10. */
export function diskProfile(volume: number): { volume: number; floor: number; nativeCache: number; sccache: number; budget: number } {
  const floor = Math.min(40 * GiB, Math.max(20 * GiB, 0.08 * volume));
  const nativeCache = Math.max(8 * GiB, 0.04 * volume);
  return { volume, floor, nativeCache, sccache: Math.max(8 * GiB, 0.04 * volume), budget: 30 * GiB + nativeCache };
}

/** This machine's: of the volume the checkout is on, where every store of
 * the loop is, `.cache/`, the worktrees, the temp root and the tools'. */
export const disk = diskProfile(volumeBytes(checkout));

/** The disk's floor of the loop's stores, DEVELOPMENT.md's: below it
 * they keep nothing (`diskProfile`). */
export const floorBytes = disk.floor;

/** The main checkout, whose `.git` is the common one: a worktree's own
 * `.git` is a file naming its directory there, whose `commondir` names the
 * common one (`git rev-parse --git-common-dir`, read without a process). */
export function mainCheckoutOf(dir: string): string {
  const git = join(dir, ".git");
  if (!existsSync(git) || statSync(git).isDirectory()) return dir;
  const named = /^gitdir: (.*)$/m.exec(readFileSync(git, "utf8"))?.[1]?.trim();
  if (!named) return dir;
  const own = resolve(dir, named);
  const common = join(own, "commondir");
  return existsSync(common) ? checkoutOfGitDir(resolve(own, readFileSync(common, "utf8").trim())) : dir;
}

/** The checkout a repository's git dir is the metadata of: the one its
 * `core.worktree` names, resolved from it, a submodule's
 * `<parent>/.git/modules/<name>`'s, which `git worktree list` gives as
 * its main worktree itself; else the directory that holds it, `.git`'s
 * (#1686). */
export function checkoutOfGitDir(gitDir: string): string {
  const config = join(gitDir, "config");
  const named = existsSync(config) ? /^\s*worktree\s*=\s*(.+?)\s*$/m.exec(readFileSync(config, "utf8"))?.[1] : undefined;
  return named ? resolve(gitDir, named) : dirname(gitDir);
}

/** Where what the loop keeps for every checkout of the machine goes: the
 * main checkout's `.cache/` (gitignored), outside any worktree's `target/`,
 * so a new worktree finds it and `cargo clean` keeps it, and inside the
 * project, where what development writes is seen (#1213): a task's times,
 * the test maps (#932), native programs' builds and verified runs (#1047). */
export const machineStore = join(mainCheckoutOf(checkout), ".cache");

/** Where the machine's stores were before they were the main checkout's
 * (#1213): `adoptRetiredStores` moves each one there, once, so no machine
 * keeps an orphan. */
export const retiredMachineStore = join(homedir(), ".cache", "rust-js");

/** The machine's log of the dev loop's times, outside the checkouts, a
 * line a run, a few KB a week (scripts/devTimes.ts writes it). */
export const devTimesLog = process.env.RUST_JS_DEV_TIMES ?? join(homedir(), "rust-js-dev-times.log");

/** The temp root of what the tests and their tools make and leave, the
 * system's temp directory's `rust-js-dev/`: every test process's `TMPDIR`
 * (test/setup.ts), so what a tool leaves there, Vitest's module cache of
 * each run (#1213), is the loop's, bounded. Outside the checkout, as a
 * fixture made there is an app outside any checkout of rust-js. */
export const tempRoot = join(process.env.RUST_JS_SYSTEM_TMPDIR ?? tmpdir(), "rust-js-dev");

/** Where Cargo keeps what it downloads, its registry among it: the one
 * place, which `bun run disk` bounds (scripts/disk.ts) and the tests name a
 * registry crate's native files under (test/support.ts's `shippedStd`,
 * #2325). */
export const cargoHome = process.env.CARGO_HOME ?? join(homedir(), ".cargo");
