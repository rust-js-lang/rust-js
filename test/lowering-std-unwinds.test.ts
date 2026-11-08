// What std's own methods keep as a drop or a clone in them panics (#899,
// #904, #906, #919): a map's `clear()` empties it as a drop unwinds, an
// `Option`'s `insert` writes its value, `dedup_by` drops each duplicate as
// it's found, of what a `&mut` to is a cell too, and a `Vec`'s
// `clone_from` writes over what it keeps in place. Where nothing in them
// may panic, or nothing can be seen, the form each always had.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS, its lines joined. */
function compiled(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  return readFileSync(join(dir, "lib.js"), "utf8").replace(/\s+/g, " ");
}

/** The body of `flat`'s exported function `name`. */
function body(flat: string, name: string): string {
  return flat.slice(flat.indexOf(`export function ${name}(`)).split("export function")[1];
}

// `Loud`'s drop may panic; `Quiet`'s only writes its field, which never unwinds
// (ADR 0902).
const types = `
#[derive(Clone)]
pub struct Quiet(pub u32);

impl Drop for Quiet {
    fn drop(&mut self) {
        self.0 = 0;
    }
}

pub struct Loud(pub u32);

impl Clone for Loud {
    fn clone(&self) -> Loud {
        println!("clone {}", self.0);
        Loud(self.0)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        if self.0 == 0 {
            panic!("drop");
        }
    }
}
`;

test("a map's clear() empties it as an entry's drop unwinds, where one may", () => {
  const flat = compiled(
    "std-unwinds-clear",
    `${types}
use std::collections::BTreeMap;

pub fn quiet(m: &mut BTreeMap<u32, Quiet>) {
    m.clear();
}

pub fn loud(m: &mut BTreeMap<u32, Loud>) {
    m.clear();
}
`,
  );
  expect(body(flat, "quiet")).not.toContain("finally");
  expect(body(flat, "quiet")).toContain("} m.clear(); }");
  expect(body(flat, "loud")).toMatch(/try \{ \$dropItems\(.*\} finally \{ m\.clear\(\); \}/);
});

test("Option::insert writes its value as the old one's drop unwinds, where it may", () => {
  const flat = compiled(
    "std-unwinds-insert",
    `${types}
pub fn quiet(o: &mut Option<Quiet>, q: Quiet) -> u32 {
    o.insert(q).0
}

pub fn loud(o: &mut Option<Loud>, l: Loud) -> u32 {
    o.insert(l).0
}
`,
  );
  expect(body(flat, "quiet")).not.toContain("finally");
  expect(body(flat, "quiet")).toContain("o.value = q;");
  expect(body(flat, "loud")).toMatch(/try \{ if \(o\.value != null\) \{ loudDrop_drop\(o\.value\); \} \} finally \{ o\.value = l; \}/);
});

test("dedup_by gives a closure each item as a `&mut` to it is, dropping each duplicate it finds", () => {
  const flat = compiled(
    "std-unwinds-dedup",
    `${types}
use std::rc::Rc;

pub fn objects(v: &mut Vec<Loud>) {
    v.dedup_by(|a, b| a.0 == b.0);
}

pub fn counted(v: &mut Vec<Rc<Loud>>) {
    v.dedup_by(|a, b| a.0 == b.0);
}

pub fn boxed(v: &mut Vec<Box<Loud>>) {
    v.dedup_by_key(|x| x.0);
}
`,
  );
  expect(body(flat, "objects")).toContain("$dedupBy(v, (a, b) => a[0] === b[0], loudDrop_drop)");
  // An \`Rc\`'s \`&mut\` is a cell: \`$dedupByCells\`, given the drop too.
  expect(body(flat, "counted")).toMatch(/\$dedupByCells\( v, \(a, b\) => a\.value\.value\[0\] === b\.value\.value\[0\], \(rc\) => \{ \$rcDrop\(rc, loudDrop_drop\); \},? \)/);
  expect(body(flat, "boxed")).toMatch(/\$dedupByCells\(.*loudDrop_drop\)/);
});

test("a Vec's clone_from writes over what it keeps in place, where its order can be seen", () => {
  const flat = compiled(
    "std-unwinds-clone-from",
    `${types}
pub fn texts(w: &mut Vec<String>, v: &Vec<String>) {
    w.clone_from(v);
}

pub fn loud(w: &mut Vec<Loud>, v: &Vec<Loud>) {
    w.clone_from(v);
}

pub fn nested(w: &mut [Vec<Loud>], v: Vec<Loud>) {
    w.fill(v);
}
`,
  );
  expect(body(flat, "texts")).not.toContain("$cloneIntoVec");
  expect(body(flat, "loud")).toContain("$cloneIntoVec(w, v, (item) => loudClone_clone(item), undefined, loudDrop_drop);");
  expect(body(flat, "nested")).toContain("(items, i, from) => { $cloneIntoVec(items[i], from, (item) => loudClone_clone(item), undefined, loudDrop_drop); }");
});

// #928: generic code's `clone_from` asks the caller's `Clone` dictionary
// for its `cloneFrom` first, of a `T` and of an `Option<T>`'s value, and is
// `*dest = source.clone()` where it has none. A std type's dictionary has
// one only where std's is in place and seen, a `Vec<Loud>`'s, not a
// `Vec<String>`'s; concrete code is as it was.
test("a generic clone_from asks its dictionary's cloneFrom first, which a std type's has where std's is in place", () => {
  const flat = compiled(
    "std-unwinds-generic-clone-from",
    `${types}
pub fn assign<T: Clone>(dest: &mut T, source: &T) {
    dest.clone_from(source);
}

pub fn assign_option<T: Clone>(dest: &mut Option<T>, source: &Option<T>) {
    dest.clone_from(source);
}

pub fn loud(w: &mut Vec<Loud>, v: &Vec<Loud>) {
    assign(w, v);
}

pub fn texts(w: &mut Vec<String>, v: &Vec<String>) {
    assign(w, v);
}

pub fn loud_option(w: &mut Option<Vec<Loud>>, v: &Option<Vec<Loud>>) {
    assign_option(w, v);
}

/// Given no caller's dictionary: its T is a value JS gives.
pub fn alone<T: Clone>(dest: &mut Option<T>, source: &Option<T>) {
    dest.clone_from(source);
}

pub fn concrete(w: &mut Option<Vec<Loud>>, v: &Option<Vec<Loud>>) {
    w.clone_from(v);
}
`,
  );
  expect(body(flat, "assign")).toContain(
    "if (TClone.cloneFrom) { TClone.cloneFrom(dest, source); } else { const old = $apart(dest.value); dest.value = TClone.clone(source); dropT?.(old); }",
  );
  expect(body(flat, "assignOption")).toContain(
    "if (dest.value != null && source != null && TClone.cloneFrom) { TClone.cloneFrom(dest, source); } else {",
  );
  expect(body(flat, "loud")).toContain("cloneFrom(to, from) { $cloneIntoVec(to.value, from, (item) => loudClone_clone(item), undefined, loudDrop_drop); }");
  expect(body(flat, "texts")).not.toContain("cloneFrom");
  expect(body(flat, "alone")).not.toContain("cloneFrom");
  expect(body(flat, "concrete")).toContain("if (w.value != null && v != null) { $cloneIntoVec(w.value, v,");
  expect(body(flat, "concrete")).not.toContain("cloneFrom");
});

// #912: `max`, `min` and `clamp` of a type with a destructor drop what they
// don't give back, and all they were given as a comparison panics, by the
// runtime's `$maxOf`, `$minOf` and `$clampBy`'s `drop`; of one with
// nothing to drop, the conditional it always was, in std's order, `other <
// self`.
test("max, min and clamp drop what they don't give back, and compare as std's do", () => {
  const flat = compiled(
    "std-unwinds-max",
    `${types}
use std::cmp::Ordering;

#[derive(PartialEq, Eq)]
pub struct Rank(pub u32);

impl PartialOrd for Rank {
    fn partial_cmp(&self, other: &Rank) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rank {
    fn cmp(&self, other: &Rank) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Held(pub Rank, pub Vec<u32>);

impl Drop for Held {
    fn drop(&mut self) {}
}

pub fn ranks(a: Rank, b: Rank) -> u32 {
    std::cmp::max(a, b).0
}

pub fn least(a: Rank, b: Rank) -> u32 {
    a.min(b).0
}

pub fn held(a: Held, b: Held) -> u32 {
    std::cmp::max(a, b).0.0
}

pub fn held_least(a: Held, b: Held) -> u32 {
    a.min(b).0.0
}

pub fn clamped(x: Held, lo: Held, hi: Held) -> u32 {
    x.clamp(lo, hi).0.0
}
`,
  );
  expect(body(flat, "ranks")).toContain("rankOrd_cmp(b, a) < 0 ? a : b");
  expect(body(flat, "least")).toContain("rankOrd_cmp(b, a) < 0 ? b : a");
  expect(body(flat, "held")).toMatch(/\$maxOf\( ?a, b, .*, heldDrop_drop,? \)/);
  expect(body(flat, "heldLeast")).toMatch(/\$minOf\( ?a, b, .*, heldDrop_drop,? \)/);
  expect(body(flat, "clamped")).toMatch(/\$clampBy\( ?x, lo, hi, .*, undefined, heldDrop_drop,? ?\)/);
});

// #901: a heap's `pop`, `into_sorted_vec` and `from` are given the items'
// drop where their comparison may panic, to drop what std's frame holds as
// it unwinds; of an order that never panics, the call it always was.
test("a heap's pop drops the top it took as a comparison unwinds, where one may", () => {
  const flat = compiled(
    "std-unwinds-heap",
    `
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Quiet(pub u32);

impl Drop for Quiet {
    fn drop(&mut self) {
        self.0 = 0;
    }
}

#[derive(PartialEq, Eq)]
pub struct Loud(pub u32);

impl PartialOrd for Loud {
    fn partial_cmp(&self, other: &Loud) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Loud {
    fn cmp(&self, other: &Loud) -> Ordering {
        println!("cmp");
        self.0.cmp(&other.0)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        self.0 = 0;
    }
}

pub fn quiet(h: &mut BinaryHeap<Quiet>) -> u32 {
    h.pop().map_or(0, |q| q.0)
}

pub fn loud(h: &mut BinaryHeap<Loud>) -> u32 {
    h.pop().map_or(0, |q| q.0)
}

pub fn sorted(h: BinaryHeap<Loud>) -> usize {
    h.into_sorted_vec().len()
}
`,
  );
  expect(body(flat, "quiet")).toMatch(/\$heapPop\(h, \(a, b\) => \$cmp\(a\[0\], b\[0\]\)\)/);
  expect(body(flat, "loud")).toContain("$heapPop(h, loudOrd_cmp, false, loudDrop_drop)");
  expect(body(flat, "sorted")).toContain("$heapSorted(h, loudOrd_cmp, loudDrop_drop)");
});

// #911: a B-tree's `insert` of keys whose order may panic is given the
// value's drop too, and a set's `replace` the item's, each dropped as the
// search unwinds; of an order that never panics, the call it always was.
test("a B-tree's insert drops what it was given as its search unwinds, where it may", () => {
  const flat = compiled(
    "std-unwinds-btree",
    `
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Quiet(pub u32);

impl Drop for Quiet {
    fn drop(&mut self) {
        self.0 = 0;
    }
}

#[derive(PartialEq, Eq)]
pub struct Loud(pub u32);

impl PartialOrd for Loud {
    fn partial_cmp(&self, other: &Loud) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Loud {
    fn cmp(&self, other: &Loud) -> Ordering {
        println!("cmp");
        self.0.cmp(&other.0)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        self.0 = 0;
    }
}

pub fn quiet(m: &mut BTreeMap<Quiet, Quiet>, k: Quiet, v: Quiet) -> bool {
    m.insert(k, v).is_some()
}

pub fn loud(m: &mut BTreeMap<Loud, Quiet>, k: Loud, v: Quiet) -> bool {
    m.insert(k, v).is_some()
}

pub fn quiet_replace(s: &mut BTreeSet<Quiet>, x: Quiet) -> bool {
    s.replace(x).is_some()
}

pub fn loud_replace(s: &mut BTreeSet<Loud>, x: Loud) -> bool {
    s.replace(x).is_some()
}
`,
  );
  expect(body(flat, "quiet")).toContain("$insert(m, k, v, quietDrop_drop)");
  expect(body(flat, "loud")).toContain("$insert(m, k, v, loudDrop_drop, quietDrop_drop)");
  expect(body(flat, "quietReplace")).toContain("$setReplace(s, x)");
  expect(body(flat, "loudReplace")).toContain("$setReplace(s, x, loudDrop_drop)");
});
