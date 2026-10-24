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
  driver: ["src/main.rs", "src/command.rs", "src/cargo.rs", "src/panic_runtime.rs"],
  front: ["src/lower.rs", "src/lower/", "src/jsx_syntax.rs", "src/jsx_syntax/"],
  owned: ["src/library.rs", "src/reachability.rs", "src/link.rs", "src/names.rs", "src/program.rs", "src/comments.rs", "src/js.rs", "src/js/",
    "src/prepare.rs", "src/prepare/", "src/output.rs", "src/publish.rs", "src/manifest.rs", "src/runtime.rs", "src/settings.rs", "src/hooks.rs",
    "src/paths.rs"],
  printing: ["src/to_oxc.rs", "src/to_oxc/", "src/format.rs", "src/typescript.rs", "src/prettier.rs"],
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
// declarations by TypeScript (ADR 0207), and has the app's Prettier lay
// them out, the map moved to its layout (ADR 0381); `hooks.rs` formats
// what a hook returns.
const allowed: Record<string, string[]> = {
  driver: ["driver", "front", "owned", "printing"],
  front: ["front", "owned"],
  owned: ["owned"],
  printing: ["printing", "owned"],
};
const orchestration: Record<string, string[]> = {
  "src/output.rs": ["src/to_oxc.rs", "src/typescript.rs", "src/prettier.rs", "src/format.rs"],
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

// Any mention of a rustc crate, `use rustc_middle as m;` too, whose
// paths, `m::ty`, name no `rustc_` (found in the third architecture audit).
// The front end runs before linking, preparation and publication: of the
// owned output it uses the tree, names, the program, libraries and the
// reachability graph, never a later phase (the second review: tidy called
// preparation's `constants`, and MIR's lowering its `unsuffixed_params`).
test("the front end uses no phase after it", () => {
  const later = /crate::(prepare|link|output|publish|manifest|settings|hooks|paths)\b/;
  const strays = ["src/lower.rs", ...files("src/lower"), "src/jsx_syntax.rs", ...files("src/jsx_syntax")]
    .filter(file => later.test(read(file)));
  expect(strays).toEqual([]);
});

test("owned compiler output and downstream phases do not depend on rustc", () => {
  for (const file of sources.filter(file => layerOf(file) === "owned" || layerOf(file) === "printing")) {
    expect(read(file), file).not.toMatch(/\brustc_\w+/);
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

// Each runtime helper is one lowering asks for, or another helper needs: a
// helper nothing asks for is JS the package ships for nothing (the third
// architecture audit: `$sliceEnd`'s, since THIR's lowering went).
test("each runtime helper is asked for, or needed by another", () => {
  const runtime = read("src/runtime.rs");
  const list = runtime.slice(runtime.indexOf("helpers! {"), runtime.indexOf("}", runtime.indexOf("helpers! {")));
  const helpers = [...list.matchAll(/^\s+([A-Z]\w*),/gm)].map(m => m[1]);
  expect(helpers.length).toBeGreaterThan(200);
  // Asked for by being written, by a name it exports (ADR 0103), or read
  // by another helper's JS.
  const files = new Map([...runtime.matchAll(/Helper::(\w+) => include_str!\("runtime\/([\w.]+)"\)/g)].map(m => [m[1], m[2]]));
  const source = (helper: string) => read(`src/runtime/${files.get(helper)}`);
  const declared = (helper: string) => [...source(helper).matchAll(/^(?:async function\*? |function\*? |class |const |let )(\$\w+)/gm)].map(m => m[1]);
  const writing = sources.filter(file => file !== "src/runtime.rs").map(read).join("\n");
  const readBy = (helper: string, name: string) => helpers.some(other => other !== helper && new RegExp(`(?<![\\w.$"'])\\$${name.slice(1)}\\b(?!\\s*:)`).test(source(other)));
  const unasked = helpers.filter(name => !declared(name).some(own => writing.includes(`"${own}"`) || readBy(name, own)));
  expect(unasked).toEqual([]);
});

// A helper is written as the runtime's, `Expr::helper("$cmp")`, which
// checks the runtime has it and is asked for by being written (ADR 0103),
// not as a variable of its name, `Expr::var("$cmp")`, which checks nothing.
test("a helper is written as the runtime's", () => {
  for (const file of ["src/lower.rs", ...files("src/lower")]) {
    expect(read(file), file).not.toMatch(/Expr::var\((?:if [^{\n]*\{ )?"\$/);
  }
});

// A `$` name the compiler writes is one the runtime declares: a helper it
// asks for, compares against, `callee.helper_name() == Some("$iter")`, or
// spells, so a misspelled one is found here and not by a comparison that
// never matches (the second architecture review). Each other `$` name is
// listed with what it is, spelled once.
test("each `$` name the compiler writes is the runtime's", () => {
  const runtime = files("src/runtime").filter(file => file.endsWith(".js")).map(read).join("\n");
  const declared = new Set([...runtime.matchAll(/^(?:export )?(?:async function\*? |function\*? |class |const |let )(\$\w+)/gm)].map(m => m[1]));
  const others = new Map([
    ["$someNone", "src/lower/std_types/option.rs"], // a `Some` of `None`'s box's field (ADR 0051)
    ["$drop", "src/js.rs"], // a `dyn`'s dictionary's destructor
    ["$steps", "src/lower/mir/rvalues.rs"], // a compiled std iterator's `next` and `size_hint`, `$stdIterator`'s, its clone's too (ADR 0561)
    ["$tested", "src/prepare.rs"], // a variable marked as a test reads it, never printed
    ["$subject", "src/lower/mir/hints.rs"], // what a variant's test is of, as a `switch`'s constant has it, never printed (ADR 0668)
    ["$circleShape", "src/names.rs"], // a dictionary's cache, in a unit test
  ]);
  const unknown: string[] = [];
  for (const file of sources.filter(file => file !== "src/runtime.rs")) {
    for (const [, name] of read(file).matchAll(/"(\$[a-zA-Z]\w*)"/g)) {
      if (!declared.has(name) && others.get(name) !== file) unknown.push(`${file}: ${name}`);
    }
  }
  expect(unknown).toEqual([]);
});

// Printing lays out what it's given: what a helper means, the same place
// `$index(v, i)` names, or a statement's readable form, `s += t`, is
// decided before it (the third architecture audit: the printer decided
// both).
test("printing names no runtime helper", () => {
  for (const file of sources.filter(file => layerOf(file) === "printing")) {
    expect(read(file), file).not.toMatch(/"\$[a-zA-Z]/);
  }
});

test("oxc APIs stay behind the printing and formatting adapters", () => {
  for (const file of sources.filter(file => layerOf(file) !== "printing")) {
    expect(read(file), file).not.toMatch(/\boxc_\w+/);
  }
});

// Files are written by the publisher, and by the driver, which stages a
// build and writes a report it's asked for: no other module, by any of
// std's ways to (found in the third architecture audit: the rule was of
// lowering and artifact planning alone, and of four of the ways).
test("only the publisher and the driver write files", () => {
  const writes = /\bfs::(?:write|rename|remove_\w+|create_dir\w*|copy|hard_link)\b|\bFile::(?:create|options)\b|\bOpenOptions\b/;
  const writers = ["src/publish.rs", "src/main.rs"];
  expect(sources.filter(file => !writers.includes(file) && writes.test(read(file)))).toEqual([]);
  for (const file of writers) expect(read(file), file).toMatch(writes);
});

// A panic's `Error` is made one way, `Expr::panic` (ADR 0706): five
// places wrote `new Error(..)` each, and `panic!("")` was `new Error("")`.
test("a panic's Error is made in one place", () => {
  const made = /Expr::var\("Error"\)/;
  expect(sources.filter(file => made.test(read(file)))).toEqual(["src/js/make.rs"]);
});

// Whether a label is MIR's, `bb12`, or one the program wrote, `'desc`, is
// answered by `is_block_label` alone (ADR 0742): tidy's decisions had a
// copy of its test.
test("whether a label is MIR's is asked of is_block_label", () => {
  const tests = /strip_prefix\("bb"\)/;
  expect(sources.filter(file => tests.test(read(file)))).toEqual(["src/lower/mir/queries.rs"]);
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

// Which of a body's string offsets are places in JS's units is a question
// of its MIR (ADR 0642): it reads the body and answers, and writes no JS.
test("a body's string offsets are found without emitting", () => {
  const source = read("src/lower/mir/offsets.rs");
  expect(source).not.toMatch(/crate::js|\bjs::|\bExpr\b|\bStmt\b|runtime::|\bState\b/);
  // It reads the function's context, and changes only what it finds.
  const asked = source.split(/\nimpl/).filter((block) => /^<'a, 'tcx> FnCx</.test(block)).map((block) => block.split("\n}\n")[0]);
  expect(asked.length).toBeGreaterThan(0);
  for (const block of asked) expect(block).not.toMatch(/&mut\s+self/);
});

test("body queries cannot access emission state", () => {
  const source = read("src/lower/body_queries.rs");
  expect(source).not.toMatch(/\b(?:FnCx|Dependencies|Evaluation)\b|\bRefCell\s*<|crate::js|runtime::/);
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

// Tidying a body lowered from its MIR (ADR 0364) rewrites JS as JS, from
// what lowering's `Hints` say of it: it asks neither rustc nor the function
// being lowered, so it's understood from the JS alone (the third
// architecture audit).
test("tidying asks neither rustc nor the function being lowered", () => {
  for (const file of ["src/lower/mir/tidy.rs", ...files("src/lower/mir/tidy")]) {
    expect(read(file), file).not.toMatch(/\brustc_\w+|\bTyCtxt\b|\btcx\b|\bFnCx\b|\bDefId\b/);
  }
});

// A tree's walks are written once, `js/walk.rs`, and made twice: to read it,
// `each_list` and `each_expr`, and to change it, `statement_lists` and
// `each_expr_mut`. What only reads a tree walks it as it is: twenty
// analyses copied a tree to walk it with the walk that changes one.
test("what reads a tree walks it without copying it", () => {
  const copying = /(?:statement_lists|each_expr_mut)\(&mut (?:vec!\[)?[\w.\[\]]+\.(?:clone|to_vec)\(\)/;
  const offenders = files("src").filter((file) => file.endsWith(".rs") && copying.test(read(file)));
  expect(offenders).toEqual([]);
});

// Tidy's passes are one list, `stages::STAGES`, in stages that each say what
// they leave a body with (ADR 0659): `tidy` runs the list and checks it, and
// calls no pass itself, so the order, and why, are read in one place. A pass
// is a `fn`, which captures nothing: what it reads of the body, it works out
// as the body is when it runs, never a fact an earlier pass left.
test("tidy runs its passes from one list", () => {
  const tidy = read("src/lower/mir/tidy.rs");
  const body = tidy.slice(tidy.indexOf("pub(super) fn tidy("), tidy.indexOf("\n}\n", tidy.indexOf("pub(super) fn tidy(")));
  expect(body).toContain("stages::STAGES");
  expect(body).not.toMatch(/statement_lists|bottom_up|each_expr_mut|declarations\(/);
  expect(read("src/lower/mir/tidy/stages.rs")).toContain("pub(super) run: fn(&mut Vec<Stmt>, &Hints),");
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
// breaks, `self\n.given\n.type_facts`, which a formatter makes of a long one,
// and through a getter, `self.walks().mutated`, which five of them were
// read by, unseen (the third architecture audit).
const owners: Record<string, string[]> = {
  "self.drop_state": [
    "src/lower/drops.rs",
    "src/lower/drops/functions.rs",
    "src/lower/drops/parts.rs",
    "src/lower/drops/scopes.rs",
    "src/lower/drops/uses.rs",
  ],
  // What lowering from MIR found of a body's variables, nine sets once
  // fields of `Locals` that mir.rs wrote and jsx.rs and binding_calls.rs
  // read (the third architecture audit).
  "self.locals.mir": [
    "src/lower/mir.rs",
    "src/lower/mir/assign.rs",
    "src/lower/mir/bounds.rs",
    "src/lower/mir/closures.rs",
    "src/lower/mir/places.rs",
    "src/lower/mir/rvalues.rs",
    "src/lower/mir/structure.rs",
    "src/lower/mir/temporaries.rs",
  ],
  // What lowering a function found of drops, which the pipeline gathers
  // and resolves: the drops module's to write, where `CrateFacts` held it
  // in cells every function wrote (the fourth review).
  "self.drops_observed": ["src/lower/drops.rs", "src/lower/drops/uses.rs"],
  "self.writing": ["src/lower/display.rs", "src/lower/display/debug.rs", "src/lower/display/written.rs"],
  "self.cloning": ["src/lower/std_impls.rs"],
  "self.ordering": ["src/lower/ordering.rs"],
  "self.given": ["src/lower/traits/given.rs", "src/lower/traits/calls.rs", "src/lower/traits/impls.rs", "src/lower/traits/routes.rs"],
  "self.walks.representable": ["src/lower/support.rs"],
  "self.walks.assumed": ["src/lower/support.rs"],
  "self.walks.mutated": ["src/lower/copies.rs"],
  "self.walks.clones": ["src/lower/std_impls.rs"],
  "self.walks.clone_assumed": ["src/lower/std_impls.rs"],
  "self.codecs": ["src/lower/serde.rs"],
};
// The rest of a function's context, which each module may read, and why:
// what it lowers and where (set entering a body, `bodies.rs`), the names it
// has given (any module names a local, `fresh`), and what it uses of other
// crates (`items.rs` notes each, MIR's lowering hands them on).
const shared: Record<string, string> = {
  krate: "the crate's facts, read only",
  tcx: "rustc, read only",
  typing_env: "the body's typing environment, set entering it",
  frame: "the body being lowered, its THIR, what's found of it before it's lowered and its owner, swapped whole entering a nested one",
  module: "the module it's in",
  module_names: "the module's names, read only",
  item: "the item lowered",
  jsx: "whether it writes JSX",
  names: "each name given, by `fresh`, kept per body by bodies.rs",
  locals: "a body's variables; its MIR facts, `locals.mir`, are mir.rs's",
  dependencies: "what it uses of other crates, noted by items.rs",
};
// Each field of `FnCx` is an owner's, or the shared context above: a field
// added with neither is one nothing says who may change (the second review).
test("each field of a function's context has an owner, or is its shared context", () => {
  const lower = read("src/lower.rs");
  const struct = lower.slice(lower.indexOf("struct FnCx<"), lower.indexOf("\n}\n", lower.indexOf("struct FnCx<")));
  const fields = [...struct.matchAll(/^    (?:pub\([a-z]+\) )?([a-z_]+):/gm)].map(m => m[1]);
  const owned = new Set(Object.keys(owners).map(path => path.split(".")[1]));
  expect(fields.filter(field => !owned.has(field) && !(field in shared))).toEqual([]);
});
// `self.given`, matched as `self . given` with any space around each dot,
// and `self.walks().mutated` as `self.walks.mutated`.
const statePattern = (state: string) =>
  new RegExp(`${state.split(".").join("\\s*(?:\\(\\s*\\))?\\s*\\.\\s*")}${state.endsWith(".") ? "" : "\\b"}`);
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

// MIR's lowering of a body keeps its state in `State`, mir.rs's own: a
// module of `mir/` that reads or changes a field of it is listed here, with
// the fields it may (the second review). One unlisted is one whose reader
// must hold all of mir.rs to know what it changes.
const mirState: Record<string, string[]> = {
  "src/lower/mir/drops.rs": ["body", "drops", "flags", "branches", "pending", "locals"],
  "src/lower/mir/writer.rs": ["body", "writer", "writes_captured", "builders", "built", "thrown_writes"],
  "src/lower/mir/hints.rs": ["locals", "captures"],
  "src/lower/mir/std_calls.rs": ["at", "body", "locals", "offsets"],
  "src/lower/mir/assign.rs": ["binding_var", "body", "boxes", "branches", "builders", "copy_backs", "cuts", "in_place", "offsets", "jsx_props", "let_spans", "locals", "maps", "named", "options", "parts", "payloads", "pending", "places", "pointed", "refs", "rests", "writer"],
  "src/lower/mir/closures.rs": ["at", "body", "locals", "offsets"],
  "src/lower/mir/dispatch.rs": ["body", "defaults", "flattened", "locals", "maps", "options"],
  "src/lower/mir/entries.rs": ["body", "captures", "locals", "thrown_writes"],
  "src/lower/mir/jsx_props.rs": ["body"],
  "src/lower/mir/locals.rs": ["body", "locals", "patterned"],
  "src/lower/mir/operands.rs": ["body", "locals", "pending", "promoted", "taken"],
  "src/lower/mir/patterns.rs": ["body", "graph", "held_arms", "in_place", "locals", "options"],
  "src/lower/mir/places.rs": ["binding_var", "body", "branches", "captures", "cuts", "defaults", "deferred", "in_place", "locals", "named", "pointed"],
  "src/lower/mir/queries.rs": ["body", "borrowed_names", "locals", "own_names"],
  "src/lower/mir/rvalues.rs": ["at", "body", "branches", "cuts", "locals", "offsets", "options", "pending", "pointed", "ranges", "statement", "writer"],
  "src/lower/mir/structure.rs": ["at", "awaits", "body", "copy_backs", "deferred", "drops", "graph", "held_arms", "labels", "jumps", "locals", "pending", "statement", "unwind", "writer", "writes_captured"],
  "src/lower/mir/temporaries.rs": ["body", "copy_backs", "in_place", "locals", "named", "own_names", "pending", "places", "pointed", "refs", "settled", "unwind"],
};
test("each module of mir/ reads only the fields of mir.rs's state it's listed for", () => {
  const mir = read("src/lower/mir.rs");
  const struct = mir.slice(mir.indexOf("struct State<"), mir.indexOf("\n}\n", mir.indexOf("struct State<")));
  const fields = [...struct.matchAll(/^    ([a-z_]+):/gm)].map((m) => m[1]);
  const found: Record<string, string[]> = {};
  for (const file of files("src/lower/mir")) {
    const used = fields.filter((field) => new RegExp(`\\bstate\\s*\\.\\s*${field}\\b`).test(read(file)));
    if (used.length > 0) found[file] = used;
  }
  const sorted = (map: Record<string, string[]>) => Object.fromEntries(Object.entries(map).map(([f, u]) => [f, [...u].sort()]).sort());
  expect(sorted(found)).toEqual(sorted(mirState));
});

// A JS form rust-js writes for one concept is written by the module that
// owns it, which the rest ask: a generic iterator as a JS iterator, a
// stepped one's `$iter`, and the box of a `Some` that looks like `None`.
// Each was written in two to five modules (the second architecture audit).
//
// Whether a value is a JS iterator or an array is iterators.rs's question,
// `is_js_iterator`, asked of the JS lowering from MIR makes (ADR 0364): it,
// `Iterator.from`, and a range's `$iterator`, written and taken apart, were
// in mir/iter.rs and tidy.rs too (the third architecture audit).
const idioms: [string, RegExp, string[]][] = [
  ["Iterator.from", /Expr::var\("Iterator"\),\s*"from"|==\s*"Iterator"/, ["src/lower/iterators.rs", "src/lower/iterators/eager.rs", "src/lower/iterators/in_place.rs", "src/lower/iterators/stepping.rs"]],
  ["$iterator", /"\$iterator"|Helper::Iterator\b/, ["src/lower/iterators.rs", "src/lower/iterators/in_place.rs"]],
  ["$iter", /"\$iter"|Helper::Iter\b/, ["src/lower/iterators.rs", "src/lower/iterators/in_place.rs"]],
  ["$lent", /"\$lent"|Helper::Lent\b/, ["src/lower/iterators/in_place.rs", "src/lower/iterators/stepping.rs"]],
  ["$someNone", /\$someNone/, ["src/lower/std_types/option.rs"]],
  // What a box, a cell or a handle holds, and a `dyn` pair's dictionary
  // (ADRs 0099, 0049): the JS tree's names of the fields, `js::HELD` and
  // `js::DICTIONARY`, wherever they're written or read.
  ["the held `value` field", /Expr::member\([^;]*?,\s*"value"\)|Field\("value"\.into\(\)|== "value"|HELD: &str = "value"/, ["src/js.rs"]],
  ["the dictionary's `impl` field", /"impl"/, ["src/js.rs"]],
  // A std type's helpers, read and written where the type is lowered: a
  // pass of tidy's over them is the type's module's (the second review).
  ["a map's entry", /"\$orInsert"|"\$mutGet"/, ["src/lower/std_types/map.rs", "src/lower/mir/entries.rs", "src/lower/mut_refs.rs"]],
  ["a range made as stepped", /"\$lazyRange"/, ["src/lower/std_types/range.rs"]],
  ["a cell's borrow", /"\$borrow(Mut)?"|"\$unborrow"/, ["src/lower/std_types/cell.rs", "src/lower/std_impls.rs", "src/lower/drops/parts.rs"]],
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

// An enum's JS, `{ TAG: "Circle", _0: radius }`, is spelled by `js.rs`
// alone, `js::TAG` and `js::positional(i)`, and a `Result`'s, `{ TAG:
// "Ok", _0: value }`, is made, tested and read by `js::result` alone:
// lowering, which makes and reads them, and preparation, which reads a
// `$try`'s `match` back into a `try`, both ask (the fourth review:
// preparation recognized a `Result`'s tag and payload on its own, and
// fifteen of lowering's reads spelled them).
test("an enum's JS is `js`'s, and a `Result`'s `js::result`'s", () => {
  // Its code, not the comments that show the JS it makes.
  const code = (file: string) => read(file).split("\n").filter(line => !/^\s*\/\//.test(line)).join("\n");
  const askers = ["src/lower.rs", ...files("src/lower"), "src/prepare.rs", ...files("src/prepare")];
  for (const [owner, spelled] of [
    ["src/js.rs", /"TAG"|format!\("_\{i\}"\)/],
    ["src/js/result.rs", /"(?:Ok|Err)"/],
  ] as const) {
    expect(code(owner), owner).toMatch(spelled);
    expect(askers.filter(file => spelled.test(code(file))), owner).toEqual([]);
  }
  // A variant's field read or written by its own spelling, `x._0`.
  const positional = /Expr::member\([^;]*?,\s*"_\d+"\)|Field\("_\d+"|field == "_\d+"/;
  expect(askers.filter(file => positional.test(code(file)))).toEqual([]);
});

// A rule of an owner is of what its owner has: one of state that's gone,
// or spelled so it's never found, holds whatever the others do. Seven of
// the twelve rules of state matched nothing, and passed (the third
// architecture audit).
test("each owner's rule finds what its owner has", () => {
  const unfound = [
    ...Object.entries(owners).map(([state, allowed]) => [state, statePattern(state), allowed] as const),
    ...idioms,
  ].filter(([, pattern, allowed]) => !allowed.some(file => pattern.test(read(file))));
  expect(unfound.map(([what]) => what)).toEqual([]);
});

// The architecture's table of owners names, for each concept, the module
// that owns it and what the others ask it: each is there. Seventeen of the
// functions it named had gone when lowering from MIR came (the third
// architecture audit).
test("each owner and question the architecture names is there", () => {
  const doc = read("docs/architecture.md");
  const table = doc.slice(doc.indexOf("| Concept | Owner | What the others ask |"), doc.indexOf("**Calls.**"));
  const rows = [...table.matchAll(/^\|[^|]*\|([^|]*)\|([^|]*)\|$/gm)].slice(2);
  const names = (cell: string) => [...cell.matchAll(/`([^`]+)`/g)].map(m => m[1]);
  // A module, `shortcuts.rs`, is a file; anything else, a function or a type.
  const isModule = (name: string) => /\.rs$|\/$/.test(name);
  const found = (module: string) => sources.some(file => file.startsWith(`src/lower/${module}`) || file === `src/${module}`);
  const asked = rows.flatMap(([, , ask]) => names(ask));
  const code = sources.map(read).join("\n");
  expect(asked.length).toBeGreaterThan(20);
  expect(asked.filter(name => isModule(name) ? !found(name) : !new RegExp(`\\b(?:fn|struct|enum|trait)\\s+${name}\\b`).test(code))).toEqual([]);
  expect(rows.flatMap(([, owner]) => names(owner)).filter(owner => !found(owner))).toEqual([]);
});

// Each decision is in the index, and one superseded or replaced, wholly or
// in part, says so there: 64 weren't in it, and five didn't say so (the
// third architecture audit).
test("each decision is in the index, a superseded one marked", () => {
  const index = read("docs/README.md");
  const decisions = readdirSync(join(root, "docs/decisions")).filter(name => /^\d{4}-.*\.md$/.test(name));
  expect(decisions.length).toBeGreaterThan(300);
  expect(decisions.filter(name => !index.includes(`(decisions/${name})`))).toEqual([]);
  const superseded = decisions.filter(name => /superseded|replaced by/i.test(read(`docs/decisions/${name}`).match(/^Status:.*$/m)?.[0] ?? ""));
  const line = (name: string) => index.split("\n").find(l => l.includes(`(decisions/${name})`)) ?? "";
  expect(superseded.length).toBeGreaterThan(4);
  expect(superseded.filter(name => !/\*\([^)]*(?:superseded|replaced)[^)]*\)\*/.test(line(name)))).toEqual([]);
});

// Each module of the front end is in the architecture, by its name or its
// folder's, so what it's for is written where a reader looks first:
// `calls.rs` was in no table (the second review).
test("each module of the front end is in the architecture", () => {
  const doc = read("docs/architecture.md");
  const named = (name: string) => doc.includes("`" + name + "`");
  const missing = files("src/lower").filter(file => {
    const rel = file.slice("src/lower/".length);
    const dir = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/") + 1) : null;
    return !(named(rel) || named(file) || (dir !== null && (named(dir) || named(`src/lower/${dir}`))));
  });
  expect(missing).toEqual([]);
});

// Each decision says its status, and one an Amends or a Supersedes names
// says what amended it: seventeen didn't, and 0075 had no status (the
// second review), so reading one alone missed what changed it.
test("each decision has a status, and one amended names what amended it", () => {
  const decisions = readdirSync(join(root, "docs/decisions")).filter(name => /^\d{4}-.*\.md$/.test(name));
  const text = (name: string) => read(`docs/decisions/${name}`);
  expect(decisions.filter(name => !/^Status:/m.test(text(name)))).toEqual([]);
  const byNumber = new Map(decisions.map(name => [name.slice(0, 4), name]));
  const unlinked: string[] = [];
  for (const name of decisions) {
    const status = text(name).match(/^Status:[\s\S]*?(?:\n\n|$)/m)?.[0] ?? "";
    for (const [, target] of status.matchAll(/(?:Amends|Supersedes|amends|supersedes)\s+\[?(\d{4})/g)) {
      const amended = byNumber.get(target);
      if (amended && !text(amended).includes(name.slice(0, 4))) unlinked.push(`${target} by ${name.slice(0, 4)}`);
    }
  }
  expect(unlinked).toEqual([]);
});

// A question recognition answers is asked of it, or through shortcuts.rs,
// where each shortcut is one line, so where it's answered is plain. They
// were in ten modules once, each looking like that module's own.
test("shortcuts to recognition are in shortcuts.rs, and only shorten", () => {
  const forward = /^ {4}(?:pub\([^)]*\) )?fn (\w+)(?:<[^>]*>)?\([^{]*\{\n\s+self\.recognition\(\)\.\w+\([^;{}]*\)\n {4}\}\n/gm;
  for (const file of ["src/lower.rs", ...files("src/lower")].filter(f => f !== "src/lower/shortcuts.rs" && !f.includes("/recognition"))) {
    const found = [...read(file).matchAll(forward)].map(m => m[1]);
    expect([file, found]).toEqual([file, []]);
  }
  const shortcuts = read("src/lower/shortcuts.rs");
  const functions = [...shortcuts.matchAll(/^ {4}(?:pub\([^)]*\) )?fn /gm)].length;
  expect([...shortcuts.matchAll(forward)].length).toBe(functions);
});

// What's called is told by a dispatcher that hands it to what knows it, a
// std function to its domain's (std_types/, iterators.rs, ..). It was a
// function of a thousand lines once: what's new goes where its kind is,
// not in the dispatcher. Lowering from MIR (ADR 0364) grew it back, its std
// calls' `mir_std_call` to 942 lines, which the rule, of `classify` alone,
// didn't see (the third architecture audit): each is held to the length it
// has, and its cap lowered as it's split, never raised. So is each function
// too long to read at once (the second review): its cap is its length now,
// which the test holds exactly, so a split says so here.
const dispatchers: [string, string, number][] = [
  ["src/lower/recognition/calls.rs", "    pub(in crate::lower) fn classify(", 8],
  ["src/lower/mir/dispatch.rs", "    pub(super) fn mir_call(", 100], // What reads its argument through, `flatten()` and `unwrap()`, in `reads_through` (ADR 0747).
  ["src/lower/mir/dispatch.rs", "    fn mir_call_values_of(", 247],
  ["src/lower/mir/dispatch.rs", "    fn mir_std_call(", 140],
  ["src/lower/mir.rs", "    fn mir_function(", 364], // The parameters handed to tidy's `scopes` stage in a cell, and whether it asks its stages again (ADR 0659, amended); the jumps the program wrote in one field (ADR 0742); the guards that hold (ADR 0790).
  ["src/lower/recognition/inherent.rs", "    pub(super) fn classify_inherent(", 754],
  ["src/lower/recognition/calls.rs", "    fn classify_trait_method(", 245],
  ["src/lower/combinators.rs", "    pub(in crate::lower) fn comb_values(", 602],
  ["src/lower/mir/iter.rs", "    pub(in crate::lower) fn mir_iter_call(", 28],
  ["src/lower/mir/iter.rs", "    fn mir_iter_method(", 282],
];
test("the dispatchers of what's called don't grow", () => {
  const length = (file: string, signature: string) => {
    const lines = read(file).split("\n");
    const start = lines.findIndex(line => line.startsWith(signature));
    expect(start, signature).toBeGreaterThanOrEqual(0);
    return lines.findIndex((line, k) => k > start && line === "    }") - start;
  };
  for (const [file, signature, cap] of dispatchers) {
    expect(length(file, signature), signature).toBe(cap);
  }
});

// Which std collections a JS array is, `VEC_LIKE`, is one list's: the
// analysis of what a change makes apart kept one of its own, without
// `LinkedList`, so its clone was the same array.
test("the std collections a JS array is are one list's", () => {
  const listed = /StdItem::VecDeque,\s*StdItem::BinaryHeap/;
  const writers = ["src/lower.rs", ...files("src/lower")].filter(file => listed.test(read(file)));
  expect(writers).toEqual(["src/lower/recognition/types.rs"]);
});

// Which operator a std trait is, `Add`'s `+`, is one list's, `value_operator`'s
// and `assign_operator`'s, which the rest ask: recognition wrote both
// lists again, the second answer to the same question.
test("an operator trait's operator is one list's", () => {
  const listed = /\(LangItem::\w+, (?:Ok\()?BinOp::\w+/;
  const writers = ["src/lower.rs", ...files("src/lower")].filter(file => listed.test(read(file)));
  expect(writers).toEqual(["src/lower/recognition/traits.rs"]);
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
  for (const file of ["src/lower/calls.rs", "src/lower/ordering.rs", "src/lower/traits.rs", ...files("src/lower/traits")]) {
    expect(read(file), file).not.toMatch(/match\s+(?:self\.tcx\.item_name\(\w+\)|name)\.as_str\(\)/);
  }
});

// Only recognition spells std's names: the rest ask it, `is_std_type(ty,
// StdItem::Result)`, `std_item(tcx, StdItem::Ord)`. Fourteen modules had 58
// checks of their own, by diagnostic item, path or method name (the second
// architecture audit).
const identity: [string, RegExp][] = [
  ["a diagnostic item", /\b(?:is|get)_diagnostic_item\s*\(/],
  ["a std type's name", /Symbol::intern\("[A-Z][A-Za-z]+"\)/],
  ["a path compared", /def_path_str\([^)]*\)(?:\.as_str\(\))?\s*(?:==|\.starts_with|\.contains)/],
  ["a method's name compared", /item_name\([^)]*\)(?:\.as_str\(\))?\s*(?:==|\.starts_with|\.ends_with|\.contains)|match\s+(?:self\.)?tcx\.item_name\([^)]*\)\.as_str\(\)/],
  ["a std type by its name", /\bis_std_adt\s*\(/],
  // A std item's own name, `sym::Target` of `Deref`'s, or std's layout,
  // `sym::value` of a `Cell`'s field (the third architecture audit); a
  // binding's attribute's, `link_name`, is the crate's.
  ["a std name by rustc's symbol", /\bsym::(?!name\b|link_name\b)\w+/],
];
// A std type is known one way, by its `StdItem`, `is_std_type(ty,
// StdItem::Mutex)`, not also by a diagnostic item's name, `Symbol::intern(
// "Mutex")`, which no list checks (the second architecture review: both
// were, five lists of cells among them).
test("a std type is known by its StdItem", () => {
  for (const file of ["src/lower.rs", ...files("src/lower")]) {
    expect(read(file), file).not.toMatch(/fn is_std_adt|is_diagnostic_item\(Symbol::intern\("[A-Z]\w*"\), adt/);
  }
});

// A JS name that's a Rust keyword is its raw identifier in every binding,
// generated or written by hand, `r#type`, which rust-js gives JS as `type`:
// not `type_` and a `rust_js::name` beside it (ADR 0376). Only the four
// keywords Rust has no raw identifier of take a `_`, `self_`. And the
// generators name a JS name one way, scripts/rust-names.ts's.
test("a binding's keyword name is its raw identifier", () => {
  const { KEYWORDS } = require("../scripts/rust-names.ts") as { KEYWORDS: Set<string> };
  const escaped = [...KEYWORDS].filter((k) => !["self", "Self", "super", "crate"].includes(k));
  const pattern = new RegExp(`\\b(?:${escaped.join("|")})_\\b`);
  const crates = ["builtins/src", "webapi/src", "react/src", "node/src", "next/src"].flatMap(files).filter((f) => f.endsWith(".rs"));
  expect(crates.length).toBeGreaterThan(10);
  expect(crates.filter((file) => pattern.test(read(file)))).toEqual([]);
  const generators = ["react/generate.ts", "react/attributes.ts", "react/literals.ts", "react/names.ts", "webapi/generate.ts", "builtins/intl.ts", "builtins/typed_arrays.ts"];
  expect(generators.filter((file) => /KEYWORDS|`r#\$\{|replace\(\/\(\[a-z0-9\]\)\(\[A-Z\]\)/.test(read(file)))).toEqual([]);
});

// A binding's optional argument is an `Option` of the one function that
// binds its JS name (ADR 0443): no second binding of that name, in one impl
// or module, whose parameters begin with another's and go on, as
// `push_state` and `push_state_with_url` did, `slice` and `slice_to_end`.
// What TypeScript declares apart, `drawImage(image, dx, dy)` and its
// nine-argument signature, differs in a parameter, not only in more.
type Binding = { file: string; line: number; name: string; link: string; scope: string; params: string[]; result: string };
const parameters = (list: string) => {
  const out: string[] = [];
  let depth = 0, current = "", previous = "";
  for (const c of list) {
    if ("(<[{".includes(c)) depth++;
    if (")]}".includes(c) || (c === ">" && previous !== "-")) depth--;
    if (c === "," && depth === 0) {
      out.push(current.trim());
      current = "";
    } else current += c;
    previous = c;
  }
  if (current.trim()) out.push(current.trim());
  return out;
};
const bindingsOf = (file: string, text: string): Binding[] => {
  const lines = text.split("\n");
  const found: Binding[] = [];
  let scope = "";
  for (let i = 0; i < lines.length; i++) {
    const opened = lines[i].match(/^(\s*)(?:pub )?(?:mod|impl(?:<[^>]*>)?|trait) ([^{]*)\{/);
    if (opened && !lines[i].includes("fn ")) scope = `${opened[1].length}:${opened[2].trim()}`;
    const fn = lines[i].match(/^\s*(?:pub )?fn (\w+)/);
    if (!fn) continue;
    let link = "", cfg = "";
    for (let j = i - 1; j >= 0 && /^\s*(\/\/|#\[)/.test(lines[j]); j--) {
      link ||= lines[j].match(/link_name = "([^"]*)"/)?.[1] ?? "";
      if (lines[j].includes("#[cfg(")) cfg += lines[j].trim();
    }
    if (!link) continue;
    let signature = "";
    for (let k = i; !signature.trimEnd().endsWith("{"); k++) signature += `${lines[k]}\n`;
    const open = signature.indexOf("(", signature.indexOf("fn "));
    let close = open;
    for (let depth = 0; close < signature.length; close++) {
      if (signature[close] === "(") depth++;
      if (signature[close] === ")" && --depth === 0) break;
    }
    const result = signature.slice(close + 1).replace(/\{\s*$/, "").trim();
    found.push({ file, line: i + 1, name: fn[1], link, scope: scope + cfg, params: parameters(signature.slice(open + 1, close)), result });
  }
  return found;
};
const secondForms = (bindings: Binding[]) =>
  bindings.flatMap((longer) => bindings
    .filter((shorter) => shorter !== longer && shorter.file === longer.file && shorter.link === longer.link && shorter.scope === longer.scope
      && shorter.result === longer.result && shorter.params.length < longer.params.length
      && shorter.params.every((param, i) => param === longer.params[i]))
    .map((shorter) => `${longer.file}:${longer.line} \`${longer.name}\` is \`${shorter.name}\` with more: give \`${shorter.name}\` each as an \`Option\``));

test("a binding's optional argument is an Option of its one function", () => {
  const crates = ["builtins/src", "webapi/src", "react/src", "node/src", "next/src"].flatMap(files).filter((f) => f.endsWith(".rs"));
  const bindings = crates.flatMap((file) => bindingsOf(file, read(file)));
  expect(bindings.length).toBeGreaterThan(5000);
  expect(secondForms(bindings)).toEqual([]);
  // The rule finds one, and leaves a signature TypeScript declares apart.
  const planted = `impl History {
    #[cfg_attr(rust_js, rust_js::link_name = "pushState")]
    pub fn push_state(&self, data: D, unused: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "pushState")]
    pub fn push_state_with_url(&self, data: D, unused: &str, url: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "drawImage")]
    pub fn draw_image(&self, image: &Image, dx: f64, dy: f64) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "drawImage")]
    pub fn draw_image_with_sw(&self, image: &Image, sx: f64, sy: f64, sw: f64) {
        unreachable!()
    }
}
`;
  expect(secondForms(bindingsOf("planted.rs", planted))).toEqual([
    "planted.rs:8 `push_state_with_url` is `push_state` with more: give `push_state` each as an `Option`",
  ]);
});

test("only recognition spells std's names", () => {
  const strays: string[] = [];
  for (const file of ["src/lower.rs", ...files("src/lower")].filter(f => !f.includes("/recognition") && !f.endsWith("/shortcuts.rs"))) {
    const text = read(file);
    for (const [what, pattern] of identity) if (pattern.test(text)) strays.push(`${what} in ${file}`);
  }
  expect(strays).toEqual([]);
});

// An app's Rust is compiled one way, `rust-js build`, its own process
// beside the app's own dev server or build (ADR 0412): no host's plugin,
// no config that compiles, no second command, and the compiler is what
// `rust-js build` runs, not the app's command.
test("an app's Rust is compiled one way, rust-js build", () => {
  const workspaces: string[] = JSON.parse(read("package.json")).workspaces;
  expect(workspaces.filter((dir) => /plugin/.test(dir))).toEqual([]);
  const build = JSON.parse(read("tooling/package.json"));
  expect(Object.keys(build.bin).sort()).toEqual(["rust-js", "rust-js-check"]);
  // No API to build with but the command: nothing exported, nothing imported.
  expect(build.exports).toBeUndefined();
  const tracked = Bun.spawnSync(["git", "ls-files"], { cwd: root }).stdout.toString().trim().split("\n");
  const importers = tracked.filter((file) => /\.(ts|js|mjs|tsx|jsx)$/.test(file) && file !== "test/architecture.test.ts")
    .filter((file) => read(file).includes("@rust-js/build/"));
  expect(importers).toEqual([]);
  expect(read("scripts/package-compiler.ts")).not.toContain("bin:");
  const cli = read("tooling/cli.js");
  expect(cli).toContain('if (command !== "build")');
  expect(cli).not.toContain("spawn");
  // The build's one entry, which the CLI is a parse of.
  expect(cli).toContain("buildApp(");
  expect(read("tooling/cargo.js")).toContain("parseManifest(readFileSync(manifest");
});

// Which crates rust-js compiles has one answer, the tooling's, from Cargo's
// metadata (ADR 0414): each package of a path, but a binding crate, which
// says so in its own `Cargo.toml`, as host-boundaries.test.ts holds of
// rust-js's. No list of names, and no wrapper of the workspace's members
// alone, which a crate from npm isn't one of.
test("the crates rust-js compiles are the ones the tooling names, a binding crate saying it's one", () => {
  expect(read("src/cargo.rs")).not.toContain("rust-js-react");
  // Nor a planner of Cargo's graph of its own, ADR 0085's, beside Cargo's.
  expect(read("tooling/cargo.js")).not.toContain("planCargoLibraries");
  expect(read("src/cargo.rs")).toContain("RUST_JS_CRATES");
  const tracked = Bun.spawnSync(["git", "ls-files"], { cwd: root }).stdout.toString().trim().split("\n");
  const wrappers = tracked.filter((file) => /\.(ts|js|rs)$/.test(file) && file !== "test/architecture.test.ts")
    .filter((file) => read(file).includes("RUSTC_WORKSPACE_WRAPPER"));
  expect(wrappers).toEqual([]);
});

// Whether a type is a raw pointer, a `NonNull`, which is its pointer, or a
// pattern type of one, has one answer, `pointers::raw_pointee` (ADR 0407):
// no other module matches rustc's raw pointer itself.
test("a raw pointer is one by raw_pointee alone", () => {
  const matching = files("src/lower").filter((file) => file.endsWith(".rs") && file !== "src/lower/pointers.rs")
    .filter((file) => /ty::RawPtr\b/.test(read(file)));
  expect(matching).toEqual([]);
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

// A refusal is said one way, by `refusal.rs` (ADR 0390): eleven places
// wrote "rust-js does not support .. yet" themselves, so a refusal of
// addressable memory couldn't say so wherever it was made.
// Whether a type is known only by its bounds, a type parameter or an
// associated type of one, is `recognition::known_by_bounds`'s question:
// written in place at nine sites, one missed associated types (ADR 0533).
test("a type known by its bounds is one by known_by_bounds alone", () => {
  const asking = files("src/lower").filter((file) => file.endsWith(".rs"))
    .flatMap((file) => read(file).split("\n").filter((line) => /matches!\(.*ty::Param\(_\) \| ty::Alias\(\.\.\)/.test(line)).map((line) => `${file}: ${line.trim()}`));
  expect(asking).toEqual(["src/lower/recognition/traits.rs: matches!(ty.kind(), ty::Param(_) | ty::Alias(..))"]);
});

// A whole body's facts, each a reading of its THIR, are its `BodyFacts`,
// collected once before it's lowered: a call lowering one looks its
// answer up, and never reads the body again for it, as four did, each
// call building a set of the whole body (#191, ADR 0791). So
// `body_queries` gives no whole body's set or map but through `BodyFacts`.
test("a whole body's facts are its BodyFacts, read once", () => {
  const source = read("src/lower/body_queries.rs");
  const signature = /^(pub(?:\([^)]*\))? )?fn (\w+)<'tcx>\([^)]*thir: &Thir<'tcx>[^)]*\)\s*->\s*\(?Hash(?:Set|Map)/gm;
  const queries = [...source.matchAll(signature)];
  expect(queries.length).toBeGreaterThan(5);
  expect(queries.filter((query) => query[1]).map((query) => query[2])).toEqual([]);
});

// A std call's kind is chosen of what it reads alone: each kind's handler
// takes `&self` and `&State` and gives the lowering of a call it takes,
// so none can change what's lowered before the one that takes it is
// chosen, and it's the one (#191, ADR 0791). A speculative handler given
// `&mut` access, which a decline had to be checked against, can't return.
test("a std call's kind is chosen of what it reads alone", () => {
  const source = read("src/lower/mir/std_calls.rs");
  const selecting = [...source.matchAll(/fn (mir_\w+_call)<'c>\(\s*&self,\s*_?state: &State<'_, 'tcx>,\s*call: StdCall<'c, 'tcx>,?\s*\) -> Option<StdLowering<'c, 'a, 'tcx>>/g)];
  expect(selecting.map((m) => m[1]).sort()).toEqual(["mir_fmt_call", "mir_iter_std_call", "mir_map_call", "mir_place_call", "mir_text_call", "mir_value_call", "mir_vec_call"]);
  const mutable = [...source.matchAll(/fn (\w+)(?:<[^>]*>)?\(\s*&mut self,[^)]*\bStdCall\b/g)].map((m) => m[1]);
  expect(mutable).toEqual([]);
});

// Where a copied default's types are known, the impl's environment with the
// default's own bounds, is `impl_typing_env`'s question: resolving a call
// read the impl's alone, and missed a bound of the default's own type
// parameter (ADR 0560).
test("a copied default's typing environment is impl_typing_env's alone", () => {
  const reading = files("src/lower").filter((file) => file.endsWith(".rs"))
    .flatMap((file) => read(file).split("\n").filter((line) => /self\.given\.self_env\b(?!\s*=)/.test(line) && !/self_env:/.test(line)).map((line) => `${file}: ${line.trim()}`));
  expect(reading).toEqual(["src/lower/traits/given.rs: let impl_env = self.given.self_env?;"]);
});

// Whether a local is written again after its first value is
// `written_again`'s, which counts a parameter's argument as its first: a
// variable read as never changing (`alike`) and a `move` closure's capture
// each counted MIR's writes alone, so a `mut` parameter assigned once read
// as never changing, and what was moved out of it read its new value (#99).
test("a local written again is written_again's alone", () => {
  const counting = files("src/lower").filter((file) => file.endsWith(".rs"))
    .flatMap((file) => read(file).split("\n").filter((line) => /writes\[[^\]]+\]\s*(>|>=)\s*[12]\b/.test(line)).map((line) => `${file}: ${line.trim()}`));
  expect(counting).toEqual([]);
});

// Whether a `&mut` of an item std gives a closure, or gives back, is a
// handle is `handles_mut`'s: a value JS can't change in place, or a type
// parameter's, which generic code boxes (ADR 0454). `retain_mut` in generic
// code asked `is_boxable` alone, gave a closure that writes `x.value` the
// item itself, and threw (#131).
test("a std call's `&mut` handle is handles_mut's alone", () => {
  const asking = ["src/lower/std_types/slice.rs", "src/lower/std_types/map.rs"].filter((file) => /\bis_boxable\(/.test(read(file)));
  expect(asking).toEqual([]);
});

// Whether an arrow is only the function it passes its arguments on to,
// `(x) => f(x)`, `() => { f(); }`, is `iterators::passed_on`'s, which an
// array method's callback (ADR 0279) and an event's handler (ADR 0198) both
// ask: each had its own (ADR 0605).
test("an arrow that passes on what it's given is passed_on's alone", () => {
  const asking = files("src/lower").filter((file) => file.endsWith(".rs"))
    .flatMap((file) => read(file).split("\n").filter((line) => /\bpassed_on\(/.test(line) && !/fn passed_on\(/.test(line)).map(() => file));
  expect([...new Set(asking)].sort()).toEqual(["src/lower/jsx.rs", "src/lower/mir/tidy/stages.rs", "src/lower/mir/tidy/values.rs"]);
  const own = files("src/lower").filter((file) => file.endsWith(".rs") && /fn passed_on\(f: &Expr/.test(read(file)));
  expect(own).toEqual(["src/lower/iterators/eager.rs"]);
});

// A `snake_case` name is made camelCase in one place, `camel_case`, and
// a word joined into one by `upper_first` (ADR 0573): a module's import,
// an impl's dictionary's clash, a default import's class each joined its
// words their own way. serde's `rename_all` is serde's rule, which a
// program sees in its JSON, and is its own.
test("a snake_case name is made camelCase by camel_case alone", () => {
  const splits = /\.split\('_'\)|\bch == '_'/;
  const others = files("src").filter((file) => file.endsWith(".rs") && file !== "src/lower.rs" && file !== "src/lower/serde.rs");
  expect(others.filter((file) => splits.test(read(file)))).toEqual([]);
  expect(read("src/lower.rs")).toMatch(/fn camel_case\(name: &str\) -> String \{[^]*?\.split\('_'\)/);
});

// A JS key the compiler or the runtime writes is camelCase, as every Rust
// name is in JS (ADR 0573): a std trait's method in a dictionary is its
// `bindings::trait_method_name`, never spelled as Rust spells it. Std's
// dictionaries were keyed `as_ref` and `into_iter` while the crate's said
// `asRef`, so a call through a dictionary found none.
test("a JS key the compiler or the runtime writes is camelCase", () => {
  const snake = '"[a-z][a-z0-9]*_[a-z0-9_]*"';
  const keyed = new RegExp(`(?:Prop::Field\\(|Expr::member\\([^;]*?,\\s*)${snake}`);
  const code = (text: string) => text.split("\n").filter((line) => !/^\s*\/\//.test(line));
  const lowering = ["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs"));
  expect(lowering.flatMap((file) => code(read(file)).filter((line) => keyed.test(line)).map((line) => `${file}: ${line.trim()}`))).toEqual([]);
  // A helper's object key or method, `writeU32(n)`, `{ writeStr: .. }`.
  const runtime = files("src/runtime").filter((file) => file.endsWith(".js"));
  expect(runtime.length).toBeGreaterThan(100);
  const defined = /^\s+[a-z][a-z0-9]*_[a-z0-9_]*\s*(?:\(|:)/;
  expect(runtime.flatMap((file) => code(read(file)).filter((line) => defined.test(line)).map((line) => `${file}: ${line.trim()}`))).toEqual([]);
});

test("only refusal.rs words a refusal", () => {
  const wording = sources.filter(file => file !== "src/lower/refusal.rs" && read(file).includes("rust-js does not support"));
  expect(wording).toEqual([]);
});

// The readability baseline counts the snapshots' JS, so its bless runs after
// theirs are written: `bun test` runs its files in name order, not the
// order given, so the one way is a second `bun test` (ADR 0565).
test("the readability baseline is blessed after the snapshots", () => {
  const bless: string = JSON.parse(read("package.json")).scripts.bless;
  const [snapshots, readability] = bless.split(" && ");
  expect(snapshots).toMatch(/^BLESS=1 bun test (test\/\w+\.test\.ts ?)+$/);
  expect(snapshots).not.toContain("readability");
  expect(readability).toBe("BLESS=1 bun test test/readability.test.ts");
});

// Bun's version is written once, `package.json`'s `packageManager`: each
// workflow's setup reads it there, `@types/bun` is the same, and the suite
// runs on it. It was written in eleven places, ten workflow steps and the
// types.
test("Bun's version is package.json's packageManager alone", () => {
  const pkg = JSON.parse(read("package.json"));
  const version = /^bun@(\d+\.\d+\.\d+)$/.exec(pkg.packageManager)?.[1];
  expect(version).toBeDefined();
  expect(pkg.devDependencies["@types/bun"]).toBe(version);
  expect(Bun.version).toBe(version!);
  const workflows = [...files(".github/workflows"), ...files(".github/actions")].filter(file => /\.ya?ml$/.test(file));
  const steps = workflows.flatMap(file => [...read(file).matchAll(/uses: oven-sh\/setup-bun@\S+\n(\s+)with:\n((?:\1  .*\n)*)/g)].map(m => ({ file, with: m[2] })));
  expect(steps.length).toBeGreaterThan(9);
  expect(steps.filter(step => !step.with.includes("bun-version-file: package.json") || step.with.includes("bun-version:"))).toEqual([]);
  expect(workflows.filter(file => read(file).includes("oven-sh/setup-bun")).length).toBe(new Set(steps.map(step => step.file)).size);
});

// Where a test's build writes is `target`, support.ts's, which a mutation's
// run gives one of its own: a path spelled under `target/` reads a copy
// another run left, and failed every mutation of the tests that did.
test("tests find what they build through target", () => {
  const spelled = readdirSync(join(root, "test"))
    .filter((file) => file.endsWith(".test.ts") && file !== "architecture.test.ts")
    // A string of a path, not a comment's word for the folder.
    .filter((file) => read(`test/${file}`).split("\n").some((line) => !/^\s*\/\//.test(line) && /["'`]target\//.test(line)));
  expect(spelled).toEqual([]);
});

// A compiler file is at most scripts/file-sizes.ts's LARGE lines, so each
// can be read and changed on its own (ADR 0572). Thirteen were over it when
// the cap came, each split by concept since; a file that would go over is
// split the same way, by what it holds, never to meet the number.
test("a compiler file is at most the cap", () => {
  const { COMPILER, LARGE, fileSizes } = require("../scripts/file-sizes.ts") as {
    COMPILER: string[];
    LARGE: number;
    fileSizes: (paths: string[]) => [string, number][];
  };
  const sizes = fileSizes(COMPILER);
  expect(sizes.length).toBeGreaterThan(400);
  expect(sizes.filter(([, lines]) => lines > LARGE)).toEqual([]);
});

// A test file is at most the cap too, so a test is found, read and moved
// as one of a concept's (ADR 0572, amended). Three were over it when the
// cap came, each split by concept since; one that would go over is split
// the same way, by what its tests hold.
test("a test file is at most the cap", () => {
  const { TESTS, LARGE, fileSizes } = require("../scripts/file-sizes.ts") as {
    TESTS: string[];
    LARGE: number;
    fileSizes: (paths: string[]) => [string, number][];
  };
  const sizes = fileSizes(TESTS);
  expect(sizes.length).toBeGreaterThan(100);
  expect(sizes.filter(([, lines]) => lines > LARGE)).toEqual([]);
});
