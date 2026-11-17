// The MIR intervals' shadow (ADR 1137, #1672): each check MIR's intervals
// prove, a `proved-<kind>` line, so what the MIR analysis proves, and what
// it doesn't, is held here. THIR's ranges, which the shadow compared it
// with, are gone (M4).
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildSerde } from "./serde-crate";
import { buildCompiler, compiler, fixture, root } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildCompiler, 600_000);

function shadowOf(source: string): string[] {
  const dir = fixture("intervals-shadow");
  const input = join(dir, "case.rs");
  const log = join(dir, "shadow.txt");
  writeFileSync(input, source);
  writeFileSync(log, "");
  const out = runSync([compiler, "--intervals-shadow", log, input, "-o", join(dir, "case.js"), "--", "--edition=2024"], dir, 60_000);
  expect(out.code, out.stderr).toBe(0);
  return readFileSync(log, "utf8").split("\n").filter(Boolean).map((line) => line.split("\t").slice(0, 2).join(" "));
}

test("an array's index in a loop to its length is proved by MIR's intervals", () => {
  expect(shadowOf(`pub fn sum(a: [u32; 4]) -> u32 {
    let mut total = 0u32;
    let mut i = 0usize;
    while i < 4 {
        total = total.wrapping_add(a[i]);
        i += 1;
    }
    total
}
pub fn main() {
    println!("{}", sum([1, 2, 3, 4]));
}
`)).toEqual(["proved-bounds sum", "proved-wrap sum"]);
});

test("a slice's index tested against its length, or its first under `!is_empty()`, is proved by MIR's relations", () => {
  expect(shadowOf(`pub fn get(s: &[u32], i: usize) -> u32 {
    if i < s.len() { s[i] } else { 0 }
}
pub fn first(s: &[u32]) -> u32 {
    if !s.is_empty() { s[0] } else { 0 }
}
pub fn main() {
    println!("{} {}", get(&[1, 2, 3], 1), first(&[4]));
}
`)).toEqual(["proved-bounds get", "proved-bounds first"]);
});

test("a slice's least length from a constant, a switch of its length, and an index less one are proved", () => {
  expect(shadowOf(`pub fn colon(b: &[u8]) -> bool {
    b.len() > 4 && b[4] == b':'
}
pub fn first_of(list: &[u32]) -> u32 {
    match list.len() {
        0 => 0,
        1 => list[0],
        _ => list[0] + 1,
    }
}
pub fn before(path: &[u32], i: usize) -> u32 {
    if i > 0 && i < path.len() { path[i - 1] } else { 0 }
}
pub fn main() {
    println!("{} {} {}", colon(b"abcd:"), first_of(&[1]), before(&[1, 2], 1));
}
`)).toEqual(["proved-bounds colon", "proved-bounds first_of", "proved-bounds first_of", "proved-bounds before", "proved-wrap before"]);
});

test("a parameter only its callers bound, and two slices' lengths summed, are proved", () => {
  expect(shadowOf(`fn plus(n: u32) -> u32 {
    n + 100
}
fn twice(n: u32) -> u32 {
    n * 2
}
fn total(a: &[u8], b: &[u8]) -> usize {
    a.len() + b.len()
}
pub fn main() {
    println!("{} {} {}", plus(5), twice(7), total(b"ab", b"c"));
}
`)).toEqual(["proved-wrap plus", "proved-wrap twice", "proved-wrap total"]);
});

test("a function that calls itself is given nothing of its outer calls: MIR proves nothing of its parameters", () => {
  expect(shadowOf(`fn grow(n: u32, k: u32) -> u32 {
    if k == 0 { n } else { grow(n + 1, k - 1) }
}
pub fn main() {
    println!("{}", grow(1, 3));
}
`)).toEqual(["proved-wrap grow"]);
});

test("a product that may be JS's -0, `-2 * 0`, isn't proved exact, though in range", () => {
  expect(shadowOf(`fn area(width: i32, height: i32) -> i32 {
    width * height
}
pub fn main() {
    println!("{} {}", area(-2, 0), area(3, 4));
}
`)).toEqual([]);
});

test("a quotient, a remainder and a value unequal to its least are proved: `n / 2 + n / 4`, `SLOTS[v % 3]`, `steps - 1` under `steps != 0`", () => {
  expect(shadowOf(`fn quarter(n: u32) -> u32 {
    n / 2 + n / 4
}
fn slot(slots: &[u8; 3], v: usize) -> u8 {
    slots[v % 3]
}
fn plain(steps: u32) -> u32 {
    if steps == 0 { 0 } else { plain(steps - 1) }
}
pub fn main() {
    let n = std::hint::black_box(7);
    println!("{} {} {}", quarter(n), slot(&[1, 2, 3], n as usize), plain(n));
}
`)).toEqual(["proved-wrap quarter", "proved-bounds slot", "proved-wrap plain", "proved-cast main"]);
});

test("a comparison of what a local copies narrows the copy: `i > 100` of `let mut i = start`", () => {
  expect(shadowOf(`fn never_ends(start: u32) -> u32 {
    let mut i = start;
    loop {
        if i > 100 {
            panic!("never ends: {i}");
        }
        i += 1;
    }
}
pub fn main() {
    let r = std::panic::catch_unwind(|| never_ends(std::hint::black_box(3)));
    println!("{}", r.is_err());
}
`)).toEqual(["proved-wrap never_ends"]);
});

test("a for loop's value over a range is at least its start and below its end, `path[i - 1]` of `for i in 1..len`; not of a range whose end is written in the loop", () => {
  expect(shadowOf(`fn walk(path: &[u32]) -> u32 {
    let len = path.len();
    let mut t = 0;
    for i in 1..len {
        t += path[i - 1] + path[i];
    }
    t
}
fn changed(v: &[u32]) -> u32 {
    let mut n = v.len();
    let mut t = 0;
    for i in 0..n {
        n = 0;
        t += v[i] + n as u32;
    }
    t
}
pub fn main() {
    println!("{} {}", walk(&[1, 2, 3]), changed(&[7]));
}
`)).toEqual(["proved-bounds walk", "proved-bounds walk", "proved-wrap walk", "proved-cast changed", "proved-wrap changed"]);
});

test("`i - 1` under `i < len` alone isn't below `len`: of `i = 0` it wraps; nor is a value of a range written a field at a time below the end it was made with", () => {
  expect(shadowOf(`fn before(p: &[u32], i: usize) -> u32 {
    if i < p.len() { p[i - 1] } else { 0 }
}
fn total(v: &[u32]) -> u32 {
    let mut r = 0..v.len();
    r.end = 3;
    let mut t = 0;
    for i in r {
        t += v[i];
    }
    t
}
pub fn main() {
    println!("{} {}", before(&[1, 2], std::hint::black_box(1)), total(&[1, 2, 3]));
}
`)).toEqual([]);
});

test("MIR decides a bounds check: `v[i >> 1]` below what `i` is below, of a `&mut [T]` a reborrow lends, `v.swap(0, i)`'s; a relation a loop's head keeps of what its entry's intervals show", () => {
  const dir = fixture("mir-decides");
  writeFileSync(join(dir, "case.rs"), `pub fn halves(v: &mut [u32], i: usize) -> u32 {
    if i < v.len() {
        v.swap(0, i);
        v[i >> 1] + v[i]
    } else {
        0
    }
}
pub fn before_zero(v: &[u32]) -> u32 {
    let mut i = 0;
    let mut seen = 0;
    while i < v.len() && v[i] != 0 {
        i += 1;
    }
    if i > 0 {
        seen = v[i - 1];
    }
    seen
}
pub fn past_five(a: [u32; 10]) -> u32 {
    let mut i = 0;
    while i < 5 {
        i += 1;
    }
    a[i]
}
pub fn later_lent(f: fn(&mut usize)) -> usize {
    let mut k = 1;
    k += 1;
    let before = k - 2;
    f(&mut k);
    let after = k - 2;
    before + after
}
pub fn later_written(w: usize) -> usize {
    let mut k = 1;
    k += 1;
    let first = k - 2;
    let r = &mut k;
    *r = w;
    let second = k - 2;
    first + second
}
fn merged(limit: u32) -> u32 {
    let mut n = 0;
    loop {
        let seen = n * 10;
        n += 1;
        if n == limit {
            break seen;
        }
    }
}
pub fn merged_four() -> u32 {
    merged(4)
}
pub fn leading(items: &[(u32, u32)]) -> usize {
    let mut i = 0;
    while let Some((a, _)) = items.get(i).cloned() {
        if a == 0 {
            break;
        }
        i += 1;
    }
    i
}
pub fn every_other(v: &[u32]) -> u32 {
    let mut total = 0;
    for i in (0..v.len()).step_by(2) {
        total += v[i];
    }
    total
}
`);
  const log = join(dir, "shadow.txt");
  writeFileSync(log, "");
  const out = runSync([compiler, "--intervals-shadow", log, join(dir, "case.rs"), "-o", join(dir, "case.js"), "--", "--edition=2024", "--crate-type=lib"], dir, 60_000);
  expect(out.code, out.stderr).toBe(0);
  const js = readFileSync(join(dir, "case.js"), "utf8");
  expect(js).toContain("return (v[i >>> 1] + v[i]) >>> 0;");
  // `i <= v.len()` at the loop's head, of `i = 0` by its interval and
  // `i + 1` past `i < v.len()`: past the loop, `v[i - 1]` of `i > 0`.
  expect(js).toContain("seen = v[i - 1];");
  // A range's `step_by(2)`: each value below its end.
  expect(js).toContain("total = (total + v[i]) >>> 0;");
  // Narrowed past widening: `i <= 5` at the loop's head, `i == 5` past it.
  expect(js).toContain("return a[i];");
  // An integer lent `&mut` is forgotten where it's lent: known before,
  // anything after, its wrap kept.
  // `n < limit` its first turn seeds, of `limit` 4 its one caller gives:
  // `n * 10` at most 30.
  expect(js).toContain("const seen = n * 10;");
  // `get(i).cloned()`'s `Some`: `i` below the slice's length.
  const leading = js.slice(js.indexOf("function leading"));
  expect(leading.slice(0, leading.indexOf("\n}\n"))).toContain("i++;");
  expect(js).toContain("const before = k - 2;");
  expect(js).toContain("const after = (k - 2) >>> 0;");
  // And one written through the `&mut` itself, lent no call (#1947).
  expect(js).toContain("const first = k - 2;");
  expect(js).toContain("const second = (k - 2) >>> 0;");
});

test("text's `len()` is at most what an allocation holds: two summed never wrap", () => {
  expect(shadowOf(`fn both(a: &str, b: &str) -> usize {
    a.len() + b.len()
}
pub fn main() {
    println!("{}", both("ab", "c"));
}
`)).toEqual(["proved-wrap both"]);
});

// Serde's derived bodies, thousands of blocks each: the callers' walk kept
// what's known at every block of every body, `examples/inbox.rs` taking
// 6.8 GB and a mutation's tests past CI's 5 GB (#1672). It keeps those at
// calls of a function only calls reach alone: under 1 GB.
test("the MIR intervals of serde's derived bodies take what a compile does, not gigabytes", () => {
  const out = join(fixture("inbox-memory"), "inbox.js");
  const p = Bun.spawnSync([compiler, "examples/inbox.rs", "-o", out, "--", ...buildSerde()], { cwd: root, stdout: "ignore", stderr: "pipe" });
  expect(p.exitCode, p.stderr.toString()).toBe(0);
  expect(p.resourceUsage.maxRSS).toBeLessThan(2 * 2 ** 30);
}, underLoad(120_000));

test("a macro's repeated index: of `[$($v[$i]),*]`, MIR proves `v[k]` alone", () => {
  expect(shadowOf(`macro_rules! pick {
    ($v:expr; $($i:expr),*) => {
        [$($v[$i]),*]
    };
}
fn pair(v: &[i32], k: usize) -> [i32; 2] {
    if k < v.len() { pick!(v; k, k + 1) } else { [0, 0] }
}
pub fn main() {
    println!("{:?}", pair(&[1, 2, 3], 1));
}
`)).toEqual(["proved-bounds pair", "proved-wrap pair"]);
});

test("a comparison narrows each copy of what it compares: `a[j]` of `let j = i` under `i < 10`", () => {
  const dir = fixture("copies-narrowed");
  writeFileSync(join(dir, "case.rs"), `pub fn at(a: [u32; 10], i: usize) -> u32 {
    let j = i;
    if i < 10 { a[j] } else { 0 }
}
`);
  const out = runSync([compiler, join(dir, "case.rs"), "-o", join(dir, "case.js"), "--", "--edition=2024", "--crate-type=lib"], dir, 60_000);
  expect(out.code, out.stderr).toBe(0);
  expect(readFileSync(join(dir, "case.js"), "utf8")).toContain("a[j]");
  expect(readFileSync(join(dir, "case.js"), "utf8")).not.toContain("$index");
});

// #1672 M3, step 5: a call's indexes, a `Vec`'s `v[i]`, `remove(i)`,
// `insert(i, x)` and `swap(i, j)`, MIR proves as THIR's facts do, of a
// `Vec` a local holds, or a `&mut Vec` points to, lent `&mut` only to the
// calls it's given to: `push` forgets its length, `v[i] = x` keeps it. Past
// `clear()` it's forgotten, `v[i]` checked by both; and of one whose `&mut`
// is moved on before a call clears it, as THIR's facts do.
test("a call's indexes are proved of a Vec lent `&mut` only to calls: `push` forgets its length, `v[i] = x` keeps it", () => {
  expect(shadowOf(`pub fn pushed(i: usize) -> u32 {
    let mut v = vec![1, 2, 3];
    v.push(4);
    if i < v.len() { v[i] } else { 0 }
}
pub fn written(i: usize) -> u32 {
    let mut v = vec![1, 2, 3];
    if i < v.len() { v[i] = 9; }
    v[0]
}
pub fn removed(targets: &mut Vec<u32>) -> u32 {
    if targets.len() > 2 {
        targets.insert(1, 5);
        return targets.remove(2);
    }
    0
}
pub fn swapped(i: usize, j: usize) -> u32 {
    let mut v = vec![1, 2, 3];
    if i < v.len() && j < v.len() { v.swap(i, j); }
    v[0]
}
pub fn cleared(i: usize) -> u32 {
    let mut v = vec![1, 2, 3];
    if i < v.len() { v.clear(); return v[i]; }
    0
}
pub fn moved(i: usize) -> u32 {
    let mut v = vec![1, 2, 3];
    if i < v.len() {
        let r = &mut v;
        let s = r;
        s.clear();
        return v[i];
    }
    0
}
pub fn main() {
    println!("{} {} {} {} {} {}", pushed(1), written(1), removed(&mut vec![1, 2, 3]), swapped(0, 1), cleared(5), moved(5));
}
`)).toEqual(["proved-call-bounds pushed", "proved-call-bounds written", "proved-call-bounds removed", "proved-call-bounds swapped"]);
});

// #1672 M3, step 5: a range's bounds, `&v[..n]`, `&v[n..]`, `&v[a..b]`,
// MIR proves where its end is at most the length, below it of `..=b`, and
// its start at most its end; `&v[..]` always. Not `&v[a..b]` of an `a` not
// known at most `b`, nor `&v[..=n]` of an `n` at most the length.
test("a range's bounds are proved where its end is at most the length and its start at most its end", () => {
  expect(shadowOf(`pub fn full(v: &[u32]) -> usize {
    v[..].len()
}
pub fn to_n(v: &[u32], n: usize) -> usize {
    if n <= v.len() { v[..n].len() } else { 0 }
}
pub fn from_n(v: &[u32], n: usize) -> usize {
    if n <= v.len() { v[n..].len() } else { 0 }
}
pub fn between(v: &[u32], a: usize, n: usize) -> usize {
    if n <= v.len() && a <= n { v[a..n].len() } else { 0 }
}
pub fn start_unknown(v: &[u32], a: usize, n: usize) -> usize {
    if n <= v.len() { v[a..n].len() } else { 0 }
}
pub fn through_n(v: &[u32], n: usize) -> usize {
    if n <= v.len() { v[..=n].len() } else { 0 }
}
pub fn main() {
    let v = [1, 2, 3];
    println!("{} {} {} {} {} {}", full(&v), to_n(&v, 2), from_n(&v, 2), between(&v, 1, 2), start_unknown(&v, 1, 2), through_n(&v, 2));
}
`)).toEqual(["proved-call-bounds full", "proved-call-bounds to_n", "proved-call-bounds from_n", "proved-call-bounds between"]);
});

// #1672 M3, step 5: a struct's fields a reference parameter is read
// through are locals of the body the walk reads: `self.0.remove(0)` past
// `!self.0.is_empty()` is proved, as THIR's facts do, and `self.items[self.at]`
// past `self.at < self.items.len()`, which they don't. Not of a method that
// gives `self` on, `self.reset()`, which may change any field.
test("a struct's fields read through a reference parameter are locals: a test of one narrows it", () => {
  expect(shadowOf(`pub struct Queue(Vec<u32>);
impl Queue {
    pub fn next(&mut self) -> Option<u32> {
        if self.0.is_empty() { None } else { Some(self.0.remove(0)) }
    }
}
pub struct It { items: Vec<u32>, at: usize }
impl It {
    pub fn step(&mut self) -> Option<u32> {
        if self.at < self.items.len() { let x = self.items[self.at]; self.at += 1; Some(x) } else { None }
    }
    pub fn whole(&mut self) -> u32 {
        if self.at < self.items.len() { self.reset(); return self.items[self.at]; }
        0
    }
    fn reset(&mut self) { self.items.clear(); }
}
pub fn main() {
    let mut q = Queue(vec![1]);
    let mut it = It { items: vec![1, 2], at: 0 };
    println!("{:?} {:?} {}", q.next(), it.step(), it.whole());
}
`)).toEqual(["proved-call-bounds Queue::next", "proved-call-bounds It::step", "proved-wrap It::step"]);
});

// #1814: an index a loop's `enumerate()` gives, `skip(1)` of it too, is below
// its slice's length; and one that's 0 before the loop and such an index in
// it, `best`, is below the length wherever the slice holds an item, as it
// does in the loop's body. Not one that's 1 before it, which a slice of one
// item doesn't hold.
test("an index a loop takes from its slice's `enumerate()`, 0 before it, is below the slice's length in it", () => {
  expect(shadowOf(`pub fn nearest(candidates: &[f64], value: f64) -> usize {
    let mut best = 0;
    for (index, candidate) in candidates.iter().enumerate().skip(1) {
        if (*candidate - value).abs() < (candidates[best] - value).abs() {
            best = index;
        }
    }
    best
}
pub fn from_one(candidates: &[f64], value: f64) -> usize {
    let mut best = 1;
    for (index, candidate) in candidates.iter().enumerate() {
        if (*candidate - value).abs() < (candidates[best] - value).abs() {
            best = index;
        }
    }
    best
}
pub fn summed(candidates: &[f64]) -> f64 {
    let mut total = 0.0;
    for (index, _) in candidates.iter().enumerate().skip(1) {
        total += candidates[index];
    }
    total
}
pub fn main() {
    println!("{} {} {}", nearest(&[1.0, 5.0, 3.0], 3.2), from_one(&[1.0, 5.0], 3.2), summed(&[1.0, 2.0]));
}
`)).toEqual(["proved-bounds nearest", "proved-bounds summed"]);
});

// #1815: a `RefCell`'s `borrow_mut()` guard, held while nothing else
// reaches the cell, keeps what's known of the cell's length past a call or
// a drop that runs code, until it drops: `take_first`'s `items.remove(i)`
// past `f(items[i])`. Through it, `index_mut` keeps the length, an index
// it gives is below it, and a drain to that index is in bounds. Not past a
// guard's `truncate`, a call of the crate's given its `&mut Vec`, a write
// through it, or a call once it's dropped, each of which may change it;
// nor is a `RefMut` a function of the crate's gives one of the cell it's
// given, `elsewhere(queue, other)` another's.
// MIR proves `take_first`'s `items[i]` too, below the loop's range's end,
// the guard's length.
test("a `RefCell`'s guard keeps its cell's length until it drops, but past what its `&mut` is given to", () => {
  expect(shadowOf(`use std::cell::{RefCell, RefMut};
use std::rc::Rc;
pub fn take_first(queue: &RefCell<Vec<u32>>, f: &dyn Fn(u32) -> bool) -> Option<u32> {
    let mut items = queue.borrow_mut();
    for i in 0..items.len() {
        if f(items[i]) {
            return Some(items.remove(i));
        }
    }
    None
}
pub fn settle(queue: &RefCell<Vec<Rc<dyn Fn() -> u32>>>, i: usize, task: Rc<dyn Fn() -> u32>) -> usize {
    if i < queue.borrow().len() {
        queue.borrow_mut()[i] = task;
        queue.borrow_mut().drain(0..i);
    }
    queue.borrow().len()
}
pub fn settle_after(queue: &RefCell<Vec<u32>>, i: usize, touch: &dyn Fn()) -> usize {
    touch();
    queue.borrow_mut()[i] = 0;
    queue.borrow_mut().drain(0..i);
    queue.borrow().len()
}
pub fn truncated(queue: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let mut items = queue.borrow_mut();
    if i < items.len() {
        items.truncate(0);
        items[i] = 1;
    }
    0
}
fn shrink(items: &mut Vec<u32>) {
    items.clear();
}
pub fn shrunk(queue: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let mut items = queue.borrow_mut();
    if i < items.len() {
        shrink(&mut items);
        return items[i];
    }
    0
}
pub fn replaced(queue: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let mut items = queue.borrow_mut();
    if i < items.len() {
        *items = Vec::new();
        return items[i];
    }
    0
}
fn elsewhere<'a>(_queue: &'a RefCell<Vec<u32>>, other: &'a RefCell<Vec<u32>>) -> RefMut<'a, Vec<u32>> {
    other.borrow_mut()
}
pub fn borrowed_elsewhere(queue: &RefCell<Vec<u32>>, other: &RefCell<Vec<u32>>, i: usize) -> u32 {
    if i < queue.borrow().len() {
        let mut items = elsewhere(queue, other);
        items[i] = 1;
    }
    0
}
pub struct Holder<'a> {
    v: &'a mut Vec<u32>,
}
// A guard's &mut kept in a struct, which no call is given whole, and the
// struct's field cleared: no write through the guard's own &mut the walk
// sees, so only the guard's rule forgets its length.
pub fn stored(queue: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let mut items = queue.borrow_mut();
    if i < items.len() {
        let h = Holder { v: &mut items };
        h.v.clear();
        return items[i];
    }
    0
}
pub fn after_its_guard(queue: &RefCell<Vec<u32>>, clear: &dyn Fn(), i: usize) -> u32 {
    {
        let items = queue.borrow_mut();
        if i >= items.len() {
            return 0;
        }
    }
    clear();
    queue.borrow()[i]
}
pub fn main() {
    let queue = Rc::new(RefCell::new(vec![1, 2, 3]));
    println!("{:?}", take_first(&queue, &|x| x > 1));
    let tasks: RefCell<Vec<Rc<dyn Fn() -> u32>>> = RefCell::new(vec![Rc::new(|| 1), Rc::new(|| 2)]);
    println!("{}", settle(&tasks, 1, Rc::new(|| 3)));
    println!("{}", settle_after(&queue, 0, &|| {}));
    println!("{} {} {}", truncated(&queue, 5), shrunk(&queue, 5), replaced(&queue, 5));
    println!("{}", borrowed_elsewhere(&queue, &RefCell::new(Vec::new()), 0));
    let other = Rc::clone(&queue);
    println!("{}", after_its_guard(&queue, &|| other.borrow_mut().clear(), 5));
    println!("{}", stored(&RefCell::new(vec![1]), 0));
}
`)).toEqual(["proved-call-bounds take_first", "proved-call-bounds take_first", "proved-call-bounds settle", "proved-call-bounds settle", "proved-call-bounds settle_after"]);
});

// #1839: an `Rc`'s `Deref` of a local neither reassigned nor lent `&mut`
// is the same cell each time, so React's `listeners.borrow()[i]` under
// `i < listeners.borrow().len()` is proved, as THIR proves it. Not past a
// call that may clear the cell through another `Rc`, nor of a `Box`'s
// cell or an `Rc` of an `Rc`'s a closure clears, which MIR proved
// before, nor through a `Deref` of the crate's: no line of the case's.
test("an `Rc`'s `Deref` gives its cell, the same each time: React's `listeners.borrow()[i]` is proved", () => {
  expect(shadowOf(readFileSync(join(root, "test/corpus/rc_refcell_bounds.rs"), "utf8"))).toEqual(["proved-call-bounds notified", "proved-wrap notified", "proved-call-bounds deleted", "proved-wrap deleted", "proved-wrap cleared_between"]);
});

// #1877: an item's write through a reference keeps the slice's length,
// and a table's row it writes is no row the table's rows hold, as those
// are reached by `Vec`'s `IndexMut` alone: `s[1]
// = vec![9]` of a `&mut [Vec<u32>]` its table lends. MIR proves `s[0]`
// and `s[1]` of the table's three rows, but no `s[1][k]` under `k <
// s[0].len()` past it.
test("an item's write through a reference keeps the length, not its table's rows", () => {
  expect(shadowOf(`pub fn written_row(v: &Vec<u32>, k: usize) -> u32 {
    let mut t = vec![v.clone(); 3];
    let s: &mut [Vec<u32>] = &mut t;
    if k < s[0].len() {
        s[1] = vec![9];
        return s[1][k];
    }
    0
}
pub fn main() {
    println!("{}", written_row(&vec![1, 2, 3], 2));
}
`)).toEqual(["proved-bounds written_row", "proved-bounds written_row", "proved-bounds written_row"]);
});

// #1672 M3, step 5: a map's `m[&k]` where `m.contains_key(&k)` holds, or
// past a first `m[&k]`, is proved, of a map a local holds or a `RefCell`
// does (rd558). Not past a `clear()`, a write of the key, a call that may
// write the cell, nor of a key with a `Cell` in it, which a call between
// may change.
test("a map's index of a key its `contains_key` showed it holds is proved, until the map or the key changes", () => {
  expect(shadowOf(`use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
/// A key whose hash and equality read its Cell.
pub struct Counted(Cell<u32>);
impl PartialEq for Counted {
    fn eq(&self, other: &Self) -> bool {
        self.0.get() == other.0.get()
    }
}
impl Eq for Counted {}
impl Hash for Counted {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.get().hash(state);
    }
}
pub fn stashed(errors: &HashMap<String, u32>, id: &String) -> u32 {
    if errors.contains_key(id) { errors[id] } else { 0 }
}
pub fn cached(cache: &RefCell<HashMap<u32, u32>>, k: u32) -> u32 {
    if cache.borrow().contains_key(&k) { cache.borrow()[&k] } else { 0 }
}
pub fn twice(m: &HashMap<u32, u32>, k: u32) -> u32 {
    m[&k] + m[&k]
}
pub fn cleared(m: &mut HashMap<u32, u32>, k: u32) -> u32 {
    if m.contains_key(&k) {
        m.clear();
        return m[&k];
    }
    0
}
pub fn rekeyed(m: &HashMap<u32, u32>, mut k: u32) -> u32 {
    if m.contains_key(&k) {
        k += 1;
        return m[&k];
    }
    0
}
pub fn called(cache: &RefCell<HashMap<u32, u32>>, k: u32, clear: &dyn Fn()) -> u32 {
    if cache.borrow().contains_key(&k) {
        clear();
        return cache.borrow()[&k];
    }
    0
}
pub fn celled(m: &HashMap<Counted, u32>, k: &Counted, bump: &dyn Fn()) -> u32 {
    if m.contains_key(k) {
        bump();
        return m[k];
    }
    0
}
pub fn main() {
    let m = HashMap::from([(String::from("a"), 1)]);
    let c = RefCell::new(HashMap::from([(1, 2)]));
    println!("{} {} {}", stashed(&m, &String::from("a")), cached(&c, 1), twice(&HashMap::from([(1, 2)]), 1));
    println!("{} {}", cleared(&mut HashMap::new(), 1), rekeyed(&HashMap::new(), 1));
    println!("{}", called(&c, 3, &|| c.borrow_mut().clear()));
    let key = Counted(Cell::new(0));
    println!("{}", celled(&HashMap::new(), &key, &|| key.0.set(1)));
}
`)).toEqual(["proved-call-bounds stashed", "proved-call-bounds cached", "proved-call-bounds twice"]);
});

// #1672 M3, step 5: a table's lengths (ADR 0576). `vec![x; n]` is as long
// as `n`, or as the slice `n` is the length of; an array as its type
// says; of a table of rows, each row as long as the one it was made of,
// where no `&mut` in the body may resize one. Not past a `pop()` of
// either, nor of a table whose rows a `truncate`, a closure or a row
// written whole may resize, nor past a call given the table `&mut` or its
// slice's, `fill`, `swap`, a fn item given to `for_each`; nor of a function
// taken as a value, which no caller's facts reach. MIR proves `i as u32`
// of `i` below `count`, which THIR keeps.
test("a table is as long as what it's made of, its rows as their row, until one may change", () => {
  expect(shadowOf(`pub fn rain(terraces: &[f64]) -> f64 {
    let mut left = vec![0.0; terraces.len()];
    for i in 1..terraces.len() {
        left[i] = terraces[i].max(left[i - 1]);
    }
    left.iter().sum()
}
pub fn sums(count: usize) -> Vec<u32> {
    let mut sums = vec![0; count];
    for i in 0..count {
        sums[i] = i as u32;
    }
    sums
}
pub fn paths(width: usize, height: usize) -> u64 {
    let mut board = vec![vec![1; width]; height];
    for r in 1..height {
        for c in 1..width {
            board[r][c] = board[r - 1][c] + board[r][c - 1];
        }
    }
    board[0][0]
}
pub fn prefix(keys: [i32; 11]) -> i32 {
    let f = || keys[..6].iter().sum::<i32>();
    f()
}
pub fn popped(terraces: &[f64]) -> f64 {
    let mut left = vec![0.0; terraces.len()];
    left.pop();
    let mut total = 0.0;
    for i in 0..terraces.len() {
        total += left[i];
    }
    total
}
pub fn renewed(count: usize) -> u32 {
    let mut sums = vec![0; count];
    sums = Vec::new();
    let mut total = 0;
    for i in 0..count {
        total += sums[i];
    }
    total
}
pub fn truncated(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    board[0].truncate(0);
    let mut total = 0;
    for r in 0..height {
        for c in 0..width {
            total += board[r][c];
        }
    }
    total
}
pub fn emptied(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    board[0] = Vec::new();
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}
pub fn closed(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    for row in board.iter_mut() {
        let mut shrink = || row.pop();
        shrink();
    }
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}
pub fn written(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    *std::ops::IndexMut::index_mut(&mut board, 0) = Vec::new();
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}
pub fn swapped_out(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    let mut empty = Vec::new();
    std::mem::swap(&mut board[0], &mut empty);
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}
pub fn taken(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    let row = std::mem::take(&mut board[0]);
    let mut total = row.len() as u32;
    for c in 0..width {
        total += board[0][c];
    }
    total
}
pub fn filled(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    board.fill(Vec::new());
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}
pub fn rows_swapped(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    board.swap(0, 1);
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}
pub fn cleared_each(width: usize, height: usize) -> u32 {
    let mut board: Vec<Vec<u32>> = vec![vec![1; width]; height];
    board.iter_mut().for_each(Vec::clear);
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}
fn first_of(width: usize, height: usize) -> u32 {
    let board = vec![vec![1; width]; height];
    board[0][0]
}
pub fn late(count: usize, i: usize) -> u32 {
    if i < count {
        let sums = vec![0; count];
        return sums[i];
    }
    0
}
fn first(width: usize, height: usize) -> u32 {
    let board = vec![vec![1; width]; height];
    board[0][0]
}
fn clear_first(board: &mut Vec<Vec<u32>>) {
    board[0].clear();
}
pub fn lent(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    clear_first(&mut board);
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}
pub fn main() {
    println!("{} {:?} {} {}", rain(&[1.0, 2.0]), sums(3), paths(3, 2), prefix([1; 11]));
    println!("{} {} {}", popped(&[]), renewed(0), truncated(0, 0));
    println!("{} {} {}", emptied(0, 1), closed(0, 1), lent(0, 1));
    println!("{} {} {} {}", written(0, 1), swapped_out(0, 1), taken(0, 1), filled(0, 1));
    println!("{} {}", rows_swapped(0, 2), cleared_each(0, 1));
    println!("{} {}", first_of(2, 2), late(3, 1));
    // Taken as a value, first has no caller's facts: its board[0] stays checked.
    let f: fn(usize, usize) -> u32 = first;
    println!("{}", f(2, 2));
}
`)).toEqual(["proved-bounds rain", "proved-wrap rain", "proved-call-bounds rain", "proved-call-bounds rain", "proved-call-bounds sums", "proved-cast sums", "proved-wrap paths", "proved-call-bounds paths", "proved-call-bounds paths", "proved-call-bounds paths", "proved-wrap paths", "proved-call-bounds paths", "proved-call-bounds paths", "proved-call-bounds paths", "proved-call-bounds prefix::{closure#0}", "proved-call-bounds truncated", "proved-cast taken", "proved-call-bounds first_of", "proved-call-bounds first_of", "proved-call-bounds late"]);
});

// #1672 M3, step 5: a test held in a `bool`, `let truncated = len >
// LIMIT`, switched on as its copy, narrows what it compares, a negated one,
// `!short`, as its negation. Not where what it compares changes between,
// the slice emptied or the integer written: its comparison is forgotten.
test("a test held in a `bool` narrows where it's switched on, until what it compares changes", () => {
  expect(shadowOf(`const LIMIT: usize = 3;
pub fn head(items: &[u32]) -> Vec<u32> {
    let truncated = items.len() > LIMIT;
    if truncated { items[..LIMIT].to_vec() } else { items.to_vec() }
}
pub fn negated(items: &[u32]) -> Vec<u32> {
    let short = items.len() <= LIMIT;
    if !short { items[..LIMIT].to_vec() } else { items.to_vec() }
}
pub fn cleared(mut items: Vec<u32>) -> Vec<u32> {
    let truncated = items.len() > LIMIT;
    items.clear();
    if truncated { items[..LIMIT].to_vec() } else { items }
}
pub fn rewritten(items: &[u32], mut n: usize) -> Vec<u32> {
    let fits = n <= items.len();
    n += 10;
    if fits { items[..n].to_vec() } else { Vec::new() }
}
pub fn main() {
    let items = [1, 2, 3, 4, 5];
    println!("{:?} {:?}", head(&items), negated(&items));
    println!("{:?} {:?}", cleared(vec![1]), rewritten(&items, 9));
}
`)).toEqual(["proved-call-bounds head", "proved-call-bounds negated"]);
});

// #1672 M3, step 5: a shared reference parameter to a struct that holds
// no cell itself keeps its fields as locals where it's also used whole,
// returned or compared, as nothing changes its fields while it's borrowed:
// `find(&node.children[0])` under `node.children.len() == 1`. Not a `&mut`
// one used whole, `reset(node)`, which may change any field.
test("a shared reference's fields are locals beside its whole uses, a mutable one's aren't", () => {
  expect(shadowOf(`pub struct Node {
    pub children: Vec<Node>,
}
pub fn find(node: &Node) -> &Node {
    if node.children.len() == 1 {
        return find(&node.children[0]);
    }
    node
}
fn reset(node: &mut Node) {
    node.children.clear();
}
pub fn reset_first(node: &mut Node) -> usize {
    if node.children.len() == 1 {
        reset(node);
        return node.children[0].children.len();
    }
    0
}
pub fn main() {
    let mut node = Node { children: vec![Node { children: Vec::new() }] };
    println!("{} {}", find(&node).children.len(), reset_first(&mut node));
}
`)).toEqual(["proved-call-bounds find"]);
});
