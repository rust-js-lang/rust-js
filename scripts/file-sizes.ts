// Every compiler source file and its line count, largest first, then each
// folder's total: where the code is growing, to read before a file gets too
// big to hold in your head. Files are git's, tracked or new, so nothing
// built or ignored.
//
//   bun run file-sizes             the compiler: src/ and build.rs
//   bun run file-sizes 'test/*.ts' the tests' own code
//   bun run file-sizes src/lower   any files or folders instead
//
// And each of them over 1,350 lines that has grown since where the change
// left main, which is to be split by concept first (#1334).
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";

const root = join(import.meta.dir, "..");
const SOURCE = /\.(rs|ts|tsx|js|mjs)$/;
// The cap on a compiler file's lines, which test/architecture.test.ts holds
// (ADR 0572): a file past it is one to split by concept.
export const LARGE = 1500;
// The lines past which a file is near the cap, and doesn't grow (#1334,
// the user's rule): one that would is split by concept first, in a commit
// of its own, never squeezed to fit, its imports moved, its lines joined
// or its comments cut. test/architecture.test.ts holds it against where
// the change left main.
export const NEAR = 1350;
// What the cap is held over, the compiler, and the tests' own code: what's
// written by hand to read and change, not a snapshot the compiler wrote.
export const COMPILER = ["src", "build.rs"];
export const TESTS = ["test/*.ts"];

// The lines in a file, as `wc -l` counts them plus an unterminated last one.
export const lineCount = (text: string) => text.split("\n").length - (text.endsWith("\n") || text === "" ? 1 : 0);

// The tracked source files under `paths`, relative to the repository, with
// their line counts, largest first: of this checkout, or the one at `at`.
export const fileSizes = (paths: string[], at = root): [string, number][] => {
  const listed = Bun.spawnSync(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", ...paths], { cwd: at });
  if (listed.exitCode !== 0) throw new Error(listed.stderr.toString());
  return [...new Set(listed.stdout.toString().split("\0"))]
    // A file deleted but not yet staged is listed and gone.
    .filter(file => SOURCE.test(file) && existsSync(join(at, file)))
    .map(file => [file, lineCount(readFileSync(join(at, file), "utf8"))] as [string, number])
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
};

// Where the checkout at `at` left `main`: its merge base with `origin/main`,
// what a change is measured from. `undefined` where there's none to ask, a
// checkout with no `origin/main`.
export const mergeBase = (at = root): string | undefined => {
  const base = Bun.spawnSync(["git", "merge-base", "HEAD", "origin/main"], { cwd: at });
  return base.exitCode === 0 ? base.stdout.toString().trim() : undefined;
};

// Each file under `paths` over `NEAR` lines that has more than it had at
// `base`, a commit, or that it hadn't: the file, its lines then and now.
// Each is to be split by concept, in a commit of its own, before it grows
// (#1334). One already over that doesn't grow is left as it is: whoever
// next adds to it splits it first.
export const grownNearCap = (paths: string[], base: string, at = root): [string, number, number][] =>
  fileSizes(paths, at)
    .filter(([, lines]) => lines > NEAR)
    .map(([file, lines]): [string, number, number] => {
      const then = Bun.spawnSync(["git", "show", `${base}:${file}`], { cwd: at });
      return [file, then.exitCode === 0 ? lineCount(then.stdout.toString()) : 0, lines];
    })
    .filter(([, then, now]) => now > then);

// What the author of a grown file is told: split it by concept first.
export const splitFirst = ([file, then, now]: [string, number, number]) =>
  `${file} grew from ${then} to ${now} lines, past ${NEAR}: split it by concept in its own commit first, never squeeze it (the user's rule, #1334)`;

// Where the checkout's `origin/main` is behind `origin`'s own `main`, the
// two commits, as `origin` says where it's asked: a stale ref, past which
// others' commits the branch was rebased onto look like its own. `undefined`
// where they agree, or `origin` can't be asked, as offline.
export const staleMain = (at = root): { local: string; remote: string } | undefined => {
  const local = Bun.spawnSync(["git", "rev-parse", "origin/main"], { cwd: at });
  const remote = Bun.spawnSync(["git", "ls-remote", "origin", "refs/heads/main"], { cwd: at, timeout: 20_000 });
  if (local.exitCode !== 0 || remote.exitCode !== 0) return undefined;
  const [local_, remote_] = [local.stdout.toString().trim(), remote.stdout.toString().split(/\s/)[0]];
  return remote_ && remote_ !== local_ ? { local: local_, remote: remote_ } : undefined;
};

// What `paths` near the cap say since where the checkout left main: each
// grown file to split first; or, where any grew and `origin/main` is stale,
// that it be fetched first, as what's grown may be others' (#1334). Asks
// `origin` only where a file grew.
export const nearCapReport = (paths: string[], at = root): string[] => {
  const base = mergeBase(at);
  if (base === undefined) return [];
  const grown = grownNearCap(paths, base, at);
  if (grown.length === 0) return [];
  const stale = staleMain(at);
  if (stale) {
    return [
      `origin/main is ${stale.local.slice(0, 9)}, behind origin's main at ${stale.remote.slice(0, 9)}: what looks grown (${grown.map(([file]) => file).join(", ")}) may be others' commits, so fetch origin first and measure again (#1334)`,
    ];
  }
  return grown.map(splitFirst);
};

if (import.meta.main) {
  const paths = process.argv.slice(2);
  const sizes = fileSizes(paths.length ? paths : COMPILER);
  if (!sizes.length) throw new Error(`no source files under ${paths.join(" ")}`);
  const row = (lines: number, name: string) => `${String(lines).padStart(7)}  ${name}`;

  console.log("## Files, in lines");
  for (const [file, lines] of sizes) console.log(row(lines, file));

  const folders = new Map<string, [number, number]>();
  for (const [file, lines] of sizes) {
    const [files, total] = folders.get(dirname(file)) ?? [0, 0];
    folders.set(dirname(file), [files + 1, total + lines]);
  }
  console.log("\n## Folders, in lines (files)");
  for (const [folder, [files, total]] of [...folders].sort((a, b) => b[1][1] - a[1][1])) {
    console.log(row(total, `${folder} (${files})`));
  }

  const large = sizes.filter(([, lines]) => lines > LARGE);
  const total = sizes.reduce((sum, [, lines]) => sum + lines, 0);
  console.log(`\n## Over ${LARGE} lines: ${large.length} of ${sizes.length} files`);
  for (const [file, lines] of large) console.log(row(lines, file));
  console.log(`\n${sizes.length} files, ${total} lines, ${Math.round(total / sizes.length)} a file on average`);
  const base = mergeBase();
  if (base) {
    const report = nearCapReport(paths.length ? paths : COMPILER);
    console.log(`\n## Over ${NEAR} lines and grown since ${base.slice(0, 9)}, where this left main: ${report.length}`);
    for (const line of report) console.log(line);
  }
}
