// What the compiler writes of `Vec`, slices, maps, sets and deques: an insert
// at the front, `flat()`, a sort, a B-tree's range, an entry's `or_insert` and
// slices of a whole slice. Each test is a program of its own, compiled and
// run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0345: an entry's `and_modify(f)` is a statement, and `or_insert`
// after it is of the map and the key, as of an entry alone; a `&mut` it
// gives that nothing reads isn't made.
test("an entry's and_modify then or_insert reads its map and key", () => {
  const dir = fixture("entry-and-modify");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/entry_and_modify.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('$andModify($entry(inventory, "apple"), (e) => {');
  expect(js).toContain('$orInsert(inventory, "apple", [0, 0]);');
  expect(js).not.toContain("entry._0");
  expect(js).not.toContain("$mutGet(counts, c);");
});

// An insert at the front, which no length refuses, is `unshift`, as
// react.dev's prepareMDX puts its Overview first.
test("an insert at the front is unshift", async () => {
  const dir = fixture("insert-front");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(mut anchors: Vec<String>) -> Vec<String> {
    if !anchors.is_empty() {
        anchors.insert(0, "Overview".to_string());
    }
    anchors
}
pub fn at(mut v: Vec<u32>, i: usize) -> Vec<u32> {
    v.insert(i, 9);
    v
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('anchors.unshift("Overview");');
  expect(js).toContain("$insertAt(v, i, 9);");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.first(["a"]), lib.first([]), lib.at([1, 2], 1)]).toEqual([["Overview", "a"], [], [1, 9, 2]]);
  expect(() => lib.at([1], 3)).toThrow();
});

// Arrays of arrays flattened are `flat()`, as react.dev's page route gathers
// each directory's Markdown, `files.flat().filter(..)`; of sets, whose items
// `flat()` wouldn't take apart, or in Rust's order, each item stepped.
test("arrays of arrays flattened are flat()", async () => {
  const dir = fixture("flatten-arrays");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::collections::BTreeSet;
pub fn markdown(groups: Vec<Vec<String>>) -> Vec<String> {
    groups.into_iter().flatten().filter(|s| s.ends_with(".md")).collect()
}
pub fn all(groups: Vec<Vec<u32>>) -> Vec<u32> {
    groups.into_iter().flatten().collect()
}
pub fn of_sets(groups: Vec<BTreeSet<u32>>) -> Vec<u32> {
    groups.into_iter().flatten().collect()
}
pub fn logged(groups: Vec<Vec<u32>>) -> (Vec<u32>, Vec<String>) {
    let log = RefCell::new(Vec::new());
    let kept = groups
        .into_iter()
        .flatten()
        .map(|n| {
            log.borrow_mut().push(format!("m{n}"));
            n
        })
        .filter(|n| {
            log.borrow_mut().push(format!("f{n}"));
            *n > 1
        })
        .collect();
    (kept, log.into_inner())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return groups.flat().filter((s) => s.endsWith(".md"));');
  expect(js).toContain("return groups.flat();");
  expect(js).not.toContain("$iter");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.markdown([["a.md", "b"], ["c.md"]]), lib.all([[1], [2, 3]])]).toEqual([["a.md", "c.md"], [1, 2, 3]]);
  expect(lib.logged([[1, 2], [3]])).toEqual([[2, 3], ["m1", "f1", "m2", "f2", "m3", "f3"]]);
  expect(lib.ofSets([new Set([1, 2]), new Set([3])])).toEqual([1, 2, 3]);
});

// `VecDeque::remove` is `$removeOpt`; of items that may look like `None`,
// refused, as its `Some` isn't boxed.
test("a deque's remove gives what was there", async () => {
  const dir = fixture("deque-remove");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;

pub fn removed(items: Vec<u32>, at: usize) -> (Option<u32>, Vec<u32>) {
    let mut d: VecDeque<u32> = items.into_iter().collect();
    let removed = d.remove(at);
    (removed, d.into_iter().collect())
}
`);
  writeFileSync(join(dir, "options.rs"), `use std::collections::VecDeque;

pub fn removed(mut d: VecDeque<Option<u32>>) -> Option<Option<u32>> {
    d.remove(0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.removed([1, 2, 3], 1), lib.removed([1], 5)]).toEqual([[2, [1, 3]], [undefined, [1]]]);
  const refused = Bun.spawnSync([compiler, join(dir, "options.rs"), "-o", join(dir, "options.js")], { stderr: "pipe" });
  expect(refused.exitCode).not.toBe(0);
});

// `&v[..]`, all of a slice, is the slice itself, as `&s[..]` of a string
// is and `&mut v[..]` is (ADR 0335): nothing changes it while it's
// borrowed, so the copy a part is made as isn't needed.
test("a shared slice of all of a slice is the slice", async () => {
  const dir = fixture("whole-slice");
  writeFileSync(join(dir, "lib.rs"), `pub fn take(v: &[&str]) -> usize {
    v.len()
}

pub fn whole(v: &Vec<u32>) -> &[u32] {
    &v[..]
}

pub fn literal() -> usize {
    take(&["a", "b"][..])
}

pub fn part(v: &Vec<u32>) -> Vec<u32> {
    v[1..].to_vec()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function whole(v) {\n  return v;\n}");
  expect(js).toContain('return take(["a", "b"]);');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.whole([1, 2]), lib.literal(), lib.part([1, 2, 3])]).toEqual([[1, 2], 2, [2, 3]]);
});

// `<[T; N]>::try_from(&v[..])` is an array of its own, as Rust's is, which
// a write to `v` after doesn't change; `<&[T; N]>`'s is `v`'s, a borrow.
test("an array made of a whole slice is a copy", async () => {
  const dir = fixture("array-of-slice");
  writeFileSync(join(dir, "lib.rs"), `pub fn owned() -> (u8, u8) {
    let mut bytes = vec![1u8, 2];
    let arr: [u8; 2] = bytes[..].try_into().unwrap();
    bytes[0] = 9;
    (arr[0], bytes[0])
}

pub fn part() -> (u8, u8) {
    let mut bytes = vec![1u8, 2, 3];
    let arr = <[u8; 2]>::try_from(&bytes[1..]).unwrap();
    bytes[1] = 9;
    (arr[0], bytes[1])
}

pub fn borrowed() -> u8 {
    let bytes = vec![1u8, 2];
    let arr = <&[u8; 2]>::try_from(&bytes[..]).unwrap();
    arr[1]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.owned(), lib.part(), lib.borrowed()]).toEqual([[1, 9], [2, 9], 2]);
});

// An unstable sort and a selection from MIR (ADR 0342).
test("an unstable sort from MIR sorts as Rust's", async () => {
  const dir = fixture("mir-unstable-sort");
  writeFileSync(join(dir, "lib.rs"), `pub fn down(mut v: Vec<i32>) -> Vec<i32> {
    v.sort_unstable_by(|a, b| b.cmp(a));
    v
}
pub fn by_len(mut v: Vec<&'static str>) -> Vec<&'static str> {
    v.sort_unstable_by_key(|s| s.len());
    v
}
pub fn third(mut v: Vec<u32>) -> u32 {
    *v.select_nth_unstable(2).1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.down([3, 1, 2]), lib.byLen(["ccc", "a", "bb"]), lib.third([5, 1, 4, 2, 3])]).toEqual([[3, 2, 1], ["a", "bb", "ccc"], 3]);
});

// A slice's own methods from MIR: split by a test, filled, checked sorted,
// searched, cut in chunks.
test("a slice's own methods from MIR run as Rust's", async () => {
  const dir = fixture("mir-slice-methods");
  writeFileSync(join(dir, "lib.rs"), `pub fn parts(v: &[i32]) -> Vec<usize> {
    v.split(|&x| x < 0).map(|p| p.len()).collect()
}
pub fn zeroed(mut v: Vec<i32>) -> Vec<i32> {
    v.fill(0);
    v
}
pub fn facts(v: &[i32]) -> (bool, usize, usize) {
    (v.is_sorted(), v.partition_point(|&x| x < 3), v.chunks_exact(2).count())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.parts([3, -1, 4, 5, -2, 6]), lib.zeroed([1, 2]), lib.facts([1, 2, 3, 4, 5])]).toEqual([[1, 2, 1], [0, 0], [true, 2, 2]]);
});

// A B-tree's range of its keys from MIR (ADR 0325).
test("a B-tree's range from MIR gives the entries in it", async () => {
  const dir = fixture("mir-tree-range");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::BTreeMap;
pub fn from_two(m: &BTreeMap<u32, &'static str>) -> Vec<u32> {
    m.range(2..).map(|(k, _)| *k).collect()
}
pub fn upto(m: &BTreeMap<u32, &'static str>) -> Vec<&'static str> {
    m.range(..=2).map(|(_, v)| *v).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  const m = new Map([[3, "c"], [1, "a"], [2, "b"]]);
  expect([lib.fromTwo(m), lib.upto(m)]).toEqual([[2, 3], ["a", "b"]]);
});

// Sets and maps from MIR made of arrays and read as arrays:
// \`new Set([1, 2, 3])\`, a set's algebra and a sorted map's entries mapped.
test("sets and maps from MIR are made of arrays and read as them", async () => {
  const dir = fixture("mir-sets-maps");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::{BTreeMap, HashSet};
pub fn both() -> Vec<u32> {
    let a: HashSet<u32> = [1, 2, 3].into_iter().collect();
    let b: HashSet<u32> = [2, 3, 4].into_iter().collect();
    let mut both: Vec<u32> = a.intersection(&b).copied().collect();
    both.sort();
    both
}
pub fn keys(m: &BTreeMap<String, u32>) -> Vec<String> {
    m.iter().map(|(k, v)| format!("{k}{v}")).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const a = new Set([1, 2, 3]);\n");
  expect(js).toContain('  const both = $setAlgebra(a, b, "intersection");\n');
  expect(js).toContain("  return $sortedEntries(m, $cmp).map(");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.both(), lib.keys(new Map([["b", 1], ["a", 2]]))]).toEqual([[2, 3], ["a2", "b1"]]);
});

// A count kept in a map, `*m.entry(k).or_insert(0) += 1`, from MIR is the
// value there or put in, then set: no handle on the key's value.
test("a count kept in a map from MIR is its value, then set", async () => {
  const dir = fixture("mir-map-count");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashMap;
pub fn counts(words: &str) -> Vec<(String, u32)> {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for w in words.split(' ') {
        *counts.entry(w).or_insert(0) += 1;
    }
    let mut all: Vec<(String, u32)> = counts.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    all.sort();
    all
}
pub fn bumped(mut m: HashMap<&'static str, u32>) -> u32 {
    *m.entry("a").or_insert(0) += 5;
    m["a"]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    const current = $orInsert(counts, w, 0);\n    counts.set(w, (current + 1) >>> 0);\n");
  expect(js).toContain('  const current = $orInsert(m, "a", 0);\n  m.set("a", (current + 5) >>> 0);\n');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.counts("a b a")).toEqual([["a", 2], ["b", 1]]);
  expect([lib.bumped(new Map()), lib.bumped(new Map([["a", 1]]))]).toEqual([5, 6]);
});

// A `VecDeque`'s iterator from MIR, of the array it is, copied where it's
// collected, `prev.slice()`, as react.dev's Console keeps its latest
// messages.
test("a VecDeque's items collected from MIR are the array's copy", async () => {
  const dir = fixture("deque-collect");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;
pub fn update(f: &dyn Fn(&dyn Fn(&VecDeque<u32>) -> VecDeque<u32>)) {
    f(&|prev| {
        let mut messages: VecDeque<u32> = prev.iter().cloned().collect();
        while messages.len() > 3 {
            messages.pop_front();
        }
        messages
    });
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    const messages = prev.slice();\n");
  const lib = await import(join(dir, "mir.js"));
  const prev = [1, 2, 3, 4, 5];
  let got: number[] = [];
  lib.update((f: (p: number[]) => number[]) => {
    got = f(prev);
  });
  expect([got, prev]).toEqual([[3, 4, 5], [1, 2, 3, 4, 5]]);
});

// Arrays joined, `[first, vec![..]].concat()`, are one array of the first's
// items and the made ones, `[...first, made("a"), ..]`: a call can't change
// the items of an array only a `&mut` changes, as react.dev's
// SandpackWithHTMLOutput adds its files to its children.
test("arrays joined are one array of their items", async () => {
  const dir = fixture("concat-spread");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.made")]
#[allow(unused_variables)]
fn made(name: &str) -> &'static Unknown {
    unreachable!()
}
pub fn all(first: Vec<&'static Unknown>) -> Vec<&'static Unknown> {
    let children = [first, vec![made("a"), made("b")]].concat();
    children
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('const children = [...first, globalThis.made("a"), globalThis.made("b")];');
  const lib = await import(join(dir, "mir.js"));
  (globalThis as any).made = (name: string) => ({ name });
  expect(lib.all([{ name: "x" }])).toEqual([{ name: "x" }, { name: "a" }, { name: "b" }]);
});

// `String::from_iter(items)` from MIR: an array's joined, `["h",
// "i"].join("")`, or what an iterator steps through, joined.
test("a string from items from MIR is them joined", async () => {
  const dir = fixture("string-from-iter");
  writeFileSync(join(dir, "lib.rs"), `pub fn letters() -> String {
    String::from_iter(['h', 'é', '!'])
}
pub fn backwards(s: &str) -> String {
    String::from_iter(s.chars().rev().filter(|c| *c != 'x'))
}
fn of<I: Iterator<Item = char>>(items: I) -> String {
    String::from_iter(items)
}
pub fn counted(n: u32) -> String {
    of((0..n).map(|i| char::from_digit(i, 10).unwrap()))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return ["h", "é", "!"].join("");');
  // A JS iterator has no \`join\` but Bun's: its items, an array's.
  expect(js).toContain('return Iterator.from(items).toArray().join("");');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.letters(), lib.backwards("axbc"), lib.counted(4)]).toEqual(["hé!", "cba", "0123"]);
});

// A chunks' `remainder()` from MIR: of one made there, its array's,
// `$chunksExact(v, 2).remainder`; of one stepped, which MIR keeps as a
// `$iter`, its items' still.
test("a chunks' remainder from MIR, made there or stepped", async () => {
  const dir = fixture("chunks-remainder");
  writeFileSync(join(dir, "lib.rs"), `pub fn rest(v: &[u32]) -> Vec<u32> {
    v.chunks_exact(2).remainder().to_vec()
}
pub fn stepped(v: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let mut c = v.chunks_exact(2);
    let first = c.next().map(|s| s.to_vec()).unwrap_or_default();
    (first, c.remainder().to_vec())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("$chunksExact(v, 2).remainder");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.rest([1, 2, 3]), lib.stepped([1, 2, 3, 4, 5])]).toEqual([[3], [[1, 2], [5]]]);
});

// A `Vec`'s range pushed again, `extend_from_within`, or replaced,
// `splice`, all of it too: `..` reads the `Vec` itself, no range helper's
// call.
test("a vec's range pushed again or replaced, all of it too", async () => {
  const dir = fixture("vec-range-edits");
  writeFileSync(join(dir, "lib.rs"), `pub fn doubled(mut v: Vec<u32>) -> Vec<u32> {
    v.extend_from_within(..);
    v
}
pub fn tail_again(mut v: Vec<u32>) -> Vec<u32> {
    v.extend_from_within(1..);
    v
}
pub fn replaced(mut v: Vec<u32>) -> (Vec<u32>, Vec<u32>) {
    let old: Vec<u32> = v.splice(.., [9, 8]).collect();
    (v, old)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.doubled([1, 2]), lib.tailAgain([1, 2, 3]), lib.replaced([1, 2, 3])]).toEqual([[1, 2, 1, 2], [1, 2, 3, 2, 3], [[9, 8], [1, 2, 3]]]);
});

// React's addObjectDiffToProperties pushes two rows at once,
// `properties.push([REMOVED + line, desc], [ADDED + line, desc])`: a
// `Vec`'s `extend` of an array written in place is JS's own `push` of its
// items, made first, in order, as the array's are (ADR 0734). Of anything
// else, of a set or a deque, it's what it was.
test("extend of an array written in place is push of its items", async () => {
  const dir = fixture("extend-written");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::{HashSet, VecDeque};

pub fn rows(properties: &mut Vec<(String, String)>, line: &str, desc: &str) {
    properties.extend([
        ("-".to_string() + line, desc.to_string()),
        ("+".to_string() + line, desc.to_string()),
    ]);
}

pub fn more(xs: &mut Vec<u32>, ys: &[u32], seen: &mut HashSet<u32>) {
    xs.extend(ys.iter().copied());
    seen.extend([1, 2]);
}

pub fn queued(queue: &mut VecDeque<u32>) {
    queue.extend([1, 2]);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  properties.push(["-" + line, desc], ["+" + line, desc]);');
  expect(js).toContain("$extend(xs, ys)");
  expect(js).toContain("$extendMap(seen, [1, 2], true)");
  expect(js).toContain("$extend(queue, [1, 2])");
  const lib = await import(join(dir, "lib.js"));
  const rows: string[][] = [["a", "b"]];
  lib.rows(rows, "k", "v");
  const xs = [1];
  const seen = new Set([2]);
  lib.more(xs, [5, 6], seen);
  expect([rows, xs, [...seen]]).toEqual([[["a", "b"], ["-k", "v"], ["+k", "v"]], [1, 5, 6], [2, 1]]);
});

// ADR 0864: React's `Transition` keeps the fibers it updated in a
// `Set<Fiber>`, each by itself. A type whose `PartialEq` is `ptr::eq` and
// whose `Hash` is `ptr::hash` is equal only to itself, as JS's `===` of an
// object is, so a set of it is JS's `Set`, keyed by the object, and `==` of
// two is `===`, of the values as of references to them. Nothing reads its
// `PartialEq`'s dictionary then, which isn't written (ADR 0864, amended),
// but where generic code is given it. One whose `ne` is its own, or whose
// `eq` compares another's address, isn't.
test("a type equal only to itself keys a JS Set by itself", () => {
  const source = (eq: string, ne: string, more = "") => `use std::collections::HashSet;
use std::hash::{Hash, Hasher};

pub struct Fiber {
    pub tag: u32,
}

impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        ${eq}
    }${ne}
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

pub fn count(fibers: &[&'static Fiber]) -> usize {
    let set: HashSet<&'static Fiber> = fibers.iter().copied().collect();
    set.len()
}

pub fn same(a: &Fiber, b: &Fiber) -> bool {
    a == b
}

pub fn differ(a: Fiber, b: Fiber) -> bool {
    a != b
}
${more}`;
  const compile = (name: string, eq: string, ne = "", more = "") => {
    const dir = fixture(name);
    writeFileSync(join(dir, "lib.rs"), source(eq, ne, more));
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
    return readFileSync(join(dir, "lib.js"), "utf8");
  };
  const js = compile("identity-set", "std::ptr::eq(self, other)");
  expect(js).toContain("const set = new Set(fibers);");
  expect(js).toContain("  return a === b;\n");
  expect(js).toContain("  return a !== b;\n");
  expect(js).not.toContain("fiberPartialEq");
  // Given to generic code, its dictionary is what it's given.
  const generic = compile("identity-set-generic", "std::ptr::eq(self, other)", "", `
fn equal<T: PartialEq>(a: &T, b: &T) -> bool {
    a == b
}

pub fn equal_fibers(a: &Fiber, b: &Fiber) -> bool {
    equal(a, b)
}
`);
  expect(generic).toContain("export const fiberPartialEq = { eq: fiberPartialEq_eq };");
  expect(generic).toContain("  return equal(a, b, fiberPartialEq);\n");
  expect(() => compile("identity-set-ne", "std::ptr::eq(self, other)", "\n\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self, other: &Self) -> bool {\n        self.tag != other.tag\n    }"))
    .toThrow("values of type");
  expect(() => compile("identity-set-other", "std::ptr::eq(self, self) && self.tag == other.tag")).toThrow("values of type");
  expect(() => compile("identity-set-self", "std::ptr::eq(self, self)")).toThrow("values of type");
});

// ADR 0033: a `&mut` of an `Option` is a handle on its place, `Some(x)`
// being `x`, never an object changed in place. React's act captures its
// `result` in a closure by `&mut`; that made every `Option<&Unknown>` in
// the crate one changed in place, and ReactJSXElement's
// `children.to_vec()` a copy of each item, `children.map((item) =>
// item)`, where it's `children.slice()`. What a `Some` holds that's
// changed through it is still copied.
test("an option written through a &mut is no object changed in place", async () => {
  const dir = fixture("option-mut-not-object");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone)]
pub struct Point {
    pub x: u32,
}

pub fn copied(xs: &[Option<&'static str>]) -> Vec<Option<&'static str>> {
    let mut last = None;
    let mut set = || last = Some("x");
    set();
    let mut v = xs.to_vec();
    v.push(last);
    v
}

pub fn nested() -> String {
    let mut a: Vec<Option<Option<u32>>> = vec![Some(Some(1))];
    let b = a.clone();
    if let Some(inner) = a[0].as_mut() {
        *inner = None;
    }
    format!("{a:?} {b:?}")
}

pub fn changed() -> (u32, u32) {
    let mut a = vec![Some(Point { x: 1 })];
    let b = a.clone();
    if let Some(p) = a[0].as_mut() {
        p.x = 2;
    }
    (a[0].as_ref().map_or(0, |p| p.x), b[0].as_ref().map_or(0, |p| p.x))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const v = xs.slice();");
  expect(js).toContain("const b = a.slice();");
  expect(js).not.toContain("(item) => item)");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.copied(["a", undefined]), lib.nested(), lib.changed()]).toEqual([["a", undefined, "x"], "[Some(None)] [Some(Some(1))]", [2, 1]]);
});

// ADR 0881: an item's place an index gives, written at once, `v[i] = x`,
// reads its index where it's written, nothing run between to change it;
// one held while its index changes is of the index it had.
test("an item's place held while its index changes is of the index it had", async () => {
  const dir = fixture("held-item-place");
  writeFileSync(join(dir, "lib.rs"), `pub fn set_at(v: &mut Vec<u32>, mut i: usize, x: u32) -> usize {
    v[i] = x;
    i += 1;
    i
}

pub fn held(v: &mut Vec<u32>, mut i: usize) -> usize {
    let item = &mut v[i];
    i += 1;
    *item = 5;
    i
}

pub fn written_twice(v: &mut Vec<u32>, mut i: usize) -> usize {
    let item = &mut v[i];
    *item = 5;
    i += 1;
    *item += 1;
    i
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function setAt(v, i, x) {\n  v[$at(v, i)] = x;\n");
  const lib = await import(join(dir, "lib.js"));
  const v = [0, 0, 0];
  expect([lib.setAt(v, 0, 7), lib.held(v, 1), lib.writtenTwice(v, 2), v]).toEqual([1, 2, 3, [7, 5, 6]]);
});

// ADR 0285, amended: `ptr::eq` of an item and a reference is `items[i] ===
// r` of an object alone. Of a type parameter's item, which may be a
// number, it's by the handles, whose size it doesn't know, refused, never
// `1 === 1` of two places.
test("ptr::eq of a type parameter's item is by its place", () => {
  const dir = fixture("ptr-eq-generic-item");
  writeFileSync(join(dir, "lib.rs"), `fn is_first<T>(items: &[T], item: &T) -> bool {
    std::ptr::eq(&items[0], item)
}

pub fn second_number() -> bool {
    let numbers = vec![1u32, 1];
    is_first(&numbers, &numbers[1])
}
`);
  expect(() => run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")])).toThrow(
    "rust-js does not support `size_of` of a type parameter yet",
  );
});

// A table made as long as another, `vec![0.0; terraces.len()]`, or as a
// count, `vec![0; n]`, and never resized, is indexed as the other is
// (#362): javascript-algorithms' `dpRainTerraces` reads `leftMaxLevels[i]`
// as upstream does. One resized, or whose other changes, keeps its check.
test("a table as long as another is indexed as it is", async () => {
  const dir = fixture("same-length");
  writeFileSync(join(dir, "lib.rs"), `pub fn running_max(terraces: &[f64]) -> Vec<f64> {
    let mut levels = vec![0.0; terraces.len()];
    for i in 1..terraces.len() {
        levels[i] = terraces[i].max(levels[i - 1]);
    }
    levels
}

pub fn squares(n: u32) -> Vec<u32> {
    let count = n as usize;
    let mut table = vec![0; count];
    let mut counted = vec![0; count];
    for i in 0..count {
        counted[i] = (i * i) as u32;
        table[i] = counted[i];
    }
    table
}

pub fn pushed(terraces: &[f64]) -> f64 {
    let mut levels = vec![0.0; terraces.len()];
    levels.pop();
    let mut sum = 0.0;
    for i in 0..terraces.len() {
        sum += levels[i];
    }
    sum
}

pub fn grown(mut terraces: Vec<f64>) -> f64 {
    let levels = vec![0.0; terraces.len()];
    terraces.push(1.0);
    let mut sum = 0.0;
    for i in 0..terraces.len() {
        sum += levels[i];
    }
    sum
}

pub fn branched(terraces: &[f64], keep: bool) -> f64 {
    let mut levels = vec![0.0; terraces.len()];
    let mut sum = 0.0;
    if keep {
        sum = 1.0;
    } else {
        levels.pop();
    }
    for i in 0..terraces.len() {
        sum += levels[i];
    }
    sum
}

pub fn celled(terraces: &std::cell::RefCell<Vec<f64>>, change: fn(&std::cell::RefCell<Vec<f64>>)) -> f64 {
    let levels = vec![0.0; terraces.borrow().len()];
    change(terraces);
    let mut sum = 0.0;
    for i in 0..terraces.borrow().len() {
        sum += levels[i];
    }
    sum
}

pub fn staircase(stairs: u16) -> f64 {
    let n = stairs as usize;
    let mut steps = vec![0.0; n.max(2) + 1];
    steps[1] = 1.0;
    steps[2] = 2.0;
    if n <= 2 {
        return steps[n];
    }
    steps[0] + steps[n - 1]
}

pub fn last_at_most(xs: &[u32], i: usize) -> u32 {
    if xs.is_empty() {
        return 0;
    }
    xs[i.min(xs.len() - 1)]
}

pub fn larger_of_two(xs: &[u32]) -> u32 {
    let mut sum = 0;
    for i in 0..xs.len() {
        for j in 0..xs.len() {
            sum += xs[i.max(j)];
        }
    }
    sum
}

pub fn unsized_table(n: usize) -> f64 {
    let mut steps = vec![0.0; n];
    steps[0] = 1.0;
    steps[0]
}

pub fn recounted(mut count: usize) -> u32 {
    let table = vec![1; count];
    count += 1;
    let mut sum = 0;
    for i in 0..count {
        sum += table[i];
    }
    sum
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("levels[i] = $f64Max(terraces[i], levels[i - 1]);");
  expect(js).toContain("counted[i] = Math.imul(i, i) >>> 0;");
  expect(js).toContain("table[i] = counted[i];");
  for (const checked of ["$index(levels, i)", "$index(table, i)"]) expect(js).toContain(checked);
  // A table at least as long as its count's least (#367).
  for (const write of ["steps[1] = 1;", "steps[2] = 2;", "return steps[n];", "steps[0] + $index(steps, n - 1)"]) expect(js).toContain(write);
  expect(js).toContain("steps[$at(steps, 0)] = 1;");
  // An index's `min` and `max`, below what either is below (#367).
  for (const read of ["return xs[Math.min(i, xs.length - 1)];", "sum = (sum + xs[Math.max(i, j)]) >>> 0;"]) expect(js).toContain(read);
  expect(js.match(/\$index\(/g)?.length).toBe(7);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.runningMax([1, 3, 2]), lib.squares(3)]).toEqual([[0, 3, 3], [0, 1, 4]]);
  expect([lib.staircase(1), lib.staircase(4), lib.unsizedTable(2)]).toEqual([1, 0, 1]);
  expect([lib.lastAtMost([4, 5], 7), lib.lastAtMost([4, 5], 0), lib.largerOfTwo([1, 2])]).toEqual([5, 4, 7]);
  expect(() => lib.unsizedTable(0)).toThrow();
  const outsides = [() => lib.pushed([1, 2]), () => lib.grown([1, 2]), () => lib.recounted(2), () => lib.branched([1, 2], false)];
  for (const outside of [...outsides, () => lib.celled([1, 2], (cell: number[]) => cell.push(3))]) {
    expect(outside).toThrow();
  }
});

// An `enumerate()` of a slice's items gives its index below the slice's
// length, which the `iter()` holds borrowed (#363): `levels[i]` of `for (i,
// &t) in terraces.iter().enumerate()`, as `for i in 0..terraces.len()`
// gives. Another slice's index, or an adapter's between, keeps its check.
test("an enumerate's index is below its slice's length", async () => {
  const dir = fixture("enumerate-bounds");
  writeFileSync(join(dir, "lib.rs"), `pub fn water(terraces: &[f64]) -> f64 {
    let levels = vec![1.0; terraces.len()];
    let mut sum = 0.0;
    for (i, &terrace) in terraces.iter().enumerate() {
        sum += levels[i] - terrace + terraces[i];
    }
    sum
}

pub fn backwards(items: &mut [u32]) {
    for (i, item) in items.iter_mut().enumerate().rev() {
        *item += i as u32;
    }
}

pub fn other(xs: &[u32], ys: &[u32]) -> u32 {
    let mut sum = 0;
    for (i, _) in ys.iter().enumerate() {
        sum += xs[i];
    }
    sum
}

pub fn rebound(a: &[u32], b: &[u32]) -> u32 {
    let mut xs = a;
    let mut sum = 0;
    for (i, _) in xs.iter().enumerate() {
        sum += xs[i];
        xs = b;
    }
    sum
}

fn doubled(xs: &[u32]) -> impl Iterator<Item = u32> + '_ {
    xs.iter().chain(xs.iter()).copied()
}

pub fn twice(xs: &[u32]) -> u32 {
    let mut sum = 0;
    for (i, _) in doubled(xs).enumerate() {
        sum += xs[i];
    }
    sum
}

pub fn skipped(xs: &[u32]) -> u32 {
    let mut sum = 0;
    for (i, _) in xs.iter().skip(1).enumerate() {
        sum += xs[i + 1];
    }
    sum
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("sum += levels[i] - terrace + terraces[i];");
  expect(js).toContain("$index(xs, i)");
  expect(js.match(/\$index\(/g)?.length).toBe(4);
  const lib = await import(join(dir, "lib.js"));
  const items = [5, 5, 5];
  lib.backwards(items);
  expect([lib.water([1, 2]), items, lib.skipped([1, 2, 3])]).toEqual([2, [5, 6, 7], 5]);
  for (const outside of [() => lib.other([1], [1, 2]), () => lib.rebound([1, 2], [3]), () => lib.twice([1])]) {
    expect(outside).toThrow();
  }
});

// A table of rows, `vec![vec![1.0; width]; height]`, has rows each as long
// as `width` (#370): `board[r][c]` of `c < width` reads as upstream's
// `dpUniquePaths` does. A row changed, through a `&mut` lent or held, an
// assignment, `iter_mut()`, or a new row, keeps the check.
test("a table's rows are as long as its row count", async () => {
  const dir = fixture("table-rows");
  writeFileSync(join(dir, "lib.rs"), `pub fn paths(width: usize, height: usize) -> f64 {
    let mut board = vec![vec![1.0; width]; height];
    for r in 1..height {
        for c in 1..width {
            board[r][c] = board[r - 1][c] + board[r][c - 1];
        }
    }
    board[height - 1][width - 1]
}

fn total(board: &[Vec<f64>], width: usize) -> f64 {
    let mut sum = 0.0;
    for row in board {
        for c in 0..width {
            sum += row[c];
        }
    }
    sum
}

pub fn popped(width: usize, height: usize) -> f64 {
    let mut board = vec![vec![1.0; width]; height];
    board[0].pop();
    let mut sum = 0.0;
    for r in 0..height {
        for c in 0..width {
            sum += board[r][c];
        }
    }
    sum + total(&board, 0)
}

pub fn replaced(width: usize, height: usize) -> f64 {
    let mut board = vec![vec![1.0; width]; height];
    board[0] = Vec::new();
    let mut sum = 0.0;
    for r in 0..height {
        for c in 0..width {
            sum += board[r][c];
        }
    }
    sum
}

pub fn held(width: usize, height: usize) -> f64 {
    let mut board = vec![vec![1.0; width]; height];
    let row = &mut board[0];
    row.pop();
    let mut sum = 0.0;
    for r in 0..height {
        for c in 0..width {
            sum += board[r][c];
        }
    }
    sum
}

pub fn added(width: usize, height: usize) -> f64 {
    let mut board = vec![vec![1.0; width]; height];
    board.push(Vec::new());
    let mut sum = 0.0;
    for r in 0..=height {
        for c in 0..width {
            sum += board[r][c];
        }
    }
    sum
}

pub fn cleared(height: usize) -> f64 {
    let mut board = vec![vec![1.0; 2]; height];
    let first = board[0][1];
    board[0].clear();
    first + board[0][1]
}

pub fn each(width: usize, height: usize) -> f64 {
    let mut board = vec![vec![1.0; width]; height];
    for row in board.iter_mut() {
        row.pop();
    }
    let mut sum = 0.0;
    for r in 0..height {
        for c in 0..width {
            sum += board[r][c];
        }
    }
    sum
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("board[r][c] = board[r - 1][c] + board[r][c - 1];");
  expect(js.match(/\$index\(board\[r\], c\)/g)?.length).toBe(4);
  expect(js).toContain("sum += $index($index(board, r), c);");
  // Each row at least 2 long, of a count of 2, till one's cleared.
  expect(js).toContain("const first = $index(board, 0)[1];");
  expect(js).toContain("return first + $index($index(board, 0), 1);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.paths(3, 3)).toBe(6);
  for (const changed of [() => lib.popped(2, 2), () => lib.replaced(2, 2), () => lib.held(2, 2), () => lib.each(2, 2), () => lib.added(2, 2), () => lib.cleared(1)]) {
    expect(changed).toThrow();
  }
});
