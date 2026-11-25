// TESTS.md's merge: git's merge driver `changes-log` (.gitattributes), so a
// cherry-pick, a `git am -3` and a rebase each keep both sides' `Changed
// on ..` lines with no hand step (#2587). Nearly every relay's cherry-pick
// conflicted on them, a minute by hand each. `bun install` names the
// driver in git's config (package.json's `prepare`); `bun run push`
// resolves by the same rule a conflict git left it where it's unnamed.
//
//   bun scripts/merge-changes.ts %O %A %B    # git's driver: base, ours, theirs

import { writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

/** `text`, TESTS.md merged with git's diff3 conflict markers, each
 * conflict of its `Changed on ..` lines alone resolved by keeping both
 * sides' new lines, newest first: what the replayed change, theirs, added
 * that the base hadn't, then ours, the commit the rebase is onto, whole, a
 * line it rewrote, a count it raised, as it rewrote it. Undefined where a
 * conflict has no base section, or holds a line that isn't a `Changed on`
 * one or blank, the table's or prose, which stays the change's author's to
 * merge. */
export function mergedChanges(text: string): string | undefined {
  const out: string[] = [];
  const lines = text.split("\n");
  const change = (line: string) => line === "" || line.startsWith("Changed on ");
  for (let i = 0; i < lines.length; i++) {
    if (!lines[i]!.startsWith("<<<<<<< ")) {
      out.push(lines[i]!);
      continue;
    }
    const sides: [string[], string[], string[]] = [[], [], []];
    let side: 0 | 1 | 2 = 0;
    let based = false;
    for (i++; i < lines.length && !lines[i]!.startsWith(">>>>>>> "); i++) {
      const line = lines[i]!;
      if (line.startsWith("||||||| ")) (side = 1), (based = true);
      else if (line === "=======") side = 2;
      else sides[side].push(line);
    }
    if (!based) return undefined;
    const [ours, base, theirs] = sides;
    if (![...ours, ...base, ...theirs].every(change)) return undefined;
    const added = theirs.filter((line) => line !== "" && !base.includes(line));
    const kept = ours.filter((line) => line !== "");
    out.push(...[...added, ...kept].flatMap((line, at) => (at === 0 ? [line] : ["", line])));
    // The blank line that parts the last from what follows, where the
    // conflict took it.
    if ((ours.at(-1) ?? "") === "" && (theirs.at(-1) ?? "") === "") out.push("");
  }
  return out.join("\n");
}

/** git's driver of the files `base`, `ours` and `theirs`: `ours` written
 * as git merges the three, each conflict of `Changed on` lines alone
 * resolved (`mergedChanges`), and 0; where one isn't, git's merge with its
 * conflicts' markers, and 1, a conflict left as git's own leaves it. */
export function driver(base: string, ours: string, theirs: string): number {
  const merge = spawnSync("git", ["merge-file", "-p", "--diff3", "-L", "ours", "-L", "base", "-L", "theirs", ours, base, theirs], { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });
  if (merge.status === null || merge.status < 0) throw new Error(`git merge-file failed: ${merge.stderr}`);
  if (merge.status === 0) {
    writeFileSync(ours, merge.stdout);
    return 0;
  }
  const merged = mergedChanges(merge.stdout);
  writeFileSync(ours, merged ?? merge.stdout);
  return merged === undefined ? 1 : 0;
}

if (import.meta.main) {
  const [base, ours, theirs] = process.argv.slice(2);
  if (base === undefined || ours === undefined || theirs === undefined) {
    console.error("usage: bun scripts/merge-changes.ts <base> <ours> <theirs>, git's %O %A %B");
    process.exit(2);
  }
  process.exit(driver(base, ours, theirs));
}
