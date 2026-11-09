// The one way a store of kept things is held to its cap (DEVELOPMENT.md,
// "every cache is bounded"; #914, #1057): the least recently used going
// first, and none kept below its disk's floor. Each store says what its
// things are, how big and when last used, and its floor; this says which
// go. An app's cache, `rust-js build`'s, and the loop's stores
// (test/bounded.ts) are held by it alike.

import { lstatSync, readdirSync, rmSync, statfsSync } from "node:fs";
import { join } from "node:path";

/** The space free on the disk `path` is on; where it can't be asked, no
 * bound.
 * @param {string} path
 * @returns {number} */
export function freeBytes(path) {
  try {
    const fs = statfsSync(path);
    return fs.bavail * fs.bsize;
  } catch {
    return Number.POSITIVE_INFINITY;
  }
}

/** What `path` takes of the disk, a file or a directory's whole tree, by
 * the blocks each file holds: a directory of many small files takes more
 * than their bytes. A link is its own, not what it links to.
 * @param {string} path
 * @returns {number} */
export function bytesOf(path) {
  let s;
  try {
    s = lstatSync(path);
  } catch {
    return 0;
  }
  if (!s.isDirectory()) return s.blocks * 512;
  let names = [];
  try {
    names = readdirSync(path);
  } catch {}
  return names.reduce((sum, name) => sum + bytesOf(join(path, name)), s.blocks * 512);
}

/** A thing a store keeps: where, its size, and when it was last used.
 * @typedef {{ path: string, bytes: number, used: number }} Kept */

/** Hold `kept` to `cap`: the least recently used removed first, until
 * what's left is under it and the disk's `free` space is at its store's
 * `floor` or above, or nothing's left. How many it removed, and the size
 * of what's left.
 * @param {Kept[]} kept
 * @param {{ cap: number, free: () => number, floor: number }} bounds
 * @returns {{ removed: number, size: number }} */
export function evict(kept, { cap, free, floor }) {
  const oldest = [...kept].sort((a, b) => a.used - b.used);
  let size = oldest.reduce((sum, thing) => sum + thing.bytes, 0);
  let removed = 0;
  for (const thing of oldest) {
    if (size <= cap && free() >= floor) break;
    rmSync(thing.path, { recursive: true, force: true });
    size -= thing.bytes;
    removed++;
  }
  return { removed, size };
}
