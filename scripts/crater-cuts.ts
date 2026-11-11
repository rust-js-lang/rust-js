// What the small Crater reads of a crate's source and cuts of it (#1087,
// #1174; scripts/crater.ts): the place each error names, the items of a
// file by a scan of its text, which of them only tests build, of rustc's own
// cfgs of the target, and the cut of each, `#[cfg(any())]` above it, or a
// name taken off a `use`, so a refused test is cut, not its crate.

import { spawnSync } from "node:child_process";
import { join } from "node:path";

const root = join(import.meta.dir, "..");

/** Each error a build said, rustc's or rust-js's, and the place it names,
 * `src/lib.rs:1812`. */
export function errorsAt(
  said: string,
): { why: string; file: string; line: number }[] {
  return [
    ...said.matchAll(/^error(?:\[E\d+\])?: (.*)\n\s*--> ([^:\n]+):(\d+):\d+/gm),
  ].map((m) => ({ why: m[1], file: m[2], line: Number(m[3]) }));
}

/** rustc's cfgs of rust-js's target, `rustc --print cfg --target
 * wasm32-unknown-unknown`'s: a name alone, `target_has_atomic`, or a name
 * and each of its values, `target_os="unknown"`. The one source of them,
 * read once (#1153). */
let wasmCfgs: Map<string, Set<string>> | undefined;

function targetCfgs(): Map<string, Set<string>> {
  if (wasmCfgs) return wasmCfgs;
  const printed = spawnSync("rustc", ["--print", "cfg", "--target", "wasm32-unknown-unknown"], { cwd: root, encoding: "utf8" });
  if (printed.status !== 0) throw new Error(`rustc --print cfg failed: ${printed.stderr}`);
  wasmCfgs = new Map();
  for (const line of printed.stdout.split("\n").filter(Boolean)) {
    const [name, value] = line.split("=");
    const values = wasmCfgs.get(name) ?? new Set<string>();
    if (value !== undefined) values.add(JSON.parse(value));
    wasmCfgs.set(name, values);
  }
  return wasmCfgs;
}

/** Each `#[cfg(..)]` of attributes' text, as `cfg(..)`. */
export function cfgsOf(attributes: string): string[] {
  const found: string[] = [];
  for (let at = attributes.indexOf("#[cfg("); at >= 0; at = attributes.indexOf("#[cfg(", at + 1)) {
    let depth = 0;
    for (let i = at + 5; i < attributes.length; i++) {
      if (attributes[i] === "(") depth++;
      else if (attributes[i] === ")" && --depth === 0) {
        found.push(attributes.slice(at + 2, i + 1));
        break;
      }
    }
  }
  return found;
}

/** Whether `cfg`, `cfg(..)`'s text, holds on wasm32 only because of
 * `test`: false of a build without it, whatever features are on, which the
 * Crater can't know, so a `feature = ".."` is neither (#1153). */
export function onlyForTests(cfg: string): boolean {
  return cfgHolds(cfg, false) === false && cfgHolds(cfg, true) !== false;
}

/** Whether `cfg` holds on wasm32, of a build with `test` or without: true,
 * false, or `undefined` where it's of a feature, which the Crater can't
 * know. Of rustc's own cfgs of the target (`targetCfgs`). */
export function cfgHolds(cfg: string, test: boolean): boolean | undefined {
  const facts = targetCfgs();
  type Truth = boolean | undefined;
  const tokens = cfg.match(/"(?:[^"\\]|\\.)*"|[A-Za-z_][A-Za-z0-9_]*|[(),=]/g) ?? [];
  const evaluate = (): Truth => {
    let i = 0;
    const list = (): Truth[] => {
      const items: Truth[] = [];
      if (tokens[i] !== "(") throw new Error(`unreadable cfg: ${cfg}`);
      i++;
      while (tokens[i] !== ")") {
        items.push(predicate());
        if (tokens[i] === ",") i++;
      }
      i++;
      return items;
    };
    const predicate = (): Truth => {
      const name = tokens[i++];
      if (name === "all") return list().reduce<Truth>((a, b) => (a === false || b === false ? false : a === undefined || b === undefined ? undefined : true), true);
      if (name === "any") return list().reduce<Truth>((a, b) => (a === true || b === true ? true : a === undefined || b === undefined ? undefined : false), false);
      if (name === "not") {
        const [inner] = list();
        return inner === undefined ? undefined : !inner;
      }
      if (tokens[i] === "=") {
        i++;
        const value = JSON.parse(tokens[i++]);
        if (name === "feature") return undefined;
        return facts.get(name)?.has(value) ?? false;
      }
      if (name === "test") return test;
      return facts.has(name) && facts.get(name)!.size === 0;
    };
    if (tokens[i++] !== "cfg") throw new Error(`unreadable cfg: ${cfg}`);
    const [whole] = list();
    return whole;
  };
  return evaluate();
}

/** Where the attributes just above line `at`, from 1, start: each line that
 * starts with `#[`, and each attribute of several lines, uuid's
 * `#[cfg_attr(\n    all(..),\n    wasm_bindgen_test\n)]` between its
 * `#[test]` and its `fn`, its lines from `#[` to `]` one balanced
 * attribute (#1121). */
export function attributesFrom(lines: string[], at: number): number {
  let from = at;
  while (from > 1) {
    const above = lines[from - 2];
    if (/^\s*#\[/.test(above)) {
      from--;
      continue;
    }
    if (!/\]\s*$/.test(above)) break;
    let start = from - 2;
    while (start > 0 && !/^\s*#\[/.test(lines[start])) start--;
    const attribute = lines.slice(start, from - 1).join("\n");
    const opened = (attribute.match(/\[/g) ?? []).length;
    if (!/^\s*#\[/.test(lines[start]) || opened !== (attribute.match(/\]/g) ?? []).length) break;
    from = start + 1;
  }
  return from;
}

/** An item of a Rust source, by a scan of its text: a `fn`'s or a `mod`'s
 * name, or an `impl` (of no name), the lines its attributes start and its body ends, from 1, and
 * whether it's a `#[test]` or `#[cfg(test)]`. Strings, characters and
 * comments are skipped for their braces; a raw string's aren't. */
export type Item = {
  kind: "fn" | "mod" | "impl";
  name: string;
  from: number;
  to: number;
  test: boolean;
  cfgTest: boolean;
  /** Its `#[cfg(..)]`s, `cfg(unix)`. */
  cfgs: string[];
  /** A module of a file of its own, `mod tests;`. */
  file?: boolean;
};

export function items(text: string): Item[] {
  const lines = text.split("\n");
  // Each offset's line, from 1.
  const lineAt: number[] = [];
  lines.forEach((line, i) => {
    for (let k = 0; k <= line.length; k++) lineAt.push(i + 1);
  });
  const found: Item[] = [];
  const open: { item: Item | undefined }[] = [];
  let pending: { kind: Item["kind"]; name: string; at: number } | undefined;
  // The brackets open since `pending`'s keyword: a `;` in them is an
  // array type's, `[u8; 4]`, no end of a `mod` of a file.
  let brackets = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === "/" && text[i + 1] === "/") {
      i = text.indexOf("\n", i);
      if (i < 0) break;
      continue;
    }
    if (c === "/" && text[i + 1] === "*") {
      i = text.indexOf("*/", i + 2) + 1;
      if (i <= 0) break;
      continue;
    }
    if (c === '"') {
      for (i++; i < text.length && text[i] !== '"'; i++)
        if (text[i] === "\\") i++;
      continue;
    }
    // A character, `'{'`, not a lifetime, `'a`.
    if (
      c === "'" &&
      (text[i + 2] === "'" ||
        (text[i + 1] === "\\" && text.indexOf("'", i + 2) - i <= 10))
    ) {
      i = text.indexOf("'", i + 2);
      continue;
    }
    const word = /^(fn|mod)\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(
      text.slice(i, i + 80),
    );
    if (word && !/[A-Za-z0-9_]/.test(text[i - 1] ?? " ")) {
      pending = { kind: word[1] as "fn" | "mod", name: word[2], at: i };
      brackets = 0;
      i += word[0].length - 1;
      continue;
    }
    // An `impl` that starts an item, its line's first word, not one of a
    // type, `x: impl Trait` or `-> impl Trait` (#1174).
    if (
      text.startsWith("impl", i) &&
      !/[A-Za-z0-9_]/.test(text[i + 4] ?? " ") &&
      /^\s*(?:unsafe\s+|default\s+)?$/.test(text.slice(text.lastIndexOf("\n", i - 1) + 1, i))
    ) {
      pending = { kind: "impl", name: "", at: i };
      brackets = 0;
      i += 3;
      continue;
    }
    if (pending && c === "[") brackets++;
    if (pending && c === "]") brackets--;
    if (c === ";" && pending && brackets === 0) {
      // A module of a file of its own, `mod tests;`.
      if (pending.kind === "mod") {
        const from = attributesFrom(lines, lineAt[pending.at]);
        const attributes = lines.slice(from - 1, lineAt[pending.at]).join("\n");
        found.push({
          kind: "mod",
          name: pending.name,
          from,
          to: lineAt[i],
          test: false,
          cfgTest: cfgsOf(attributes).some(onlyForTests),
          cfgs: cfgsOf(attributes),
          file: true,
        });
      }
      pending = undefined;
    }
    if (c === "{") {
      let item: Item | undefined;
      if (pending) {
        // Its attributes: those just above it (`attributesFrom`).
        const from = attributesFrom(lines, lineAt[pending.at]);
        const attributes = lines.slice(from - 1, lineAt[pending.at]).join("\n");
        item = {
          kind: pending.kind,
          name: pending.name,
          from,
          to: 0,
          test: /#\[(?:\w+::)*test\b/.test(attributes),
          cfgTest: cfgsOf(attributes).some(onlyForTests),
          cfgs: cfgsOf(attributes),
        };
        pending = undefined;
      }
      open.push({ item });
    }
    if (c === "}") {
      const closed = open.pop();
      if (closed?.item) {
        closed.item.to = lineAt[i];
        found.push(closed.item);
      }
    }
  }
  return found;
}

/** A module's path in its crate, of its file, `src/a/b.rs` or `src/a/b/mod.rs`
 * `a::b`, `src/lib.rs` none. */
export function modulePath(file: string): string[] {
  const parts = file
    .replace(/^src\//, "")
    .replace(/\.rs$/, "")
    .split("/");
  if (
    parts.at(-1) === "mod" ||
    (parts.length === 1 && (parts[0] === "lib" || parts[0] === "main"))
  )
    parts.pop();
  return parts;
}

/** The file that declares the module of `file`, `src/a/b.rs`'s `mod b;` in
 * `src/a.rs` or `src/a/mod.rs`, and the module's name; none of `src/lib.rs`. */
export function declaredIn(file: string, exists: (path: string) => boolean): { parent: string; name: string } | undefined {
  const path = modulePath(file);
  const name = path.at(-1);
  if (!name) return undefined;
  const above = path.slice(0, -1);
  const parent =
    above.length === 0
      ? exists("src/lib.rs")
        ? "src/lib.rs"
        : "src/main.rs"
      : exists(`src/${above.join("/")}.rs`)
        ? `src/${above.join("/")}.rs`
        : `src/${above.join("/")}/mod.rs`;
  return { parent, name };
}

/** Whether `file`'s module, or one it's in, is declared `#[cfg(test)]`,
 * `#[cfg(test)] mod tests;`: built only for tests. */
export function testOnlyFile(file: string, read: (path: string) => string | undefined): boolean {
  const declared = declaredIn(file, (path) => read(path) !== undefined);
  if (!declared) return false;
  const text = read(declared.parent);
  if (text === undefined) return false;
  const all = items(text);
  const at = all.find((item) => item.kind === "mod" && item.file && item.name === declared.name);
  if (!at) return false;
  return (
    at.cfgTest ||
    all.some((item) => item.kind === "mod" && item.cfgTest && item.from < at.from && at.to <= item.to) ||
    testOnlyFile(declared.parent, read)
  );
}

/** What to cut of the test build for an error at `line` of `file`, whose
 * text is `text`: the `#[test]` it's in, by its path, `tests::t`, or a
 * function or an impl only tests reach, one inside a `#[cfg(test)]` module or a file
 * only tests build, `testOnly`, each its attributes' first line; or
 * nothing, where it's the library's own code. `path` is its module's, of an
 * integration test's crate `testModulePath`'s. `why` is the error's: an
 * unresolved import of names of a `use` of several only tests build is
 * those names taken off its line, `drop`, not the whole `use`. */
export function cutFor(file: string, text: string, line: number, testOnly = false, path = modulePath(file), why = ""): { from: number; test?: string; item?: string; drop?: string[] } | undefined {
  const all = items(text);
  const around = all
    .filter((item) => item.from <= line && line <= item.to)
    .sort((a, b) => b.from - a.from);
  const test = around.find((item) => item.kind === "fn" && item.test);
  const mods = (at: Item) =>
    all
      .filter(
        (item) =>
          item.kind === "mod" && item.from < at.from && at.to <= item.to,
      )
      .sort((a, b) => a.from - b.from);
  if (test)
    return {
      from: test.from,
      test: [
        ...path,
        ...mods(test).map((m) => m.name),
        test.name,
      ].join("::"),
    };
  // The outermost function or impl around it, inside a module only tests
  // build: base64's `impl Distribution<..> for Standard` of its tests.
  const outer = around.filter((item) => item.kind !== "mod").at(-1);
  // A function's path too, `item`: the tests a macro of its makes, rstest's
  // `#[apply(..)]` or `#[rstest]`'s `engine::tests::f::case_1`, are cut with it.
  if (outer && (testOnly || mods(outer).some((m) => m.cfgTest))) {
    return { from: outer.from, ...(outer.kind === "fn" ? { item: [...path, ...mods(outer).map((m) => m.name), outer.name].join("::") } : {}) };
  }
  // A `use` only tests build, of what an earlier round cut: base64's
  // `use crate::tests::random_config;` in its test files (#1174). Its
  // first line, where the error is in it, of one of several lines too. Of
  // a `use` of several names, `use crate::{alphabet, tests::random_config};`,
  // the one name alone, which its users then miss and are cut for: the
  // whole `use` cut, every name its file used went with it, and base64's
  // trait and const of its tests, which no cut takes, refused its library.
  const inTests = testOnly || all.some((item) => item.kind === "mod" && item.cfgTest && item.from < line && line <= item.to);
  if (!outer && inTests) {
    const lines = text.split("\n");
    for (let at = line; at >= 1 && (at === line || !lines[at - 1].includes(";")); at--) {
      if (!/^\s*(?:pub(?:\([^)]*\))?\s+)?use\b/.test(lines[at - 1])) continue;
      const end = lines.findIndex((text, i) => i >= at - 1 && text.includes(";"));
      const braced = lines.slice(at - 1, end + 1).join("\n").includes("{");
      // rustc says each name of one `use` it can't resolve in one error,
      // `unresolved imports \`a\`, \`b\``: those of the error's line.
      const names = /^unresolved imports? `/.test(why) ? [...why.matchAll(/`(?:\w+::)*(\w+)`/g)].map((m) => m[1]) : [];
      const drop = names.filter((name) => new RegExp(`\\b${name}\\b`).test(lines[line - 1]));
      return braced && drop.length > 0 ? { from: line, drop } : { from: at };
    }
  }
  return undefined;
}

/** Each of `names`, tests, under a function `items` names, cut with it:
 * the tests a macro made of it, rstest's `f::case_1`, which no `#[test]`
 * names, so they'd be read as not run, a wrong answer (#1174). */
export function underItems(names: string[], items: Record<string, string>): Record<string, string> {
  const cut: Record<string, string> = {};
  for (const name of names) {
    const item = Object.keys(items).find((path) => name.startsWith(`${path}::`));
    if (item !== undefined) cut[name] = items[item];
  }
  return cut;
}

/** `text` with `name` of a `use` of several names taken off `line`, its
 * path and comma with it: `tests::random_config,` of `use crate::{alphabet,
 * tests::random_config};` (#1174). */
export function dropped(text: string, line: number, name: string): string {
  const lines = text.split("\n");
  lines[line - 1] = lines[line - 1].replace(new RegExp(`(?:\\w+::)*\\b${name}\\b\\s*,?\\s*`), "");
  return lines.join("\n");
}

/** `#[cfg(any())]` above each line of `froms` in `text`, so what starts
 * there isn't built. */
export function cut(text: string, froms: number[]): string {
  const lines = text.split("\n");
  for (const from of [...new Set(froms)].sort((a, b) => b - a)) {
    const indent = /^\s*/.exec(lines[from - 1])?.[0] ?? "";
    lines.splice(
      from - 1,
      0,
      `${indent}#[cfg(any())] // rust-js's Crater: refused`,
    );
  }
  return lines.join("\n");
}

/** A module's path in an integration test's crate, whose root is `root`,
 * `tests/version.rs`: none of the root, `util` of `tests/util/mod.rs` or
 * `tests/util.rs`. */
export function testModulePath(file: string, root: string): string[] {
  if (file === root) return [];
  return file
    .replace(/^tests\//, "")
    .replace(/(\/mod)?\.rs$/, "")
    .split("/");
}

/** Where the `#[test]` of path `test` starts, `uint::tests::binary` or
 * `tests/zip.rs::t`: its file, the deepest of its module path there is,
 * and the line of its attributes' first, in the modules the rest names. */
export function placeOf(test: string, read: (path: string) => string | undefined): { file: string; from: number } | undefined {
  const integration = /^(tests\/[^/]+\.rs)::(.+)$/.exec(test);
  const segments = (integration?.[2] ?? test).split("::");
  const name = segments.pop()!;
  const files = integration
    ? [{ file: integration[1], inner: segments }]
    : segments
        .map((_, k) => segments.length - k)
        .flatMap((k) => [`src/${segments.slice(0, k).join("/")}.rs`, `src/${segments.slice(0, k).join("/")}/mod.rs`].map((file) => ({ file, inner: segments.slice(k) })))
        .concat([{ file: "src/lib.rs", inner: segments }]);
  for (const { file, inner } of files) {
    const text = read(file);
    if (text === undefined) continue;
    const all = items(text);
    const found = all.find(
      (item) =>
        item.kind === "fn" &&
        item.test &&
        item.name === name &&
        all.filter((m) => m.kind === "mod" && m.from < item.from && item.to <= m.to).sort((a, b) => a.from - b.from).map((m) => m.name).join("::") === inner.join("::"),
    );
    return found ? { file, from: found.from } : undefined;
  }
  return undefined;
}

/** A file's inner attributes, `#![..]`, each its own, at its start before
 * any item, comments and blank lines skipped. */
export function innerAttributes(text: string): string {
  const found: string[] = [];
  let at = 0;
  for (;;) {
    const rest = text.slice(at);
    const skipped = /^(\s+|\/\/[^\n]*\n?|\/\*[\s\S]*?\*\/)/.exec(rest);
    if (skipped) {
      at += skipped[0].length;
      continue;
    }
    if (!rest.startsWith("#![")) return found.join("\n");
    let depth = 0;
    let end = 2;
    for (; end < rest.length; end++) {
      if (rest[end] === "[") depth++;
      else if (rest[end] === "]" && --depth === 0) break;
    }
    found.push(rest.slice(0, end + 1));
    at += end + 1;
  }
}

/** Whether the test `name`, `a::b::tests::t`, is of a module a cfg leaves
 * off wasm32 in a test build, `#[cfg(unix)] mod tz_info;`, read of the
 * crate's source through `read` (#1153); or of an integration test's file,
 * `tests/dateutils.rs::t`, whose own `#![cfg(unix)]` leaves it off; or is
 * itself left off, `#[cfg(target_pointer_width = "64")] #[test] fn t`,
 * unicode-ident's `test_trieset_size`, which the Crater took for a test
 * the JS lost. */
export function offTarget(name: string, read: (path: string) => string | undefined): boolean {
  // The test function itself, of `text`, within `scope` where it's inline.
  const fnOff = (text: string, fn: string, scope?: { from: number; to: number }) =>
    items(text).some(
      (item) =>
        item.kind === "fn" && item.name === fn && (!scope || (scope.from < item.from && item.to <= scope.to)) && item.cfgs.some((cfg) => cfgHolds(cfg, true) === false),
    );
  const integration = /^(tests\/[^/]+\.rs)::/.exec(name)?.[1];
  if (integration !== undefined) {
    const text = read(integration) ?? "";
    const rest = name.slice(integration.length + 2).split("::");
    return cfgsOf(innerAttributes(text).replaceAll("#![", "#[")).some((cfg) => cfgHolds(cfg, true) === false) || (rest.length === 1 && fnOff(text, rest[0]));
  }
  const modules = name.split("::").slice(0, -1);
  let file = read("src/lib.rs") !== undefined ? "src/lib.rs" : "src/main.rs";
  let scope: { from: number; to: number } | undefined;
  const path: string[] = [];
  for (const module of modules) {
    const text = read(file);
    if (text === undefined) return false;
    const declared = items(text).find((item) => item.kind === "mod" && item.name === module && (!scope || (scope.from < item.from && item.to <= scope.to)));
    if (!declared) return false;
    if (declared.cfgs.some((cfg) => cfgHolds(cfg, true) === false)) return true;
    path.push(module);
    if (declared.file) {
      const flat = `src/${path.join("/")}.rs`;
      file = read(flat) !== undefined ? flat : `src/${path.join("/")}/mod.rs`;
      scope = undefined;
    } else scope = declared;
  }
  const text = read(file);
  return text !== undefined && fnOff(text, name.split("::").at(-1)!, scope);
}
