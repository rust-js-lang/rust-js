// The rules of the architecture about who owns each part of a function's
// state as it's lowered, `FnCx`'s groups and MIR's `State`, split from
// architecture.test.ts by concept at its cap (a test file is at most 1500
// lines).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't; none of an
// install's or a build's (#1174).
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .filter(entry => !(entry.isDirectory() && (entry.name === "node_modules" || entry.name === "target")))
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

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
    "src/lower/drops/glue.rs",
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
  "self.recursive": ["src/lower/recursive_fns.rs"],
  "self.given": ["src/lower/traits/given.rs", "src/lower/traits/calls.rs", "src/lower/traits/impls.rs", "src/lower/traits/routes.rs"],
  "self.walks.representable": ["src/lower/support.rs"],
  "self.walks.assumed": ["src/lower/support.rs"],
  "self.walks.mutated": ["src/lower/copies.rs"],
  "self.walks.clones": ["src/lower/std_impls/clones.rs"],
  "self.walks.clone_assumed": ["src/lower/std_impls/clones.rs"],
  "self.codecs": ["src/lower/serde.rs"],
  // A function's scope struct's methods, functions declared in it (ADR
  // 1043): their names, which calls of them call, and while one is
  // lowered, the variable it closes over, its `self`'s.
  "self.scope_functions": ["src/lower/mir/calls.rs", "src/lower/mir/scopes.rs"],
  "self.scope_self": ["src/lower/mir/locals.rs", "src/lower/mir/scopes.rs"],
  // And the lowering of each its function's lowering declares, which a
  // call of one calls and its body takes its type parameters as (#500).
  "self.scope_copies": ["src/lower/mir/calls.rs", "src/lower/mir/scopes.rs", "src/lower/representation/options.rs"],
  // And each scope's value's variable, whose fields its function's body,
  // once lowered, makes its variables (ADR 1043, amended).
  "self.scope_values": ["src/lower/mir.rs", "src/lower/mir/scopes.rs"],
  // While a `match`'s tests of its arms are made, which it refuses itself
  // once it knows its arms differ, where a class is told by `instanceof`
  // (#555): set by its switch, read where a test is.
  "self.telling_arms": ["src/lower/mir/structure.rs", "src/lower/untagged/classes.rs"],
  // While a call is lowered, the local it makes its value into, its maker's
  // MIR identity, which what's stepped from its back asks (#814).
  "self.made_into": ["src/lower/iterators/back_facts.rs"],
  "self.made_from": ["src/lower/iterators/back_facts.rs"],
  "self.asking_made": ["src/lower/iterators/back_facts.rs", "src/lower/mir/iter/backs.rs"],
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
  copy: "which copy of the item it lowers, one its callers need besides its own JS (ADR 0051), set entering it",
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

// A rule of an owner is of what its owner has: one of state that's gone, or
// spelled so it's never found, holds whatever the others do. Seven of the
// twelve rules of state matched nothing, and passed (the third audit).
test("each owner's rule of a function's state finds what its owner has", () => {
  const unfound = Object.entries(owners).filter(([state, allowed]) => !allowed.some(file => statePattern(state).test(read(file))));
  expect(unfound.map(([what]) => what)).toEqual([]);
});

// MIR's lowering of a body keeps its state in `State`, mir.rs's own: a
// module of `mir/` that reads or changes a field of it is listed here, with
// the fields it may (the second review). One unlisted is one whose reader
// must hold all of mir.rs to know what it changes. Not its `facts`, what it
// knows of the body before it writes any of it, nor its `naming`, what
// naming its locals settles, `local_facts.rs`'s, whose fields are private
// there: any module reads them, none changes them (the review's P3).
const mirState: Record<string, string[]> = {
  "src/lower/mir/bounds.rs": ["at", "body"],
  "src/lower/mir/assign.rs": ["binding_var", "body", "branches", "builders", "commented", "cuts", "in_place", "iter_locals", "locals", "named", "parts", "pending", "places", "refs", "writer"],
  "src/lower/mir/assign/boxed_args.rs": ["copy_backs"],
  "src/lower/mir/calls.rs": ["at", "body", "flattened"],
  "src/lower/mir/closures.rs": ["at", "body", "in_place", "locals"],
  "src/lower/mir/dispatch.rs": ["at", "body", "defaults", "flattened", "iter_locals", "locals"],
  "src/lower/mir/drops.rs": ["body", "branches", "locals", "pending"],
  "src/lower/mir/entries.rs": ["body", "locals", "thrown_writes"],
  "src/lower/mir/hints.rs": ["locals"],
  "src/lower/mir/dispatch/known_calls.rs": ["at", "body"],
  "src/lower/mir/hints/clones.rs": ["locals"],
  "src/lower/mir/jsx_props.rs": ["body"],
  "src/lower/mir/keyed_writes.rs": ["body"],
  "src/lower/mir/locals.rs": ["body", "locals"],
  "src/lower/mir/calls/defined.rs": ["body"],
  "src/lower/mir/calls/method_paths.rs": ["body"],
  "src/lower/mir/nested_options.rs": ["body"],
  "src/lower/mir/nulled.rs": ["at", "body", "locals", "writer"],
  "src/lower/mir/operands.rs": ["body", "locals", "pending", "taken"],
  "src/lower/mir/patterns.rs": ["in_place", "locals"],
  "src/lower/mir/patterns/let_names.rs": ["locals"],
  "src/lower/mir/places.rs": ["binding_var", "body", "branches", "cuts", "defaults", "deferred", "in_place", "locals", "named"],
  "src/lower/mir/queries.rs": ["body", "locals", "own_names"],
  "src/lower/mir/queries/reads.rs": ["body", "locals"],
  "src/lower/mir/rvalues.rs": ["at", "body", "branches", "cuts", "locals", "pending", "statement", "writer"],
  "src/lower/mir/scopes.rs": ["body", "locals"],
  "src/lower/mir/std_calls.rs": ["at", "body", "locals"],
  "src/lower/mir/std_calls/iterators.rs": ["at"],
  "src/lower/mir/stored.rs": ["at", "body", "pending"],
  "src/lower/mir/structure.rs": ["at", "body", "commented", "copy_backs", "deferred", "in_place", "labels", "left", "locals", "pending", "statement", "unwind", "writer", "writes_captured"],
  "src/lower/mir/temporaries.rs": ["body", "copy_backs", "in_place", "iter_locals", "locals", "named", "own_names", "pending", "places", "refs", "settled", "unwind"],
  "src/lower/mir/writer.rs": ["body", "builders", "built", "thrown_writes", "writer", "writes_captured"],
};
test("each module of mir/ reads only the fields of mir.rs's state it's listed for", () => {
  const mir = read("src/lower/mir.rs");
  const struct = mir.slice(mir.indexOf("struct State<"), mir.indexOf("\n}\n", mir.indexOf("struct State<")));
  const fields = [...struct.matchAll(/^    ([a-z_]+):/gm)].map((m) => m[1]).filter((field) => field !== "facts" && field !== "naming");
  expect(struct).toContain("    facts: local_facts::LocalFacts<'m, 'tcx>,");
  expect(struct).toContain("    naming: local_facts::NamingFacts,");
  // What naming settles is made once, where naming's done, never changed.
  const named = ["src/lower/mir.rs", ...files("src/lower/mir")].flatMap((file) => [...read(file).matchAll(/\bstate\s*\.\s*naming\s*=[^=]/g)].map(() => file));
  expect(named).toEqual(["src/lower/mir.rs"]);
  const found: Record<string, string[]> = {};
  for (const file of files("src/lower/mir")) {
    const used = fields.filter((field) => new RegExp(`\\bstate\\s*\\.\\s*${field}\\b`).test(read(file)));
    if (used.length > 0) found[file] = used;
  }
  const sorted = (map: Record<string, string[]>) => Object.fromEntries(Object.entries(map).map(([f, u]) => [f, [...u].sort()]).sort());
  expect(sorted(found)).toEqual(sorted(mirState));
});
