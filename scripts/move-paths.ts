// Moving source files without breaking what names them (#2294): each
// `<from> <to>` pair, a file or a directory's prefix, rewritten wherever a
// path to it is written, in the scripts, the tests and their catalogs, the
// docs, the sources' own comments and CI's workflows, and a moved file's
// mutation list (scripts/mutations/<path>.ts) moved beside it, its `file:`
// fields with the rest. Re-runnable: a pair already applied rewrites
// nothing. The files themselves are moved by `git mv`, before or after.
//
//   bun scripts/move-paths.ts src/lower/iterators/mir_steps/ src/lower/iterators/mir_steps/
//   bun scripts/move-paths.ts --check src/lower/iterators/mir_steps/   # what still names it

import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";

const root = join(import.meta.dir, "..");
const searched = ["scripts", "test", "docs", "src", ".github", "DEVELOPMENT.md", "CONTRIBUTING.md", "TESTS.md", "AGENTS.md"];
const skipped = new Set(["node_modules", "corpus", "known", "fixtures", "target"]);

/** Each text file under `searched`, by its path from the checkout. */
export function textFiles(at = root): string[] {
  const out: string[] = [];
  const walk = (path: string) => {
    const full = join(at, path);
    if (!existsSync(full)) return;
    if (statSync(full).isDirectory()) {
      for (const name of readdirSync(full)) if (!skipped.has(name)) walk(join(path, name));
    } else if (/\.(ts|js|rs|md|txt|ya?ml|json)$/.test(path)) out.push(path);
  };
  for (const path of searched) walk(path);
  return out;
}

/** `text` with each of `pairs`' `from` written as its `to`: a path, the
 * whole of it or a prefix of one, never a longer name it begins (a `from`
 * of `a/b.rs` leaves `a/b.rsx`). */
export function rewritten(text: string, pairs: [string, string][]): string {
  let out = text;
  for (const [from, to] of pairs) {
    const escaped = from.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const boundary = from.endsWith("/") ? "" : "(?![\\w.-])";
    out = out.replace(new RegExp(`${escaped}${boundary}`, "g"), to);
  }
  return out;
}

/** The mutation list of a source file, scripts/mutations/<path below src/>.ts. */
const listOf = (path: string) => join("scripts", "mutations", path.replace(/^src\//, "").replace(/\.rs$/, ".ts"));

if (import.meta.main) {
  const args = process.argv.slice(2);
  if (args[0] === "--check") {
    const named = textFiles().filter((file) => readFileSync(join(root, file), "utf8").includes(args[1]!));
    console.log(named.length === 0 ? `nothing names ${args[1]}` : named.join("\n"));
    process.exit(named.length === 0 ? 0 : 1);
  }
  if (args.length === 0 || args.length % 2 !== 0) throw new Error("usage: bun scripts/move-paths.ts <from> <to> [<from> <to> ..]");
  const pairs: [string, string][] = [];
  for (let i = 0; i < args.length; i += 2) pairs.push([args[i]!, args[i + 1]!]);
  // A moved file's mutation list, moved beside it, as each list is its
  // file's (test/mutations.test.ts); the lists' paths rewritten with the rest.
  for (const [from, to] of [...pairs]) {
    const lists = from.endsWith("/") ? [join("scripts", "mutations", from.replace(/^src\//, ""))] : [listOf(from)];
    for (const list of lists) {
      const target = from.endsWith("/") ? join("scripts", "mutations", to.replace(/^src\//, "")) : listOf(to);
      if (!existsSync(join(root, list))) continue;
      mkdirSync(dirname(join(root, target)), { recursive: true });
      renameSync(join(root, list), join(root, target));
      // Each list's import of the catalog, scripts/mutations.ts, from where
      // it now is.
      const moved = statSync(join(root, target)).isDirectory() ? readdirSync(join(root, target), { recursive: true }).map((name) => join(target, String(name))).filter((name) => name.endsWith(".ts")) : [target];
      for (const each of moved) {
        const text = readFileSync(join(root, each), "utf8");
        const catalog = `${relative(dirname(join(root, each)), join(root, "scripts"))}/mutations`;
        writeFileSync(join(root, each), text.replace(/from "(?:\.\.\/)+mutations"/g, `from "${catalog}"`));
      }
      console.log(`moved ${list} to ${target}`);
    }
    // Its directory's lists named by it, `../../mutations` imports among
    // them, rewritten below with the rest.
    pairs.push([from.endsWith("/") ? join("scripts", "mutations", from.replace(/^src\//, "")) + "/" : listOf(from), from.endsWith("/") ? join("scripts", "mutations", to.replace(/^src\//, "")) + "/" : listOf(to)]);
  }
  let changed = 0;
  for (const file of textFiles()) {
    const text = readFileSync(join(root, file), "utf8");
    const next = rewritten(text, pairs);
    if (next !== text) {
      writeFileSync(join(root, file), next);
      changed++;
    }
  }
  console.log(`rewrote ${changed} files`);
}
