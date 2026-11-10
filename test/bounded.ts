// What the loop keeps on disk, bounded (DEVELOPMENT.md, "every cache is
// bounded"): the disk's floor, below which the loop's stores keep nothing,
// and where every store is, the one owner of each path development writes
// (#1213): the machine's stores in the main checkout's `.cache/`, which
// every worktree shares; its scratch in the temp root. Each is held to its
// cap by tooling/bounded.js's `evict`, the one rule an app's cache is held
// by too, and `bun run disk` reports each.

import { existsSync, readFileSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

/** The disk's floor of the loop's stores, DEVELOPMENT.md's: below it
 * they keep nothing. */
export const floorBytes = 40 * 2 ** 30;

/** The checkout this file is in. */
export const checkout = resolve(import.meta.dir, "..");

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
  return existsSync(common) ? dirname(resolve(own, readFileSync(common, "utf8").trim())) : dir;
}

/** Where what the loop keeps for every checkout of the machine goes: the
 * main checkout's `.cache/` (gitignored), outside any worktree's `target/`,
 * so a new worktree finds it and `cargo clean` keeps it, and inside the
 * project, where what development writes is seen (#1213): a task's times,
 * the test maps (#932), native programs' builds and verified runs (#1047). */
export const machineStore = join(mainCheckoutOf(checkout), ".cache");

/** The temp root of what the tests and their tools make and leave, the
 * system's temp directory's `rust-js-dev/`: every test process's `TMPDIR`
 * (test/setup.ts), so what a tool leaves there, Vitest's module cache of
 * each run (#1213), is the loop's, bounded. Outside the checkout, as a
 * fixture made there is an app outside any checkout of rust-js. */
export const tempRoot = join(process.env.RUST_JS_SYSTEM_TMPDIR ?? tmpdir(), "rust-js-dev");
