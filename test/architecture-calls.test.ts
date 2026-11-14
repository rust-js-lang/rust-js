// The crate's calls are one call graph, call_graph.rs's, of the MIR the
// emitter reads and its constants' (#1671 change 4 M3; ADR 1148): each
// analysis that follows calls from one function to another asks it, never
// a scan of its own. Split from architecture.test.ts by concept.
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

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
  expect(graph).toContain("\nfn named_by<'tcx>(body: &Body<'tcx>)");
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
  expect(read("src/lower/analysis/naming.rs")).toContain("for &(named, _, _) in names.names_of(body.def_id) {");
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
