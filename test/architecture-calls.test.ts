// The crate's calls are one call graph, call_graph.rs's, of the MIR the
// emitter reads and its constants' (#1671 change 4 M3; ADR 1148): each
// analysis that follows calls from one function to another asks it, never
// a scan of its own. Split from architecture.test.ts by concept.
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";
import { compilerRustFiles } from "../scripts/compiler-crates";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// The lowering's sources, in name order, as macOS lists them and Linux,
// CI's, doesn't.
// Of the base's crate and MIR's, and the pipeline (#2299's cut).
const lowering = ([["crates/rust-js-lower/src", "crates/rust-js-lower/src"], ["crates/rust-js-mir/src", "crates/rust-js-mir/src"], ["src/pipeline", "src/pipeline"]] as const)
  .flatMap(([dir, prefix]) => readdirSync(join(root, dir), { recursive: true, encoding: "utf8" }).filter((file) => file.endsWith(".rs")).map((file) => `${prefix}/${file}`))
  .concat(["src/pipeline.rs"])
  .sort();

// What each body names, each function it calls or names as a value, is
// scanned once, by the call graph's `Names::of`, its scan private to it:
// each analysis, and the instances' callers, ask its table, `names_of`.
test("an analysis that follows the crate's calls asks the one call graph", () => {
  const graph = read("crates/rust-js-lower/src/call_graph.rs");
  expect(graph).toContain("\nfn named_by<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>)");
  expect(graph.match(/named_by\(/g)).toHaveLength(1);
  const namers = lowering.filter((file) => /Names::of\(/.test(read(file)));
  expect(namers).toEqual(["crates/rust-js-lower/src/analysis/ground.rs"]);
  expect(read("crates/rust-js-lower/src/instances.rs")).toContain("names.names_of(body.def_id)");
  for (const asks of ["closures", "drops", "fmt_failures", "prints", "written_params"].map((name) => `crates/rust-js-lower/src/analysis/${name}.rs`)) {
    expect(read(asks)).toContain("graph.names_of(body.def_id)");
  }
});

// Nor does an analysis build a map of its own of who names whom, each
// function by the bodies that name it or each body by what it names: the
// graph's names are that map, and the callers' carrier (ADR 1115,
// instances.rs) is made of them. A list that only shrinks. What a body may
// run, `may_run.rs`, is a walk per instance, and the functions that never
// unwind whole-crate fixed points, no map of names (ADR 1148).
// collected.rs' is of the bodies as they're collected, before any analysis,
// each of a function item's type, a variable's too.
const ownNameMaps = ["crates/rust-js-lower/src/analysis/collected.rs"];

test("an analysis builds no map of who names whom of its own", () => {
  const builds = lowering.filter(
    (file) =>
      !["crates/rust-js-lower/src/call_graph.rs", "crates/rust-js-lower/src/instances.rs"].includes(file) &&
      /\.entry\(\w+\)\.or_default\(\)\.(insert|push)\(\(?(caller|root)\b/.test(read(file)),
  );
  expect(builds).toEqual(ownNameMaps);
  // The functions a function's body writes, by who names them, of the
  // graph's names (ADR 0308).
  expect(read("crates/rust-js-lower/src/analysis/naming.rs")).toContain("for &(named, _, _, _) in names.names_of(body.def_id) {");
});

// What the crate names as a value, which anything may call with anything,
// is one catalog, the graph's names' (`only_called::named_as_values`),
// which `only_called` and `nulled_params` ask, a `function::bind` of each
// parameter its own naming, `Naming::Bound`, each question's to read
// (#1675). And a method that may be its function is one shape,
// `passed_on::forwarded_function`, asked early, its names not yet known,
// by `only_called`, and with them by `reached::forwarded`.
test("what the crate names as a value is the graph's names' one catalog", () => {
  const valueReaders = lowering.filter((file) => /Naming::Value\b/.test(read(file)));
  expect(valueReaders).toEqual(["crates/rust-js-lower/src/analysis/only_called.rs", "crates/rust-js-lower/src/call_graph.rs"]);
  const askers = lowering.filter((file) => /named_as_values\(/.test(read(file)));
  expect(askers).toEqual(["crates/rust-js-lower/src/analysis.rs", "crates/rust-js-lower/src/analysis/only_called.rs"]);
  const nulled = read("crates/rust-js-lower/src/analysis/nulled_params.rs");
  expect(nulled).not.toContain("fn visit_const_operand");
  expect(nulled).toContain("(values.keys())");
  const forwards = lowering.filter((file) => /forwarded_function\(/.test(read(file)));
  expect(forwards).toEqual(["crates/rust-js-lower/src/analysis/only_called.rs", "src/pipeline/reached.rs"]);
  expect(read("crates/rust-js-lower/src/analysis/only_called.rs")).toContain("forwarded_function(tcx, body.def_id, &body.thir, None)");
  expect(read("src/pipeline/reached.rs")).toContain("forwarded_function(tcx, method, &body.thir, Some(&exported))");
});

// The graph is built once, of the crate's bodies and its constants', and
// what's moved onto it asks that one: the MIR intervals' caller-given
// parameters and the throwing functions, each component after its callees.
test("the crate's call graph is built once, and asked", () => {
  const builders = lowering.filter((file) => /call_graph::crate_graph\(/.test(read(file)));
  expect(builders).toEqual(["crates/rust-js-lower/src/analysis.rs"]);
  const graphs = lowering.filter((file) => file !== "crates/rust-js-lower/src/call_graph.rs" && /call_graph::call_graph\(/.test(read(file)));
  expect(graphs).toEqual([]);
  expect(read("crates/rust-js-lower/src/analysis/throwing.rs")).toContain("for component in &calls.graph.sccs {");
});

// Instance resolution is asked one way too, `traits::resolve`, which takes
// a type a binder inside binds the regions of, erased as codegen's are: a
// call of rustc's own from anywhere else may give it a method's late-bound
// lifetime, which panicked (#2022, the class of #460, #1111, #1201, #1224).
test("instance resolution is asked through `traits::resolve` alone", () => {
  const callers = [...lowering, "crates/rust-js-lower/src/lib.rs"].filter(
    (file) => file !== "crates/rust-js-lower/src/traits.rs" && /Instance::try_resolve\(/.test(read(file)),
  );
  expect(callers).toEqual([]);
});

// A function's signature read by skipping its binder, `fn_sig(..)` ..
// `.skip_binder()`, leaves its late-bound lifetimes bound, and a query
// asked of a type holding one panics (#2022): `traits::erased_sig` reads
// it as a query takes it. test/sig-binders.txt counts each file's skips,
// a ratchet that only falls, none in a file it doesn't list.
test("a signature's binder is skipped only as the ratchet allows", () => {
  const allowed: Record<string, number> = Object.fromEntries(
    read("test/sig-binders.txt").split("\n").filter((line) => line && !line.startsWith("#"))
      .map((line) => line.split("\t")).map(([file, n]) => [file, Number(n)]),
  );
  const sources = compilerRustFiles().filter((file) => file !== "crates/rust-js-lower/src/traits.rs").sort();
  const counted = sources
    .map((file) => [file, (read(file).match(/fn_sig\([^;]*?\.skip_binder\(\)/gs) ?? []).length] as const)
    .filter(([, n]) => n > 0);
  expect(counted.filter(([file, n]) => n > (allowed[file] ?? 0)).map(([file, n]) => `${file}: ${n}, at most ${allowed[file] ?? 0}`)).toEqual([]);
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
  ["crates/rust-js-lower/src/recognition/calls.rs", "    pub fn classify(", 2], // What it was, `classified`, which every answer goes through it by (ADR 1128).
  ["crates/rust-js-lower/src/recognition/calls.rs", "    fn classified(", 8],
  ["crates/rust-js-mir/src/dispatch.rs", "    fn mir_call_of(", 92], // What reads its argument through, `flatten()` and `unwrap()`, in `reads_through` (ADR 0747). Whether a variable holding a function is called as it, `called_as_held` (#581). A method std's impl for a `&mut W` passes on, `W`'s, in `passed_through` (#1309).
  ["crates/rust-js-mir/src/dispatch.rs", "    fn mir_call_values_of(", 203], // A trait call's `same_nones` read before it, a `let` (#2299's `MirCx`). A comparison of `&mut`s to cells, `cells_compared` (#2188). A serde_json call, `mir_json_call` (#1545). Whether `?` is written as it is, `tried_as_written` (ADR 0913). A method std's impl for a `Box<I>` passes on, `I`'s, `passed_through_box`'s, called again of `I` (#1278).
  ["crates/rust-js-mir/src/dispatch/known_calls.rs", "    pub(crate) fn mir_std_call(", 132], // An `Option`'s `as_mut()`, `option_as_mut` (#323).
  ["crates/rust-js-mir/src/lib.rs", "    fn mir_function(", 299], // Its body's tree lowered in the base's `with_given` as `MirCx` again, `MirCx(cx)` (#2299). A switch's split arms, `split`, which `mir_within` hands its switch (#2144); its debug check of a split left to tidy removed. Its parameters' cells and the iterators a JS caller gives as arrays, `parameters_as_given` (#1993). State's `labels`, never read, gone (ADR 1144). A writer's state set up apart, `writer_set_up` (#1637). Where the call being lowered stores what it gives, `called_into`, whose `Result`'s use `result_use` answers (#1545). What an item written in its body may read that tidy can't see, `items_read` (#1472). Its parameters' JS names, `mir_param` (#1381). An or-pattern's alternatives JS holds as one, `mir_alike_alternatives`, for its locals' counts and facts (ADR 1045). The names of its hooks' callbacks, `callback_names`; a scope struct's method's `self`, named in `mir_locals` (ADR 1043). Its plain types asked of `plain_type`, and what's settled until it's written (#121). The `break`s and `continue`s with comments before them its structure found, for tidy (rd109). Its dictionaries written twice, declared at its top, `declare_kept` (ADR 0979). Its floats, for tidy (ADR 0966). What a closure took by value, `kept_captures` (ADR 0879). What it knows of the body before it writes any of it in one, `local_facts` (the review's P3); the parameters handed to tidy's `scopes` stage in a cell, and whether it asks its stages again (ADR 0659, amended); the jumps the program wrote in one field (ADR 0742); the guards that hold (ADR 0790); what its `let`s bind in one (ADR 0598, amended). A class's method's `self` made `this` before tidy (ADR 0761, amended). Its `bool` paths renamed `this`'s too (#355).
  ["crates/rust-js-lower/src/recognition/inherent.rs", "    pub(super) fn classify_inherent(", 744], // Whether a hash table's hasher is a `RandomState`, `random_state_table` (#1420). `assume_init_read()` a copy of its own (#1265). A `RefCell` guard's maps, `guard_map`'s (#1275). What `as_deref` gives as it is, `deref_is_itself`'s (#1311). A number's method and a `Saturating`'s, `number_method`'s (#1434). An `Ordering`'s methods, `ordering_method`'s (#1827).
  ["crates/rust-js-lower/src/recognition/calls.rs", "    fn classify_trait_method(", 225], // `Termination::report` of a type std writes it for, `std_report`, a crate's own impl's left its method (#2292). A `to_owned()`'s answer, `to_owned_of` (#316). A writer's `write!` and `write_char`, `writes_through_write_str` (#1652).
  ["crates/rust-js-lower/src/combinators.rs", "    pub(crate) fn comb_values(", 535], // An `and_then`'s parts, its found index's too, `and_then_parts` (#2074). A `Result`'s `as_mut` and `iter_mut`, `result_lent_mut` (#1474). `inspect`'s call and what it holds, `inspected` (#903). `concat()` of text, `concatenated`'s (#495); a `Result`'s `as_mut` by its form (ADR 1086). What an `Option<&mut T>` or a `Result<&mut T, E>` points at, as its form reads it, `read_through_held`'s (#1311). Its subject held once, `held_subject`'s (#1908), or read twice, `subject_to_read_twice`'s (#1908's scope).
  ["crates/rust-js-mir/src/iter.rs", "    pub(crate) fn mir_iter_call(", 19], // A consumed iterator's call, `consumed_call` (#424), in `consumed_or_held`, beside `from_iter`'s `collected_from_iter` (#1116).
  ["crates/rust-js-mir/src/iter.rs", "    fn mir_iter_method(", 162], // Its `next_back()` and `len()`, `next_back_or_len`, which decides by what the wrappers pass them on to (#2189). Its receiver and its callbacks, a write's result each gives a value, `receiver_and_callbacks` (#1410). A map's, `mapped` (#1342). Its `from_iter`, `collected_from_iter` before it (#1116). Its `rev()`, `rev_called`, and the crate's own step through a `&mut`, `own_step_through_mut` (#745, #755). `len()` of one made in place, `len_in_place` (ADR 0986), its counted chain's too. Its receiver's type revealed, an `impl Iterator`'s (ADR 0985). `next_back()` of what's made here, `next_back_made_here` (#395). `advance_by` in std's bodies, `advanced_by` (ADR 0913).
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

// Where a std call goes has one answer, the call's, `compiled_instance(id,
// args)`, and what std compiles a body of, a definition's,
// `compiled_std_body(id)`: two questions, each recognition's (#2398). The
// catalog's membership itself, `is_compiled_std`, is read in recognition
// alone; and `compiled_ahead_of_place`, the call's answer before what's
// made in place is known, by the in-place analysis alone, as it's what
// decides the rest.
test("std's catalog is read through compiled_instance and compiled_std_body alone", () => {
  const sources = compilerRustFiles();
  const code = (file: string) => read(file).split("\n").filter((line) => !/^\s*\/\//.test(line)).join("\n");
  const recognition = (file: string) => file.startsWith("crates/rust-js-lower/src/recognition/") || file === "crates/rust-js-lower/src/recognition.rs";
  const membership = sources
    .filter((file) => !recognition(file) && /\bis_compiled_std\(/.test(code(file)))
    .map((file) => `${file}: reads \`is_compiled_std\`; ask \`compiled_instance(id, args)\` of a call or \`compiled_std_body(id)\` of a definition`);
  expect(membership).toEqual([]);
  const ahead = sources
    .filter((file) => !recognition(file) && file !== "crates/rust-js-lower/src/body_queries/in_place.rs" && /\bcompiled_ahead_of_place\(/.test(code(file)))
    .map((file) => `${file}: asks \`compiled_ahead_of_place\`, the in-place analysis's alone; ask \`compiled_instance(id, args)\``);
  expect(ahead).toEqual([]);
});
