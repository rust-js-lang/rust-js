// What the compiler writes of numbers and indexes: a cast of a whole float, a
// division by a constant, and an enum's discriminant; what a body's integer
// ranges show is test/lowering-integer-ranges.test.ts. Each test is a program
// of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target, wrapsDisagreed } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0627: a number literal written with a base or an exponent is
// written as the program wrote it, `1e4`, `0xff`, where JS reads it as the
// same value: its type's suffix, its separators and a decimal's leading
// zeros aren't JS's. Any other
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

#[allow(clippy::zero_prefixed_literal)]
pub fn zeros() -> f64 {
    00e+10 + 007.5E2
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export const OWNER_STACK_LIMIT = 1e4;");
  expect(js).toContain("export const SEED = 0x811c9dc5;");
  expect(js).toContain("return n & 0xff;");
  expect(js).toContain("return 2.5e-3;");
  expect(js).toContain("return 5000;");
  expect(js).toContain("return -1e4;");
  expect(js).toContain("return n | 0b1010;");
  // A decimal's leading zeros aren't JS's, \`00e+10\`: one before its point.
  expect(js).toContain("return 0e10 + 7.5e2;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.OWNER_STACK_LIMIT, lib.SEED, lib.masked(0x1234), lib.small(), lib.whole(), lib.below(), lib.flags(1)]).toEqual([10000, 0x811c9dc5, 0x34, 0.0025, 5000, -10000, 11]);
});

// ADR 0627, amended: react-dom-bindings' EventSystemFlags, `export const
// IS_NON_DELEGATED = 1 << 1;`. A constant of literals and operators is as
// written where JS reads it as rustc's value; one JS reads otherwise, `1 <<
// 31` of a `u32`, negative in JS, is its value.
test("a constant expression of literals is as written", async () => {
  const dir = fixture("constant-expressions-as-written");
  writeFileSync(join(dir, "lib.rs"), `pub const IS_NON_DELEGATED: i32 = 1 << 1;
pub const IS_CAPTURE_PHASE: i32 = 1 << 2;
pub const HIGH: u32 = 1 << 31;
pub const MASK: u32 = (1 << 4) - 1;
pub const BELOW: i32 = -(1 << 3);
pub const NOT: i32 = !0;
pub const BITS: u8 = 0b1 << 3 | 0x1;
pub const ORDER: u32 = 1 | 2 << 2;
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export const IS_NON_DELEGATED = 1 << 1;");
  expect(js).toContain("export const IS_CAPTURE_PHASE = 1 << 2;");
  expect(js).toContain("export const HIGH = 2147483648;");
  expect(js).toContain("export const MASK = (1 << 4) - 1;");
  expect(js).toContain("export const BELOW = -(1 << 3);");
  expect(js).toContain("export const NOT = -1;");
  expect(js).toContain("export const BITS = (0b1 << 3) | 0x1;");
  expect(js).toContain("export const ORDER = 1 | (2 << 2);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.IS_NON_DELEGATED, lib.IS_CAPTURE_PHASE, lib.HIGH, lib.MASK, lib.BELOW, lib.NOT, lib.BITS, lib.ORDER]).toEqual([
    2, 4, 2147483648, 15, -8, -1, 9, 9,
  ]);
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
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "indexOf")] #[rust_js::whole = "-1..=4294967294"] fn index_of(this: &str, search: &str) -> f64 { unreachable!() }
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
fn halve(x: &mut f64) {
    *x /= 2.0;
}
pub fn lent(n: u8) -> u8 {
    let mut x = n as f64;
    halve(&mut x);
    x as u8
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return index;\n}");
  expect(js).toContain("export function unchecked(line, word) {\n  const index = line.indexOf(word);\n  return $f64ToInt(index, 0, 4294967295);\n}");
  expect(js).toContain("  index = -1;\n  return $f64ToInt(index, 0, 4294967295);\n}");
  expect(js).toContain("export function byte(n) {\n  const x = n;\n  return x;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.found("abc", "c"), lib.unchecked("abc", "z"), lib.setAgain("abc", "a"), lib.byte(7), lib.shifted("abc", "z"), lib.wide(7), lib.lent(3)]).toEqual([2, 0, 0, 7, 0, 7n, 1]);
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

pub fn shrunk(mut items: Vec<u32>) -> u32 {
    let mut sum = 0;
    for i in 0..items.len() {
        items.truncate(1);
        sum += items[i];
    }
    sum
}
`);
  expect(wrapsDisagreed(dir)).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  for (const read of ["items[0]", "items[1]", "items[i]", "items[i] + items[0]"]) expect(js).toContain(read);
  expect(js).toContain("$index(items, 2)");
  expect(js).toContain("$index(items, 0)");
  // A loop below a length its body changes, `truncate`'s `&mut`, checks.
  expect(js).toContain("$index(items, i)");
  expect(js.match(/\$index/g)?.length).toBe(9);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first([]), lib.first([4]), lib.second([1, 2]), lib.second([1]), lib.total([1, 2, 3]), lib.below([5, 6], 1), lib.below([5], 3)]).toEqual([undefined, 4, 2, 0, 6, 11, 0]);
  expect(() => lib.unchecked([1, 2])).toThrow();
  expect(() => lib.changed([1])).toThrow();
  for (const unknown of [() => lib.either([1]), () => lib.third([1, 2]), () => lib.after([]), () => lib.wrong([]), () => lib.other([]), () => lib.shrunk([1, 2])]) {
    expect(unknown).toThrow();
  }
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
  // The discriminant itself, not a test of each variant's.
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return $cmp(a, b);");
  expect(js).toContain("return { Red: 0, Green: 5, Blue: 6 }[c];");
});

// `min_value()` and `max_value()`, deprecated for `MIN` and `MAX`, which
// their bodies give, as a crate of crates.io's still calls them: num-integer's
// gcd asks them of `Self` in an impl, where MIR keeps the call (#327).
test("an integer's min_value() and max_value() are its MIN and MAX", async () => {
  const dir = fixture("min-max-value");
  writeFileSync(join(dir, "lib.rs"), `#![allow(deprecated)]
pub trait Extremes {
    fn at_an_end(&self) -> bool;
}
impl Extremes for i8 {
    fn at_an_end(&self) -> bool {
        *self == Self::min_value() || *self == Self::max_value()
    }
}
pub fn ends(n: i8) -> bool {
    n.at_an_end()
}
pub fn top() -> u16 {
    let f: fn() -> u16 = u16::max_value;
    f()
}
pub fn bottom() -> i128 {
    i128::min_value()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.ends(-128), lib.ends(5), lib.ends(127), lib.top(), lib.bottom()]).toEqual([true, false, true, 65535, -(2n ** 127n)]);
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
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("$copyOf(segments, 0, Math.max(segments.length - 1, 0))");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.crumbs(["a", "b", "c"]), lib.crumbs([])]).toEqual([["a", "b"], []]);
});

// ADR 0067: a range pattern's bound its type has anyway, \`i32::MIN..=-1\`'s
// lower one or \`200..=u8::MAX\`'s upper one, is no test: what's left is
// what the program says. A side that does something is still done.
test("a range pattern tests no bound its type has", async () => {
  const dir = fixture("range-bounds");
  writeFileSync(join(dir, "lib.rs"), `pub fn sign(n: i32) -> &'static str {
    match n {
        i32::MIN..=-1 => "negative",
        0 => "zero",
        _ => "positive",
    }
}

pub fn top(n: u8) -> bool {
    matches!(n, 200..=u8::MAX)
}

pub fn wide(n: i64) -> bool {
    matches!(n, i64::MIN..=-10)
}

pub fn kind(c: char) -> u8 {
    match c {
        '\\0'..='/' => 0,
        '\\u{80}'..=char::MAX => 1,
        _ => 2,
    }
}

// Its other side still runs.
#[allow(clippy::absurd_extreme_comparisons)]
pub fn popped(v: &mut Vec<u8>) -> bool {
    v.pop().unwrap_or(0) <= u8::MAX
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  if (n <= -1) {\n    return "negative";\n  }\n');
  expect(js).toContain("return n >= 200;");
  expect(js).toContain("return n <= -10n;");
  expect(js).toContain('  if (c <= "/") {\n    return 0;\n  }\n');
  expect(js).not.toContain("$cmp");
  const lib = await import(join(dir, "lib.js"));
  expect([-2147483648, -1, 0, 7].map(lib.sign)).toEqual(["negative", "negative", "zero", "positive"]);
  expect([0, 199, 200, 255].map(lib.top)).toEqual([false, false, true, true]);
  expect([-(2n ** 63n), -10n, -9n].map(lib.wide)).toEqual([true, true, false]);
  expect(["\0", "/", "0", "\x80", "\u{10FFFF}"].map(lib.kind)).toEqual([0, 0, 2, 1, 1]);
  const v = [1, 2];
  expect([lib.popped(v), v]).toEqual([true, [1]]);
});

// ADR 0614: a range pattern's two bounds are one test, as its arm is one
// pattern: \`score >= 90 && score <= 100\`, not an \`if\` in an \`if\`.
test("a range pattern's bounds are one test", async () => {
  const dir = fixture("range-arms");
  writeFileSync(join(dir, "lib.rs"), `pub fn grade(score: u32) -> char {
    match score {
        90..=100 => 'A',
        80..=89 => 'B',
        _ => 'F',
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  if (score >= 90 && score <= 100) {\n    return "A";\n  }\n  if (score >= 80 && score <= 89) {\n');
  const lib = await import(join(dir, "lib.js"));
  expect([100, 90, 89, 80, 79, 101].map(lib.grade)).toEqual(["A", "A", "B", "B", "F", "F"]);
});


// React's ReactEventPriorities: `EventPriority`, a `#[repr(transparent)]`
// `i32` whose order is derived, compared `a < b` as upstream's numbers
// are. The struct is its field in JS (ADR 0584), so its derived order is
// the field's, JS's `<`; an `f64`'s too, which `NaN` makes false either
// way. A hand-written order stays its impl's.
test("a transparent struct's derived order is its number's <", async () => {
  const dir = fixture("transparent-order");
  writeFileSync(join(dir, "lib.rs"), `use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EventPriority(i32);

pub fn is_higher_event_priority(a: EventPriority, b: EventPriority) -> bool {
    a.0 != 0 && a < b
}

pub fn at_least(a: EventPriority, b: EventPriority) -> bool {
    a >= b
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Ms(f64);

pub fn before(a: Ms, b: Ms) -> bool {
    a < b
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Reversed(i32);

impl PartialOrd for Reversed {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Reversed {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0)
    }
}

pub fn reversed_less(a: Reversed, b: Reversed) -> bool {
    a < b
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return a !== 0 && a < b;\n");
  expect(js).toContain("  return a >= b;\n");
  expect(js).toContain("export function before(a, b) {\n  return a < b;\n");
  expect(js).not.toContain("$cmp(a, b)");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.isHigherEventPriority(2, 8), lib.isHigherEventPriority(8, 2), lib.isHigherEventPriority(0, 8), lib.atLeast(8, 8)]).toEqual([true, false, false, true]);
  expect([lib.before(1, 2), lib.before(NaN, 2), lib.reversedLess(1, 2), lib.reversedLess(2, 1)]).toEqual([true, false, false, true]);
});

// ADR 0064 (amended, #346): a square of a variable, or of a field of one or
// of `this`, is `x * x`, read twice; but of a getter's read, which may do
// something, once, `$powi(window.scrollY, 2)`.
test("a square of a field is x * x, of a getter's read $powi", () => {
  const dir = fixture("field-squares");
  writeFileSync(join(dir, "lib.rs"), `pub struct Point {
    pub x: f64,
}

pub fn norm(point: &Point) -> f64 {
    point.x.powi(2)
}

pub fn scrolled(window: &webapi::Window) -> f64 {
    window.scroll_y().powi(2)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return point.x * point.x;\n");
  expect(js).toContain("  return $powi(window.scrollY, 2);\n");
});
