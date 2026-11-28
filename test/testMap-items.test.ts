// The test map's item layer (scripts/testMapItems.ts, #2288): which of the
// map's functions reach each item outside any function, and what a change
// there selects, on a crate small enough to read, its SCIP index written
// here by hand. And the layer's selection in shadow beside the one that
// runs, as check:fast and the push's line report it.

import { expect, test } from "bun:test";
import { implBodyAt, inUnitTests, itemLayer, readScip, symbolName, touchedByItem, touchedByNewFile } from "../scripts/testMapItems";
import { pick, type TestMap } from "../scripts/testMap";
import { shadowMissed } from "../scripts/select";
import { shadowSaid } from "../scripts/push";

// A SCIP index's bytes: varints and length-delimited fields.
const varint = (n: number) => {
  const out: number[] = [];
  do {
    let byte = n & 0x7f;
    n = Math.floor(n / 128);
    if (n > 0) byte |= 0x80;
    out.push(byte);
  } while (n > 0);
  return out;
};
const field = (number: number, bytes: number[]) => [...varint((number << 3) | 2), ...varint(bytes.length), ...bytes];
const number = (n: number, value: number) => [...varint(n << 3), ...varint(value)];
const text = (s: string) => [...new TextEncoder().encode(s)];
const packed = (values: number[]) => values.flatMap(varint);
type Occ = { line: number; col: number; end: number; symbol: string; definition?: boolean; enclosing?: [number, number, number, number] };
const occurrence = (o: Occ) => [
  ...field(1, packed([o.line, o.col, o.end])),
  ...field(2, text(o.symbol)),
  ...(o.definition ? number(3, 1) : []),
  ...(o.enclosing ? field(7, packed(o.enclosing)) : []),
];
const index = (documents: { path: string; occurrences: Occ[] }[]) =>
  new Uint8Array(documents.flatMap((d) => field(2, [...field(1, text(d.path)), ...d.occurrences.flatMap((o) => field(2, occurrence(o)))])));

const p = "rust-analyzer cargo rust-js 0.1.0 ";
// The crate, src/a.rs, its lines numbered from 1.
const source = [
  "pub enum Kind {", // 1
  "    A,", // 2
  "    B,", // 3
  "}", // 4
  "fn f() -> Kind {", // 5
  "    Kind::A", // 6
  "}", // 7
  "const JUMPS: &[u8] = &[1];", // 8
  "const STAGES: &[&[u8]] = &[JUMPS];", // 9
  "fn g() -> usize {", // 10
  "    STAGES.len()", // 11
  "}", // 12
  "trait Tr { fn m(&self); }", // 13
  "impl Tr for Kind {", // 14
  "    fn m(&self) {", // 15
  "        let _ = 1;", // 16
  "    }", // 17
  "}", // 18
  "impl Kind {", // 19
  "    fn n(&self) {", // 20
  "        let _ = 2;", // 21
  "    }", // 22
  "}", // 23
  "fn h(k: &Kind) {", // 24
  "    k.m()", // 25
  "}", // 26
  "#[cfg(test)]", // 27
  "mod tests {", // 28
  "    fn t() {}", // 29
  "}", // 30
];
const documents = readScip(
  index([
    {
      path: "src/a.rs",
      occurrences: [
        { line: 0, col: 9, end: 13, symbol: `${p}a/Kind#`, definition: true, enclosing: [0, 0, 3, 1] },
        { line: 1, col: 4, end: 5, symbol: `${p}a/Kind#A#`, definition: true, enclosing: [1, 4, 1, 5] },
        { line: 2, col: 4, end: 5, symbol: `${p}a/Kind#B#`, definition: true, enclosing: [2, 4, 2, 5] },
        { line: 4, col: 3, end: 4, symbol: `${p}a/f().`, definition: true, enclosing: [4, 0, 6, 1] },
        { line: 5, col: 10, end: 11, symbol: `${p}a/Kind#A#` },
        { line: 7, col: 6, end: 11, symbol: `${p}a/JUMPS.`, definition: true, enclosing: [7, 0, 7, 26] },
        { line: 8, col: 6, end: 12, symbol: `${p}a/STAGES.`, definition: true, enclosing: [8, 0, 8, 34] },
        { line: 8, col: 27, end: 32, symbol: `${p}a/JUMPS.` },
        { line: 9, col: 3, end: 4, symbol: `${p}a/g().`, definition: true, enclosing: [9, 0, 11, 1] },
        { line: 10, col: 4, end: 10, symbol: `${p}a/STAGES.` },
        { line: 12, col: 6, end: 8, symbol: `${p}a/Tr#`, definition: true, enclosing: [12, 0, 12, 25] },
        { line: 13, col: 5, end: 7, symbol: `${p}a/Tr#` },
        { line: 14, col: 7, end: 8, symbol: `${p}a/Kind#m().`, definition: true, enclosing: [14, 4, 16, 5] },
        { line: 19, col: 7, end: 8, symbol: `${p}a/Kind#n().`, definition: true, enclosing: [19, 4, 21, 5] },
        { line: 23, col: 3, end: 4, symbol: `${p}a/h().`, definition: true, enclosing: [23, 0, 25, 1] },
        { line: 24, col: 6, end: 7, symbol: `${p}a/Tr#m().` },
      ],
    },
  ]),
);
// The map's functions: f, g, m, n, h, each its counted lines.
const functions: TestMap["functions"] = [
  { file: "src/a.rs", spans: [[6, 6]] },
  { file: "src/a.rs", spans: [[11, 11]] },
  { file: "src/a.rs", spans: [[16, 16]] },
  { file: "src/a.rs", spans: [[21, 21]] },
  { file: "src/a.rs", spans: [[25, 25]] },
];
const [F, G, M, N, H] = [0, 1, 2, 3, 4];
const layer = itemLayer(documents, functions, (file) => (file === "src/a.rs" ? source : []));
const item = (symbol: string) => layer.items.find((i) => i.symbol === `${p}${symbol}`);

test("SCIP's documents read: each occurrence's place, symbol, role and extent", () => {
  expect(documents.map((d) => d.path)).toEqual(["src/a.rs"]);
  const kind = documents[0]!.occurrences[0]!;
  expect(kind).toEqual({ line: 0, col: 9, end: 13, symbol: `${p}a/Kind#`, definition: true, enclosing: [0, 3] });
  expect(symbolName(`${p}a/Kind#A#`)).toBe("A");
  expect(symbolName(`${p}a/STAGES.`)).toBe("STAGES");
});

test("an item's referencers are SCIP's, by name, and through another item that names it", () => {
  // An enum: the function that constructs its variant.
  expect(layer.items.find((i) => i.symbol === `${p}a/Kind#`)!.refs).toContain(F);
  // A const named only by another const a function reads: that function.
  expect(layer.items.find((i) => i.symbol === `${p}a/JUMPS.`)!.refs).toEqual([G]);
  // Each function's own name, for one moved to another file.
  expect(layer.names).toEqual(["f", "g", "m", "n", "h"]);
  expect(inUnitTests(layer, "src/a.rs", 29)).toBe(true);
  expect(inUnitTests(layer, "src/a.rs", 25)).toBe(false);
  expect(item("a/Kind#A#")?.span).toEqual([2, 2]);
});

test("a change outside any function selects what reaches its item, or its impl's", () => {
  // A variant's line: what reaches the enum.
  expect(touchedByItem(layer, "src/a.rs", 3)).toContain(F);
  // A trait impl's header: its methods, and what names its trait.
  const header = touchedByItem(layer, "src/a.rs", 14)!;
  expect(header).toContain(M);
  expect(header).toContain(H);
  // An inherent impl's body, between its methods: itself, no trait.
  expect(implBodyAt(layer, "src/a.rs", 22)?.trait).toBe(false);
  expect(implBodyAt(layer, "src/a.rs", 17)?.trait).toBe(true);
  expect(touchedByItem(layer, "src/a.rs", 31)).toBeUndefined();
});

test("a new file selects what moved there by name, and what reaches the items it defines", () => {
  const moved = touchedByNewFile(layer, "fn g() -> usize {\n    1\n}\nconst JUMPS: &[u8] = &[];\n") as number[];
  expect(moved).toContain(G);
  expect(touchedByNewFile(layer, "fn brand_new() {}\n")).toEqual([]);
  expect(touchedByNewFile(layer, "macro_rules! m { () => {} }\n")).toBeString();
});

test("pick selects by the item layer where a change is outside any function, else everything", () => {
  const bits = (...at: number[]) => {
    const bytes = new Uint8Array(1);
    for (const f of at) bytes[f >> 3]! |= 1 << (f & 7);
    return Buffer.from(bytes).toString("base64");
  };
  const map: TestMap = {
    version: 4,
    commit: "x",
    toolchain: "t",
    made: 0,
    functions,
    units: { "test/f.test.ts": bits(F), "test/g.test.ts": bits(G), "test/h.test.ts": bits(H), "test/m.test.ts": bits(M) },
    includes: {},
    corpusReaders: [],
    noCompiler: [],
  };
  const sources = {
    text: (file: string) => (file === "src/a.rs" ? source.join("\n") : undefined),
    naming: (word: string) => source.flatMap((line, i) => (new RegExp(`\\b${word}\\b`).test(line) ? [{ file: "src/a.rs", line: i + 1 }] : [])),
  };
  const files = Object.keys(map.units);
  const at = (changes: Parameters<typeof pick>[1], items = true) => pick(items ? { ...map, items: layer } : map, changes, files, [], sources);
  // Of the map's own test files, those picked.
  const own = (picked: ReturnType<typeof pick>) => (picked.kind === "picked" ? picked.files.filter((file) => file in map.units).sort() : picked.kind);
  // A variant added: without the layer, what names the enum, by name, the
  // signatures `fn f() -> Kind` and `fn h(k: &Kind)` naming it the
  // functions they open (#2523), as the layer says here too.
  const variant = [{ file: "src/a.rs", hunks: [[3, 0, ["    C,"], []]] }] as Parameters<typeof pick>[1];
  expect(own(at(variant, false))).toEqual(["test/f.test.ts", "test/h.test.ts"]);
  // A variant added: what constructs the enum, and what names it.
  expect(own(at(variant))).toEqual(["test/f.test.ts", "test/h.test.ts"]);
  // A const's value, named by the const a function reads: g's test.
  const value = [{ file: "src/a.rs", hunks: [[8, 1, ["const JUMPS: &[u8] = &[2];"], ["const JUMPS: &[u8] = &[1];"]]] }] as Parameters<typeof pick>[1];
  expect(own(at(value))).toEqual(["test/g.test.ts"]);
  // A trait impl's header: its method's and its trait's users' tests.
  const header = [{ file: "src/a.rs", hunks: [[14, 1, ["impl Tr for Kind where Kind: Sized {"], ["impl Tr for Kind {"]]] }] as Parameters<typeof pick>[1];
  expect(own(at(header))).toEqual(["test/h.test.ts", "test/m.test.ts"]);
  // An attribute added above the enum, outside any function, the layer asked
  // for the lines either side of it: what reaches the enum, f's and h's
  // tests; the search by name alone can't place a new attribute line and
  // selects everything (#2288).
  const attribute = [{ file: "src/a.rs", hunks: [[0, 0, ["#[derive(Clone)]"], []]] }] as Parameters<typeof pick>[1];
  expect(own(at(attribute, false))).toBe("all");
  expect(own(at(attribute))).toEqual(["test/f.test.ts", "test/h.test.ts"]);
});

test("the layer's selection in shadow: what it would leave out of what failed, and the push's line", () => {
  const shadow = { all: false, files: ["test/a.test.ts"], cases: ["one.rs"], forms: ["two.rs"] };
  expect(
    shadowMissed(shadow, [
      { file: "test/a.test.ts", tests: ["x"] },
      { file: "test/b.test.ts", tests: ["y"] },
      { file: "test/corpus.test.ts", tests: ["one.rs", "three.rs"] },
      { file: "test/iterator-forms.test.ts", tests: ["two.rs with std's structs", "four.rs with std's structs"] },
    ]),
  ).toEqual(["test/b.test.ts", "test/corpus.test.ts: three.rs", "test/iterator-forms.test.ts: four.rs with std's structs"]);
  expect(shadowMissed({ ...shadow, all: true }, [{ file: "test/b.test.ts", tests: [] }])).toEqual([]);
  expect(shadowSaid({ outcome: "all (x)" })).toBe("all (x)");
  expect(shadowSaid({ outcome: "all (x)", shadows: { items: { outcome: "some (3 files, 2 cases)", missed: [] } } })).toBe("all (x) [items: some (3 files, 2 cases), missed 0]");
  // Each rule in shadow, the loads' beside the layer's (#2482).
  expect(shadowSaid({ outcome: "all (x)", shadows: { items: { outcome: "none" }, loads: { outcome: "some (2 files, 0 cases)", missed: ["test/b.test.ts"] } } })).toBe("all (x) [items: none] [loads: some (2 files, 0 cases), missed 1 (test/b.test.ts)]");
});

// The selection that runs is the map's without its item layer, the layer's
// beside it in shadow, until the switch (#2288); the loads' (#2482); and
// the edges' (#2608).
test("the selection that runs is the map's without its item layer", () => {
  const source = require("node:fs").readFileSync(require("node:path").join(import.meta.dir, "..", "scripts", "select.ts"), "utf8") as string;
  expect(source).toContain("const plain = map === undefined ? undefined : { ...map, items: undefined, edges: undefined };");
  expect(source).toContain("const ran = selectOf(plain, base, sanity, rule, on, given);");
  expect(source).toContain("if (map?.items !== undefined) shadows.items = selectOf({ ...map, edges: undefined }, base, sanity, rule, on, given);");
  expect(source).toContain("if (!on && plain?.loads !== undefined) shadows.loads = selectOf(plain, base, sanity, rule, true, given);");
  // And the edges' (#2608), by a map without the layer.
  expect(source).toContain('if (typeof map?.edges === "string") shadows.edges = selectOf({ ...map, items: undefined }, base, sanity, rule, on, given);');
  // An incremental recording's carry asks the map without its edges too.
  const recording = require("node:fs").readFileSync(require("node:path").join(import.meta.dir, "..", "scripts", "testMap.ts"), "utf8") as string;
  expect(recording).toContain("const selection = fullSelection({ ...from, edges: undefined });");
});
