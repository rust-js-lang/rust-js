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

// Each analysis that scans each body for the functions it names, the call
// graph's own scan, `call_graph::names`, rather than asking the graph's
// table of them, `Crate::names_of`: a list that only shrinks, each moved
// onto the graph leaving it. instances.rs's, run before the analyses,
// whose graph is built after the instances, is the graph's to take next.
const scansLeft = [
  "src/lower/analysis/closures.rs",
  "src/lower/analysis/drops.rs",
  "src/lower/analysis/prints.rs",
  "src/lower/analysis/written_params.rs",
  "src/lower/instances.rs",
];

test("an analysis that follows the crate's calls asks the one call graph", () => {
  const scans = lowering.filter(
    (file) => file !== "src/lower/call_graph.rs" && /call_graph::names\(|instances::calls_of\(/.test(read(file)),
  );
  expect(scans).toEqual(scansLeft);
  expect(read("src/lower/analysis/fmt_failures.rs")).toContain("graph.names_of(body.def_id)");
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
