// What the compiler writes of a JS string's length, as its integer ranges
// show it: below 2^31 - 1, as every engine's string is, so a counter that
// steps below it doesn't wrap, and two such lengths add without a wrap.
// Split by concept from lowering-integer-ranges.test.ts (ADRs 0632, 0682).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, fixture, target, wrapsProved } from "./support";
import { withoutBoundary } from "./without-boundary";

beforeAll(buildReact, 600_000);

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

pub fn tail(format: &str) -> u32 {
    let mut i = 0;
    let mut last = 0;
    while i < js::string::length(format) {
        last = js::string::length(format) - i - 1;
        i += 1;
    }
    last
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

// An array's get length is at most 2^32 - 1, not a string's 2^31 - 1:
// i += 2 under i < units(items) may wrap, so it's no proved wrap.
pub fn strides(items: &[f64]) -> u32 {
    let mut i = 0;
    while i < units(items) {
        i += 2;
    }
    i
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
  expect(wrapsProved(dir, ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target])).toEqual(["proved-wrap count", "proved-wrap count", "proved-wrap count", "proved-wrap count", "proved-wrap pairs", "proved-wrap pairs", "proved-wrap pairs", "proved-wrap placeholders", "proved-wrap placeholders", "proved-wrap placeholders", "proved-wrap skips", "proved-wrap skips", "proved-wrap skips", "proved-wrap tail", "proved-wrap tail", "proved-wrap tail", "proved-wrap upto", "proved-wrap upto"]);
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  // \`i + 1 < n\` puts \`i\` below \`n - 1\`, at most 253 of a \`u8\`: its \`+= 1\`
  // can't wrap, and it settles round the loop at most 254.
  expect(js).toContain("  while (i + 1 < n) {\n    i++;\n  }\n");
  // A JS string's length a length of the text: `i` at most it round the
  // loop, its `i + 1` and its steps of two in range (MIR's intervals).
  expect(js).toContain("  while (i + 1 < format.length) {\n");
  expect(js).toContain("    if (formatString.charCodeAt(i) !== 37) {\n      i++;\n      continue;\n    }\n    i++;\n    n = (n + 1) >>> 0;\n    i++;\n");
  // Below the text's JS length, `i` leaves `length - i` at least 1: a
  // length of it, so its `- 1` can't wrap.
  expect(js).toContain("    last = format.length - i - 1;\n");
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
  expect([lib.tail("abc"), lib.tail(""), lib.tail("%")]).toEqual([0, 0, 0]);
  expect([lib.skips([0, 1, 0, 0, 2]), lib.skips([])]).toEqual([2, 0]);
  expect([lib.upto(0), lib.upto(1), lib.upto(255)]).toEqual([0, 0, 254]);
  // \`i + 1\` of \`u32::MAX\` wraps to 0, below 5, so \`i\` steps on from 1.
  expect(js).toContain("    i = (i + 2) >>> 0;\n    k = (k + 1) >>> 0;\n");
  expect(lib.hops(0xffffffff, 5)).toEqual([3, 5]);
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
  expect(wrapsProved(dir, ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target])).toEqual(["proved-wrap short", "proved-wrap three", "proved-wrap total"]);
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain("  return substr.length < 10 && str.length + substr.length < 40;\n");
  expect(js).toContain("  return a.length + b.length;\n");
  expect(js).toContain("  return (a.length + b.length + c.length) >>> 0;\n");
  expect(js).toContain("  return (s.length - 1) >>> 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.short("hello", "lo"), lib.short("x".repeat(36), "four"), lib.short("", "ten units!")]).toEqual([true, false, false]);
  expect([lib.total("😀", "é"), lib.three("a", "bc", "def"), lib.last("abc"), lib.last("")]).toEqual([3, 6, 2, 0xffffffff]);
});
