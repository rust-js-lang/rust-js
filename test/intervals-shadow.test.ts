// The MIR intervals' shadow (ADR 1137, #1672): a check THIR's ranges leave
// out is logged where MIR's intervals don't prove it, so what the MIR
// analysis proves, and doesn't yet, is held here.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildCompiler, buildSerde, compiler, fixture, root } from "./support";
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

test("an array's index in a loop to its length is proved by MIR's intervals alone, a check THIR's facts keep: a mir-only line", () => {
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
`)).toEqual(["mir-only-bounds sum"]);
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
`)).toEqual([]);
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
`)).toEqual([]);
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
`)).toEqual([]);
});

test("a function that calls itself is given nothing of its outer calls: MIR proves nothing of its parameters THIR doesn't", () => {
  expect(shadowOf(`fn grow(n: u32, k: u32) -> u32 {
    if k == 0 { n } else { grow(n + 1, k - 1) }
}
pub fn main() {
    println!("{}", grow(1, 3));
}
`)).toEqual([]);
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
`)).toEqual(["mir-only-bounds slot"]);
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
`)).toEqual([]);
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
`)).toEqual(["bounds changed"]);
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
pub fn every_other(v: &[u32]) -> u32 {
    let mut total = 0;
    for i in (0..v.len()).step_by(2) {
        total += v[i];
    }
    total
}
`);
  const out = runSync([compiler, join(dir, "case.rs"), "-o", join(dir, "case.js"), "--", "--edition=2024", "--crate-type=lib"], dir, 60_000);
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
});

test("text's `len()` is at most what an allocation holds: two summed never wrap, a wrap THIR's ranges keep", () => {
  expect(shadowOf(`fn both(a: &str, b: &str) -> usize {
    a.len() + b.len()
}
pub fn main() {
    println!("{}", both("ab", "c"));
}
`)).toEqual(["mir-only-wrap both"]);
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

test("a macro's repeated index is joined to THIR's one index there or none: of `[$($v[$i]),*]`, MIR proves `v[k]` alone", () => {
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
`)).toEqual(["mir-only-bounds pair"]);
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
