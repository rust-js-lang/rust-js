// The test map's edges (ADR 1210, #2608): rustc's resolved references, as
// the driver the counting build compiles the compiler's crates with writes
// them (scripts/edges), read into the map's `defs` and `edges`
// (scripts/testMapEdges.ts), every crate's or none.
import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { edgePairs, edgesDriver, recordedEdges } from "../scripts/testMapEdges";
import { pick, type Sources, type TestMap, withDefs } from "../scripts/testMap";
import { fixture, root, target } from "./support";

const crate = `pub enum Shape {
    Circle(f64),
    Square { side: f64 },
}

pub struct Holder {
    pub count: u32,
    pub shape: Shape,
}

pub type Alias = Holder;

pub trait Area {
    fn area(&self) -> f64;
}

impl Area for Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle(r) => r * r,
            Shape::Square { side } => side * side,
        }
    }
}

impl Holder {
    pub fn bump(&mut self) {
        self.count += 1;
    }
}

macro_rules! doubled {
    ($e:expr) => {
        $e * 2
    };
}

pub fn reads(h: &Alias) -> u32 {
    doubled!(h.count)
}

pub fn calls(h: &mut Holder) -> f64 {
    h.bump();
    h.shape.area()
}

pub fn shown(x: &dyn std::fmt::Display) -> String {
    x.to_string()
}
`;

/** The fixture crate compiled by the driver: its record, each edge by its
 * two definitions' kinds, names and first lines. */
function compiled(dir: string) {
  const driver = edgesDriver(root, join(target, "edges-build"));
  if (typeof driver === "string") throw new Error(driver);
  const out = join(dir, "edges");
  mkdirSync(out, { recursive: true });
  writeFileSync(join(dir, "lib.rs"), crate);
  // Its root by a path from where it's compiled, as Cargo gives a
  // workspace's crate: its record names it from the root.
  const p = Bun.spawnSync([driver.path, "rustc", "--crate-type=lib", "--edition=2024", "--crate-name=fixture", "-o", join(dir, "libfixture.rlib"), "lib.rs"], { cwd: dir, env: { ...process.env, RUST_JS_EDGES_DIR: out }, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  const said: { defs: { id: string; kind: string; name: string; lines: [number, number] }[]; edges: [string, string][] } = JSON.parse(readFileSync(join(out, "fixture.json"), "utf8"));
  const by = new Map(said.defs.map((def) => [def.id, `${def.kind} ${def.name}:${def.lines[0]}`]));
  return { out, edges: new Set(said.edges.map(([namer, named]) => `${by.get(namer)} -> ${by.get(named)}`)) };
}

// Each kind of reference rustc resolves, an edge: a path, a method and a
// trait's method a type decides, a field read, an alias, a macro's call,
// an `impl`'s trait and trait item to its own and its self type to it, and
// a definition's parent to it. A text search guessed at each.
test("the driver records each kind of reference rustc resolved, an edge from what names to what's named", () => {
  const dir = fixture("edges-kinds");
  const { edges } = compiled(dir);
  const expected = [
    // A path: a type in a signature.
    "fn calls:42 -> struct Holder:6",
    // A method a type decides, inherent and a trait's.
    "fn calls:42 -> assoc fn bump:27",
    "fn calls:42 -> assoc fn area:14",
    // A field read.
    "fn reads:38 -> field count:7",
    "assoc fn bump:27 -> field count:7",
    // An alias, and what it's written of.
    "fn reads:38 -> type Alias:11",
    "type Alias:11 -> struct Holder:6",
    // A macro's call.
    "fn reads:38 -> macro doubled:32",
    // An `impl`: its trait's, its trait item's, its self type's.
    "trait Area:13 -> impl :17",
    "assoc fn area:14 -> assoc fn area:18",
    "enum Shape:1 -> impl :17",
    // A parent's: a variant, a field, an item.
    "enum Shape:1 -> variant Circle:2",
    "struct Holder:6 -> field shape:8",
    "impl :26 -> assoc fn bump:27",
  ];
  expect(expected.filter((edge) => !edges.has(edge))).toEqual([]);
}, 300_000);

// The map's edges are every crate's whole or none: a crate with no record,
// a walk that failed, or a record of sources since changed, and there are
// none, said why.
test("a crate's record is read where its sources are the tree's; none of a crate unrecorded, failed or stale", () => {
  const dir = fixture("edges-read");
  const { out } = compiled(dir);
  const functions = [{ file: "lib.rs", spans: [[39, 39]] as [number, number][] }, { file: "lib.rs", spans: [[28, 28]] as [number, number][] }];
  const read = recordedEdges(out, ["fixture"], dir, functions);
  if (typeof read === "string") throw new Error(read);
  // Each definition of the checkout's once, a function's holding its code.
  const reads = read.defs.findIndex((def) => def.lines[0] === 38);
  const bump = read.defs.findIndex((def) => def.lines[0] === 27);
  expect([read.defs[reads]!.fns, read.defs[bump]!.fns, read.defs.every((def) => def.file === "lib.rs")]).toEqual([[0], [1], true]);
  // By each name, what names one of it, another crate's too: what a new
  // item of the name may take the place of.
  const shown = read.defs.findIndex((def) => def.lines[0] === 47);
  expect([read.names.Display, read.names.to_string, read.names.Holder]).toEqual([[shown], [shown], expect.arrayContaining([read.defs.findIndex((def) => def.lines[0] === 42)])]);
  const pairs = edgePairs(read.edges);
  expect(pairs.length % 2).toBe(0);
  expect(pairs.length).toBeGreaterThan(20);
  // None whole: a crate no record names, a walk that failed, a source since.
  expect(recordedEdges(out, ["fixture", "other"], dir, functions)).toBe("no record of other: it wasn't compiled with the driver");
  writeFileSync(join(out, "failed.failed"), "");
  expect(recordedEdges(out, ["failed"], dir, functions)).toBe("the walk of failed failed");
  writeFileSync(join(dir, "lib.rs"), `${crate}\n`);
  expect(recordedEdges(out, ["fixture"], dir, functions)).toBe("fixture's record is of lib.rs as it was, not as the tree has it");
  // The map's: each definition's file in its table of paths, the loads'
  // indices with it; none and `null` where the plan has none.
  const placed = withDefs(["a.ts", "z.ts"], read);
  expect([placed.all, placed.defs[0]!.file, placed.edges]).toEqual([["a.ts", "lib.rs", "z.ts"], 1, read.edges]);
  expect(withDefs(["a.ts"], "its driver didn't build (exit 1)")).toEqual({ all: ["a.ts"], defs: [], edges: null });
}, 300_000);

// A line outside any function's counted code, by the edges: what reaches
// the innermost definition holding it, backwards, each function's tests;
// no name searched. A field read by one function and named, through its
// struct, by another; a constant nothing names; a line no definition
// holds, everything, said (#2608).
test("a change outside any function selects, by the edges, the functions that reach its definition", () => {
  const bits = (fn: number) => Buffer.from([1 << fn]).toString("base64");
  const pairs = (...edges: [number, number][]) => Buffer.from(new Int32Array(edges.flat()).buffer).toString("base64");
  // src/a.rs: struct S at 1-5, its field at 3; fn a at 10-20 reads the
  // field; fn b at 30-40 names S; a const at 25; a `use` at 45; at 47,
  // none.
  const map: TestMap = {
    version: 6,
    commit: "0",
    toolchain: "0",
    made: 0,
    functions: [
      { file: "src/a.rs", spans: [[10, 20]] },
      { file: "src/a.rs", spans: [[30, 40]] },
    ],
    units: { "test/one.test.ts": bits(0), "test/two.test.ts": bits(1) },
    includes: {},
    corpusReaders: [],
    noCompiler: [],
    paths: ["src/a.rs"],
    defs: [
      { file: 0, lines: [1, 5] },
      { file: 0, lines: [3, 3] },
      { file: 0, lines: [10, 20], fns: [0] },
      { file: 0, lines: [30, 40], fns: [1] },
      { file: 0, lines: [25, 25] },
      { file: 0, lines: [45, 45] },
      { file: 0, lines: [27, 27] },
    ],
    // a reads the field and the const at 27, b names S and calls a, S
    // holds its field.
    edges: pairs([2, 1], [3, 0], [0, 1], [2, 6], [3, 2]),
  };
  const text = Array.from({ length: 50 }, (_, i) => `    let x${i + 1} = ${i + 1};`);
  text[44] = "use std::fmt;";
  const sources: Sources = { text: (file) => (file === "src/a.rs" ? text.join("\n") : undefined), naming: () => [] };
  const files = ["test/one.test.ts", "test/two.test.ts"];
  const at = (hunks: [number, number, string[]?, string[]?][]) => pick(map, [{ file: "src/a.rs", hunks }], files, [], sources);
  // The field: what reads it, and what names its struct.
  expect(at([[3, 1]])).toMatchObject({ kind: "picked", functions: [0, 1] });
  // The struct's own line: what names it.
  expect(at([[1, 1]])).toMatchObject({ kind: "picked", functions: [1] });
  // A line added inside the struct, between 2 and 3: the struct's.
  expect(at([[2, 0, ["    pub more: u8,"], []]])).toMatchObject({ kind: "picked", functions: [1] });
  // A constant a names: a, not a's caller b, whose tests run a already.
  expect(at([[27, 1]])).toMatchObject({ kind: "picked", functions: [0] });
  // A constant nothing names: no function.
  expect(at([[25, 1]])).toMatchObject({ kind: "picked", functions: [] });
  // A `use`, a definition nothing names: nothing; a line no definition
  // holds: everything.
  expect(at([[45, 1]])).toMatchObject({ kind: "picked", functions: [] });
  expect(at([[47, 1]])).toEqual({ kind: "all", why: "src/a.rs's change at line 47 is outside any function's counted code and any definition the edges know" });
  // A new item or `use` at a module's top level: what names a definition
  // of its name, which may resolve to it now, a glob's or the prelude's.
  const named = { ...map, edgeNames: { Fresh: [3], Bar: [2] } };
  const added = (lines: string[]) => pick(named, [{ file: "src/a.rs", hunks: [[47, 0, lines, []]] }], files, [], sources);
  expect(added(["pub struct Fresh;"])).toMatchObject({ kind: "picked", functions: [1] });
  expect(added(["use crate::other::{Bar, Baz as Qux};"])).toMatchObject({ kind: "picked", functions: [0] });
  expect(added(["pub fn unnamed() {}"])).toMatchObject({ kind: "picked", functions: [] });
  // A glob's names no edge says: each function of its module's file, whose
  // prelude's names it may take the place of; a macro's call there may
  // write any item: everything.
  expect(added(["use crate::other::*;"])).toMatchObject({ kind: "picked", functions: [0, 1] });
  expect(added(["thread_local! { static X: u8 = 0; }"]).kind).toBe("all");
  // The crate's unit tests', which the counting build compiles none of:
  // nothing, a line of a `#[cfg(test)]` function's or of its module's.
  const tested = Array.from({ length: 50 }, (_, i) => `    let x${i + 1} = ${i + 1};`);
  tested[45] = "#[cfg(test)]";
  tested[46] = "mod tests {";
  tested[47] = "    #[test]";
  tested[48] = "    fn checks() {";
  const inTests: Sources = { text: () => tested.join("\n"), naming: () => [] };
  expect(pick(named, [{ file: "src/a.rs", hunks: [[50, 1], [47, 1]] }], files, [], inTests)).toMatchObject({ kind: "picked", functions: [] });
  expect(pick(named, [{ file: "src/a.rs", hunks: [[47, 0, ["    fn more() {}"], []]] }], files, [], inTests)).toMatchObject({ kind: "picked", functions: [] });
  // A map whose edges have no names can't say: everything.
  expect(pick(map, [{ file: "src/a.rs", hunks: [[47, 0, ["pub struct Fresh;"], []]] }], files, [], sources).kind).toBe("all");
  // Without edges, a map selects as before: here, by the names, everything.
  expect(pick({ ...map, edges: null }, [{ file: "src/a.rs", hunks: [[25, 1]] }], files, [], sources).kind).toBe("all");
});
