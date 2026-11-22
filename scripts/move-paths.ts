// Moving source files without breaking what names them (#2294): each
// `<from> <to>` pair, a file or a directory's prefix, rewritten wherever a
// path to it is written, in the scripts, the tests and their catalogs, the
// docs, the sources' own comments and CI's workflows, and a moved file's
// mutation list (scripts/mutations/<path>.ts) moved beside it, its `file:`
// fields with the rest. Re-runnable: a pair already applied rewrites
// nothing. The files themselves are moved by `git mv`, before or after.
//
// And a module moved to another of the compiler's crates (ADR 1181): once
// its file is there and the crates' roots declare it, `mod x;`, each Rust
// path to it through its old crate's root is its new crate's, in code and
// in comments. Which crate a top-level module is of is the layout's answer
// (scripts/compiler-crates.ts), so no map is given:
//
//   crate::collections::IndexMap        rust_js_program::collections::IndexMap
//   super::super::js::Expr              rust_js_program::js::Expr (where the
//                                       `super`s reach the crate's root)
//   use crate::{collections::IndexSet,  use crate::lower::Body;
//       lower::Body};                   use rust_js_program::collections::IndexSet;
//
// With no pairs it rewrites those alone: run it on a branch after a move
// lands, so the branch's own code follows. rustfmt lays out what it writes
// (`bun scripts/format.ts`). A path from a crate to the binary's module
// can't be written, as no crate depends on the binary: it's said, and left.
//
//   bun scripts/move-paths.ts crates/rust-js-lower/src/iterators/mir_steps/ crates/rust-js-lower/src/iterators/mir_steps/
//   bun scripts/move-paths.ts src/collections.rs crates/rust-js-program/src/collections.rs
//   bun scripts/move-paths.ts                    # the Rust paths alone
//   bun scripts/move-paths.ts --widen ..         # and make pub what another crate
//                                                # names, as rustc says (`widened`)
//   bun scripts/move-paths.ts --check crates/rust-js-lower/src/iterators/mir_steps/   # what still names it

import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { compilerCrates, compilerRustFiles, crateOf, modulePath, topModules, type CompilerCrate } from "./compiler-crates";

const root = join(import.meta.dir, "..");
const searched = ["scripts", "test", "docs", "src", "crates", ".github", "DEVELOPMENT.md", "CONTRIBUTING.md", "TESTS.md", "AGENTS.md"];
// And the guide, whose paths are an app's, `src/lib.rs`, never the compiler's.
const skipped = new Set(["node_modules", "corpus", "known", "fixtures", "target", "guide"]);

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
 * of `a/b.rs` leaves `a/b.rsx`), nor one it ends (`crates/rust-js-program/src/js.rs` leaves
 * `crates/rust-js-program/src/js.rs`, which a second run met as written),
 * but one relative to a folder below the root, `"../a/b.rs"`. */
/** Where `text` spells `path`, a file's or a directory's, as a join's
 * string segments, `"src", "lower", "iterators.rs"`: a form the rewrite
 * can't see, said so it's written whole (#2299's cut missed two). */
export function segmented(text: string, path: string): boolean {
  const parts = path.replace(/\/$/, "").split("/").filter(Boolean);
  if (parts.length < 2) return false;
  const quoted = parts.map((part) => `["'\`]${part.replace(/[.*+?^${}()|[\]\\]/g, "\\export function rewritten(text: string, pairs: [string, string][]): string {")}["'\`]`);
  return new RegExp(quoted.join("\\s*,\\s*")).test(text);
}

export function rewritten(text: string, pairs: [string, string][]): string {
  let out = text;
  for (const [from, to] of pairs) {
    const escaped = from.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const boundary = from.endsWith("/") ? "" : "(?![\\w.-])";
    out = out.replace(new RegExp(`(?<=(?:^|[^\\w./-])(?:\\.\\./)*)${escaped}${boundary}`, "g"), to);
  }
  return out;
}

/** A source's path below its crate's `src/`, `crates/rust-js-program/src/
 * decisions.rs`'s `decisions.rs`, as the crates' layout says
 * (scripts/compiler-crates.ts). */
const belowSrc = (path: string) => {
  const crate = crateOf(path);
  return crate ? path.slice(crate.src.length + 1) : path;
};

/** The mutation list of a source file, scripts/mutations/<path below its crate's src/>.ts. */
const listOf = (path: string) => join("scripts", "mutations", belowSrc(path).replace(/\.rs$/, ".ts"));

/** `text` with comments, strings and chars blanked, each byte kept where
 * it is, newlines too, so an offset of one is the other's; its comments
 * kept where `comments` says. */
export function codeOf(text: string, comments = false): string {
  let out = "";
  let i = 0;
  const blank = (s: string) => s.replace(/[^\n]/g, " ");
  while (i < text.length) {
    const rest = text.slice(i, i + 3);
    if (rest.startsWith("//")) {
      const end = text.indexOf("\n", i);
      const stop = end < 0 ? text.length : end;
      out += comments ? text.slice(i, stop) : blank(text.slice(i, stop));
      i = stop;
    } else if (rest.startsWith("/*")) {
      let depth = 0;
      let j = i;
      while (j < text.length) {
        if (text.startsWith("/*", j)) {
          depth++;
          j += 2;
        } else if (text.startsWith("*/", j)) {
          depth--;
          j += 2;
          if (depth === 0) break;
        } else j++;
      }
      out += comments ? text.slice(i, j) : blank(text.slice(i, j));
      i = j;
    } else if (/^b?r#*"/.test(rest) && /[^\w]/.test(text[i - 1] ?? " ")) {
      const m = /^b?r(#*)"/.exec(text.slice(i))!;
      const close = `"${m[1]}`;
      const end = text.indexOf(close, i + m[0].length);
      const stop = end < 0 ? text.length : end + close.length;
      out += blank(text.slice(i, stop));
      i = stop;
    } else if (text[i] === '"' || (text[i] === "b" && text[i + 1] === '"' && /[^\w]/.test(text[i - 1] ?? " "))) {
      let j = text.indexOf('"', i) + 1;
      while (j < text.length && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
      out += blank(text.slice(i, j + 1));
      i = j + 1;
    } else if (text[i] === "'") {
      // A char, `'x'` or `'\n'`, not a lifetime, `'a`.
      const m = /^'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'/u.exec(text.slice(i));
      if (m) {
        out += blank(m[0]);
        i += m[0].length;
      } else {
        out += text[i];
        i++;
      }
    } else {
      out += text[i];
      i++;
    }
  }
  return out;
}

/** Where each inline module's body opens and closes in `code`, `mod tests
 * { .. }`'s: a `super` there climbs out of it first. */
function inlineModules(code: string): [number, number][] {
  const found: [number, number][] = [];
  const open: { at: number; module: boolean }[] = [];
  for (const m of code.matchAll(/\bmod\s+\w+\s*\{|\{|\}/g)) {
    if (m[0] === "}") {
      const top = open.pop();
      if (top?.module) found.push([top.at, m.index!]);
    } else open.push({ at: m.index! + m[0].length - 1, module: m[0] !== "{" });
  }
  return found;
}

type Rewrite = { from: number; to: number; text: string };

/** A use tree's parts: `a::b::{c, d::e}` is `["a", "b"]` and its group. */
type Tree = { path: string[]; group?: Tree[]; tail?: string };

/** The use tree `text` is, parsed, or nothing where it's of a shape this
 * doesn't read (it's left as it is then, and said). */
function parseTree(text: string): Tree | undefined {
  let at = 0;
  const space = () => {
    while (/\s/.test(text[at] ?? "")) at++;
  };
  const tree = (): Tree | undefined => {
    const path: string[] = [];
    for (;;) {
      space();
      if (text[at] === "{") {
        at++;
        const group: Tree[] = [];
        for (;;) {
          space();
          if (text[at] === "}") break;
          const item = tree();
          if (!item) return undefined;
          group.push(item);
          space();
          if (text[at] === ",") at++;
          else if (text[at] !== "}") return undefined;
        }
        at++;
        return { path, group };
      }
      if (text[at] === "*") {
        at++;
        return { path, tail: "*" };
      }
      const m = /^(?:r#)?[A-Za-z_]\w*/.exec(text.slice(at));
      if (!m) return undefined;
      path.push(m[0]);
      at += m[0].length;
      space();
      if (text.startsWith("::", at)) {
        at += 2;
        continue;
      }
      const alias = /^as\s+((?:r#)?[A-Za-z_]\w*)/.exec(text.slice(at));
      if (alias) {
        at += alias[0].length;
        return { path, tail: `as ${alias[1]}` };
      }
      return { path };
    }
  };
  const parsed = tree();
  space();
  return parsed && at === text.length ? parsed : undefined;
}

function printTree(tree: Tree): string {
  const path = tree.path.join("::");
  const joined = (rest: string) => (path ? `${path}::${rest}` : rest);
  if (tree.group) {
    if (tree.group.length === 1 && tree.group[0]!.path[0] !== "self") {
      return joined(printTree(tree.group[0]!));
    }
    return joined(`{${tree.group.map(printTree).join(", ")}}`);
  }
  if (tree.tail === "*") return joined("*");
  return tree.tail ? `${path} ${tree.tail}` : path;
}

/** What `file`'s text becomes, its paths into other crates' modules made
 * theirs, and what it can't rewrite. */
export function rustPathsRewritten(file: string, text: string, modulesOf: Map<string, CompilerCrate>, at = root): { text: string; problems: string[] } {
  const own = crateOf(file, at)!;
  const ownModule = modulePath(file, at);
  const code = codeOf(text);
  const inline = inlineModules(code);
  const problems: string[] = [];
  const rewrites: Rewrite[] = [];
  const line = (at: number) => code.slice(0, at).split("\n").length;
  // The crate a top-level module of `own`'s root path is of, where it's
  // another's: never one `own` declares itself, as the lowering's base and
  // MIR's crate both have a `drops` (#2299).
  const ownModules = new Set(topModules(own, at));
  const elsewhere = (module: string): CompilerCrate | undefined => {
    const crate = ownModules.has(module) ? undefined : modulesOf.get(module);
    return crate && crate !== own ? crate : undefined;
  };
  const reachable = (crate: CompilerCrate, at: number, module: string) => {
    if (crate.dir !== "") return true;
    problems.push(`${file}:${line(at)} names the binary's \`${module}\` from ${own.package}, which no crate can: none depends on the binary`);
    return false;
  };
  // How many `super`s at `at` reach the crate's root.
  const toRoot = (at: number) => ownModule.length + inline.filter(([open, close]) => open < at && at < close).length;

  // `use` items whose tree is grouped at the root: split.
  const handled: [number, number][] = [];
  for (const m of code.matchAll(/(^|[^\w$])use\s+((?:crate|(?:super\s*::\s*)*super)\s*::\s*\{)/g)) {
    const treeStart = m.index! + m[1]!.length + m[0].slice(m[1]!.length).indexOf(m[2]!);
    const end = code.indexOf(";", treeStart);
    const treeText = code.slice(treeStart, end);
    const tree = parseTree(treeText);
    if (!tree) {
      problems.push(`${file}:${line(treeStart)}: a use tree this can't read, left as it is`);
      continue;
    }
    const supers = tree.path.filter((p) => p === "super").length;
    if (tree.path[0] !== "crate" && supers !== toRoot(treeStart)) continue;
    const moved = new Map<CompilerCrate, Tree[]>();
    const kept: Tree[] = [];
    for (const item of tree.group!) {
      const crate = item.path[0] && item.path[0] !== "self" ? elsewhere(item.path[0]) : undefined;
      if (crate && reachable(crate, treeStart, item.path[0]!)) moved.set(crate, [...(moved.get(crate) ?? []), item]);
      else kept.push(item);
    }
    if (moved.size === 0) continue;
    if (text.slice(treeStart, end) !== treeText) {
      problems.push(`${file}:${line(treeStart)}: a comment in a use tree that splits, left as it is`);
      continue;
    }
    // The item: its attributes, visibility and `use`, as written.
    const useAt = m.index! + m[1]!.length;
    let itemStart = useAt;
    const before = code.slice(0, useAt);
    const vis = /pub(?:\s*\([^)]*\))?\s*$/.exec(before);
    if (vis) itemStart = vis.index;
    let attrs = "";
    for (;;) {
      const prefix = code.slice(0, itemStart);
      const attr = /#\[[^\]]*\]\s*$/.exec(prefix);
      if (!attr) break;
      attrs = text.slice(attr.index, itemStart) + attrs;
      itemStart = attr.index;
    }
    const head = text.slice(vis ? vis.index : useAt, treeStart);
    const statements: string[] = [];
    if (kept.length > 0) statements.push(`${head}${printTree({ path: tree.path, group: kept })};`);
    for (const [crate, items] of moved) statements.push(`${attrs}${head}${printTree({ path: [crate.name], group: items })};`);
    const from = vis ? vis.index : useAt;
    rewrites.push({ from, to: end + 1, text: statements.join("\n") });
    handled.push([from, end + 1]);
  }
  const inHandled = (at: number) => handled.some(([from, to]) => from <= at && at < to);

  // A path through the root, in code: `crate::x` or the `super`s that reach it.
  for (const m of code.matchAll(/(^|[^\w$])((?:crate|super(?:\s*::\s*super)*)\s*::\s*)([a-z_]\w*)\b/g)) {
    const at = m.index! + m[1]!.length;
    if (inHandled(at)) continue;
    const prefix = m[2]!;
    if (!prefix.startsWith("crate")) {
      const supers = prefix.split("super").length - 1;
      if (supers !== toRoot(at)) continue;
    }
    const crate = elsewhere(m[3]!);
    if (!crate || !reachable(crate, at, m[3]!)) continue;
    rewrites.push({ from: at, to: at + prefix.length, text: `${crate.name}::` });
  }
  // A crate's root names its own modules bare, `decisions::path()`: there,
  // out of any inline module, and not a later part of a path or of a use
  // tree's group, `std::{collections::HashMap}`'s.
  if (ownModule.length === 0) {
    const uses = [...code.matchAll(/(^|[^\w$])use\s+[^;]*;/g)].map((m) => [m.index! + m[1]!.length, m.index! + m[0].length] as const);
    for (const m of code.matchAll(/(^|[^\w$:.])([a-z_]\w*)(\s*::)/g)) {
      const at = m.index! + m[1]!.length;
      if (inHandled(at) || toRoot(at) !== 0) continue;
      const inUse = uses.find(([from, to]) => from <= at && at < to);
      if (inUse && !/^use\s+$/.test(code.slice(inUse[0], at))) continue;
      const crate = elsewhere(m[2]!);
      if (!crate || !reachable(crate, at, m[2]!)) continue;
      rewrites.push({ from: at, to: at, text: `${crate.name}::` });
    }
  }
  // A path through its own crate's name, of a file that moved into the
  // crate it named: `crate::`, as a crate names itself.
  if (own.dir !== "") {
    for (const m of codeOf(text, true).matchAll(new RegExp(`(^|[^\\w$:])(${own.name}\\s*::)`, "g"))) {
      const at = m.index! + m[1]!.length;
      rewrites.push({ from: at, to: at + m[2]!.length, text: "crate::" });
    }
  }
  if (/\$crate\s*::/.test(code)) {
    for (const m of code.matchAll(/\$crate\s*::\s*([a-z_]\w*)/g)) {
      if (elsewhere(m[1]!)) problems.push(`${file}:${line(m.index!)}: \`$crate::${m[1]}\` names a module of another crate: write its crate's path`);
    }
  }
  // In comments: `crate::x`, written as a path is.
  const commented = codeOf(text, true);
  for (const m of commented.matchAll(/(^|[^\w$])(crate::)([a-z_]\w*)\b/g)) {
    const at = m.index! + m[1]!.length;
    if (code.slice(at, at + 5) === "crate") continue;
    const crate = elsewhere(m[3]!);
    if (crate && crate.dir !== "") rewrites.push({ from: at, to: at + m[2]!.length, text: `${crate.name}::` });
  }
  rewrites.sort((a, b) => b.from - a.from);
  let out = text;
  for (const r of rewrites) out = out.slice(0, r.from) + r.text + out.slice(r.to);
  return { text: out, problems };
}


/** Each item another crate names that its own crate keeps private, made
 * `pub`, as rustc says them, until it says none: what the front end uses
 * of a crate is what the crate makes public, and nothing more (ADR 1181).
 * rustc's privacy errors, a private item (E0603), method (E0624) or field
 * (E0616), each point at where the item is defined. */
/** A mutation list's text (scripts/mutations/**), its `find`s and
 * `replace`s Rust of its file, with the paths `rustPathsRewritten` writes
 * there: `crate::x` of a module another crate has, its crate's, and a
 * crate's own name, where its file moved into it, `crate::`. */
export function listRewritten(text: string, modulesOf: Map<string, CompilerCrate>, at = root): string {
  const file = /\bfile:\s*"([^"]+)"/.exec(text)?.[1];
  const own = file === undefined ? undefined : crateOf(file, at);
  if (!own || !file!.endsWith(".rs")) return text;
  const ownModules = new Set(topModules(own, at));
  const moved = (module: string) => {
    const crate = ownModules.has(module) ? undefined : modulesOf.get(module);
    return crate && crate !== own && crate.dir !== "" ? crate : undefined;
  };
  const rust = (code: string) =>
    code
      .replace(/(^|[^\w$])crate::([a-z_]\w*)\b/g, (whole, before, module) => (moved(module) ? `${before}${moved(module)!.name}::${module}` : whole))
      .replace(new RegExp(`(^|[^\\w$:])${own.name}::`, "g"), own.dir === "" ? "$&" : "$1crate::")
      // A crate's root names its modules bare, `paths::absolute(..)`.
      .replace(/(^|[^\w$:.{,]\s*|\()([a-z_]\w*)::/g, (whole, before, module) =>
        file === own.rootFile && moved(module) ? `${before}${moved(module)!.name}::${module}::` : whole,
      );
  // Its `find`s and `replace`s, each a string of its file's Rust.
  return text.replace(/\b(find|replace):(\s*)("(?:[^"\\]|\\.)*")/g, (_, key, space, string) => `${key}:${space}${rust(string)}`);
}

type Span = { file_name: string; line_start: number; label?: string | null };
type Message = { message: string; code?: { code: string } | null; spans: Span[]; children: { message: string; spans: Span[] }[] };

/** Where rustc says an item another crate names is private, each the line
 * it's *defined* on, `file:line`, and its name: a "defined here" note's,
 * never the use's own span, which a field's literal or a parameter's type
 * can be, where `pub` is a syntax error (#2299's cut); and a type a public
 * item hands out that its crate keeps (`private_interfaces`). Of the files
 * `ours` says are a crate's of the compiler's. */
export function privateSites(message: Message, ours: (file: string) => boolean): [string, string | undefined][] {
  const code = message.code?.code ?? "";
  const at = (span: Span) => `${span.file_name}:${span.line_start}`;
  if (["private_interfaces", "private_bounds"].includes(code)) {
    return message.children
      .filter((child) => /only usable at visibility/.test(child.message))
      .flatMap((child) => child.spans.filter((span) => ours(span.file_name)).map((span): [string, undefined] => [at(span), undefined]));
  }
  if (!["E0603", "E0624", "E0616"].includes(code)) return [];
  const name = /`(\w+)` is private/.exec(message.message)?.[1];
  return message.children
    .filter((child) => /defined here/.test(child.message))
    .flatMap((child) => child.spans.filter((span) => ours(span.file_name)).map((span): [string, string | undefined] => [at(span), name]));
}

/** The line, 1-based, of the `use` whose list holds line `n` of `lines`,
 * which runs over lines from above the item rustc names; `n` where none
 * does, up to the statement before. */
export function statementLine(lines: string[], n: number): number {
  for (let k = n; k >= 1; k--) {
    if (/^\s*(?:pub(?:\s*\([^)]*\))?\s+)?use\s/.test(lines[k - 1]!)) return k;
    if (k > 1 && (/[;}]\s*$/.test(lines[k - 2]!) || /^\s*$/.test(lines[k - 2]!))) break;
  }
  return n;
}

export function widened(at = root): string[] {
  const made: string[] = [];
  const visibility = /^(\s*)(?:pub(?:\s*\([^)]*\))?\s+)?/;
  // An item, or a field, `name: T`, at the start of `line`, by its
  // visibility and what follows it.
  const declares = (line: string, name = "[A-Za-z_]\\w*") =>
    new RegExp(`^(\\s*)(?:pub(?:\\s*\\([^)]*\\))?\\s+)?((?:(?:const|unsafe|async|extern\\s+"[^"]*")\\s+)*(?:fn|struct|enum|union|const|static|type|trait|mod|use)\\s+${name}\\b|${name}\\s*:(?!:))`).exec(line);
  const widen = (file: string, number: number): boolean => {
    const lines = readFileSync(join(at, file), "utf8").split("\n");
    const line = lines[number - 1]!;
    const head = visibility.exec(line)!;
    const next = `${head[1]}pub ${line.slice(head[0].length)}`;
    if (/^\s*pub\s/.test(line) || next === line) return false;
    lines[number - 1] = next;
    writeFileSync(join(at, file), lines.join("\n"));
    made.push(`${file}:${number}`);
    return true;
  };
  for (let round = 0; round < 20; round++) {
    // Every target, the tests' too, which name a crate's `testing` items.
    const built = Bun.spawnSync(["cargo", "build", "--quiet", "--all-targets", "--message-format=json"], { cwd: at, stdout: "pipe", stderr: "pipe" });
    const sites = new Map<string, string | undefined>();
    const walk = (message: Message) => {
      for (const [site, name] of privateSites(message, (file) => !!crateOf(file, at)?.dir)) sites.set(site, name);
    };
    for (const line of built.stdout.toString().split("\n")) {
      if (!line.startsWith("{")) continue;
      const parsed = JSON.parse(line);
      if (parsed.reason === "compiler-message") walk(parsed.message);
    }
    if (sites.size === 0) return made;
    let changed = 0;
    for (const [site, name] of sites) {
      const [file, number] = [site.slice(0, site.lastIndexOf(":")), Number(site.slice(site.lastIndexOf(":") + 1))];
      const lines = readFileSync(join(at, file), "utf8").split("\n");
      const line = lines[number - 1]!;
      if (declares(line)) {
        if (widen(file, number)) changed++;
        continue;
      }
      const start = statementLine(lines, number);
      if (start !== number && /^\s*(?:pub(?:\s*\([^)]*\))?\s+)?use\s/.test(lines[start - 1]!)) {
        if (widen(file, start)) changed++;
        continue;
      }
      // A glob's re-export, `stmt_walk::*,`, which rustc names for the item
      // it re-exports: that item, declared in the glob's module.
      const glob = /\b([a-z_]\w*)::\*/.exec(line)?.[1];
      if (!glob || !name) continue;
      const base = /(?:^|\/)(?:lib|main|mod)\.rs$/.test(file) ? dirname(file) : file.replace(/\.rs$/, "");
      const module = [`${base}/${glob}.rs`, `${base}/${glob}/mod.rs`].find((path) => existsSync(join(at, path)));
      if (!module) continue;
      const text = readFileSync(join(at, module), "utf8").split("\n");
      for (const [i, each] of text.entries()) if (declares(each, name) && widen(module, i + 1)) changed++;
    }
    if (changed === 0) throw new Error(`rustc still says items are private: ${[...sites.keys()].join(", ")}`);
  }
  throw new Error("still widening after 20 rounds");
}

if (import.meta.main) {
  const widen = process.argv.includes("--widen");
  const args = process.argv.slice(2).filter((arg) => arg !== "--widen");
  if (args[0] === "--check") {
    const named = textFiles().filter((file) => readFileSync(join(root, file), "utf8").includes(args[1]!));
    console.log(named.length === 0 ? `nothing names ${args[1]}` : named.join("\n"));
    process.exit(named.length === 0 ? 0 : 1);
  }
  if (args.length % 2 !== 0) throw new Error("usage: bun scripts/move-paths.ts [<from> <to> ..]");
  const pairs: [string, string][] = [];
  for (let i = 0; i < args.length; i += 2) pairs.push([args[i]!, args[i + 1]!]);
  // A moved file's mutation list, moved beside it, as each list is its
  // file's (test/mutations.test.ts); the lists' paths rewritten with the rest.
  for (const [from, to] of [...pairs]) {
    const lists = from.endsWith("/") ? [join("scripts", "mutations", belowSrc(from))] : [listOf(from)];
    for (const list of lists) {
      const target = from.endsWith("/") ? join("scripts", "mutations", belowSrc(to)) : listOf(to);
      if (target === list) continue;
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
    pairs.push([from.endsWith("/") ? join("scripts", "mutations", belowSrc(from)) + "/" : listOf(from), from.endsWith("/") ? join("scripts", "mutations", belowSrc(to)) + "/" : listOf(to)]);
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
  for (const file of textFiles()) {
    const text = readFileSync(join(root, file), "utf8");
    for (const [from] of pairs) if (segmented(text, from)) console.log(`${file} spells ${from} as a join's segments, which this can't rewrite: write it whole`);
  }
  // And the Rust paths into a module another crate now has.
  const modulesOf = new Map<string, CompilerCrate>();
  // A name two crates both declare, `drops` of the lowering's base and of
  // MIR's, is neither's from a third: each crate names its own first, and a
  // path through another root to it can't be told, so it's left (#2299).
  const declaring = new Map<string, CompilerCrate[]>();
  for (const crate of compilerCrates()) for (const module of topModules(crate)) declaring.set(module, [...(declaring.get(module) ?? []), crate]);
  for (const [module, crates] of declaring) if (crates.length === 1) modulesOf.set(module, crates[0]!);
  const problems: string[] = [];
  let paths = 0;
  for (const file of compilerRustFiles()) {
    const text = readFileSync(join(root, file), "utf8");
    const next = rustPathsRewritten(file, text, modulesOf);
    problems.push(...next.problems);
    if (next.text !== text) {
      writeFileSync(join(root, file), next.text);
      paths++;
    }
  }
  // And the Rust of each mutation list's `find`s and `replace`s.
  for (const list of textFiles().filter((file) => file.startsWith("scripts/mutations/") && file.endsWith(".ts"))) {
    const text = readFileSync(join(root, list), "utf8");
    const next = listRewritten(text, modulesOf);
    if (next !== text) {
      writeFileSync(join(root, list), next);
      paths++;
    }
  }
  console.log(`rewrote the Rust paths of ${paths} files`);
  for (const problem of problems) console.error(problem);
  if (problems.length > 0) process.exit(1);
  if (widen) console.log(`made pub: ${widened().join(", ") || "nothing"}`);
}
