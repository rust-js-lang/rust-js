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
    "crates/rust-js-lower/src/drops.rs",
    "crates/rust-js-lower/src/drops/functions.rs",
    "crates/rust-js-lower/src/drops/glue.rs",
    "crates/rust-js-lower/src/drops/parts.rs",
    "crates/rust-js-lower/src/drops/scopes.rs",
    "crates/rust-js-lower/src/drops/uses.rs",
  ],
  // What lowering from MIR found of a body's variables, nine sets once
  // fields of `Locals` that mir.rs wrote and jsx.rs and binding_calls.rs
  // read (the third architecture audit).
  "self.locals.mir": [
    "crates/rust-js-mir/src/lib.rs",
    "crates/rust-js-mir/src/assign.rs",
    "crates/rust-js-mir/src/bounds.rs",
    "crates/rust-js-mir/src/closures.rs",
    "crates/rust-js-mir/src/local_facts.rs",
    "crates/rust-js-lower/src/held.rs",
    "crates/rust-js-lower/src/noted.rs",
    "crates/rust-js-mir/src/places.rs",
    "crates/rust-js-mir/src/rvalues.rs",
    "crates/rust-js-mir/src/structure.rs",
    "crates/rust-js-mir/src/switch_tests.rs",
    "crates/rust-js-mir/src/temporaries.rs",
  ],
  // What lowering a function found of drops, which the pipeline gathers
  // and resolves: the drops module's to write, where `CrateFacts` held it
  // in cells every function wrote (the fourth review).
  "self.drops_observed": ["crates/rust-js-lower/src/drops.rs", "crates/rust-js-lower/src/drops/uses.rs"],
  // What of its libraries lowering a function found the crate compiles
  // itself, which the pipeline gathers, as `drops_observed` (ADR 1148):
  // asked where an item is named, and of a library's impl or default.
  // The module's functions a body made and called, which the pipeline
  // settles after it, recursive_fns.rs's to write (ADR 1148).
  "self.module_fns_made": ["crates/rust-js-lower/src/recursive_fns.rs"],
  // What a body saw of the calls and returns of the crate's functions with
  // own nullable parameters, mir/nulled.rs's to write (ADR 1148).
  "self.nulled_seen": ["crates/rust-js-mir/src/nulled.rs", "crates/rust-js-lower/src/combinators/callbacks.rs"],
  "self.asked_of_libraries": ["crates/rust-js-lower/src/items.rs", "crates/rust-js-lower/src/traits/impls.rs", "crates/rust-js-lower/src/traits/impls/defaults.rs"],
  "self.writing": ["crates/rust-js-lower/src/display.rs", "crates/rust-js-lower/src/display/debug.rs", "crates/rust-js-lower/src/display/written.rs"],
  "self.recursive": ["crates/rust-js-lower/src/recursive_fns.rs"],
  "self.given": ["crates/rust-js-lower/src/traits/given.rs", "crates/rust-js-lower/src/traits/calls.rs", "crates/rust-js-lower/src/traits/impls.rs", "crates/rust-js-lower/src/traits/impls/defaults.rs", "crates/rust-js-lower/src/traits/routes.rs"],
  "self.walks.representable": ["crates/rust-js-lower/src/support.rs"],
  "self.walks.assumed": ["crates/rust-js-lower/src/support.rs"],
  "self.walks.mutated": ["crates/rust-js-lower/src/copies.rs"],
  "self.walks.clones": ["crates/rust-js-lower/src/std_impls/clones.rs"],
  "self.walks.clone_assumed": ["crates/rust-js-lower/src/std_impls/clones.rs"],
  "self.codecs": ["crates/rust-js-lower/src/serde/codecs.rs"],
  // A function's scope struct's methods, functions declared in it (ADR
  // 1043): their names, which calls of them call, and while one is
  // lowered, the variable it closes over, its `self`'s.
  "self.scope_functions": ["crates/rust-js-mir/src/calls.rs", "crates/rust-js-mir/src/scopes.rs"],
  "self.scope_self": ["crates/rust-js-mir/src/locals.rs", "crates/rust-js-mir/src/scopes.rs"],
  // And the lowering of each its function's lowering declares, which a
  // call of one calls and its body takes its type parameters as (#500).
  "self.scope_copies": ["crates/rust-js-mir/src/calls.rs", "crates/rust-js-mir/src/scopes.rs", "crates/rust-js-lower/src/representation/options.rs"],
  // And each scope's value's variable, whose fields its function's body,
  // once lowered, makes its variables (ADR 1043, amended).
  "self.scope_values": ["crates/rust-js-mir/src/lib.rs", "crates/rust-js-mir/src/scopes.rs"],
  // While a `match`'s tests of its arms are made, which it refuses itself
  // once it knows its arms differ, where a class is told by `instanceof`
  // (#555): set by its switch, read where a test is.
  "self.telling_arms": ["crates/rust-js-mir/src/structure.rs", "crates/rust-js-lower/src/untagged/classes.rs"],
  // While a call is lowered, the local it makes its value into, its maker's
  // MIR identity, which what's stepped from its back asks (#814).
  "self.made_into": ["crates/rust-js-lower/src/iterators/back_facts.rs"],
  "self.made_from": ["crates/rust-js-lower/src/iterators/back_facts.rs"],
  // The first name of each base `fresh` gives out in the item, what one
  // given apart from it is of, by its id (ADR 1150): `fresh`'s alone.
  "self.first_of": ["crates/rust-js-lower/src/lib.rs"],
  "self.asking_made": ["crates/rust-js-lower/src/iterators/back_facts.rs", "crates/rust-js-lower/src/iterators/mir_steps/backs.rs"],
  // While the crate's iterator is made the JS iterator behind a `dyn`
  // wrapper, that wrapper, which `own_methods` filters by (#2217): set
  // and read by stepping.rs alone, `seen_through`'s.
  "self.seen_through": ["crates/rust-js-lower/src/iterators/stepping.rs"],
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
  locals: "a body's variables; its MIR facts, `locals.mir`, are mir.rs's, what it notes of them `noted.rs`'s and what it holds `held.rs`'s",
  dependencies: "what it uses of other crates, noted by items.rs",
  settled: "what the bodies lowered before it settled, the pipeline's, read only (ADR 1148)",
};
// Each field of `FnCx` is an owner's, or the shared context above: a field
// added with neither is one nothing says who may change (the second review).
test("each field of a function's context has an owner, or is its shared context", () => {
  const lower = read("crates/rust-js-lower/src/lib.rs");
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
  for (const file of [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")]) {
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
  "crates/rust-js-mir/src/bounds.rs": ["at", "body"],
  "crates/rust-js-mir/src/assign.rs": ["binding_var", "body", "branches", "builders", "commented", "cuts", "in_place", "iter_locals", "locals", "named", "parts", "pending", "places", "refs", "writer"],
  "crates/rust-js-mir/src/assign/boxed_args.rs": ["copy_backs"],
  "crates/rust-js-mir/src/assign/let_rest.rs": ["body", "locals"],
  "crates/rust-js-mir/src/assign/rewritten.rs": ["body"],
  "crates/rust-js-mir/src/given_arguments.rs": ["at", "body"],
  "crates/rust-js-mir/src/calls.rs": ["at", "body", "flattened"],
  "crates/rust-js-mir/src/closures.rs": ["at", "body", "in_place", "locals"],
  "crates/rust-js-mir/src/dispatch.rs": ["at", "body", "called_into", "defaults", "flattened", "iter_locals", "locals"],
  "crates/rust-js-mir/src/drops.rs": ["body", "branches", "locals", "parts", "pending"],
  "crates/rust-js-mir/src/entries.rs": ["body", "locals", "thrown_writes"],
  "crates/rust-js-mir/src/dispatch/equalities.rs": ["body"],
  "crates/rust-js-mir/src/dispatch/known_calls.rs": ["at", "body"],
  "crates/rust-js-mir/src/hints/clones.rs": ["locals"],
  "crates/rust-js-mir/src/hints/names.rs": ["locals"],
  "crates/rust-js-mir/src/jsx_props.rs": ["body"],
  "crates/rust-js-mir/src/keyed_writes.rs": ["body"],
  "crates/rust-js-mir/src/locals.rs": ["body", "locals"],
  "crates/rust-js-mir/src/calls/defined.rs": ["body"],
  "crates/rust-js-mir/src/calls/method_paths.rs": ["body"],
  "crates/rust-js-mir/src/nested_options.rs": ["body"],
  "crates/rust-js-mir/src/nulled.rs": ["at", "body", "locals", "writer"],
  "crates/rust-js-mir/src/nulled/field_reads.rs": ["at", "body", "locals"],
  "crates/rust-js-mir/src/nulled/captured.rs": ["at", "body"],
  "crates/rust-js-mir/src/nulled/defined.rs": ["at", "body"],
  "crates/rust-js-mir/src/nulled/items.rs": ["body", "locals"],
  "crates/rust-js-mir/src/nulled/lends.rs": ["body", "locals"],
  "crates/rust-js-mir/src/nulled/one_none.rs": ["at", "body", "locals", "writer"],
  "crates/rust-js-mir/src/nulled/presence_paths.rs": ["body", "locals"],
  "crates/rust-js-mir/src/none_forms.rs": ["at", "body"],
  "crates/rust-js-mir/src/nulled/proofs.rs": ["at", "body", "locals"],
  "crates/rust-js-mir/src/operands.rs": ["at", "body", "locals", "pending", "statement", "taken"],
  "crates/rust-js-mir/src/patterns.rs": ["in_place", "locals"],
  "crates/rust-js-mir/src/patterns/let_names.rs": ["locals"],
  "crates/rust-js-mir/src/places.rs": ["binding_var", "body", "branches", "cuts", "defaults", "deferred", "in_place", "locals", "named", "refs"],
  "crates/rust-js-mir/src/queries.rs": ["body", "locals", "own_names"],
  "crates/rust-js-mir/src/queries/reads.rs": ["body", "locals"],
  "crates/rust-js-mir/src/rvalues.rs": ["at", "body", "branches", "cuts", "locals", "pending", "statement"],
  "crates/rust-js-mir/src/rvalues/fields.rs": ["body"],
  "crates/rust-js-mir/src/rvalues/aggregate.rs": ["body", "locals"],
  "crates/rust-js-mir/src/scopes.rs": ["body", "locals"],
  "crates/rust-js-mir/src/std_calls.rs": ["at", "body", "locals"],
  "crates/rust-js-mir/src/std_calls/iterators.rs": ["at"],
  "crates/rust-js-mir/src/std_calls/written_into.rs": ["at", "body", "called_into", "writer"],
  "crates/rust-js-mir/src/stored.rs": ["at", "body", "pending"],
  "crates/rust-js-mir/src/structure.rs": ["at", "body", "called_into", "commented", "copy_backs", "deferred", "in_place", "left", "locals", "pending", "split", "statement", "unwind", "writer", "writes_captured"],
  "crates/rust-js-mir/src/temporaries.rs": ["body", "copy_backs", "in_place", "iter_locals", "locals", "named", "own_names", "pending", "places", "refs", "settled", "unwind"],
  "crates/rust-js-mir/src/writer.rs": ["body", "builders", "built", "thrown_writes", "writer", "writes_captured"],
};
test("each module of mir/ reads only the fields of mir.rs's state it's listed for", () => {
  const mir = read("crates/rust-js-mir/src/lib.rs");
  const struct = mir.slice(mir.indexOf("struct State<"), mir.indexOf("\n}\n", mir.indexOf("struct State<")));
  const fields = [...struct.matchAll(/^    ([a-z_]+):/gm)].map((m) => m[1]).filter((field) => field !== "facts" && field !== "naming");
  expect(struct).toContain("    facts: local_facts::LocalFacts<'m, 'tcx>,");
  expect(struct).toContain("    naming: local_facts::NamingFacts,");
  // What naming settles is made once, where naming's done, never changed.
  const named = files("crates/rust-js-mir/src").flatMap((file) => [...read(file).matchAll(/\bstate\s*\.\s*naming\s*=[^=]/g)].map(() => file));
  expect(named).toEqual(["crates/rust-js-mir/src/lib.rs"]);
  const found: Record<string, string[]> = {};
  for (const file of files("crates/rust-js-mir/src").filter((file) => file !== "crates/rust-js-mir/src/lib.rs")) {
    const used = fields.filter((field) => new RegExp(`\\bstate\\s*\\.\\s*${field}\\b`).test(read(file)));
    if (used.length > 0) found[file] = used;
  }
  const sorted = (map: Record<string, string[]>) => Object.fromEntries(Object.entries(map).map(([f, u]) => [f, [...u].sort()]).sort());
  expect(sorted(found)).toEqual(sorted(mirState));
});
