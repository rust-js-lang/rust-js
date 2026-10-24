// A decision's number (docs/decisions/README.md), which several changes
// made at once may each take:
//
//   bun run adr:next                    # the next number neither main nor this branch has
//   bun run adr:renumber <from> <to>    # move this branch's decision <from> to <to>
//
// `next` reads `main` as it's pushed, fetched first, and this checkout, so
// a number another change pushed is taken. `renumber` moves a decision
// this branch added, its file and its title, and rewrites the number in
// each line this branch added since it left `main`: the lines that name a
// number `main` gave another decision are `main`'s, and left as they are.

import { readdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const dir = "docs/decisions";

function git(args: string[]): string {
  const run = Bun.spawnSync(["git", ...args], { cwd: root });
  if (run.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${run.stderr.toString().trim()}`);
  return run.stdout.toString();
}

/** The numbers a list of decision files' names take. */
export function numbers(names: string[]): number[] {
  return names.flatMap((name) => (/^(\d{4})-.*\.md$/.exec(name) ? [Number(name.slice(0, 4))] : []));
}

/** The next number neither list takes: one past the highest. */
export function nextNumber(...lists: number[][]): string {
  return String(Math.max(0, ...lists.flat()) + 1).padStart(4, "0");
}

/** `line` with the decision number `from` as `to`, where it's a number
 * of its own, not part of a longer one: `ADR 0774`, `0774-room.md`. */
export function renumbered(line: string, from: string, to: string): string {
  return line.replace(new RegExp(`(?<!\\d)${from}(?!\\d)`, "g"), to);
}

/** Of a `git diff -U0`, each file's added lines' numbers, in the new file. */
export function addedLines(diff: string): Map<string, Set<number>> {
  const added = new Map<string, Set<number>>();
  let file = "";
  for (const line of diff.split("\n")) {
    const path = /^\+\+\+ b\/(.*)$/.exec(line);
    if (path) {
      file = path[1];
      continue;
    }
    const hunk = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@/.exec(line);
    if (hunk && file) {
      const [start, count] = [Number(hunk[1]), hunk[2] === undefined ? 1 : Number(hunk[2])];
      const lines = added.get(file) ?? new Set<number>();
      for (let n = start; n < start + count; n++) lines.add(n);
      added.set(file, lines);
    }
  }
  return added;
}

function onMain(): string[] {
  git(["fetch", "--quiet", "origin", "main"]);
  return git(["ls-tree", "--name-only", "origin/main", `${dir}/`]).split("\n").map((path) => path.slice(dir.length + 1));
}

function next() {
  console.log(nextNumber(numbers(onMain()), numbers(readdirSync(join(root, dir)))));
}

function renumber(from: string, to: string) {
  if (!/^\d{4}$/.test(from) || !/^\d{4}$/.test(to)) throw new Error("usage: bun run adr:renumber <from> <to>, each four digits");
  const main = onMain();
  const local = readdirSync(join(root, dir)).filter((name) => name.startsWith(`${from}-`) && name.endsWith(".md"));
  const ours = local.filter((name) => !main.includes(name));
  if (ours.length !== 1) throw new Error(`no decision ${from} of this branch's own: ${local.join(", ") || "none"} (main's are main's)`);
  if (numbers(main).includes(Number(to)) || numbers(readdirSync(join(root, dir))).includes(Number(to))) {
    throw new Error(`${to} is taken: bun run adr:next gives a free one`);
  }
  const [name] = ours;
  const moved = name.replace(from, to);
  // Each line this branch added since it left `main`, in what's checked out
  // now, committed or not.
  const base = git(["merge-base", "HEAD", "origin/main"]).trim();
  const added = addedLines(git(["diff", "-U0", "--no-color", base, "--", "."]));
  let rewritten = 0;
  for (const [file, lines] of added) {
    const path = join(root, file);
    let text: string;
    try {
      text = readFileSync(path, "utf8");
    } catch {
      continue;
    }
    const out = text.split("\n").map((line, i) => (lines.has(i + 1) ? renumbered(line, from, to) : line));
    if (out.join("\n") !== text) {
      writeFileSync(path, out.join("\n"));
      rewritten++;
    }
  }
  renameSync(join(root, dir, name), join(root, dir, moved));
  git(["add", "-A", `${dir}/${name}`, `${dir}/${moved}`]);
  console.log(`${name} is ${moved}; ${rewritten} file${rewritten === 1 ? "" : "s"} of this branch's lines renumbered: read the diff`);
}

if (import.meta.main) {
  const [command, ...args] = process.argv.slice(2);
  try {
    if (command === "next") next();
    else if (command === "renumber") renumber(args[0] ?? "", args[1] ?? "");
    else {
      console.error("usage: bun scripts/adr.ts next | renumber <from> <to>");
      process.exit(2);
    }
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  }
}
