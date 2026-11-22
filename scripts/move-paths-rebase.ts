// move-paths' rebase mode (#2294): a branch rebased onto moves `main` made
// is written against the layout it started from, and only its own lines
// are. So each of them, and nothing of main's, is rewritten: a path to a
// file, as the moves' renames say, and a Rust path, resolved where it was
// written, in its file's old crate and module, then written for where that
// module is now.
//
//   crate::lower::iterators::step   (src/lower/x.rs, the binary's)
//   crate::iterators::step          (crates/rust-js-lower/src/x.rs)
//   pub(in crate::lower)            pub(crate)
//   "src/lower/iterators.rs"        "crates/rust-js-lower/src/iterators.rs"
//
// Main's own lines are never touched: they're right as they are, prose
// that names an old path on purpose too (a sweep of every file rewrote
// five of main's, #2294's P1).
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { compilerCrates, type CompilerCrate } from "./compiler-crates";
import { boundNames, codeOf, parseTree, printTree, rewritten, type Tree } from "./move-paths";

/** A file's move, by its paths from the checkout. */
export type Move = { from: string; to: string };

/** Where a module is: its crate and its path there. */
export type Place = { crate: string; path: string[] };

/** What a rewrite needs of the moves: the paths' pairs, each moved module's
 * place by its old one (`rust_js::lower::iterators`), the crates before
 * and after, and the names each crate's root declares now, by its name. */
export type Moved = {
  pairs: [string, string][];
  places: Map<string, Place>;
  before: CompilerCrate[];
  after: CompilerCrate[];
  declared: Map<string, Set<string>>;
  /** Each crate's dependencies of the compiler's crates, by name. */
  depends: Map<string, Set<string>>;
};

/** Where a file was and is: `old` its path before the moves. */
export type Site = { file: string; old: string };

const key = (crate: string, path: string[]) => [crate, ...path].join("::");

const git = (at: string, ...args: string[]): string => {
  const run = Bun.spawnSync(["git", ...args], { cwd: at });
  if (run.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${run.stderr.toString().trim()}`);
  return run.stdout.toString();
};

/** The crate whose sources hold `file`, of `crates`. */
const crateIn = (crates: CompilerCrate[], file: string) => crates.find((crate) => file.startsWith(`${crate.src}/`));

/** `file`'s module path in `crate`, as compiler-crates' `modulePath`. */
const moduleIn = (crate: CompilerCrate, file: string): string[] => {
  if (file === crate.rootFile) return [];
  const parts = file.slice(crate.src.length + 1).replace(/\.rs$/, "").split("/");
  if (parts.at(-1) === "mod") parts.pop();
  return parts;
};

/** The renames from `from` to `to`, a commit each, or the working tree
 * where `to` is none. */
export function renames(at: string, from: string, to?: string): Move[] {
  const out = git(at, "diff", "-M", "--name-status", "--diff-filter=R", from, ...(to ? [to] : []));
  return out
    .split("\n")
    .filter(Boolean)
    .map((line) => line.split("\t"))
    .map(([, from, to]) => ({ from: from!, to: to! }));
}

/** The paths' pairs of `moves`: each file's, and each directory's whose
 * files most moved to one other, `src/lower/` to `crates/rust-js-lower/src/`,
 * where none of its files stayed (`stayed`); longest first, so a file or
 * directory that went elsewhere, `src/lower/mir/`, is its own. */
export function pathPairs(moves: Move[], stayed: (file: string) => boolean, before: string[]): [string, string][] {
  const votes = new Map<string, Map<string, number>>();
  for (const { from, to } of moves) {
    const a = from.split("/");
    const b = to.split("/");
    for (let j = 1; j < a.length && j < b.length && a[a.length - j] === b[b.length - j]; j++) {
      const old = `${a.slice(0, a.length - j).join("/")}/`;
      const now = `${b.slice(0, b.length - j).join("/")}/`;
      const of = votes.get(old) ?? votes.set(old, new Map()).get(old)!;
      of.set(now, (of.get(now) ?? 0) + 1);
    }
  }
  const directories: [string, string][] = [];
  for (const [old, of] of votes) {
    const [now] = [...of].sort((x, y) => y[1] - x[1])[0]!;
    if (old !== now && !before.some((file) => file.startsWith(old) && stayed(file))) directories.push([old, now]);
  }
  const pairs: [string, string][] = [...moves.map(({ from, to }): [string, string] => [from, to]), ...directories];
  return pairs.sort((x, y) => y[0].length - x[0].length);
}

/** Each moved module's place, by its old one: the moves of Rust files of
 * the compiler's crates, before's and after's; and of a directory's pair,
 * its module's, where git saw no file's rename, a root rewritten as it
 * moved (`src/lower/` to `crates/rust-js-lower/src/`: `lower` is that
 * crate's root). */
export function places(moves: Move[], before: CompilerCrate[], after: CompilerCrate[], pairs: [string, string][] = []): Map<string, Place> {
  const out = new Map<string, Place>();
  for (const [from, to] of pairs) {
    if (!from.endsWith("/")) continue;
    const was = crateIn(before, from);
    const now = crateIn(after, to) ?? after.find((crate) => `${crate.src}/` === to);
    if (!was || !now || `${was.src}/` === from) continue;
    const path = `${now.src}/` === to ? [] : moduleIn(now, `${to.slice(0, -1)}.rs`);
    out.set(key(was.name, moduleIn(was, `${from.slice(0, -1)}.rs`)), { crate: now.name, path });
  }
  for (const { from, to } of moves) {
    if (!from.endsWith(".rs") || !to.endsWith(".rs")) continue;
    const was = crateIn(before, from);
    const now = crateIn(after, to);
    if (was && now) out.set(key(was.name, moduleIn(was, from)), { crate: now.name, path: moduleIn(now, to) });
  }
  return out;
}

/** The crates at `commit`: its manifests and roots, written out for
 * compiler-crates to read. */
export function cratesAt(at: string, commit: string): CompilerCrate[] {
  const dir = mkdtempSync(join(tmpdir(), "move-paths-"));
  const files = git(at, "ls-tree", "-r", "--name-only", commit)
    .split("\n")
    .filter((file) => /^(?:Cargo\.toml|src\/main\.rs|crates\/[^/]+\/(?:Cargo\.toml|src\/lib\.rs))$/.test(file));
  for (const file of files) {
    mkdirSync(dirname(join(dir, file)), { recursive: true });
    writeFileSync(join(dir, file), git(at, "show", `${commit}:${file}`));
  }
  return compilerCrates(dir);
}

/** `crate`'s dependencies of `crates`, by name, as its manifest says. */
export function dependencies(at: string, crate: CompilerCrate, crates: CompilerCrate[]): Set<string> {
  const manifest = Bun.TOML.parse(readFileSync(join(at, crate.dir, "Cargo.toml"), "utf8")) as { dependencies?: Record<string, unknown> };
  const named = new Set(Object.keys(manifest.dependencies ?? {}));
  return new Set(crates.filter((each) => named.has(each.package)).map((each) => each.name));
}

/** The names a crate's root `text` declares: its modules, items and imports. */
export function declaredNames(text: string): Set<string> {
  const code = codeOf(text);
  const names = boundNames(code);
  for (const m of code.matchAll(/\b(?:fn|struct|enum|trait|type|const|static|union|macro_rules!)\s+([A-Za-z_]\w*)/g)) names.add(m[1]!);
  return names;
}

/** The modules a file's code is in at each offset: its own, then each
 * inline module's, `mod tests { .. }`, that holds it. */
function inlineModules(code: string): { name: string; open: number; close: number }[] {
  const found: { name: string; open: number; close: number }[] = [];
  const open: { at: number; name?: string }[] = [];
  for (const m of code.matchAll(/\bmod\s+(\w+)\s*\{|\{|\}/g)) {
    if (m[0] === "}") {
      const top = open.pop();
      if (top?.name) found.push({ name: top.name, open: top.at, close: m.index! });
    } else open.push({ at: m.index! + m[0].length - 1, name: m[1] });
  }
  return found;
}

type Rewrite = { from: number; to: number; text: string };

/** `site`'s text with the paths of `lines`, its lines from 1 or all of
 * them, rewritten for the moves; and what it can't rewrite. Rust where
 * the file is the compiler's, its paths to files anywhere. */
export function rebasedText(site: Site, text: string, moved: Moved, lines: Set<number> | "all"): { text: string; problems: string[] } {
  const ours = (line: number) => lines === "all" || lines.has(line);
  const pathed = text
    .split("\n")
    .map((line, i) => (ours(i + 1) ? rewritten(line, moved.pairs) : line))
    .join("\n");
  if (!site.file.endsWith(".rs")) return { text: pathed, problems: [] };
  return rustRebased(site, pathed, moved, ours);
}

/** The Rust of `site`'s `text`, each path of a line `ours` says resolved
 * in its old crate and module and written for where it now is. */
function rustRebased(site: Site, text: string, moved: Moved, ours: (line: number) => boolean): { text: string; problems: string[] } {
  const was = crateIn(moved.before, site.old);
  const own = crateIn(moved.after, site.file);
  if (!was || !own) return { text, problems: [] };
  const problems: string[] = [];
  const code = codeOf(text);
  const commented = codeOf(text, true);
  const lineOf = (at: number) => text.slice(0, at).split("\n").length;
  const inline = inlineModules(code);
  const oldModule = moduleIn(was, site.old);
  const newModule = moduleIn(own, site.file);
  const within = (at: number) => inline.filter(({ open, close }) => open < at && at < close).sort((x, y) => x.open - y.open).map(({ name }) => name);
  const rewrites: Rewrite[] = [];
  const crates = new Set(moved.before.map((crate) => crate.name));

  // A path's place: its longest moved prefix's, else where it was.
  const placed = (crate: string, path: string[]): Place => {
    for (let k = path.length; k >= 0; k--) {
      const place = moved.places.get(key(crate, path.slice(0, k)));
      if (place) return { crate: place.crate, path: [...place.path, ...path.slice(k)] };
    }
    return { crate, path };
  };
  // A path's segments from its crate's root, of its head where it was
  // written: `crate`, `self`, the `super`s, or a crate's name.
  const absolute = (head: string, at: number): { crate: string; path: string[] } | undefined => {
    const here = [...oldModule, ...within(at)];
    if (head === "crate") return { crate: was.name, path: [] };
    if (head === "self") return { crate: was.name, path: here };
    if (/^super/.test(head)) {
      const supers = head.split("super").length - 1;
      return supers <= here.length ? { crate: was.name, path: here.slice(0, here.length - supers) } : undefined;
    }
    return crates.has(head) ? { crate: head, path: [] } : undefined;
  };
  // How `place` is written from `at` in the file as it now is: `crate::`
  // in its own crate, a crate's name in another's; the binary's, none.
  const written = (place: Place, at: number): string | undefined => {
    if (place.crate === own.name) return ["crate", ...place.path].join("::");
    if (moved.after.find((crate) => crate.name === place.crate)?.dir === "") {
      // Of a name its own crate's root declares now, it's written as the
      // crate is, a run's second time or a line a hand wrote so.
      if (moved.declared.get(own.name)?.has(place.path[0]!)) return undefined;
      problems.push(`${site.file}:${lineOf(at)} names the binary's \`${place.path.join("::")}\` from ${own.package}, which no crate can: none depends on the binary`);
      return undefined;
    }
    if (!moved.depends.get(own.name)?.has(place.crate)) {
      problems.push(`${site.file}:${lineOf(at)} names ${place.crate}'s \`${place.path.join("::")}\` from ${own.package}, which doesn't depend on it: the moves put it where this crate can't reach`);
      return undefined;
    }
    return [place.crate, ...place.path].join("::");
  };
  // Whether a relative path, `super::x`, still reaches `place` from `at`.
  const reaches = (head: string, rest: string[], place: Place, at: number) => {
    if (place.crate !== own.name || !/^(?:super|self)/.test(head)) return false;
    const here = [...newModule, ...within(at)];
    const supers = head.split("super").length - 1;
    return supers <= here.length && key(own.name, [...here.slice(0, here.length - supers), ...rest]) === key(own.name, place.path);
  };

  // `use` items whose tree reaches the root: each leaf placed, a tree a
  // crate, where they now part.
  const handled: [number, number][] = [];
  const bound = boundNames(code);
  const root = site.file === own.rootFile && site.old === was.rootFile;
  for (const m of code.matchAll(/(^|[^\w$])use\s+([^;]*);/g)) {
    const treeAt = m.index! + m[0].indexOf(m[2]!, m[1]!.length + 3);
    if (!ours(lineOf(treeAt))) continue;
    const tree = parseTree(m[2]!.trim());
    if (!tree || tree.path.length === 0) continue;
    // Its head: `crate`, `self`, the `super`s or a crate's name; or in a
    // root, a moved module's bare name.
    let head = tree.path[0]!;
    let fromHead = tree.path.slice(1);
    while (fromHead[0] === "super") {
      head = `${head}::super`;
      fromHead = fromHead.slice(1);
    }
    const bare = root && !absolute(head, treeAt) && !bound.has(head) && moved.places.has(key(was.name, [head]));
    const start = bare ? { crate: was.name, path: [] } : absolute(head, treeAt);
    if (!start) continue;
    // Each leaf, where it was and is: right as written where a relative
    // path still reaches it, or its root's names it where it was.
    const leaves: { place: Place; tail?: string; right: boolean }[] = [];
    const walk = (each: Tree, above: string[]) => {
      const path = each.path[0] === "self" ? above : [...above, ...each.path];
      if (each.group) each.group.forEach((item) => walk(item, path));
      else {
        const place = placed(start.crate, [...start.path, ...path]);
        const right = bare
          ? false
          : /^(?:super|self)/.test(head)
            ? reaches(head, path, place, treeAt)
            : place.crate === (head === "crate" ? own.name : head) && place.path.join("::") === path.join("::");
        leaves.push({ place, tail: each.tail, right });
      }
    };
    walk({ ...tree, path: bare ? tree.path : fromHead }, []);
    if (leaves.every(({ right }) => right)) continue;
    const by = new Map<string, Tree[]>();
    for (const { place, tail } of leaves) {
      const path = written(place, treeAt);
      if (path === undefined) return { text, problems };
      const [first, ...rest] = path.split("::");
      const item: Tree = rest.length === 0 ? { path: ["self"], tail } : { path: rest, tail: tail === "*" ? "*" : tail };
      by.set(first!, [...(by.get(first!) ?? []), item]);
    }
    const end = m.index! + m[0].length;
    if (text.slice(treeAt, end - 1) !== code.slice(treeAt, end - 1)) {
      problems.push(`${site.file}:${lineOf(treeAt)}: a comment in a use tree, left as it is`);
      continue;
    }
    const useAt = m.index! + m[1]!.length;
    const vis = /pub(?:\s*\([^)]*\))?\s*$/.exec(code.slice(0, useAt));
    const from = vis ? vis.index : useAt;
    const head0 = text.slice(from, treeAt);
    if (by.size > 1 && /#\[[^\]]*\]\s*$/.test(code.slice(0, from))) {
      problems.push(`${site.file}:${lineOf(treeAt)}: a use with attributes whose leaves part for two crates: split it by hand`);
      continue;
    }
    const indent = /[^\S\n]*$/.exec(text.slice(0, from))![0];
    const statements = [...by].map(([first, items]) => `${head0}${printTree({ path: [first], group: items })};`);
    rewrites.push({ from, to: end, text: statements.join(`\n${indent}`) });
    handled.push([from, end]);
  }
  const inHandled = (at: number) => handled.some(([from, to]) => from <= at && at < to);

  // `pub(in path)` and `pub(super)`: the module it names, placed, as its
  // own crate says it, or `pub` where that's another crate.
  for (const m of code.matchAll(/\bpub\s*\(\s*(?:in\s+((?:crate|self|super)(?:\s*::\s*\w+)*)|(super))\s*\)/g)) {
    const at = m.index!;
    if (!ours(lineOf(at))) continue;
    const segments = (m[1] ?? m[2]!).split(/\s*::\s*/);
    let head = segments[0]!;
    let rest = segments.slice(1);
    while (rest[0] === "super") {
      head = `${head}::super`;
      rest = rest.slice(1);
    }
    const base = absolute(head, at);
    if (!base) continue;
    const place = placed(base.crate, [...base.path, ...rest]);
    const now =
      place.crate !== own.name ? "pub" : reaches(head, rest, place, at) ? m[0] : place.path.length === 0 ? "pub(crate)" : `pub(in crate::${place.path.join("::")})`;
    if (now !== m[0]) rewrites.push({ from: at, to: at + m[0].length, text: now });
    handled.push([at, at + m[0].length]);
  }

  // A path through a root, in code and in comments: `crate::x`, the `super`s
  // or `self`, or a crate's name.
  const seen = new Set<number>();
  for (const source of [code, commented]) {
    for (const m of source.matchAll(/(^|[^\w$:])((?:crate|self|super(?:\s*::\s*super)*|[a-z_]\w*)(?:\s*::\s*[a-z_]\w*)+)/g)) {
      const at = m.index! + m[1]!.length;
      if (seen.has(at) || inHandled(at) || !ours(lineOf(at))) continue;
      seen.add(at);
      const segments = m[2]!.split(/\s*::\s*/);
      let head = segments[0]!;
      let rest = segments.slice(1);
      while (rest[0] === "super") {
        head = `${head}::super`;
        rest = rest.slice(1);
      }
      const bare = root && !absolute(head, at) && !bound.has(head) && within(at).length === 0 && moved.places.has(key(was.name, [head]));
      const base = bare ? { crate: was.name, path: [] } : absolute(head, at);
      if (!base) continue;
      const path = bare ? [head, ...rest] : rest;
      const place = placed(base.crate, [...base.path, ...path]);
      if (reaches(head, rest, place, at)) continue;
      const before = bare ? m[2]! : [head, ...rest].join("::");
      if (!bare && base.crate === own.name && place.crate === own.name && key(place.crate, place.path) === key(base.crate, [...base.path, ...rest]) && !/^(?:super|self)/.test(head)) continue;
      const now = written(place, at);
      if (now !== undefined && now !== before.replace(/\s+/g, "")) rewrites.push({ from: at, to: at + m[2]!.length, text: now });
    }
  }
  rewrites.sort((a, b) => b.from - a.from);
  let out = text;
  for (const r of rewrites) out = out.slice(0, r.from) + r.text + out.slice(r.to);
  return { text: out, problems };
}

/** A mutation list's text, its `file:`'s Rust in its `find`s and
 * `replace`s, on the lines `ours` says, rewritten as its file's would be. */
export function listRebased(text: string, moved: Moved, ours: (line: number) => boolean, oldOf: (file: string) => string): string {
  const file = /\bfile:\s*"([^"]+)"/.exec(text)?.[1];
  if (!file?.endsWith(".rs")) return text;
  const site = { file, old: oldOf(file) };
  return text.replace(/\b(find|replace):(\s*)"((?:[^"\\]|\\.)*)"/g, (whole, name, space, string, at) => {
    if (!ours(text.slice(0, at).split("\n").length)) return whole;
    const snippet = JSON.parse(`"${string}"`) as string;
    const next = rustRebased(site, snippet, moved, () => true).text;
    return next === snippet ? whole : `${name}:${space}${JSON.stringify(next)}`;
  });
}

/** The lines of `file` the branch wrote, against `base`: its diff's hunks,
 * or all of it where `base` hasn't it. */
export function branchLines(at: string, base: string, file: string): Set<number> | "all" {
  if (git(at, "ls-tree", "--name-only", base, "--", file).trim() === "") return "all";
  const lines = new Set<number>();
  for (const m of git(at, "diff", "-U0", base, "--", file).matchAll(/^@@ -\S+ \+(\d+)(?:,(\d+))? @@/gm)) {
    const start = Number(m[1]);
    const count = m[2] === undefined ? 1 : Number(m[2]);
    for (let line = start; line < start + count; line++) lines.add(line);
  }
  return lines;
}

/** The files the branch changed against `base`: its commits, the working
 * tree's changes and what's untracked. */
export function branchFiles(at: string, base: string): string[] {
  const changed = git(at, "diff", "--name-only", "--diff-filter=AMR", base).split("\n");
  const untracked = git(at, "ls-files", "--others", "--exclude-standard").split("\n");
  return [...new Set([...changed, ...untracked].filter(Boolean))];
}

/** Rewrite the branch at `at` for the moves from `oldBase` to `newBase`:
 * each file it changed, its own lines alone. Says what it rewrote and what
 * it couldn't. */
export function rebase(at: string, oldBase: string, newBase: string): { files: string[]; problems: string[] } {
  const moves = renames(at, oldBase, newBase);
  const before = cratesAt(at, oldBase);
  const after = compilerCrates(at);
  const beforeFiles = git(at, "ls-tree", "-r", "--name-only", oldBase).split("\n").filter(Boolean);
  const newFiles = new Set(git(at, "ls-tree", "-r", "--name-only", newBase).split("\n").filter(Boolean));
  const pairs = pathPairs(moves, (file) => newFiles.has(file), beforeFiles);
  const declared = new Map(after.map((crate) => [crate.name, declaredNames(readFileSync(join(at, crate.rootFile), "utf8"))]));
  const depends = new Map(after.map((crate) => [crate.name, dependencies(at, crate, after)]));
  const moved: Moved = { pairs, places: places(moves, before, after, pairs), before, after, declared, depends };
  const oldOf = (file: string) => {
    const move = moves.find(({ to }) => to === file);
    if (move) return move.from;
    const directory = pairs.find(([from, to]) => from.endsWith("/") && file.startsWith(to));
    return directory ? directory[0] + file.slice(directory[1].length) : file;
  };
  const files: string[] = [];
  const problems: string[] = [];
  for (const file of branchFiles(at, newBase)) {
    const away = pairs.find(([from]) => from.endsWith("/") ? file.startsWith(from) : file === from);
    if (away) {
      problems.push(`${file} is where main moved ${away[0]} from, to ${away[1]}: git mv it there, then run this again`);
      continue;
    }
    if (!/\.(ts|js|rs|md|txt|ya?ml|json|toml)$/.test(file)) continue;
    const text = readFileSync(join(at, file), "utf8");
    const lines = branchLines(at, newBase, file);
    const ours = (line: number) => lines === "all" || lines.has(line);
    let next = rebasedText({ file, old: oldOf(file) }, text, moved, lines);
    problems.push(...next.problems);
    if (file.startsWith("scripts/mutations/") && file.endsWith(".ts")) next = { text: listRebased(next.text, moved, ours, oldOf), problems: [] };
    if (next.text !== text) {
      writeFileSync(join(at, file), next.text);
      files.push(file);
    }
  }
  return { files, problems };
}
