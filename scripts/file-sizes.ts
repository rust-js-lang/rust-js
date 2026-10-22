// Every compiler source file and its line count, largest first, then each
// folder's total: where the code is growing, to read before a file gets too
// big to hold in your head. Files are git's, so nothing built or ignored.
//
//   bun run file-sizes             the compiler: src/ and build.rs
//   bun run file-sizes src/lower   any files or folders instead
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";

const root = join(import.meta.dir, "..");
const SOURCE = /\.(rs|ts|tsx|js|mjs)$/;
// A file past this many lines is listed as one to split.
export const LARGE = 2000;

// The lines in a file, as `wc -l` counts them plus an unterminated last one.
export const lineCount = (text: string) => text.split("\n").length - (text.endsWith("\n") || text === "" ? 1 : 0);

// The tracked source files under `paths`, relative to the repository, with
// their line counts, largest first.
export const fileSizes = (paths: string[]): [string, number][] => {
  const listed = Bun.spawnSync(["git", "ls-files", "-z", "--", ...paths], { cwd: root });
  if (listed.exitCode !== 0) throw new Error(listed.stderr.toString());
  return listed.stdout
    .toString()
    .split("\0")
    .filter(file => SOURCE.test(file))
    .map(file => [file, lineCount(readFileSync(join(root, file), "utf8"))] as [string, number])
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
};

if (import.meta.main) {
  const paths = process.argv.slice(2);
  const sizes = fileSizes(paths.length ? paths : ["src", "build.rs"]);
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
}
