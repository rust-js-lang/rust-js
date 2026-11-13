// The MIR intervals' shadow (ADR 1137, #1672): a check THIR's ranges leave
// out is logged where MIR's intervals don't prove it, so what the MIR
// analysis proves, and doesn't yet, is held here.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildCompiler, compiler, fixture } from "./support";

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

test("an array's index in a loop to its length is proved by MIR's intervals alone", () => {
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
`)).toEqual([]);
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

test("a function that calls itself is given nothing of its outer calls: its parameters may be anything", () => {
  expect(shadowOf(`fn down(n: u32) -> u32 {
    if n == 0 { 0 } else { down(n - 1) + n }
}
pub fn main() {
    println!("{}", down(5));
}
`)).toEqual(["wrap down"]);
});
