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
const lowering = readdirSync(join(root, "src", "lower"), { recursive: true, encoding: "utf8" })
  .filter((file) => file.endsWith(".rs"))
  .map((file) => `src/lower/${file}`)
  .sort();

// What each body names, each function it calls or names as a value, is
// scanned once, by the call graph's `Names::of`, its scan private to it:
// each analysis, and the instances' callers, ask its table, `names_of`.
test("an analysis that follows the crate's calls asks the one call graph", () => {
  const graph = read("src/lower/call_graph.rs");
  expect(graph).toContain("\nfn named_by<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>)");
  expect(graph.match(/named_by\(/g)).toHaveLength(1);
  const namers = lowering.filter((file) => /Names::of\(/.test(read(file)));
  expect(namers).toEqual(["src/lower/analysis/ground.rs"]);
  expect(read("src/lower/instances.rs")).toContain("names.names_of(body.def_id)");
  for (const asks of ["closures", "drops", "fmt_failures", "prints", "written_params"].map((name) => `src/lower/analysis/${name}.rs`)) {
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
const ownNameMaps = ["src/lower/analysis/collected.rs"];

test("an analysis builds no map of who names whom of its own", () => {
  const builds = lowering.filter(
    (file) =>
      !["src/lower/call_graph.rs", "src/lower/instances.rs"].includes(file) &&
      /\.entry\(\w+\)\.or_default\(\)\.(insert|push)\(\(?(caller|root)\b/.test(read(file)),
  );
  expect(builds).toEqual(ownNameMaps);
  // The functions a function's body writes, by who names them, of the
  // graph's names (ADR 0308).
  expect(read("src/lower/analysis/naming.rs")).toContain("for &(named, _, _, _) in names.names_of(body.def_id) {");
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
  expect(valueReaders).toEqual(["src/lower/analysis/only_called.rs", "src/lower/call_graph.rs"]);
  const askers = lowering.filter((file) => /named_as_values\(/.test(read(file)));
  expect(askers).toEqual(["src/lower/analysis.rs", "src/lower/analysis/only_called.rs"]);
  const nulled = read("src/lower/analysis/nulled_params.rs");
  expect(nulled).not.toContain("fn visit_const_operand");
  expect(nulled).toContain("(values.keys())");
  const forwards = lowering.filter((file) => /forwarded_function\(/.test(read(file)));
  expect(forwards).toEqual(["src/lower/analysis/only_called.rs", "src/lower/pipeline/reached.rs"]);
  expect(read("src/lower/analysis/only_called.rs")).toContain("forwarded_function(tcx, body.def_id, &body.thir, None)");
  expect(read("src/lower/pipeline/reached.rs")).toContain("forwarded_function(tcx, method, &body.thir, Some(&exported))");
});

// The graph is built once, of the crate's bodies and its constants', and
// what's moved onto it asks that one: the MIR intervals' caller-given
// parameters and the throwing functions, each component after its callees.
test("the crate's call graph is built once, and asked", () => {
  const builders = lowering.filter((file) => /call_graph::crate_graph\(/.test(read(file)));
  expect(builders).toEqual(["src/lower/analysis.rs"]);
  const graphs = lowering.filter((file) => file !== "src/lower/call_graph.rs" && /call_graph::call_graph\(/.test(read(file)));
  expect(graphs).toEqual([]);
  expect(read("src/lower/analysis/throwing.rs")).toContain("for component in &calls.graph.sccs {");
});

// Instance resolution is asked one way too, `traits::resolve`, which takes
// a type a binder inside binds the regions of, erased as codegen's are: a
// call of rustc's own from anywhere else may give it a method's late-bound
// lifetime, which panicked (#2022, the class of #460, #1111, #1201, #1224).
test("instance resolution is asked through `traits::resolve` alone", () => {
  const callers = [...lowering, "src/lower.rs"].filter(
    (file) => file !== "src/lower/traits.rs" && /Instance::try_resolve\(/.test(read(file)),
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
  const sources = compilerRustFiles().filter((file) => file !== "src/lower/traits.rs").sort();
  const counted = sources
    .map((file) => [file, (read(file).match(/fn_sig\([^;]*?\.skip_binder\(\)/gs) ?? []).length] as const)
    .filter(([, n]) => n > 0);
  expect(counted.filter(([file, n]) => n > (allowed[file] ?? 0)).map(([file, n]) => `${file}: ${n}, at most ${allowed[file] ?? 0}`)).toEqual([]);
});
