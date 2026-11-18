// Whether std/, `@rust-js/std` as `bun run std` makes it, is what its
// sources make now: made again in place and compared with what's
// committed. A runtime change tested against std/ as it was passes against
// the old code (#1971). The one check of it, which `bun run push`'s std
// phase and check.yml's lint job run, the path a merge on GitHub takes too.
//
//   bun scripts/std-fresh.ts    std/ made again by the compiler built,
//                               exit 1 naming each file that differs
//
// Run after the compiler's built: `bun scripts/std.ts` alone, not
// `bun run std`, whose `cargo build` each caller has done already.

import { spawnSync } from "node:child_process";
import { join } from "node:path";

const root = join(import.meta.dir, "..");

/** Each file of std/ in `dir`'s checkout that differs from what's
 * committed, as `git status --porcelain` says it: none where it's fresh. */
export function staleStd(dir = root): string[] {
  const status = spawnSync("git", ["status", "--porcelain", "--", "std"], { cwd: dir, encoding: "utf8" });
  if (status.status !== 0) throw new Error(`git status failed in ${dir}:\n${status.stderr}`);
  return status.stdout.split("\n").filter((line) => line.trim() !== "");
}

/** What a stale std/ says, naming the fix. */
export function staleSaid(stale: string[]): string {
  return `std/ isn't what its sources make now; made again from this tree, these differ:\n${stale.join("\n")}\nrun \`bun run std\` and commit std/`;
}

if (import.meta.main) {
  const made = spawnSync("bun", ["scripts/std.ts"], { cwd: root, stdio: "inherit" });
  if (made.status !== 0) process.exit(made.status ?? 1);
  const stale = staleStd();
  if (stale.length > 0) {
    console.error(staleSaid(stale));
    process.exit(1);
  }
  console.log("std/ is what its sources make");
}
