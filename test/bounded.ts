// What the loop keeps on disk, bounded (DEVELOPMENT.md, "every cache is
// bounded"): the disk's floor, below which the loop's stores keep nothing,
// and where the machine's stores are. Each is held to its cap by
// tooling/bounded.js's `evict`, the one rule an app's cache is held by too.

import { homedir } from "node:os";
import { join } from "node:path";

/** The disk's floor of the loop's stores, DEVELOPMENT.md's: below it
 * they keep nothing. */
export const floorBytes = 40 * 2 ** 30;

/** Where what the loop keeps for every checkout of the machine goes,
 * outside any worktree's `target/`, so a new worktree finds it and `cargo
 * clean` keeps it: a task's times, the test maps (#932). */
export const machineStore = join(homedir(), ".cache", "rust-js");
