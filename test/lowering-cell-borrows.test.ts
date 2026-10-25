// What a `RefCell`'s borrow held across a binding's call meets: one that
// runs only what it's given, `console.error(..)` and `Math.random()`,
// given only numbers and text, runs no code of the crate's, so the borrow
// isn't counted and its cell is the array it holds; given an object, whose
// `toString` may be the crate's, it's counted, and panics as Rust's would.
// Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withBindings = [
  "--",
  "--extern",
  `js=${join(target, "libjs.rmeta")}`,
  "--extern",
  `webapi=${join(target, "libwebapi.rmeta")}`,
  "-L",
  target,
];

// ADR 0898: React's ReactFiberLane computes a lane's expiration time while
// it holds `root.expirationTimes`, and the computation may `console.error`.
test("a borrow held while a binding given only numbers and text runs isn't counted", async () => {
  const dir = fixture("binding-given-borrows");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use webapi::console;

pub struct Root {
    pub expiration_times: RefCell<Vec<f64>>,
    pub reported: RefCell<Vec<u32>>,
}

pub fn root() -> Root {
    Root {
        expiration_times: RefCell::new(vec![-1.0; 4]),
        reported: RefCell::new(Vec::new()),
    }
}

fn compute_expiration_time(lane: u32, current_time: f64) -> f64 {
    if lane == 0 {
        console::error(("Should have found matching lanes. This is a bug in React.", lane));
        return -1.0;
    }
    current_time + js::math::max((250.0, js::math::random()))
}

pub fn mark(root: &Root, index: usize, lane: u32, now: f64) {
    let mut expiration_times = root.expiration_times.borrow_mut();
    expiration_times[index] = compute_expiration_time(lane, now);
}

pub fn expiration_time(root: &Root, index: usize) -> f64 {
    root.expiration_times.borrow()[index]
}

pub fn report(root: &Root, value: &js::Unknown) {
    let mut reported = root.reported.borrow_mut();
    console::error(("%s", value));
    reported.push(1);
}

pub fn touch(root: &Root) {
    root.reported.borrow_mut().push(2);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const expirationTimes = root.expirationTimes;\n");
  expect(js).toContain("  return $index(root.expirationTimes, index);\n");
  expect(js).not.toContain("$borrowMut(root.expirationTimes");
  expect(js).toContain("$borrowMut(root.reported");
  const lib = await import(join(dir, "lib.js"));
  const error = console.error;
  const logged: unknown[][] = [];
  try {
    console.error = (...args: unknown[]) => {
      logged.push(args);
      args.forEach((arg) => String(arg));
    };
    const r = lib.root();
    lib.mark(r, 1, 0, 10);
    lib.mark(r, 2, 4, 10);
    expect([lib.expirationTime(r, 1), lib.expirationTime(r, 2)]).toEqual([-1, 260]);
    expect(logged).toEqual([["Should have found matching lanes. This is a bug in React.", 0]]);
    lib.report(r, { toString: () => "quiet" });
    expect(() => lib.report(r, { toString: () => (lib.touch(r), "touched") })).toThrow("already borrowed");
  } finally {
    console.error = error;
  }
});

// ADR 0899: React's ReactFiberLane picks a lane's index through `clz32`, a
// thread-local fn pointer of `Math.clz32` or its fallback, while it holds
// `root.entanglements`. What a fn pointer may be is each function the
// crate makes one of, where no value of its type comes from outside.
test("a borrow held while a fn pointer runs what the crate made it of isn't counted", async () => {
  const dir = fixture("fn-pointer-borrows");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    pub static CLZ32: fn(u32) -> u32 = if js::is_defined(u32::leading_zeros) {
        u32::leading_zeros
    } else {
        clz32_fallback
    };
}

fn clz32_fallback(x: u32) -> u32 {
    if x == 0 {
        return 32;
    }
    31 - x.ilog2()
}

pub struct Root {
    pub entanglements: RefCell<Vec<i32>>,
    pub given: RefCell<Vec<u32>>,
}

pub fn root() -> Root {
    Root {
        entanglements: RefCell::new(vec![1, 2, 4, 8]),
        given: RefCell::new(vec![0]),
    }
}

fn pick_arbitrary_lane_index(lanes: i32) -> usize {
    31 - CLZ32.with(|clz32| clz32(lanes as u32)) as usize
}

pub fn entangled_lanes(root: &Root, mut lanes: i32) -> i32 {
    let entanglements = root.entanglements.borrow();
    let mut entangled = lanes;
    while lanes > 0 {
        let index = pick_arbitrary_lane_index(lanes);
        entangled |= entanglements[index];
        lanes &= !(1 << index);
    }
    entangled
}

pub fn apply(root: &Root, f: fn(i32) -> i32) -> i32 {
    let given = root.given.borrow();
    f(given[0] as i32)
}

pub fn touch(root: &Root) {
    root.given.borrow_mut().push(1);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const entanglements = root.entanglements;\n");
  expect(js).not.toContain("$borrow(root.entanglements");
  // What JS calls `apply` with may be any function: counted.
  expect(js).toContain("$borrow(root.given, true)");
  const lib = await import(join(dir, "lib.js"));
  const r = lib.root();
  expect(lib.entangledLanes(r, 0b101)).toBe(0b101);
  expect(lib.apply(r, (n: number) => n + 1)).toBe(1);
  expect(() => lib.apply(r, () => (lib.touch(r), 0))).toThrow("already borrowed");
});
