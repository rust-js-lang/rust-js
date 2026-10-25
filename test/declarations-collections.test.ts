// What a crate's declarations (ADR 0196) say of std's collections: a
// `HashMap` or `BTreeMap` a `Map`, a `HashSet` or `BTreeSet` a `Set`, of
// what each holds, as the JS is one (ADR 0876), and TypeScript that uses
// them is checked against them.

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
