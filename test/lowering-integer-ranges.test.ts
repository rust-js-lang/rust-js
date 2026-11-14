// What the compiler writes of what a body's integer ranges show: an index
// in bounds read unchecked, an operator that can't wrap JS's own, a counter
// below a length kept below it, a length held in a variable. Each test is a
// program of its own, compiled and run (ADRs 0576, 1137).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, fixture, performanceTrack, target, wrapsDisagreed } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

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
  expect(wrapsDisagreed(dir)).toEqual([]);
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
  writeFileSync(join(dir, "lib.rs"), `pub fn placeholders(format: &str) -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i + 1 < js::string::length(format) {
        if js::string::char_code_at(format, i) == 37.0 {
            n += 1;
            i += 1;
        }
        i += 1;
    }
    n
}

pub fn count(format_string: &str) -> u32 {
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

pub fn upto(n: u8) -> u8 {
    let mut i: u8 = 0;
    while i + 1 < n {
        i += 1;
    }
    i
}
`);
  expect(wrapsDisagreed(dir, ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target])).toEqual(["mir-only-wrap skips", "mir-only-wrap skips", "mir-only-wrap skips"]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // \`i + 1 < n\` puts \`i\` below \`n - 1\`, at most 253 of a \`u8\`: its \`+= 1\`
  // can't wrap, and it settles round the loop at most 254.
  expect(js).toContain("  while (i + 1 < n) {\n    i++;\n  }\n");
  // A JS string's length a length of the text: `i` at most it round the
  // loop, its `i + 1` and its steps of two in range (MIR's intervals).
  expect(js).toContain("  while (i + 1 < format.length) {\n");
  expect(js).toContain("    if (formatString.charCodeAt(i) !== 37) {\n      i++;\n      continue;\n    }\n    i++;\n    n = (n + 1) >>> 0;\n    i++;\n");
  expect(js).toContain("  for (let i = 0; i + 1 < items.length; i++) {\n");
  expect(js).toContain("    s = (s ^ (items[i] ^ items[i + 1])) >>> 0;\n  }\n  return s;");
  // `items[i]`'s check is made before each step, so `i` is at most the
  // slice's length round the loop, a length of `f64`s far below `u32`'s
  // most: `i + 1` can't wrap (MIR's intervals, ADR 1137, #1672).
  expect(js).toContain("  while (i + 1 < items.length) {\n");
  expect(js).toContain("    i = (i + 2) >>> 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.count("a%b%c"), lib.count("%%%%"), lib.count(""), lib.count("%")]).toEqual([2, 2, 0, 0]);
  expect([lib.pairs([1, 2, 4]), lib.pairs([]), lib.pairs([5])]).toEqual([5, 0, 0]);
  expect([lib.skips([0, 1, 0, 0, 2]), lib.skips([])]).toEqual([2, 0]);
  expect([lib.upto(0), lib.upto(1), lib.upto(255)]).toEqual([0, 0, 254]);
  // \`i + 1\` of \`u32::MAX\` wraps to 0, below 5, so \`i\` steps on from 1.
  expect(js).toContain("    i = (i + 2) >>> 0;\n    k = (k + 1) >>> 0;\n");
  expect(lib.hops(0xffffffff, 5)).toEqual([3, 5]);
});

// ADR 0576 (amended, #335): an integer is bound by another's value as by a
// slice's length: \`k < n\` puts \`n - k\` at least 1, so \`n - k - 1\` can't
// wrap; \`line + 1\`, even wrapped, is at most \`line + 1\`, so \`k <
// size\` puts \`line - k\` at least 0. Where \`k\` may pass \`n\`, or \`n\`
// changed since, each keeps its wrap; and a slice's length bounds an index
// before another's value does. A \`for k in 1..=n\` puts \`k\` from 1 to \`n\`.
test("an integer below another's value doesn't wrap", async () => {
  const dir = fixture("value-bounds");
  writeFileSync(join(dir, "lib.rs"), `pub fn reversed(n: u32) -> Vec<u32> {
    let mut found = Vec::new();
    for k in 0..n {
        found.push(n - k - 1);
    }
    found
}

pub fn coefficients(line: u32) -> Vec<u32> {
    let mut found = Vec::new();
    let size = line + 1;
    for k in 1..size {
        found.push(line - k + 1);
    }
    found
}

pub fn tested(n: u32, k: u32) -> u32 {
    if k < n { n - k - 1 } else { k - n }
}

pub fn past(n: u32) -> Vec<u32> {
    let mut found = Vec::new();
    for k in 0..n + 2 {
        found.push(n - k);
    }
    found
}

pub fn shrunk(mut n: u32, k: u32) -> u32 {
    if n > 0 {
        n -= 1;
        if k < n {
            return n - k - 1 - 1;
        }
    }
    0
}

pub fn through(n: u32) -> Vec<u32> {
    let mut found = Vec::new();
    for k in 1..=n {
        found.push((k - 1) * (n - k));
    }
    found
}

pub fn backwards(v: &[u32]) -> u32 {
    let mut s = 0;
    for i in (0..v.len()).rev() {
        s += v[i] * i as u32;
    }
    s
}

pub fn back_through(v: &[u32], n: usize) -> u32 {
    let mut s = 0;
    if n < v.len() {
        for i in (0..=n).rev() {
            s += v[i];
        }
    }
    s
}

pub fn sum_below(xs: &[u32], n: usize) -> u32 {
    let mut sum = 0;
    let mut i = 0;
    while i < xs.len() {
        if i < n {
            sum += xs[i];
        }
        i += 1;
    }
    sum
}
`);
  expect(wrapsDisagreed(dir)).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    found.push(n - k - 1);\n");
  expect(js).toContain("    found.push(line - k + 1);\n");
  expect(js).toContain("  return k < n ? n - k - 1 : k - n;\n");
  expect(js).toContain("    found.push((n - k) >>> 0);\n");
  expect(js).toContain("      return (n - k - 1 - 1) >>> 0;\n");
  expect(js).toContain("      sum = (sum + xs[i]) >>> 0;\n");
  // A range's \`rev()\` is the range's items (#356).
  expect(js).toContain("    s = (s + (Math.imul(v[i], i) >>> 0)) >>> 0;\n");
  expect(js).toContain("      s = (s + v[i]) >>> 0;\n");
  // \`k\` of \`1..=n\` is from 1 to \`n\` (#337).
  expect(js).toContain("    found.push(Math.imul(k - 1, n - k) >>> 0);\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.reversed(3), lib.coefficients(3), lib.coefficients(0xffffffff)]).toEqual([[2, 1, 0], [3, 2, 1], []]);
  expect([lib.tested(5, 2), lib.tested(5, 9), lib.past(1), lib.shrunk(5, 3), lib.sumBelow([1, 2, 3], 2), lib.through(3), lib.backwards([1, 2, 3]), lib.backThrough([1, 2, 3], 1)]).toEqual([2, 4, [1, 0, 0xffffffff], 0xffffffff, 3, [0, 1, 0], 8, 3]);
});

// ADR 0682: a JS string's `length` is at most 2^31 - 1, as every engine's
// is, so the sum of two is at most 2^32 - 2 and can't wrap: React's
// ReactSerializationErrors' `str.length + substr.length < 40` is written as
// upstream's. A third may pass `u32`'s most, so it keeps its wrap, and so
// does a `length - 1`, which is `u32::MAX` of an empty string.
test("two JS string lengths add without a wrap", async () => {
  const dir = fixture("string-length-sum");
  writeFileSync(join(dir, "lib.rs"), `use js::string::length;

pub fn short(str: &str, substr: &str) -> bool {
    length(substr) < 10 && length(str) + length(substr) < 40
}

pub fn total(a: &str, b: &str) -> u32 {
    length(a) + length(b)
}

pub fn three(a: &str, b: &str, c: &str) -> u32 {
    length(a) + length(b) + length(c)
}

pub fn last(s: &str) -> u32 {
    length(s) - 1
}
`);
  expect(wrapsDisagreed(dir, ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target])).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return substr.length < 10 && str.length + substr.length < 40;\n");
  expect(js).toContain("  return a.length + b.length;\n");
  expect(js).toContain("  return (a.length + b.length + c.length) >>> 0;\n");
  expect(js).toContain("  return (s.length - 1) >>> 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.short("hello", "lo"), lib.short("x".repeat(36), "four"), lib.short("", "ten units!")]).toEqual([true, false, false]);
  expect([lib.total("😀", "é"), lib.three("a", "bc", "def"), lib.last("abc"), lib.last("")]).toEqual([3, 6, 2, 0xffffffff]);
});

// ADR 0701: an `as` of an integer already in its target's range is the
// number as it is, `s` of `s as usize` under `s >= 0`, and a slice's
// length of items that take up room is at most `isize`'s most, so `len()
// as isize - 1` is `length - 1`. A counter below a length stays below it
// past the loop, so its `as isize` is itself too, and `i != xs.len()` of
// one at most the length puts it below. React's describeNativeComponentFrame
// matches two stacks from their ends so, its indices `isize`s that go
// below zero. Where nothing shows the sign, `i as usize` keeps its `>>> 0`
// and its check, and an `isize`'s `- 1` its `| 0`.
test("a signed index tested not negative is a JS index as it is", async () => {
  const dir = fixture("signed-index");
  writeFileSync(join(dir, "lib.rs"), `pub fn common(sample: &[&str], control: &[&str]) -> isize {
    let mut s = 0;
    while s < sample.len() && sample[s] != "Root" {
        s += 1;
    }
    let mut s = s as isize;
    let mut c = control.len() as isize - 1;
    if s as usize == sample.len() {
        s = sample.len() as isize - 1;
    }
    while s >= 1 && c >= 0 {
        if sample[s as usize] != control[c as usize] {
            return s;
        }
        s -= 1;
        c -= 1;
    }
    -1
}

pub fn at(items: &[i32], i: isize) -> i32 {
    items[i as usize]
}

pub fn down(s: isize) -> isize {
    s - 1
}

pub fn units(items: &[()]) -> isize {
    items.len() as isize - 1
}

pub fn guarded(items: &[i32], s: isize) -> i32 {
    if s as usize >= 1 && (s as usize) < items.len() { items[s as usize] } else { 0 }
}

pub fn past(items: &[i32]) -> i32 {
    let i = items.len() + 1;
    if i != items.len() { items[i] } else { 0 }
}
`);
  expect(wrapsDisagreed(dir)).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  let c = control.length - 1;\n");
  expect(js).toContain("  if (s === sample.length) {\n    s = sample.length - 1;\n  }\n");
  expect(js).toContain("    if (sample[s] !== control[c]) {\n      return s;\n    }\n    s--;\n    c--;\n");
  expect(js).not.toContain("s | 0");
  // Nothing shows `i`'s sign, nor `s`'s: `usize`'s, `isize`'s wraps.
  expect(js).toContain("  return $index(items, i >>> 0);\n");
  expect(js).toContain("  return (s - 1) | 0;\n");
  // Items of no size may be more than `isize`'s most.
  expect(js).toContain("  return ((items.length | 0) - 1) | 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.common(["a", "x", "Root", "m"], ["b", "Root", "m"])).toBe(2);
  expect(lib.common(["a", "x", "m"], ["b", "y", "m"])).toBe(1);
  expect(lib.common(["Root"], ["Root"])).toBe(-1);
  expect(lib.at([4, 5], 1)).toBe(5);
  expect(() => lib.at([4, 5], -1)).toThrow("index out of bounds");
  expect([lib.down(-(2 ** 31)), lib.down(0)]).toEqual([2 ** 31 - 1, -1]);
  // A test of an `as` that may change the number shows nothing of `s`.
  expect([lib.guarded([4, 5], 1), lib.guarded([4, 5], -1), lib.guarded([4, 5], 0)]).toEqual([5, 0, 0]);
  // One past the length isn't below it for not being the length.
  expect(() => lib.past([1])).toThrow("index out of bounds");
});

// ADR 0709: an `isize` counter from 0 below `len() as isize` is at most the
// length round its loop, not one below `isize`'s most, so `s as usize` is
// `s` and the index in bounds; past the loop, `s != len() as isize` puts it
// below the length, through the `as` that keeps it. React's ported
// `describeNativeComponentFrame` reads `sampleLines[s]` as upstream does.
test("a signed counter below a length stays below it past its loop", async () => {
  const dir = fixture("signed-counter");
  writeFileSync(join(dir, "lib.rs"), `pub fn root(lines: &[&str], control: &[&str]) -> isize {
    let mut s: isize = 0;
    while s < lines.len() as isize && !lines[s as usize].contains("Root") {
        s += 1;
    }
    if s == lines.len() as isize {
        return -1;
    }
    let mut c = control.len() as isize - 1;
    while s >= 1 && c >= 0 {
        if lines[s as usize] != control[c as usize] {
            return s;
        }
        s -= 1;
        c -= 1;
    }
    0
}

pub fn past(lines: &[&str]) -> usize {
    let mut s: isize = 0;
    while s < lines.len() as isize && !lines[s as usize].contains("Root") {
        s += 1;
    }
    lines[s as usize].len()
}
`);
  expect(wrapsDisagreed(dir)).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  while (s < lines.length && !lines[s].includes("Root")) {\n    s++;\n  }\n');
  expect(js).toContain("    if (lines[s] !== control[c]) {\n      return s;\n    }\n");
  // Past its loop it may be the length: checked where it's read.
  expect(js).toContain("$index(lines, s)");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.root(["a", "Root", "x"], ["b", "c", "x"])).toBe(1);
  expect(lib.root(["a", "b"], ["b"])).toBe(-1);
  expect(lib.root(["Root"], ["Root"])).toBe(0);
  expect(lib.past(["a", "Root"])).toBe(4);
  expect(() => lib.past(["a"])).toThrow("index out of bounds");
});

// ADR 0720: a labeled block's `break` is a jump the walk follows, past the
// block, as a loop's is, so a depth tested below a constant after one adds
// one without a wrap; and a `const` of an integer is its value, so a
// counter tested against `LIMIT` is as one tested against `100`. React's
// ported `addValueToProperties` adds to its `indent` and its count as
// upstream does.
test("a labeled block and a named constant keep what a body's integers can be", async () => {
  const dir = fixture("labeled-ranges");
  writeFileSync(join(dir, "lib.rs"), `const LIMIT: usize = 100;
const FLOOR: i32 = -5;

pub fn top(n: usize) -> usize {
    n >> (usize::BITS - 1)
}

pub fn lower(n: i32) -> i32 {
    if n > FLOOR { n - 1 } else { FLOOR }
}

pub fn describe(out: &mut Vec<String>, indent: usize, name: &str) {
    let desc = 'desc: {
        if name.len() > 3 {
            break 'desc "long".to_string();
        }
        if indent < 3 {
            describe(out, indent + 1, name);
        }
        "short".to_string()
    };
    out.push(desc);
}

pub fn counted(keys: &[String], out: &mut Vec<String>) -> usize {
    let mut seen = 0;
    for key in keys {
        if seen > LIMIT {
            break;
        }
        out.push(key.clone());
        seen += 1;
    }
    seen
}

pub fn joined(indent: usize) -> usize {
    'desc: {
        if indent >= 3 {
            break 'desc;
        }
    }
    indent + 1
}

pub fn deep(indent: usize) -> usize {
    'desc: {
        if indent < 3 {
            break 'desc;
        }
        return indent + 1;
    }
    indent
}
`);
  expect(wrapsDisagreed(dir)).toEqual(["mir-only-wrap top"]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("describe(out, indent + 1, name);");
  expect(js).toContain("    seen++;\n");
  expect(js).toContain("n > FLOOR ? n - 1 : FLOOR");
  // Of constants alone, the constant.
  // MIR's intervals prove `usize::BITS - 1` in range: JS's own `-` of
  // the constants rustc gives (ADR 1137).
  expect(js).toContain("return n >>> (32 - 1);");
  // Past a test it isn't below, its `+ 1` may wrap.
  expect(js).toContain("(indent + 1) >>> 0");
  const lib = await import(join(dir, "lib.js"));
  const out: string[] = [];
  lib.describe(out, 0, "ab");
  expect(out).toEqual(["short", "short", "short", "short"]);
  expect(lib.counted(Array.from({ length: 200 }, (_, i) => String(i)), [])).toBe(101);
  expect([lib.deep(1), lib.deep(5), lib.deep(4294967295)]).toEqual([1, 6, 0]);
  // What a \`break\` knows goes past the block: there it may be the most.
  expect([lib.joined(1), lib.joined(4294967295)]).toEqual([2, 0]);
  expect([lib.lower(-4), lib.lower(-5), lib.lower(2147483647)]).toEqual([-5, -5, 2147483646]);
});

// ADR 0746: a `push` makes a `Vec` at least one long, whatever was known
// of it, and a variable at least `n` that's at most a slice's length shows
// the slice at least `n` long. So React's ported addObjectDiffToProperties
// reads `properties[prevLength - 1]`, its `prevLength` read after a push,
// and addValueToProperties `propsKeys[0]` past `propsLength === 1`.
test("a length held in a variable shows the slice at least so long", async () => {
  const dir = fixture("held-length");
  writeFileSync(join(dir, "lib.rs"), `fn nested(properties: &mut Vec<(String, String)>, more: bool) {
    if more {
        properties.push(("x".to_string(), String::new()));
    }
}

pub fn diff(properties: &mut Vec<(String, String)>, more: bool) {
    properties.push(("key".to_string(), String::new()));
    let prev_length = properties.len();
    nested(properties, more);
    if prev_length == properties.len() {
        properties[prev_length - 1].1 = "unchanged".to_string();
    }
}

pub fn only_children(keys: &[String]) -> bool {
    let length = keys.len();
    length == 1 && keys[0] == "children"
}

pub fn first_of_some(keys: &[String]) -> String {
    let length = keys.len();
    if length >= 2 { keys[1].clone() } else { keys[0].clone() }
}
`);
  expect(wrapsDisagreed(dir)).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // `prevLength - 1` is JS's own: the `push` before made the vector at
  // least one long (#1700).
  expect(js).toContain('properties[prevLength - 1][1] = "unchanged";');
  expect(js).toContain('return length === 1 && keys[0] === "children";');
  expect(js).toContain("length >= 2 ? keys[1] : $index(keys, 0)");
  const lib = await import(join(dir, "lib.js"));
  const properties: [string, string][] = [];
  lib.diff(properties, false);
  lib.diff(properties, true);
  expect(properties).toEqual([["key", "unchanged"], ["key", ""], ["x", ""]]);
  expect([lib.onlyChildren(["children"]), lib.onlyChildren(["a"]), lib.onlyChildren([])]).toEqual([true, false, false]);
  // Where the length may be 0, its index is checked.
  expect(lib.firstOfSome(["ab", "c"])).toBe("c");
  expect(() => lib.firstOfSome([])).toThrow("index out of bounds");
  const of = performanceTrack();
  // As above, past its `push` (#1700).
  expect(of("addObjectDiffToProperties")).toContain("properties[prevLength - 1][1] =");
  expect(of("addValueToProperties")).toContain('(propsLength === 1 && propsKeys[0] === "children" && key == null)');
}, underLoad(60_000));

// A counter a loop breaks out of at a bound, which a `continue` brings back
// to the loop's head as it was, React's `estimateBandwidth`'s `count` and
// `addObjectToProperties`' `addedProperties`: widened to the constant it's
// compared with, not its type's, which no narrowing leaves, `count + 1` is
// JS's own (ADR 1137). And past a `push`, a vector's length at least one
// (#1700), until it's lent `&mut` again, written or pointed to raw.
test("a counter a loop breaks out of at a bound, and a length past a push, never wrap", async () => {
  const dir = fixture("counter-thresholds");
  writeFileSync(join(dir, "lib.rs"), `pub fn average(items: &[f64]) -> f64 {
    let mut count = 0;
    let mut total = 0.0;
    let mut i = 0;
    while i < items.len() {
        if items[i] < 0.0 {
            i += 1;
            continue;
        }
        total += items[i];
        i += 1;
        count += 1;
        if count > 10 {
            break;
        }
    }
    if count > 0 { total / f64::from(count) } else { 0.0 }
}

pub fn shown(keys: &[&str]) -> u32 {
    let mut added: u32 = 0;
    for key in keys {
        if !key.starts_with('_') {
            added += 1;
            if added >= 3 {
                break;
            }
        }
    }
    added
}

pub fn last(mut items: Vec<u32>) -> u32 {
    items.push(7);
    items[items.len() - 1]
}

pub fn cleared(v: &mut Vec<u32>, f: fn(&mut Vec<u32>)) -> usize {
    v.push(1);
    f(v);
    v.len() - 1
}

pub fn before(v: &mut Vec<u32>) -> usize {
    let n = v.len();
    v.push(1);
    n - 1
}

pub fn one_branch(v: &mut Vec<u32>, b: bool) -> usize {
    if b {
        v.push(1);
    }
    v.len() - 1
}

pub fn popped(mut v: Vec<u32>) -> usize {
    v.push(1);
    v.pop();
    v.len() - 1
}

pub fn through_raw(mut v: Vec<u32>) -> usize {
    let p = &raw mut v;
    v.push(1);
    unsafe { (*p).clear() };
    v.len() - 1
}
`);
  expect(wrapsDisagreed(dir)).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    count++;\n");
  expect(js).toContain("      added++;\n");
  expect(js).toContain("  return items[items.length - 1];\n");
  // Where the vector may be empty again, or the length is the one before
  // the push, `- 1` keeps its wrap.
  expect(js.match(/\(v\.length - 1\) >>> 0/g)).toHaveLength(4);
  expect(js).toContain("  return (n - 1) >>> 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.average(Array.from({ length: 20 }, (_, i) => i))).toBe(5);
  expect(lib.average([-1, 4])).toBe(4);
  expect(lib.shown(["a", "_b", "c", "d", "e"])).toBe(3);
  expect(lib.last([1, 2])).toBe(7);
  expect(lib.cleared([5], (v: number[]) => { v.length = 0; })).toBe(4294967295);
  expect(lib.before([])).toBe(4294967295);
  expect(lib.oneBranch([], false)).toBe(4294967295);
  expect(lib.popped([])).toBe(4294967295);
  expect(lib.throughRaw([])).toBe(4294967295);
});

// An integer `as` another whose value MIR's intervals show in its target's
// range is the same number, written as it is (ADR 0701, #1672 M3): a
// slice's length as an `i32`, below what an allocation holds, a `u32`
// below 200 as a `u8`, one tested not negative as a `u32`. Where it may
// not fit, it's converted; and of a 64-bit target, a `BigInt`, though its
// value fits.
test("an integer cast MIR's intervals show in its target's range is the number it casts", async () => {
  const dir = fixture("integer-casts");
  writeFileSync(join(dir, "lib.rs"), `pub fn width(s: &[u32]) -> i32 {
    s.len() as i32
}

pub fn low(n: u32) -> u8 {
    if n < 200 { n as u8 } else { 0 }
}

pub fn any(n: u32) -> u8 {
    n as u8
}

pub fn wide(n: u32) -> u64 {
    if n < 200 { n as u64 } else { 0 }
}

pub fn signed(n: i32) -> u32 {
    if n >= 0 { n as u32 } else { 0 }
}

pub fn negative(n: i32) -> u32 {
    n as u32
}
`);
  expect(wrapsDisagreed(dir)).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return s.length;\n");
  expect(js).toContain("  return n < 200 ? n : 0;\n");
  expect(js).toContain("  return n & 255;\n");
  expect(js).toContain("  return n < 200 ? BigInt(n) : 0n;\n");
  expect(js).toContain("  return n >= 0 ? n : 0;\n");
  expect(js).toContain("  return n >>> 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.width([1, 2, 3]), lib.low(199), lib.low(200), lib.any(300), lib.wide(5), lib.signed(-1), lib.negative(-1)]).toEqual([3, 199, 0, 44, 5n, 0, 4294967295]);
});

// An index an outer loop's `get` shows below its slice's length stays
// below it across an inner loop: widened at the inner loop's head, as it
// grows by the outer's turns, to the most an index of the slice is, not its
// type's, which no narrowing leaves (#1742). Each `+ 1` is JS's own, by MIR
// alone.
test("an outer loop's index stays below its slice's length across an inner loop", async () => {
  const dir = fixture("outer-index-inner-loop");
  writeFileSync(join(dir, "lib.rs"), `pub fn summed(rows: &[Vec<u32>]) -> u32 {
    let mut r = 0;
    let mut total: u32 = 0;
    while let Some(row) = rows.get(r) {
        let mut i = 0;
        while let Some(&n) = row.get(i) {
            total = total.wrapping_add(n);
            i += 1;
        }
        r += 1;
    }
    total
}
`);
  expect(wrapsDisagreed(dir)).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("      i++;\n");
  expect(js).toContain("    r++;\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.summed([[1, 2], [], [3]])).toBe(6);
});

// React's ReactPostPaintCallback, ported: `while let Some(callback) =
// CALLBACKS.with_borrow(|c| c.get(i).copied()) { callback(t); i += 1; }`.
// Where an item at `i` is there, `i` is below a slice's length, so below
// `usize`'s most, and `i += 1` can't wrap: `i++`, not `i = (i + 1) >>> 0`;
// of a slice's own `get(i)` too (ADR 0881, amended). A count with no such
// test still wraps.
test("an index an item is there at counts on without its wrap", async () => {
  const dir = fixture("present-index");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    static CALLBACKS: RefCell<Vec<&'static dyn Fn(u32) -> u32>> = const { RefCell::new(Vec::new()) };
}

fn twice(n: u32) -> u32 {
    n * 2
}

pub fn flushed(n: u32) -> u32 {
    CALLBACKS.with_borrow_mut(|callbacks| {
        callbacks.push(&twice);
        callbacks.push(&twice);
    });
    let mut total = 0;
    let mut i = 0;
    while let Some(callback) = CALLBACKS.with_borrow(|callbacks| callbacks.get(i).copied()) {
        total += callback(n);
        i += 1;
    }
    total + i as u32
}

pub fn summed(items: &[u32]) -> u32 {
    let mut total = 0;
    let mut i = 0;
    while let Some(&item) = items.get(i) {
        total += item;
        i += 1;
    }
    total
}

pub fn counted(n: usize) -> usize {
    let mut i = 0;
    while i != n {
        i += 1;
    }
    i
}

/// Where it isn't there, \`i\` may be any.
pub fn skipped(items: &[u32], i: usize) -> usize {
    if let Some(&item) = items.get(i) { item as usize } else { i + 1 }
}
`);
  expect(wrapsDisagreed(dir, ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target])).toEqual(["bridged flushed"]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // `summed`'s, and `flushed`'s, whose `get(i)` is in `with_borrow`'s
  // closure: its `Some` MIR's intervals don't carry out of it yet (#1701),
  // so the bridge takes THIR's verdict there (ADR 1137).
  expect(js.match(/    i\+\+;\n/g)).toHaveLength(2);
  expect(js).toContain("?? (i + 1) >>> 0");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.flushed(3 as never), lib.summed([1, 2, 3] as never), lib.counted(4 as never)]).toEqual([14, 6, 4]);
  expect([lib.skipped([7] as never, 0 as never), lib.skipped([7] as never, 4294967295 as never)]).toEqual([7, 0]);
});

// ADR 0576, amended: a `*const` of what a shared reference refers to
// changes nothing but a cell, by a call the walk sees: a `RefCell`'s vec
// cleared through one, its index below the length tested before is checked
// still, Rust's panic.
test("a const pointer to a cell keeps its variable unknown", async () => {
  const dir = fixture("const-pointer-to-cell");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

pub fn read(v: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let p: *const RefCell<Vec<u32>> = v;
    if i < v.borrow().len() {
        unsafe { (*p).borrow_mut().clear() };
        return v.borrow()[i];
    }
    0
}
`);
  expect(wrapsDisagreed(dir)).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    return $index(v, i);\n");
  const lib = await import(join(dir, "lib.js"));
  expect(() => lib.read([1, 2], 0)).toThrow("index out of bounds: the len is 0 but the index is 0");
});
