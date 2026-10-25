// What the compiler writes of an integer's bits, as JS writes a bit set's:
// a compound assignment of a bitwise operator or a shift, `lanes &= ~lane`;
// a shift and a negation without a wrap where JS's operator wraps anyway,
// `1 << i`, `lanes & -lanes`; and std's `isolate_lowest_one`. Each test is
// a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS, and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: (number | object)[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// React's ReactFiberLane: `lanes &= ~lane`, `root.pendingLanes |= lane`.
// A statement `t = t op v` of a bitwise operator or a shift is `t op= v`,
// as it is of `+`, of a variable, a cell's field and a field alike: the
// target is a path, read once, its object and key evaluated before the
// value either way.
test("a bitwise operator's or a shift's assignment is its compound", async () => {
  const { js, lib } = await compiled("bits-compound", `use std::cell::Cell;

pub struct Fiber {
    pub flags: i32,
}

pub struct Root {
    pub pending_lanes: Cell<i32>,
    pub current: Fiber,
}

pub fn removed(mut lanes: i32, lane: i32) -> i32 {
    lanes &= !lane;
    lanes |= 4;
    lanes ^= 2;
    lanes >>= 1;
    lanes
}

pub fn halved(mut n: u32) -> u32 {
    n >>= 1;
    n
}

pub fn marked(root: &mut Root, lane: i32) {
    root.pending_lanes.set(root.pending_lanes.get() | lane);
    root.current.flags &= lane;
}
`);
  expect(js).toContain("  lanes &= ~lane;\n  lanes |= 4;\n  lanes ^= 2;\n  lanes >>= 1;\n");
  expect(js).toContain("  n >>>= 1;\n");
  expect(js).toContain("  root.pendingLanes |= lane;\n");
  expect(js).toContain("  root.current.flags &= lane;\n");
  expect(js).not.toMatch(/(\w+) = \1 [&|^]/);
  expect(lib.removed(0b1111, 0b0001)).toBe(0b0110);
  expect(lib.removed(-1, 0)).toBe(-2);
  expect(lib.halved(0xffffffff)).toBe(0x7fffffff);
  const root = { pendingLanes: 1, current: { flags: 7 } };
  lib.marked(root, 6);
  expect(root).toEqual({ pendingLanes: 7, current: { flags: 6 } });
});
