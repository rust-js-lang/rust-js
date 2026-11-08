// What the loop keeps on disk, bounded (DEVELOPMENT.md, "every cache is
// bounded"): the disk's floor, below which nothing's kept, and the one way
// a store of kept things is held to its cap, the least recently used going
// first. Each store says what its things are, how big and when last used;
// this says which go.

import { lstatSync, readdirSync, rmSync, type Stats, statfsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

/** The disk's floor, DEVELOPMENT.md's: below it nothing's kept. */
export const floorBytes = 40 * 2 ** 30;

/** Where what the loop keeps for every checkout of the machine goes,
 * outside any worktree's `target/`, so a new worktree finds it and `cargo
 * clean` keeps it: a task's times, the test maps (#932). */
export const machineStore = join(homedir(), ".cache", "rust-js");

/** The space free on the disk `path` is on; where it can't be asked, no
 * bound. */
export function freeBytes(path: string): number {
  try {
    const fs = statfsSync(path);
    return fs.bavail * fs.bsize;
  } catch {
    return Number.POSITIVE_INFINITY;
  }
}

/** What `path` takes of the disk, a file or a directory's whole tree, by
 * the blocks each file holds: a directory of many small files takes more
 * than their bytes. A link is its own, not what it links to. */
export function bytesOf(path: string): number {
  let s: Stats;
  try {
    s = lstatSync(path);
  } catch {
    return 0;
  }
  if (!s.isDirectory()) return s.blocks * 512;
  let names: string[] = [];
  try {
    names = readdirSync(path);
  } catch {}
  return names.reduce((sum, name) => sum + bytesOf(join(path, name)), s.blocks * 512);
}

/** A thing a store keeps: where, its size, and when it was last used. */
export type Kept = { path: string; bytes: number; used: number };

/** Hold `kept` to `cap`: the least recently used removed first, until
 * what's left is under it and the disk's `free` space is above its floor,
 * or nothing's left. How many it removed, and the size of what's left. */
export function evict(kept: Kept[], { cap, free }: { cap: number; free: () => number }): { removed: number; size: number } {
  const oldest = [...kept].sort((a, b) => a.used - b.used);
  let size = oldest.reduce((sum, thing) => sum + thing.bytes, 0);
  let removed = 0;
  for (const thing of oldest) {
    if (size <= cap && free() >= floorBytes) break;
    rmSync(thing.path, { recursive: true, force: true });
    size -= thing.bytes;
    removed++;
  }
  return { removed, size };
}
