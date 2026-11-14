// What a crate's declarations (ADR 0196) say of std's collections: a
// `HashMap` or `BTreeMap` a `Map`, a `HashSet` or `BTreeSet` a `Set`, of
// what each holds, as the JS is one (ADR 0876); and of a `Cow`, the tagged
// union of what it borrowed or owns (#1748). TypeScript that uses them is
// checked against them.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, root, run } from "./support";

/** TypeScript's errors in `file` of `dir`, strict, its declarations checked too. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, "tsconfig.json");
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, typeRoots: [join(root, "node_modules/@types")] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

// ADR 0876: React's ReactTaint keeps `TaintRegistryByteLengths`, a
// `Set<number>`, as an `Rc<RefCell<HashSet<u32>>>` the JS is `new Set()`
// of, and a set of objects by themselves, `HashSet<&'static Fiber>`.
test("a HashMap is declared a Map and a HashSet a Set", () => {
  const dir = fixture("declarations-collections");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub struct Fiber {
    pub tag: u32,
}

impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

thread_local! {
    pub static BYTE_LENGTHS: Rc<RefCell<HashSet<u32>>> = Rc::new(RefCell::new(HashSet::new()));
    pub static FIBERS: RefCell<HashSet<&'static Fiber>> = RefCell::new(HashSet::new());
}

pub fn ages() -> HashMap<String, u32> {
    HashMap::from([("Ada".to_string(), 36)])
}

pub fn ranks() -> BTreeMap<u32, Vec<String>> {
    BTreeMap::from([(1, vec!["gold".to_string()])])
}

pub fn points() -> HashSet<Point> {
    HashSet::from([Point { x: 1, y: 2 }])
}

pub fn count(tags: &BTreeSet<String>) -> usize {
    tags.len()
}

pub fn sum(lengths: &HashSet<u32>) -> u32 {
    lengths.iter().sum()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export const byteLengths: Set<number>;");
  expect(declarations).toContain("export const fibers: Set<Fiber>;");
  expect(declarations).toContain("export function ages(): Map<string, number>;");
  expect(declarations).toContain("export function ranks(): Map<number, string[]>;");
  expect(declarations).toContain("export function points(): Set<Point>;");
  expect(declarations).toContain("export function count(tags: Set<string>): number;");
  expect(declarations).toContain("export function sum(lengths: Set<number>): number;");
  writeFileSync(join(dir, "use.ts"), `import { ages, byteLengths, count, ranks, sum } from "./lib.js";
byteLengths.add(4);
export const total: number = sum(byteLengths) + count(new Set(["a"]));
export const age: number | undefined = ages().get("Ada");
export const first: string[] | undefined = ranks().get(1);
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
});

// #1748, found porting a WebGPU page: a function giving back a buffer as it
// was given, or one it made, `Cow<'_, [f32; 16]>`, was `any` where its JS
// is `{ TAG: "Borrowed", _0: value }`. Declared as each variant of an enum
// is (ADR 0033), its `Owned` what `ToOwned` makes of what it borrows.
test("a Cow is declared the tagged union of what it borrowed or owns", async () => {
  const dir = fixture("declarations-cow");
  writeFileSync(join(dir, "lib.rs"), `use std::borrow::Cow;

pub fn text(value: &str) -> Cow<'_, str> {
    Cow::Borrowed(value)
}

pub fn numbers(value: &[f32; 16]) -> Cow<'_, [f32; 16]> {
    Cow::Borrowed(value)
}

pub fn shouted(value: &str) -> Cow<'_, str> {
    if value.ends_with('!') {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(format!("{value}!"))
    }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  const cow = (borrowed: string, owned: string) =>
    `{\n    TAG: "Borrowed";\n    _0: ${borrowed};\n} | {\n    TAG: "Owned";\n    _0: ${owned};\n};`;
  expect(declarations).toContain(`export function text(value: string): ${cow("string", "string")}`);
  expect(declarations).toContain(`export function numbers(value: number[]): ${cow("number[]", "number[]")}`);
  expect(declarations).toContain(`export function shouted(value: string): ${cow("string", "string")}`);
  writeFileSync(join(dir, "use.ts"), `import { numbers, shouted } from "./lib.js";
const said = shouted("hi");
export const owned: boolean = said.TAG === "Owned";
export const shown: string = said._0;
export const first: number | undefined = numbers(new Array<number>(16).fill(1))._0[0];
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { shouted } from "./lib.js";
export const wrong: number = shouted("hi")._0;
`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([expect.stringContaining("error TS2322")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shouted("hi"), lib.shouted("hi!")]).toEqual([{ TAG: "Owned", _0: "hi!" }, { TAG: "Borrowed", _0: "hi!" }]);
});
