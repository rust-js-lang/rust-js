// Architectural dependencies are executable rules, not only a diagram.
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true }).flatMap(entry =>
  entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// Every source module is in a layer, so a module added later is checked
// by the rules of its layer, not left out of a list (found in review).
//
//   driver ──► front end (rustc) ──► owned output ──► printing (oxc, TypeScript)
const layers = {
  driver: ["src/main.rs", "src/cargo.rs"],
  front: ["src/lower.rs", "src/lower/", "src/jsx_syntax.rs", "src/jsx_syntax/"],
  owned: ["src/library.rs", "src/reachability.rs", "src/link.rs", "src/names.rs", "src/program.rs", "src/js.rs",
    "src/prepare.rs", "src/output.rs", "src/publish.rs", "src/manifest.rs", "src/runtime.rs", "src/settings.rs", "src/hooks.rs",
    "src/paths.rs"],
  printing: ["src/to_oxc.rs", "src/format.rs", "src/typescript.rs"],
};
const layerOf = (file: string) =>
  Object.entries(layers).find(([, paths]) => paths.some(p => p.endsWith("/") ? file.startsWith(p) : file === p))?.[0];
const sources = files("src").filter(path => path.endsWith(".rs"));

test("every source module is in a layer", () => {
  expect(sources.filter(file => !layerOf(file))).toEqual([]);
  expect(sources.length).toBeGreaterThan(40);
});

// The layers each layer's modules may use, crate-wide: a module of the
// crate is its layer's, `crate::js` owned output's. Found in review: the
// checks below forbid chosen APIs, so an owned module using lowering
// broke none. Printing's own modules are named where an owned one starts
// it: `output.rs` prints each module it plans, its JS by oxc and its
// declarations by TypeScript (ADR 0207), and `hooks.rs` formats what a
// hook returns.
const allowed: Record<string, string[]> = {
  driver: ["driver", "front", "owned", "printing"],
  front: ["front", "owned"],
  owned: ["owned"],
  printing: ["printing", "owned"],
};
const orchestration: Record<string, string[]> = {
  "src/output.rs": ["src/to_oxc.rs", "src/typescript.rs"],
  "src/hooks.rs": ["src/format.rs"],
};
// A file's module path, `src/lower/drops/types.rs`'s `lower::drops::types`;
// `main.rs` is the crate's root.
const modulePath = (file: string): string[] => {
  const parts = file.replace(/^src\//, "").replace(/\.rs$/, "").split("/");
  if (parts.at(-1) === "mod") parts.pop();
  return parts.length === 1 && parts[0] === "main" ? [] : parts;
};
// The first names after a path's base: `x` of `x::y`, or each of a
// group's, `{js::{self, Expr}, program}`'s `js` and `program`.
const firstNames = (rest: string): string[] => {
  if (!rest.startsWith("{")) return [rest.match(/^[a-z_]\w*/)?.[0]].filter((n): n is string => !!n);
  const items: string[] = [];
  let depth = 0, item = "";
  for (const c of rest) {
    if (c === "{" && depth++ === 0) continue;
    if (c === "}" && --depth === 0) break;
    if (c === "," && depth === 1) {
      items.push(item);
      item = "";
      continue;
    }
    item += c;
  }
  items.push(item);
  return items.map(item => item.trim().match(/^[a-z_]\w*/)?.[0]).filter((n): n is string => !!n && n !== "self");
};
// What a file names when it takes the crate's root whole, `use crate::*;`
// or `use crate as root;`: every module, then, by a name the paths below
// can't follow (found in review). Only the driver may.
const WHOLE = "the crate's root, whole";
// The crate's top-level modules a file names, as the files they are: by
// `crate::`, or by `super::` up to the crate's root, `super::lower` in a
// top-level file, each `super` first leaving an inline `mod tests { .. }`;
// or `WHOLE`. Not in a comment, a string or a `char`.
const usedModules = (file: string, text: string): string[] => {
  const code = text
    .replace(/"(?:[^"\\]|\\.)*"/g, '""')
    .replace(/'(?:[^'\\]|\\.)'/g, "' '")
    .replace(/\/\/.*$/gm, "");
  const own = modulePath(file);
  const names = new Set<string>();
  const inline: number[] = [];
  let depth = 0;
  for (const m of code.matchAll(/\bmod\s+\w+\s*\{|\{|\}|\b(crate|super)((?:::super)*)(::|\s+as\b)/g)) {
    if (m[0].startsWith("mod")) inline.push(depth++);
    else if (m[0] === "{") depth++;
    else if (m[0] === "}") {
      if (inline.at(-1) === --depth) inline.pop();
    } else {
      // How far up the file's own module it climbs: `crate` to the root.
      const supers = 1 + (m[2].match(/super/g)?.length ?? 0) - inline.length;
      const up = m[1] === "crate" ? own.length : Math.max(supers, 0);
      const base = own.slice(0, own.length - up);
      const rest = code.slice(m.index! + m[0].length);
      // The root, whole: aliased, globbed, or `self` of a group of it.
      const group = rest.startsWith("{") ? rest.slice(1, rest.indexOf("}")) : "";
      const whole = m[3] !== "::" || rest.startsWith("*") || /(^|,)\s*(\*|self\b)/.test(group);
      if (base.length === 0 && whole) names.add(WHOLE);
      for (const name of base.length > 0 ? [base[0]] : firstNames(rest)) names.add(name);
    }
  }
  // Its own module's items are no other module's.
  names.delete(own[0]);
  return [...names]
    .map(name => (name === WHOLE ? name : sources.includes(`src/${name}.rs`) ? `src/${name}.rs` : `src/${name}/`))
    .sort();
};
// The checker finds a module however a file names it (found in review:
// `super::lower` in a top-level file was missed).
test("the layer check finds a module however a file names it", () => {
  const cases: [string, string, string[]][] = [
    ["src/manifest.rs", "use crate::lower::Body;", ["src/lower.rs"]],
    ["src/manifest.rs", "use super::lower::Body;", ["src/lower.rs"]],
    ["src/output.rs", "use crate::{js::{self, Expr}, program, to_oxc};", ["src/js.rs", "src/program.rs", "src/to_oxc.rs"]],
    ["src/lower/drops/types.rs", "use super::super::super::js::Expr;", ["src/js.rs"]],
    ["src/lower/drops/types.rs", "use super::super::traits::Route;", []],
    ["src/format.rs", 'mod tests {\n    use super::*;\n    fn f() { let s = "{"; let c = \'}\'; }\n}\nuse super::lower::Body;', ["src/lower.rs"]],
    ["src/manifest.rs", '// crate::lower, in a comment\nfn f() { let s = "super::lower"; }', []],
    ["src/main.rs", "use crate::cargo; mod tests { use super::lower; }", ["src/cargo.rs", "src/lower.rs"]],
    ["src/manifest.rs", "use crate::*;\nfn sneak(_: lower::Body) {}", [WHOLE]],
    ["src/manifest.rs", "use super::*;", [WHOLE]],
    ["src/manifest.rs", "use crate as root;\nfn sneak(_: root::lower::Body) {}", [WHOLE]],
    ["src/manifest.rs", "use crate::{self as root, js};", ["src/js.rs", WHOLE]],
    ["src/format.rs", "mod tests {\n    use super::*;\n}", []],
    ["src/lower/drops/types.rs", "use super::*;", []],
  ];
  for (const [file, text, expected] of cases) expect(usedModules(file, text), text).toEqual(expected);
});
test("each module uses only the layers its layer may", () => {
  const strays: string[] = [];
  for (const file of sources) {
    const layer = layerOf(file)!;
    for (const used of usedModules(file, read(file))) {
      if (used === WHOLE) {
        if (layer !== "driver") strays.push(`${file} takes ${WHOLE}: name each module it uses`);
        continue;
      }
      const usedLayer = layerOf(used.endsWith("/") ? `${used}mod.rs` : used) ?? layerOf(used.replace(/\/$/, ".rs"));
      if (!usedLayer) strays.push(`${file} uses ${used}, which is in no layer`);
      else if (!allowed[layer].includes(usedLayer) && !orchestration[file]?.includes(used)) {
        strays.push(`${file} (${layer}) uses ${used} (${usedLayer})`);
      }
    }
  }
  expect(strays).toEqual([]);
});

// No modules use each other in a cycle, within a layer too: each is
// understood by what it uses (found in review: hooks and libraries asked
// artifact planning, which runs hooks and writes libraries' manifests, for
// its path and fingerprint helpers). Within the front end, `lower/`'s
// modules are one: they're its parts. A type used without its module's
// path, as a field's, isn't seen here.
test("modules don't use each other in a cycle", () => {
  const key = (file: string) => {
    const [name] = modulePath(file);
    if (!name) return file;
    return sources.includes(`src/${name}.rs`) ? `src/${name}.rs` : `src/${name}/`;
  };
  const edges = new Map<string, Set<string>>();
  for (const file of sources) {
    const from = key(file);
    const to = edges.get(from) ?? new Set<string>();
    for (const used of usedModules(file, read(file))) if (used !== from) to.add(used);
    edges.set(from, to);
  }
  const cycles: string[] = [];
  const state = new Map<string, "open" | "done">();
  const visit = (node: string, path: string[]) => {
    if (state.get(node) === "done") return;
    if (state.get(node) === "open") {
      cycles.push([...path.slice(path.indexOf(node)), node].join(" -> "));
      return;
    }
    state.set(node, "open");
    for (const next of edges.get(node) ?? []) visit(next, [...path, node]);
    state.set(node, "done");
  };
  for (const node of edges.keys()) visit(node, []);
  expect(cycles).toEqual([]);
});

test("owned compiler output and downstream phases do not depend on rustc", () => {
  for (const file of sources.filter(file => layerOf(file) === "owned" || layerOf(file) === "printing")) {
    expect(read(file), file).not.toMatch(/(?:use\s+|\b)rustc_\w+::/);
  }
});

test("linking uses owned output without lowering dependencies", () => {
  for (const file of ["src/reachability.rs", "src/link.rs", "src/names.rs"]) {
    expect(read(file), file).not.toMatch(/(?:crate|super)::lower\b/);
  }
});

// What a module imports of the package is found in its tree before it's
// printed (ADR 0103), not in the printed text, where a string can spell a
// helper's name (found in review).
test("runtime imports are chosen before printing", () => {
  // The printer is given them, chosen where the module is prepared.
  expect(read("src/to_oxc.rs")).not.toMatch(/imported_helpers|read_vars/);
  expect(read("src/output.rs")).toContain("imported_helpers(&helper_sources, &js_module.read_vars())");
  expect(read("src/runtime.rs")).not.toMatch(/fn imported_helpers\([^)]*code: &str/);
});

test("oxc APIs stay behind the printing and formatting adapters", () => {
  for (const file of sources.filter(file => layerOf(file) !== "printing")) {
    expect(read(file), file).not.toMatch(/\boxc_\w+::/);
  }
});

test("lowering and artifact planning cannot publish files", () => {
  for (const file of ["src/lower.rs", ...files("src/lower"), "src/output.rs"]) {
    expect(read(file), file).not.toMatch(/(?:std::fs|fs)::(?:write|rename|remove_file|create_dir|create_dir_all|File::create)\b/);
  }
});

test("library recognition cannot access function emission state", () => {
  for (const file of ["src/lower/recognition.rs", ...files("src/lower/recognition")]) {
    expect(read(file), file).not.toMatch(/\b(?:FnCx|ExprId|Stmt)\b|\bRefCell\s*<|crate::js|runtime::/);
    // HIR visitors accumulate local query results; classification methods
    // must not mutate their shared recognition context.
    expect(read(file), file).not.toMatch(/\bfn\s+(?!visit_)\w+\s*\([^)]*&mut\s+self/);
  }
  expect(read("src/lower/recognition.rs")).toContain("pub(super) struct Recognition");
});

test("body queries cannot access emission state", () => {
  const source = read("src/lower/body_queries.rs");
  expect(source).not.toMatch(/\b(?:FnCx|Dependencies|Evaluation)\b|\bRefCell\s*<|crate::js|runtime::/);
  expect(source).not.toMatch(/\bfn\s+\w+\s*\([^)]*&mut\s+self/);
});

// What an expression can do that can be seen is a question of the THIR
// alone (ADRs 0098, 0139): asked by emission, it never reaches into it.
test("effects analysis cannot access emission state", () => {
  const source = read("src/lower/effects.rs");
  expect(source).not.toMatch(/\b(?:FnCx|Dependencies|Evaluation)\b|\bRefCell\s*<|crate::js|runtime::|super::drops\b/);
  expect(source).not.toMatch(/\bfn\s+\w+\s*\([^)]*&mut\s+self/);
});

// What a body does with its values that have destructors is found before
// it's lowered (ADR 0098): the walk reads the function's context, and
// neither writes JS nor changes what lowering keeps.
test("the destructors' facts are found without emitting", () => {
  // Both walks are given a `DropQuery`: types, the crate's facts, which
  // type parameters have drops and the bounds of the dictionaries given,
  // not the function being lowered (found in review: they were given all
  // of `FnCx`, and one built JS to ask whether a dictionary was given).
  for (const file of ["src/lower/drops/facts.rs", "src/lower/drops/types.rs"]) {
    expect(read(file), file).not.toMatch(/crate::js|runtime::|\bFnCx\b|\bdrop_state\b|\bevidence_for\b|\bitem_drop\(|\bdrop_function\b/);
  }
  // A JS `Expr` too: the facts' `Expr` is the THIR's.
  expect(read("src/lower/drops/types.rs")).not.toMatch(/\bExpr\b/);
});

// Whether a value is a JS iterator is one question, `is_lazy_value`: its
// type says so, or a chain's consumer made it so (ADR 0139). Asked of the
// type alone elsewhere, the two disagreed: `next()` of a chain took all
// of it. Only iterators.rs reads the type's answer, and the marks.
test("iterators.rs alone answers whether a value is lazy", () => {
  for (const file of ["src/lower.rs", ...files("src/lower")].filter(f => f !== "src/lower/iterators.rs" && !f.includes("/recognition"))) {
    expect(read(file), file).not.toMatch(/\.is_lazy_iter\(|\.chains\.\w/);
  }
});

// What the front end keeps as it lowers a function is grouped by concern,
// each group read and written by the module that owns it, which the rest
// ask: `self.is_boxed(var)`, not `self.locals.mut_refs.boxes`. Six modules
// once wrote the `&mut`s' sets, and four the stepped iterators' (the
// architecture audit).
//
// What a generic function was given, its dictionaries, the copied default's
// arguments and the facts of its type parameters, is traits.rs's: seven
// modules read it, four of them instantiating a copied default's arguments
// each its own way (the second architecture audit). Each walk's cache of
// what it found of a type is its walk's own. A path is matched across line
// breaks, `self\n.given\n.type_facts`, which a formatter makes of a long one.
const owners: Record<string, string[]> = {
  "self.drop_state": ["src/lower/drops.rs", "src/lower/drops/types.rs"],
  "self.chains": ["src/lower/iterators.rs"],
  "self.stepping": ["src/lower/iterators.rs"],
  "self.writing": ["src/lower/display.rs"],
  ".mut_refs.": ["src/lower/mut_refs.rs"],
  "self.cloning": ["src/lower/std_impls.rs"],
  "self.given": ["src/lower/traits.rs"],
  "self.walks.representable": ["src/lower/support.rs"],
  "self.walks.assumed": ["src/lower/support.rs"],
  "self.walks.mutated": ["src/lower/copies.rs"],
  "self.walks.clones": ["src/lower/std_impls.rs"],
  "self.walks.clone_assumed": ["src/lower/std_impls.rs"],
};
// `self.given`, matched as `self . given` with any space around each dot.
const statePattern = (state: string) =>
  new RegExp(`${state.split(".").join("\\s*\\.\\s*")}${state.endsWith(".") ? "" : "\\b"}`);
test("each group of a function's state is read and written by its owner", () => {
  const strays: string[] = [];
  for (const file of ["src/lower.rs", ...files("src/lower")]) {
    const text = read(file);
    for (const [state, allowed] of Object.entries(owners)) {
      if (!allowed.includes(file) && statePattern(state).test(text)) strays.push(`${state} in ${file}`);
    }
  }
  expect(strays).toEqual([]);
});

// A JS form rust-js writes for one concept is written by the module that
// owns it, which the rest ask: a generic iterator as a JS iterator, a
// stepped one's `$iter`, and the box of a `Some` that looks like `None`.
// Each was written in two to five modules (the second architecture audit).
const idioms: [string, RegExp, string[]][] = [
  ["Iterator.from", /Expr::var\("Iterator"\),\s*"from"/, ["src/lower/iterators.rs"]],
  ["$iter", /"\$iter"|Helper::Iter\b/, ["src/lower/iterators.rs"]],
  ["$lent", /"\$lent"|Helper::Lent\b/, ["src/lower/iterators.rs"]],
  ["$someNone", /\$someNone/, ["src/lower/options.rs"]],
];
test("a JS form of a concept is written by the concept's owner", () => {
  const strays: string[] = [];
  for (const file of ["src/lower.rs", ...files("src/lower")]) {
    const text = read(file);
    for (const [idiom, pattern, allowed] of idioms) {
      if (!allowed.includes(file) && pattern.test(text)) strays.push(`${idiom} in ${file}`);
    }
  }
  expect(strays).toEqual([]);
});

// calls.rs lowers calls, and nothing else asks it anything: what a function
// is in JS is items.rs's, a `&mut` given to one mut_refs.rs's. Sixteen
// modules called it once (the architecture audit).
test("calls.rs is a dispatcher only lower.rs calls", () => {
  const defined = (file: string) => new Set([...read(file).matchAll(/^ {4}(?:pub\([^)]*\) )?fn (\w+)/gm)].map(m => m[1]));
  const others = ["src/lower.rs", ...files("src/lower")].filter(f => f !== "src/lower/calls.rs");
  const elsewhere = new Set(others.flatMap(f => [...defined(f)]));
  const own = [...defined("src/lower/calls.rs")].filter(name => !elsewhere.has(name));
  expect(own.length).toBeGreaterThan(0);
  for (const file of others.filter(f => f !== "src/lower.rs")) {
    const called = own.filter(name => new RegExp(`\\bself\\.${name}\\(`).test(read(file)));
    expect([file, called]).toEqual([file, []]);
  }
});

// A question recognition answers is asked of it, or through shortcuts.rs,
// where each shortcut is one line, so where it's answered is plain. They
// were in ten modules once, each looking like that module's own.
test("shortcuts to recognition are in shortcuts.rs, and only shorten", () => {
  const forward = /^ {4}(?:pub\([^)]*\) )?fn (\w+)(?:<[^>]*>)?\([^{]*\{\n\s+self\.recognition\(\)\.\w+\([^;{}]*\)\n {4}\}\n/gm;
  for (const file of ["src/lower.rs", ...files("src/lower")].filter(f => f !== "src/lower/shortcuts.rs" && !f.includes("/recognition"))) {
    // iterators.rs's own way to the answer its rule keeps it alone in asking.
    const found = [...read(file).matchAll(forward)].map(m => m[1]).filter(name => !(file.endsWith("/iterators.rs") && name === "is_lazy_iter"));
    expect([file, found]).toEqual([file, []]);
  }
  const shortcuts = read("src/lower/shortcuts.rs");
  const functions = [...shortcuts.matchAll(/^ {4}(?:pub\([^)]*\) )?fn /gm)].length;
  expect([...shortcuts.matchAll(forward)].length).toBe(functions);
});

// A call is lowered by a dispatcher that hands it to what knows it: the
// crate's own functions, bindings and closures to `special_call`, a std
// function to `std_call` and its domain's function (vecs.rs, options.rs,
// cells.rs, ..). Each was a function of a thousand lines once: what's new
// goes where its kind is, not in the dispatcher.
test("the dispatchers of calls and of what's called stay short", () => {
  const length = (file: string, signature: string) => {
    const lines = read(file).split("\n");
    const start = lines.findIndex(line => line.startsWith(signature));
    expect(start, signature).toBeGreaterThanOrEqual(0);
    return lines.findIndex((line, k) => k > start && line === "    }") - start;
  };
  expect(length("src/lower/calls.rs", "    pub(super) fn call(")).toBeLessThanOrEqual(60);
  expect(length("src/lower/recognition.rs", "    pub(super) fn classify(")).toBeLessThanOrEqual(15);
});

// Recognition is what the rest asks: it asks neither the destructors'
// analysis nor the effects one, which ask it.
test("recognition depends on neither destructors nor effects", () => {
  for (const file of ["src/lower/recognition.rs", ...files("src/lower/recognition")]) {
    expect(read(file), file).not.toMatch(/\b(?:super|crate::lower)::(?:drops|effects)\b/);
  }
});

test("library identity checks and method tables stay in recognition", () => {
  for (const file of files("src/lower").filter(file => !file.includes("/recognition") && !file.endsWith("/library.rs"))) {
    // The scalar linkage adapter owns canonical crate names, not library intrinsics.
    expect(read(file), file).not.toMatch(/\.crate_name\s*\(|fn classify(?:_\w+)?\s*\(/);
  }
  for (const file of ["src/lower/calls.rs", "src/lower/ordering.rs", "src/lower/traits.rs"]) {
    expect(read(file), file).not.toMatch(/match\s+(?:self\.tcx\.item_name\(\w+\)|name)\.as_str\(\)/);
  }
});

// Only recognition.rs spells std's names: the rest ask it, `is_std_type(ty,
// StdItem::Result)`, `std_item(tcx, StdItem::Ord)`. Fourteen modules had 58
// checks of their own, by diagnostic item, path or method name (the second
// architecture audit).
const identity: [string, RegExp][] = [
  ["a diagnostic item", /\b(?:is|get)_diagnostic_item\s*\(/],
  ["a std type's name", /Symbol::intern\("[A-Z][A-Za-z]+"\)/],
  ["a path compared", /def_path_str\([^)]*\)(?:\.as_str\(\))?\s*(?:==|\.starts_with|\.contains)/],
  ["a method's name compared", /item_name\([^)]*\)(?:\.as_str\(\))?\s*(?:==|\.starts_with|\.ends_with|\.contains)|match\s+(?:self\.)?tcx\.item_name\([^)]*\)\.as_str\(\)/],
  ["a std type by its name", /\bis_std_adt\s*\(/],
];
test("only recognition spells std's names", () => {
  const strays: string[] = [];
  for (const file of ["src/lower.rs", ...files("src/lower")].filter(f => !f.includes("/recognition") && !f.endsWith("/shortcuts.rs"))) {
    const text = read(file);
    for (const [what, pattern] of identity) if (pattern.test(text)) strays.push(`${what} in ${file}`);
  }
  expect(strays).toEqual([]);
});

test("Vite delegates build preparation and validates build results", () => {
  const plugin = read("vite-plugin/index.js");
  expect(plugin).not.toMatch(/react\/build\.sh|libreact\.rmeta|rust-toolchain\.toml|child_process/);
  expect(plugin).toContain("createNativeBuilder");
  expect(plugin).toContain("parseManifest");
});

test("native and WASI compiler dependency versions agree", () => {
  const dependencies = (path: string) => {
    const section = read(path).split("[dependencies]\n")[1].split(/\n\[/)[0];
    return new Map(section.split("\n").filter(line => /^(?:serde|oxc_)/.test(line)).map(line => {
      const equals = line.indexOf("=");
      return [line.slice(0, equals).trim(), line.slice(equals + 1).trim()];
    }));
  };
  expect(dependencies("wasm/Cargo.toml")).toEqual(dependencies("Cargo.toml"));
});

test("crate analysis cannot emit functions or invoke linking", () => {
  for (const path of ["src/lower/analysis.rs", ...files("src/lower/analysis")]) {
    expect([path, read(path)]).not.toEqual([path, expect.stringMatching(/\b(?:FnCx|CrateFacts|LoweredModule|LoweredFn)\b|\blink::|\.lower_(?:fn|codec|dictionary)\(/)]);
  }
});

test("lowering returns symbolic output and leaves linking to the driver", () => {
  expect(read("src/lower/pipeline.rs")).not.toMatch(/crate::link|link::|runtime::resolve/);
  expect(read("src/lower/pipeline.rs")).toContain("Option<Unlinked>");
  expect(read("src/main.rs")).toContain("link::link(unlinked)");
});
