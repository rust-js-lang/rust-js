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
