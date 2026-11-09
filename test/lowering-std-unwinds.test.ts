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

// A B-tree whose clone may panic partway and drop what it made, whatever
// its keys, is std's nodes, which `clone_subtree` drops by, node by node
// (#1069); one whose clone can't is the sorted map it always was.
test("a B-tree map is std's nodes where its clone may panic and drop what it made", () => {
  const flat = compiled(
    "std-unwinds-btree-clone",
    `${types}
use std::collections::BTreeMap;

/// Its clone may panic.
pub struct Picky(pub u32);

impl Clone for Picky {
    fn clone(&self) -> Picky {
        if self.0 == 0 {
            panic!("clone");
        }
        Picky(self.0)
    }
}

impl Drop for Picky {
    fn drop(&mut self) {
        self.0 = 0;
    }
}

pub fn quiet() -> BTreeMap<u32, Quiet> {
    BTreeMap::new()
}

pub fn picky() -> BTreeMap<u32, Picky> {
    BTreeMap::new()
}

pub fn numbers() -> BTreeMap<u32, u32> {
    BTreeMap::new()
}
`,
  );
  expect(body(flat, "quiet")).toContain("new $BTreeMap()");
  expect(body(flat, "numbers")).toContain("new $BTreeMap()");
  expect(body(flat, "picky")).toContain("new ($ordTreeMap($cmp))()");
});

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
  // An `Rc`'s `&mut` is a cell: `$dedupByCells`, given the drop too.
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
  expect(body(flat, "held")).toContain("$maxOf(a, b, heldPartialOrd, heldDrop_drop)");
  expect(body(flat, "heldLeast")).toContain("$minOf(a, b, heldPartialOrd, heldDrop_drop)");
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
  // Given the items' `PartialOrd`, whose `<=`, `>=` and `<` std's heap asks (#937).
  // A derived type's, its module's `const`, made once (#944).
  expect(body(flat, "quiet")).toContain("$heapPop(h, quietPartialOrd)");
  expect(body(flat, "loud")).toContain("$heapPop(h, loudPartialOrd, false, loudDrop_drop)");
  expect(body(flat, "sorted")).toContain("$heapSorted(h, loudPartialOrd, loudDrop_drop)");
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

// #926: a B-tree of keys whose order can be seen, or of a type parameter's,
// may be std's nodes: built by `$treeFromSorted`, given the keys' `Ord`
// dictionary, whose `PartialOrd()` and `PartialEq()` a crate impl's has,
// else, of a std type's of the crate's, those dictionaries; and what std's
// sort picks by the entries' type. One of a quiet and total order keeps
// `$treeFrom`, a sorted array's, in today's shape.
test("a B-tree built of items is std's bulk build where its keys' order may be seen", () => {
  const flat = compiled(
    "std-unwinds-btree-built",
    `
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(PartialEq, Eq)]
pub struct Quiet(pub u32);

impl PartialOrd for Quiet {
    fn partial_cmp(&self, other: &Quiet) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Quiet {
    fn cmp(&self, other: &Quiet) -> Ordering {
        self.0.cmp(&other.0)
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

pub fn quiet(xs: Vec<(Quiet, u32)>) -> BTreeMap<Quiet, u32> {
    xs.into_iter().collect()
}

pub fn loud(xs: Vec<(Loud, u32)>) -> BTreeMap<Loud, u32> {
    xs.into_iter().collect()
}

pub fn pairs(xs: Vec<(Loud, u32)>) -> BTreeSet<(Loud, u32)> {
    xs.into_iter().collect()
}

pub fn generic<T: Ord>(xs: Vec<T>) -> BTreeSet<T> {
    xs.into_iter().collect()
}
`,
  );
  expect(body(flat, "quiet")).toContain("$treeFrom(new ($ordMap(quietOrd_cmp))(), $iter(xs), false, quietOrd_cmp)");
  expect(body(flat, "loud")).toContain('$treeFromSorted(new ($ordTreeMap(loudOrd_cmp))(), $iter(xs), loudOrd, "general8", 8)');
  // A tuple's dictionary holds its `PartialOrd()` too, as every `Ord`
  // dictionary does, its module's `const`, `tupleOrd` (#944).
  expect(body(flat, "pairs")).toContain('$iter(xs), tupleOrd, "general8", 8, )');
  expect(body(flat, "generic")).toContain("$treeFromSorted(new TTreeSet(), $iter(xs), TOrd, TStableSort, TSize)");
});

// #931: `==` of two `Rc`s in generic code bounded by `T: PartialEq` asks
// whether they're one allocation first where its caller's `T` is `Eq`, as
// std's `RcEqIdent` specializes each instance: `TEq`, the caller's, `true`
// of an `Eq` type and `false` of one only `PartialEq`; refused of a caller
// whose `Eq` value JS can't tell two allocations of apart (#930). A generic
// function comparing no `Rc` is given none, and an impl's dictionary given
// one is still cached weakly by the dictionaries it's given.
test("== of Rcs in generic code asks the same allocation first where the caller's T is Eq", () => {
  const rust = `
use std::rc::Rc;

pub struct Key(pub u32);

impl PartialEq for Key {
    fn eq(&self, other: &Key) -> bool {
        self.0 == other.0
    }
}

impl Eq for Key {}

pub struct Partial(pub u32);

impl PartialEq for Partial {
    fn eq(&self, other: &Partial) -> bool {
        self.0 == other.0
    }
}

pub struct Holder<T>(pub Rc<T>);

impl<T: PartialEq> PartialEq for Holder<T> {
    fn eq(&self, other: &Holder<T>) -> bool {
        self.0 == other.0
    }
}

pub fn same<T: PartialEq>(a: &Rc<T>, b: &Rc<T>) -> bool {
    a == b
}

pub fn plain<T: PartialEq>(a: &T, b: &T) -> bool {
    a == b
}

pub fn maybe<T: PartialEq>(a: &Option<Rc<T>>, b: &Option<Rc<T>>) -> bool {
    a == b
}

pub fn maybe_keys(a: &Option<Rc<Key>>) -> bool {
    maybe(a, a)
}

pub fn keys(a: &Rc<Key>) -> bool {
    same(a, &a.clone())
}

pub fn partials(a: &Rc<Partial>) -> bool {
    same(a, &a.clone())
}

pub fn plain_keys(a: &Key) -> bool {
    plain(a, a)
}

pub fn holders(a: &Holder<Key>) -> bool {
    plain(a, a)
}
`;
  const flat = compiled("std-unwinds-generic-rc-eq", rust);
  expect(body(flat, "same")).toContain("same(a, b, TPartialEq, TEq) { return (TEq && a === b) || TPartialEq.eq(a, b); }");
  expect(body(flat, "keys")).toContain("same(a, a, keyPartialEq, true)");
  expect(body(flat, "partials")).toContain("same(a, a, partialPartialEq, false)");
  expect(body(flat, "plain")).toContain("plain(a, b, TPartialEq) {");
  expect(body(flat, "plainKeys")).toContain("plain(a, a, keyPartialEq)");
  expect(body(flat, "maybeKeys")).toContain("maybe(a, a, keyPartialEq, true)");
  expect(body(flat, "holders")).toContain("holderPartialEq(keyPartialEq, true)");
  expect(flat).toContain("const $holderPartialEq = new WeakMap();");
  const kind = `${rust}
#[derive(Clone, Copy)]
pub enum Kind {
    A,
    B,
}

impl PartialEq for Kind {
    fn eq(&self, other: &Kind) -> bool {
        matches!((self, other), (Kind::A, Kind::A) | (Kind::B, Kind::B))
    }
}

impl Eq for Kind {}

pub fn kinds(a: &Rc<Kind>) -> bool {
    same(a, &Rc::new(Kind::A))
}
`;
  expect(() => compiled("std-unwinds-generic-rc-eq-refused", kind)).toThrow(/`==` of two `std::rc::Rc<Kind>`s, which std compares by address first/);
});
