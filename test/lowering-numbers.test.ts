// What the compiler writes of numbers and indexes: a cast of a whole float, an
// index shown in bounds read unchecked, integer ranges that can't wrap, a
// division by a constant, and an enum's discriminant. Each test is a program
// of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0627: a number literal written with a base or an exponent is
// written as the program wrote it, `1e4`, `0xff`, where JS reads it as the
// same value: its type's suffix and its separators aren't JS's. Any other
// is JS's own digits, `5000` of `5000.0`.
test("a number literal keeps its base and its exponent", async () => {
  const dir = fixture("literals-as-written");
  writeFileSync(join(dir, "lib.rs"), `pub const OWNER_STACK_LIMIT: f64 = 1e4;
pub const SEED: u32 = 0x811c_9dc5;

pub fn masked(n: u32) -> u32 {
    n & 0xffu32
}

pub fn small() -> f64 {
    2.5e-3
}

pub fn whole() -> f64 {
    5000.0
}

pub fn below() -> f64 {
    -1e4
}

pub fn flags(n: u8) -> u8 {
    n | 0b1010
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export const OWNER_STACK_LIMIT = 1e4;");
  expect(js).toContain("export const SEED = 0x811c9dc5;");
  expect(js).toContain("return (n & 0xff) >>> 0;");
  expect(js).toContain("return 2.5e-3;");
  expect(js).toContain("return 5000;");
  expect(js).toContain("return -1e4;");
  expect(js).toContain("return n | 0b1010;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.OWNER_STACK_LIMIT, lib.SEED, lib.masked(0x1234), lib.small(), lib.whole(), lib.below(), lib.flags(1)]).toEqual([10000, 0x811c9dc5, 0x34, 0.0025, 5000, -10000, 11]);
});

// ADR 0631: a statement `x += 1` is `x++`, and `x -= 1` is `x--`, of a
// variable, a field or an item: a step of 1, a number literal, is only a
// number's. Any other step, a string's text, `2`, is as it's written.
test("a statement stepping by one is ++ or --", async () => {
  const dir = fixture("step-by-one");
  writeFileSync(join(dir, "lib.rs"), `pub struct Counter {
    pub hits: f64,
}

pub fn steps(items: &[u32], counter: &mut Counter, counts: &mut [f64], i: usize) -> (usize, f64, String) {
    let mut j = 0;
    while j < items.len() {
        j += 1;
    }
    let mut t = 2.5;
    t += 1.0;
    t -= 1.0;
    let mut s = String::from("a");
    s += "1";
    counter.hits += 1.0;
    if i < counts.len() {
        counts[i] -= 1.0;
    }
    if j < 2 {
        j += 2;
    }
    (j, t, s)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    j++;\n");
  expect(js).toContain("  t++;\n  t--;\n");
  expect(js).toContain("  counter.hits++;\n");
  expect(js).toContain("    counts[i]--;\n");
  expect(js).toContain('s += "1";');
  expect(js).toContain("j += 2;");
  expect(js).not.toMatch(/[+-]= 1;/);
  const lib = await import(join(dir, "lib.js"));
  const counter = { hits: 4 };
  const counts = [1, 5];
  expect(lib.steps([7, 8, 9], counter, counts, 1)).toEqual([3, 2.5, "a1"]);
  expect([counter, counts]).toEqual([{ hits: 5 }, [1, 4]]);
  expect(lib.steps([7], counter, counts, 2)[0]).toBe(3);
  expect([counter, counts]).toEqual([{ hits: 6 }, [1, 4]]);
});

// std's `From<bool>` of each number and `From<char>` of `u32`, `u64` and
// `u128` are what `as` makes of them: `flag ? 1 : 0`, `c.codePointAt(0)`,
// a BigInt of 64 bits and more (ADR 0086), as a function too.
test("a number from a bool or a char is its cast", async () => {
  const dir = fixture("from-bool-char");
  writeFileSync(join(dir, "lib.rs"), `pub fn counted(flag: bool) -> usize {
    usize::from(flag)
}

pub fn wide(flag: bool) -> u64 {
    flag.into()
}

pub fn half(flag: bool) -> f64 {
    f64::from(flag)
}

pub fn code(c: char) -> u32 {
    u32::from(c)
}

pub fn big_code(c: char) -> u128 {
    c.into()
}

pub fn all(flags: &[bool]) -> Vec<i8> {
    flags.iter().copied().map(i8::from).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return flag ? 1 : 0;");
  expect(js).toContain("return flag ? 1n : 0n;");
  expect(js).toContain("return c.codePointAt(0);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.counted(true), lib.counted(false), lib.wide(true), lib.half(true), lib.code("é"), lib.bigCode("😀"), lib.all([true, false])])
    .toEqual([1, 0, 1n, 1, 233, 128512n, [1, 0]]);
});

// An f64 known to be a whole number in an integer's range is that integer as
// it is (ADR 0302): where `indexOf` found it, as react.dev's CodeBlock writes
// `startColumn: index` once `index` isn't -1, and an integer's `as f64`. One
// that may be -1, or is set again after its test, is cast as `as` casts.
test("an f64 known to be a whole number in range is cast as it is", async () => {
  const dir = fixture("whole-casts");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "indexOf")] #[rust_js::position] #[allow(unused_variables)] fn index_of(this: &str, search: &str) -> f64 { unreachable!() }
pub fn found(line: &str, word: &str) -> u32 {
    let index = index_of(line, word);
    if index == -1.0 {
        panic!("Could not find {word}");
    }
    index as u32
}
pub fn unchecked(line: &str, word: &str) -> u32 {
    let index = index_of(line, word);
    index as u32
}
pub fn set_again(line: &str, word: &str) -> u32 {
    let mut index = index_of(line, word);
    if index == -1.0 {
        panic!("Could not find {word}");
    }
    index = -1.0;
    index as u32
}
pub fn byte(n: u8) -> u8 {
    let x = n as f64;
    x as u8
}
pub fn shifted(line: &str, word: &str) -> u32 {
    let mut index = index_of(line, word);
    index -= 1.0;
    if index == -1.0 {
        panic!("Could not find {word}");
    }
    index as u32
}
pub fn wide(n: u32) -> u64 {
    let x = n as f64;
    x as u64
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return index;\n}");
  expect(js).toContain("export function unchecked(line, word) {\n  const index = line.indexOf(word);\n  return $f64ToInt(index, 0, 4294967295);\n}");
  expect(js).toContain("  index = -1;\n  return $f64ToInt(index, 0, 4294967295);\n}");
  expect(js).toContain("export function byte(n) {\n  const x = n;\n  return x;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.found("abc", "c"), lib.unchecked("abc", "z"), lib.setAgain("abc", "a"), lib.byte(7), lib.shifted("abc", "z"), lib.wide(7)]).toEqual([2, 0, 0, 7, 0, 7n]);
});

// ADR 0292: an index a condition shows is in bounds is JS's own: react.dev's
// Preview reads `lintErrors[0]` where `lintErrors.length === 0` is false. In
// a branch, after an early return and in `for i in 0..xs.len()`, of a slice
// and an index the function never changes; anywhere else it's checked.
test("an index a condition shows in bounds is read as it is", async () => {
  const dir = fixture("known-bounds");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(items: &[u32]) -> Option<u32> {
    if items.is_empty() { None } else { Some(items[0]) }
}

pub fn second(items: &Vec<u32>) -> u32 {
    if items.len() < 2 {
        return 0;
    }
    items[1]
}

pub fn total(items: &[u32]) -> u32 {
    let mut sum = 0;
    for i in 0..items.len() {
        sum += items[i];
    }
    sum
}

pub fn below(items: &[u32], i: usize) -> u32 {
    if i < items.len() && !items.is_empty() { items[i] + items[0] } else { 0 }
}

pub fn unchecked(items: &[u32]) -> u32 {
    if items.len() > 1 { items[2] } else { 0 }
}

pub fn changed(mut items: Vec<u32>) -> u32 {
    if items.is_empty() {
        return 0;
    }
    items.clear();
    items[0]
}

pub fn either(items: &[u32]) -> u32 {
    if items.len() > 3 || !items.is_empty() { items[3] } else { 0 }
}

pub fn third(items: &[u32]) -> u32 {
    if items.len() < 2 {
        return 0;
    }
    items[2]
}

pub fn after(items: &[u32]) -> u32 {
    let mut n = 0;
    if items.is_empty() {
        n = 1;
    }
    n + items[0]
}

pub fn wrong(items: &[u32]) -> u32 {
    if items.is_empty() { items[0] } else { 0 }
}

pub fn other(items: &[u32]) -> u32 {
    if items.len() != 5 { items[0] } else { 0 }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  for (const read of ["items[0]", "items[1]", "items[i]", "items[i] + items[0]"]) expect(js).toContain(read);
  expect(js).toContain("$index(items, 2)");
  expect(js).toContain("$index(items, 0)");
  expect(js.match(/\$index/g)?.length).toBe(8);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first([]), lib.first([4]), lib.second([1, 2]), lib.second([1]), lib.total([1, 2, 3]), lib.below([5, 6], 1), lib.below([5], 3)]).toEqual([undefined, 4, 2, 0, 6, 11, 0]);
  expect(() => lib.unchecked([1, 2])).toThrow();
  expect(() => lib.changed([1])).toThrow();
  for (const unknown of [() => lib.either([1]), () => lib.third([1, 2]), () => lib.after([]), () => lib.wrong([]), () => lib.other([])]) {
    expect(unknown).toThrow();
  }
});

// ADR 0576: what a body's integers can be, walked as it runs, shows an index
// in bounds and an operator that can't wrap: React's scheduler heap reads
// `heap[leftIndex]`, `(index + 1) * 2 - 1` and a swap of two items, as
// upstream's does. What could leave its range, or an index the walk can't
// see stays below its slice's length, keeps its wrap or its check.
test("a body's integer ranges show indexes in bounds and operators that can't wrap", async () => {
  const dir = fixture("integer-ranges");
  writeFileSync(join(dir, "lib.rs"), `pub fn sift_down(heap: &mut Vec<u32>, i: usize) {
    let mut index = i;
    let length = heap.len();
    let half_length = length >> 1;
    while index < half_length {
        let left_index = (index + 1) * 2 - 1;
        let right_index = left_index + 1;
        if heap[left_index] < heap[index] {
            if right_index < length && heap[right_index] < heap[left_index] {
                heap.swap(index, right_index);
                index = right_index;
            } else {
                heap.swap(index, left_index);
                index = left_index;
            }
        } else if right_index < length && heap[right_index] < heap[index] {
            heap.swap(index, right_index);
            index = right_index;
        } else {
            return;
        }
    }
}

pub fn pop(heap: &mut Vec<u32>) -> Option<u32> {
    if heap.is_empty() {
        return None;
    }
    let first = heap.swap_remove(0);
    if !heap.is_empty() {
        sift_down(heap, 0);
    }
    Some(first)
}

pub fn last(items: &[u32]) -> u32 {
    if items.is_empty() { 0 } else { items[items.len() - 1] }
}

pub fn head(items: &[u32]) -> u32 {
    if !items.is_empty() { items[0] } else { 0 }
}

pub fn wrapped(n: u32, k: i32, m: u8) -> (u32, i32, u8) {
    (n + 1, k * 2, m + 1)
}

pub fn small(n: u32, k: i32, m: u8) -> (u32, i32, u8) {
    let a = if n < 10 { n * 3 + 1 } else { 0 };
    let b = if k > 0 { 0 - k } else { 1 };
    let c = if m < 255 { m + 1 } else { 0 };
    (a, b, c)
}

pub fn counted(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}

pub fn stepped(n: u32) -> u32 {
    let mut steps = 0;
    loop {
        steps += 1;
        if steps == n {
            return steps;
        }
    }
}

pub fn skipped(n: u32) -> u32 {
    let mut i = 0;
    let mut k = 0;
    while i < 10 {
        i += 1;
        if i == 5 {
            k = n;
            continue;
        }
        k = 0;
    }
    k + 1
}

pub fn at_end(items: &[u32], i: usize) -> u32 {
    if i <= items.len() { items[i] } else { 0 }
}

pub fn popped(items: &mut Vec<u32>) -> u32 {
    let mut sum = 0;
    for i in 0..items.len() {
        sum ^= items[i];
        items.pop();
    }
    sum
}

pub fn swapped(items: &mut [u32], i: usize) {
    items.swap(0, i);
}

pub fn later(items: &mut Vec<u32>) -> u32 {
    if items.is_empty() {
        return 0;
    }
    items.clear();
    items[0]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    const leftIndex = (index + 1) * 2 - 1;\n    const rightIndex = leftIndex + 1;\n    if (heap[leftIndex] < heap[index]) {\n      if (rightIndex < length && heap[rightIndex] < heap[leftIndex]) {\n        [heap[index], heap[rightIndex]] = [heap[rightIndex], heap[index]];\n");
  expect(js).toContain("  const halfLength = length >>> 1;\n");
  expect(js).toContain("  [heap[0], heap[heap.length - 1]] = [heap[heap.length - 1], heap[0]];\n  const first = heap.pop();\n");
  expect(js).toContain("items[items.length - 1]");
  expect(js).toContain("items.length !== 0 ? items[0] : 0");
  // What could leave its range wraps; what can't is JS's own.
  expect(js).toContain("[(n + 1) >>> 0, (k * 2) | 0, (m + 1) & 255]");
  expect(js).toContain("n < 10 ? n * 3 + 1 : 0");
  expect(js).toContain("k > 0 ? 0 - k : 1");
  expect(js).toContain("m < 255 ? m + 1 : 0");
  expect(js).toContain("  while (i < n) {\n    i++;\n  }\n");
  expect(js).toContain("steps = (steps + 1) >>> 0;");
  expect(js).toContain("return (k + 1) >>> 0;");
  expect(js).toContain("i <= items.length ? $index(items, i) : 0");
  // A slice changed in a loop, an index nothing bounds, a length changed.
  expect(js).toContain("$index(items, i)");
  expect(js).toContain("$swap(items, 0, i);");
  expect(js).toContain("$index(items, 0)");
  const lib = await import(join(dir, "lib.js"));
  const heap = [1, 3, 2, 7, 4, 6, 5];
  const out = [];
  for (let next = lib.pop(heap); next !== undefined; next = lib.pop(heap)) out.push(next);
  expect(out).toEqual([1, 2, 3, 4, 5, 6, 7]);
  expect([lib.last([]), lib.last([1, 7])]).toEqual([0, 7]);
  expect(lib.wrapped(4294967295, 1 << 30, 255)).toEqual([0, -2147483648, 0]);
  expect([lib.small(9, 5, 254), lib.counted(3), lib.stepped(3), lib.popped([5])]).toEqual([[28, -5, 255], 3, 3, 5]);
  expect(() => lib.popped([1, 2, 4])).toThrow("index out of bounds");
  expect(() => lib.swapped([1], 3)).toThrow("index out of bounds");
  expect(() => lib.atEnd([1], 1)).toThrow("index out of bounds");
  expect(lib.skipped(4294967295)).toBe(1);
  expect(() => lib.later([1])).toThrow("index out of bounds");
});

// `get_unchecked(i)` of a slice is its item, `xs[i]`, unchecked, as JS reads
// one (ADR 0294): react.dev's NavigationBar reads `entry.contentBoxSize[0]`.
test("get_unchecked is the item itself", async () => {
  const dir = fixture("get-unchecked");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(xs: &[u32]) -> u32 {
    unsafe { *xs.get_unchecked(0) }
}
pub fn second(names: &Vec<String>) -> &str {
    unsafe { names.get_unchecked(1) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return xs[0];");
  expect(js).toContain("return names[1];");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first([4, 5]), lib.second(["a", "b"])]).toEqual([4, "b"]);
});

// A division by a named constant that isn't zero, nor a signed -1, can't
// panic: JS's `/`.
test("a division by a named constant is JS's", async () => {
  const dir = fixture("named-divisor");
  writeFileSync(join(dir, "lib.rs"), `const HALVES: u32 = 2;
const NEGATIVE: i32 = -1;

pub fn half(x: u32) -> u32 {
    x / HALVES
}

pub fn negated(x: i32) -> i32 {
    x / NEGATIVE
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return (x / HALVES) >>> 0;");
  expect(js).toContain("$div(x, NEGATIVE, -2147483648)");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.half(7), lib.negated(6)]).toEqual([3, -6]);
  expect(() => lib.negated(-2147483648)).toThrow("attempt to divide with overflow");
});

test("an index from MIR nothing proves in bounds is checked where it's read or written", async () => {
  const dir = fixture("mir-checked-index");
  writeFileSync(join(dir, "lib.rs"), `pub struct Crumb { pub path: Option<&'static str> }
pub fn last_is(crumbs: &[Crumb], path: Option<&str>) -> bool {
    crumbs.len() > 1 && crumbs[crumbs.len() - 1].path == path
}
pub fn second(xs: &[u32]) -> u32 {
    xs[1] + 1
}
pub fn set(xs: &mut [u32], i: usize) {
    xs[i] = 3;
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return crumbs.length > 1 && crumbs[crumbs.length - 1].path == path;\n");
  expect(js).toContain("  return ($index(xs, 1) + 1) >>> 0;\n");
  expect(js).toContain("  xs[$at(xs, i)] = 3;\n");
  const lib = await import(join(dir, "mir.js"));
  expect(() => lib.second([1])).toThrow("index out of bounds: the len is 1 but the index is 1");
  expect(() => lib.set([1], 4)).toThrow("index out of bounds: the len is 1 but the index is 4");
  expect([lib.second([1, 2]), lib.lastIs([{}, { path: "a" }], "a")]).toEqual([3, true]);
});

// An enum made a number, \`ordering as i32\` or a fieldless one's, from
// MIR: its discriminant.
test("an enum cast to a number from MIR is its discriminant", async () => {
  const dir = fixture("mir-enum-cast");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Color { Red, Green = 5, Blue }
pub fn order(a: &str, b: &str) -> i32 {
    a.cmp(b) as i32
}
pub fn code(c: Color) -> i32 {
    c as i32
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.order("abc", "abd"), lib.order("b", "a"), lib.code("Red"), lib.code("Green"), lib.code("Blue")]).toEqual([-1, 1, 0, 5, 6]);
});

// A float's sum from MIR starts at -0.0, as Rust's does: an empty sum and
// one of negative zeros are -0.0, not 0.
test("a float's sum from MIR keeps the sign of zero", async () => {
  const dir = fixture("mir-float-sum");
  writeFileSync(join(dir, "lib.rs"), `pub fn total(xs: &[f64]) -> f64 {
    xs.iter().sum()
}
pub fn owned(xs: Vec<f64>) -> f64 {
    xs.into_iter().sum()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([Object.is(lib.total([]), -0), Object.is(lib.total([-0]), -0), Object.is(lib.owned([-0, -0]), -0), lib.total([1, 2])]).toEqual([true, true, true, 3]);
});

// A wide field updated through a \`&mut\`, from MIR, is written where it's
// made, \`m.cents = BigInt.asIntN(64, m.cents + 1n)\`: wrapping it does
// nothing a place could see.
test("a wide field updated from MIR is written where it's made", async () => {
  const dir = fixture("mir-wide-field");
  writeFileSync(join(dir, "lib.rs"), `pub struct Money { pub cents: i64 }
pub fn bump(m: &mut Money) {
    m.cents += 1;
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("export function bump(m) {\n  m.cents = BigInt.asIntN(64, m.cents + 1n);\n}");
  const lib = await import(join(dir, "mir.js"));
  const m = { cents: 9223372036854775807n };
  lib.bump(m);
  expect(m.cents).toBe(-9223372036854775808n);
});

// MIR's checks of a `%` or `/`, `2 === -1` of a constant divisor, are
// none: the division checks what can fail, `$rem(a, b, min)`.
test("a division by a constant from MIR has no overflow check", async () => {
  const dir = fixture("mir-constant-divisor");
  writeFileSync(join(dir, "lib.rs"), `pub fn odds(n: i32) -> Vec<i32> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        i += 1;
        if i % 2 == 0 {
            continue;
        }
        out.push(i / 3);
    }
    out
}
pub fn by(a: i32, b: i32) -> i32 {
    a % b
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`    if (i % 2 === 0) {
      continue;
    }
    out.push((i / 3) | 0);`);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.odds(7)).toEqual([0, 1, 1, 2]);
  // A divisor not known is checked where it divides, once.
  expect(js).toContain("export function by(a, b) {\n  return $rem(a, b, -2147483648) | 0;\n}");
  expect(() => lib.by(-2147483648, -1)).toThrow("remainder with overflow");
  expect(() => lib.by(1, 0)).toThrow("divisor of zero");
});

// An element's place checked then set, `args[0] = v`, from MIR is
// `args[$at(args, 0)] = v`, its index read in place where nothing changes
// it, as react.dev's ErrorDecoder sets its first argument. What's set is
// made before the check, as Rust makes it.
test("a checked element set is one statement from MIR", async () => {
  const dir = fixture("checked-set");
  writeFileSync(join(dir, "lib.rs"), `pub fn fill(args: &mut Vec<String>) {
    if args.len() == 1 && args[0].is_empty() {
        args[0] = "HTML".to_string();
    }
}
pub fn put(args: &mut Vec<String>, i: usize, made: impl Fn() -> String) {
    args[i] = made();
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // One the length's test shows in bounds is set as it is (ADR 0576).
  expect(js).toContain('    args[0] = "HTML";\n');
  expect(js).toMatch(/  args\[\$at\(args, i\)\] = \w+;\n}/);
  const lib = await import(join(dir, "mir.js"));
  const args = [""];
  lib.fill(args);
  const made: string[] = [];
  expect(() => lib.put(args, 1, () => (made.push("x"), "x"))).toThrow();
  expect([args, made]).toEqual([["HTML"], ["x"]]);
});

// A float THIR's facts know whole and in range where it's cast, `index as
// u32` of `indexOf`'s once it isn't -1, is the value as it is
// (`whole_casts`), not `$f64ToInt(index, ..)`, as react.dev's CodeBlock
// finds a highlight's column.
// ADR 0632: a JS string's `length`, a binding's `get length` of a string,
// is below 2^32 - 1, as every engine's string is, so a counter that steps
// below it, `i + 1 < length(s)`, as React's normalizeConsoleFormat's does,
// can't wrap: `i + 1` and its `i += 1` are written as they are. A test of
// `i + 1` narrows `i` where the add can't wrap, and a counter that keeps
// growing is taken first as one below its type's most: a step of one
// below a slice's length is `i++` too. Another `get length`, of an array,
// may be 2^32 - 1, so two steps below it keep their wrap, as a test of an
// `i + 1` that may have wrapped shows nothing of `i`.
test("a counter below a JS string's length doesn't wrap", async () => {
  const dir = fixture("string-length-counter");
  writeFileSync(join(dir, "lib.rs"), `pub fn count(format_string: &str) -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i + 1 < js::string::length(format_string) {
        if js::string::char_code_at(format_string, i) != 37.0 {
            i += 1;
            continue;
        }
        i += 1;
        n += 1;
        i += 1;
    }
    n
}

pub fn pairs(items: &[u32]) -> u32 {
    let mut s = 0;
    let mut i = 0;
    while i + 1 < items.len() {
        s ^= items[i] ^ items[i + 1];
        i += 1;
    }
    s
}

#[cfg_attr(rust_js, rust_js::link_name = "get length")]
#[allow(unused_variables)]
fn units(this: &[f64]) -> u32 {
    unreachable!()
}

pub fn skips(items: &[f64]) -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i + 1 < units(items) {
        if items[i as usize] != 0.0 {
            i += 1;
            continue;
        }
        n += 1;
        i += 2;
    }
    n
}

pub fn hops(mut i: u32, n: u32) -> (u32, u32) {
    let mut k = 0;
    while i + 1 < n {
        i += 2;
        k += 1;
    }
    (k, i)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  while (i + 1 < formatString.length) {\n");
  expect(js).toContain("    if (formatString.charCodeAt(i) !== 37) {\n      i++;\n      continue;\n    }\n    i++;\n    n = (n + 1) >>> 0;\n    i++;\n");
  expect(js).toContain("  while (i + 1 < items.length) {\n");
  expect(js).toMatch(/\n {4}i\+\+;\n {2}\}\n {2}return s;/);
  expect(js).toContain("  while ((i + 1) >>> 0 < items.length) {\n");
  expect(js).toContain("    i = (i + 2) >>> 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.count("a%b%c"), lib.count("%%%%"), lib.count(""), lib.count("%")]).toEqual([2, 2, 0, 0]);
  expect([lib.pairs([1, 2, 4]), lib.pairs([]), lib.pairs([5])]).toEqual([5, 0, 0]);
  expect([lib.skips([0, 1, 0, 0, 2]), lib.skips([])]).toEqual([2, 0]);
  // \`i + 1\` of \`u32::MAX\` wraps to 0, below 5, so \`i\` steps on from 1.
  expect(js).toContain("    i = (i + 2) >>> 0;\n    k = (k + 1) >>> 0;\n");
  expect(lib.hops(0xffffffff, 5)).toEqual([3, 5]);
});

test("a whole float's cast is the float from MIR too", async () => {
  const dir = fixture("whole-cast");
  writeFileSync(join(dir, "lib.rs"), `use js::string;
pub fn column(line: &str, substr: &str) -> u32 {
    let index = string::index_of(line, substr, None);
    if index == -1.0 {
        panic!("Could not find: '{substr}'");
    }
    index as u32 + string::length(substr)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return (index + substr.length) >>> 0;");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.column("abcd", "cd")).toBe(4);
});

// `Math`'s functions but `random` only compute their value: a range's end
// of one, `..len.saturating_sub(1)`, is read where it's used, not a
// `{ end }` object made first.
test("a slice to a computed end from MIR reads the end where it's used", async () => {
  const dir = fixture("slice-computed-end");
  writeFileSync(join(dir, "lib.rs"), `pub fn crumbs(segments: Vec<&str>) -> Vec<&str> {
    segments[..segments.len().saturating_sub(1)].to_vec()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("$slice(segments, 0, Math.max(segments.length - 1, 0))");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.crumbs(["a", "b", "c"]), lib.crumbs([])]).toEqual([["a", "b"], []]);
});
