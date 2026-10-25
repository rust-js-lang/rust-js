// What the compiler writes of a std function or constant that's JS's
// `Math`'s, as a value (ADR 0887): the function or the constant itself,
// `Math.clz32` of `u32::leading_zeros` taken as a `fn`, `Math.LN2` of
// `std::f64::consts::LN_2`, and `js::is_defined` of one whether the host
// has it. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// React's react-reconciler/src/clz32.js, ported as it's written: `Math.clz32`
// where the host has it, else its fallback by `Math.log` and `Math.LN2`.
test("a std function that's Math's is its function, and is_defined asks of it", async () => {
  const dir = fixture("math-clz32");
  writeFileSync(join(dir, "lib.rs"), `// TODO: This is pretty well supported by browsers. Maybe we can drop it.

thread_local! {
    pub static CLZ32: fn(u32) -> u32 = if js::is_defined(u32::leading_zeros) {
        u32::leading_zeros
    } else {
        clz32_fallback
    };
}

// Count leading zeros.
// Based on:
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/clz32
const LOG: fn(f64) -> f64 = f64::ln;
const LN2: f64 = std::f64::consts::LN_2;
fn clz32_fallback(x: u32) -> u32 {
    let as_uint = x;
    if as_uint == 0 {
        return 32;
    }
    31 - (LOG(as_uint as f64) / LN2) as u32
}

pub fn count(x: u32) -> u32 {
    CLZ32.with(|clz32| clz32(x))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export const clz32 = typeof Math.clz32 !== "undefined" ? Math.clz32 : clz32Fallback;`);
  expect(js).toContain("const { log: LOG, LN2 } = Math;");
  expect(js).not.toContain("0.6931471805599453");
  const inputs = [0, 1, 3, 1000, 0x7fffffff, 0x80000000, 0xffffffff];
  const expected = [32, 31, 30, 22, 1, 0, 0];
  const lib = await import(join(dir, "lib.js"));
  expect(lib.clz32).toBe(Math.clz32);
  expect(inputs.map(lib.count)).toEqual(expected);
  // A host without it: the fallback, chosen where the module loads.
  copyFileSync(join(dir, "lib.js"), join(dir, "absent.js"));
  const clz32 = Math.clz32;
  try {
    (Math as { clz32?: unknown }).clz32 = undefined;
    const absent = await import(join(dir, "absent.js"));
    expect(absent.clz32).not.toBe(clz32);
    expect(inputs.map(absent.count)).toEqual(expected);
  } finally {
    Math.clz32 = clz32;
  }
});

// Where `Math`'s function isn't the Rust function for every value, its
// value is its call's arrow: `f64::round`'s half away from zero, an `f32`'s
// `sqrt` rounded to one, a `u8`'s `leading_zeros` of its 8 bits. A literal
// is the number written; a function item handed to an iterator, its arrow.
test("a std function that isn't Math's for every value is the arrow of its call", async () => {
  const dir = fixture("math-values");
  writeFileSync(join(dir, "lib.rs"), `use std::f64::consts::PI;

const WRITTEN: f64 = 3.141592653589793;

pub fn values(x: f64) -> Vec<f64> {
    let sqrt: fn(f64) -> f64 = f64::sqrt;
    let atan2: fn(f64, f64) -> f64 = f64::atan2;
    let round: fn(f64) -> f64 = f64::round;
    let single: fn(f32) -> f32 = f32::sqrt;
    let narrow: fn(u8) -> u32 = u8::leading_zeros;
    vec![
        sqrt(x),
        atan2(x, 1.0),
        round(x),
        f64::from(single(x as f32)),
        f64::from(narrow(x as u8)),
        PI * x,
        WRITTEN,
    ]
}

pub fn roots(v: Vec<f64>) -> Vec<f64> {
    v.into_iter().map(f64::sqrt).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  for (const line of [
    // Side by side, each by its property's name: one destructuring (ADR 0598, amended).
    "const { sqrt, atan2 } = Math;",
    "const round = (n) => $round(n);",
    "const single = (n) => Math.fround(Math.sqrt(n));",
    "const narrow = (n) => Math.clz32(n) - 24;",
    "Math.PI * x",
    "const WRITTEN = 3.141592653589793;",
    "return v.map((n) => Math.sqrt(n));",
  ]) {
    expect(js).toContain(line);
  }
  const lib = await import(join(dir, "lib.js"));
  expect(lib.values(-2.5)).toEqual([NaN, Math.atan2(-2.5, 1), -3, NaN, 8, -2.5 * Math.PI, Math.PI]);
  expect(lib.roots([4, 9])).toEqual([2, 3]);
});

// `js::is_defined` of a std function that isn't `Math`'s is refused, as one
// of what isn't a global's binding is.
test("js::is_defined of a std function that isn't Math's is refused", () => {
  const dir = fixture("math-undefined");
  writeFileSync(join(dir, "lib.rs"), `pub fn rounds() -> bool {
    js::is_defined(f64::round)
}
`);
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 60_000);
  expect([p.code === 0, p.stderr]).toEqual([false, expect.stringContaining("`js::is_defined` of what isn't a JS global's binding")]);
});
