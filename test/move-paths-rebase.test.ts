// move-paths' rebase mode (scripts/move-paths-rebase.ts): a branch made on
// the layout before a move, rebased onto it, has its own lines rewritten,
// each path resolved where it was written and written for where it is;
// main's lines never, prose that names an old path on purpose too (#2294's
// P1: a sweep of every file rewrote five of main's settled ones).
import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { pathPairs, rebase } from "../scripts/move-paths-rebase";

const git = (at: string, ...args: string[]) => {
  const run = Bun.spawnSync(["git", "-c", "user.name=fixture", "-c", "user.email=fixture@example.com", ...args], { cwd: at });
  if (run.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${run.stderr}`);
  return run.stdout.toString().trim();
};
const write = (at: string, files: Record<string, string>) => {
  for (const [file, text] of Object.entries(files)) {
    mkdirSync(dirname(join(at, file)), { recursive: true });
    writeFileSync(join(at, file), text);
  }
};
const manifest = (name: string, dependencies: string[] = [], members?: string[]) =>
  `[package]\nname = "${name}"\nversion = "0.0.0"\nedition = "2024"\n` +
  (members ? `\n[workspace]\nmembers = [${members.map((m) => `"${m}"`).join(", ")}]\n` : "") +
  `\n[dependencies]\n${dependencies.map((d) => `${d} = { path = "crates/${d}" }`).join("\n")}\n`;

// The layout before: the binary holds the lowering, `src/lower.rs`, with
// `iterators` and `mir` below it; owned output is a crate.
const before: Record<string, string> = {
  "Cargo.toml": manifest("app", ["app-program"], ["crates/app-program"]),
  "crates/app-program/Cargo.toml": manifest("app-program"),
  "crates/app-program/src/lib.rs": "pub mod js;\n",
  "crates/app-program/src/js.rs": "pub struct Name;\n",
  "src/main.rs": "mod lower;\n\nfn main() {\n    lower::run();\n}\n",
  "src/lower.rs": "pub mod iterators;\npub mod mir;\n\npub fn run() {}\n",
  "src/lower/iterators.rs": "pub(in crate::lower) fn step() {}\n",
  "src/lower/mir.rs": "pub fn plain() {}\n",
  "scripts/mutations/lower/iterators.ts": `export const mutations = [\n  {\n    name: "step-dropped",\n    file: "src/lower/iterators.rs",\n    find: "pub(in crate::lower) fn step() {}\\n",\n    replace: "",\n  },\n];\n`,
  "test/a.test.ts": `const a = "src/lower/iterators.rs";\n`,
};

test("a rebased branch's own lines are written for the moves, and main's are left as they are", () => {
  const at = mkdtempSync(join(tmpdir(), "move-paths-rebase "));
  try {
    git(at, "init", "-q", "-b", "main");
    write(at, before);
    git(at, "add", "-A");
    git(at, "commit", "-qm", "before");
    const base = git(at, "rev-parse", "HEAD");

    // Main: the lowering is two crates, its base and MIR's; and a line
    // of its own that a sweep took for a moved module's, `js::Name` of
    // the root's import, and prose naming the old path on purpose.
    mkdirSync(join(at, "crates/app-lower/src"), { recursive: true });
    mkdirSync(join(at, "crates/app-mir/src"), { recursive: true });
    git(at, "mv", "src/lower.rs", "crates/app-lower/src/lib.rs");
    git(at, "mv", "src/lower/iterators.rs", "crates/app-lower/src/iterators.rs");
    git(at, "mv", "src/lower/mir.rs", "crates/app-mir/src/lib.rs");
    const settled =
      "pub mod iterators;\n\nuse app_program::js;\n\n/// It was `src/lower.rs`, and `crate::lower::run` its entry.\npub fn run() {}\n\npub fn name() -> js::Name {\n    js::Name\n}\n";
    write(at, {
      "Cargo.toml": manifest("app", ["app-program", "app-lower", "app-mir"], ["crates/app-program", "crates/app-lower", "crates/app-mir"]),
      "crates/app-lower/Cargo.toml": manifest("app-lower", ["app-program"]).replaceAll("crates/", "../"),
      "crates/app-mir/Cargo.toml": manifest("app-mir", ["app-lower"]).replaceAll("crates/", "../"),
      "crates/app-lower/src/lib.rs": settled,
      "src/main.rs": "fn main() {\n    app_lower::run();\n}\n",
      "scripts/mutations/lower/iterators.ts": before["scripts/mutations/lower/iterators.ts"]!.replace("src/lower/iterators.rs", "crates/app-lower/src/iterators.rs").replace("pub(in crate::lower)", "pub(crate)"),
    });
    git(at, "add", "-A");
    git(at, "commit", "-qm", "the cut");
    const main = git(at, "rev-parse", "HEAD");

    // The branch, on the layout before: new lines in a moved file, MIR's
    // naming the base, a test's path and a mutation.
    git(at, "checkout", "-q", "-b", "branch", base);
    write(at, {
      "src/lower/iterators.rs": "pub(in crate::lower) fn step() {}\n\n/// Steps as `crate::lower::run` does.\npub(in crate::lower) fn probe() {\n    super::run();\n    crate::lower::mir::plain();\n}\n",
      "src/lower/mir.rs": "pub fn plain() {\n    crate::lower::iterators::step();\n}\n",
      "test/a.test.ts": `const a = "src/lower/iterators.rs";\nconst b = join(root, "src/lower/mir.rs");\n`,
    });
    git(at, "commit", "-qam", "the branch");
    git(at, "rebase", "-q", main);

    const done = rebase(at, base, main);
    const read = (file: string) => readFileSync(join(at, file), "utf8");
    expect(read("crates/app-lower/src/iterators.rs")).toBe(
      "pub(in crate::lower) fn step() {}\n\n/// Steps as `crate::run` does.\npub(crate) fn probe() {\n    super::run();\n    crate::lower::mir::plain();\n}\n",
    );
    expect(read("crates/app-mir/src/lib.rs")).toBe("pub fn plain() {\n    app_lower::iterators::step();\n}\n");
    // Its own line, not main's, whose path main left as it was.
    expect(read("test/a.test.ts")).toBe(`const a = "src/lower/iterators.rs";\nconst b = join(root, "crates/app-mir/src/lib.rs");\n`);
    // Main's lines: the root's import, and its prose; and iterators.rs's
    // first, which main moved as it was, stale as that is.
    expect(read("crates/app-lower/src/lib.rs")).toBe(settled);
    // MIR from the base, where the base doesn't depend on MIR: said, left.
    expect(done.problems).toEqual([
      "crates/app-lower/src/iterators.rs:6 names app_mir's `plain` from app-lower, which doesn't depend on it: the moves put it where this crate can't reach",
    ]);
    // And again, nothing more.
    expect(rebase(at, base, main).files).toEqual([]);
  } finally {
    rmSync(at, { recursive: true, force: true });
  }
});

test("a directory's pair is where its files went, but one of them stayed, or went elsewhere", () => {
  const moves = [
    { from: "src/lower/a.rs", to: "crates/l/src/a.rs" },
    { from: "src/lower/b/c.rs", to: "crates/l/src/b/c.rs" },
    { from: "src/lower/mir/d.rs", to: "crates/m/src/d.rs" },
    { from: "src/lower/mir/e.rs", to: "crates/m/src/e.rs" },
  ];
  const pairs = pathPairs(moves, () => false, moves.map(({ from }) => from));
  expect(pairs.filter(([from]) => from.endsWith("/"))).toEqual([
    ["src/lower/mir/", "crates/m/src/"],
    ["src/lower/b/", "crates/l/src/b/"],
    ["src/lower/", "crates/l/src/"],
  ]);
  // A file that stayed keeps its directory unpaired.
  const stayed = pathPairs(moves, (file) => file === "src/lower/kept.rs", [...moves.map(({ from }) => from), "src/lower/kept.rs"]);
  expect(stayed.some(([from]) => from === "src/lower/")).toBe(false);
});
