// What the compiler writes of a `&mut` to an object made a `&mut dyn
// Trait`, its pair (ADR 0049): `{ value, impl }`, the object itself, where
// the impl has `&mut self` as the object; where it's generic code's box,
// `impl<T> Trait for T`'s, a getter and a setter that writes the whole in
// place, `$assign`, so `*self = v` changes the caller's object (ADRs 0099,
// 0454, #968). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, run } from "./support";

beforeAll(buildCompiler, 600_000);

/** `rust` compiled in a fixture of `name`: its JS, and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// #968: a blanket impl's `reset` writes `*self`, the pair's `value`; a pair
// of `{ value: c, impl }` left `c` as it was. A concrete impl's entry takes
// the object out of the pair, `cReset_reset(arg0.value)`, and changes it in
// place itself: its pair is the object's, as it was.
test("a &mut dyn of an object is written in place where its impl is generic, and is the object where it's not", async () => {
  const { js, lib } = await compiled("dyn-pairs-written-whole", `#[derive(Debug, Default)]
pub struct C {
    n: i32,
}

pub trait Reset {
    fn reset(&mut self);
}

impl<T: Default> Reset for T {
    fn reset(&mut self) {
        *self = T::default();
    }
}

pub trait Bump {
    fn bump(&mut self);
}

impl Bump for C {
    fn bump(&mut self) {
        *self = C { n: self.n + 1 };
    }
}

fn reset(r: &mut dyn Reset) {
    r.reset();
}

fn bump(b: &mut dyn Bump) {
    b.bump();
}

pub fn generic() -> String {
    let mut c = C { n: 5 };
    reset(&mut c);
    format!("{c:?}")
}

pub fn concrete() -> String {
    let mut c = C { n: 5 };
    bump(&mut c);
    format!("{c:?}")
}
`);
  expect(js).toContain(`  reset({
    impl: tReset({ default: () => ({ n: 0 }) }),
    get value() {
      return c;
    },
    set value(value) {
      $assign(c, value);
    },
  });`);
  expect(js).toContain("bump({ value: c, impl: cBump });");
  // The setter writes `c` in place, never sets it: `c` stays a `const`.
  expect(js).not.toContain("let c");
  expect([lib.generic(), lib.concrete()]).toEqual(["C { n: 0 }", "C { n: 6 }"]);
});
