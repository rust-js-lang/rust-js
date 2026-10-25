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

// ADR 0891: JS's `<<` of an `i32` is an int32 already, so it's no wrap's,
// `1 << i`; and an operand JS's `&`, `|`, `^`, `~` or a shift takes modulo
// 2^32 itself needs no wrap of its own, `lanes & -mask`, `(a + b) & m`,
// `1 << (i + 1)`. What isn't such an operand keeps its wrap: `-lanes | 0`
// of `i32::MIN` is itself, and `(1 << 31) - 1` wraps to `i32::MAX`.
test("a shift and an operand of a bitwise operator are unwrapped", async () => {
  const { js, lib } = await compiled("bits-unwrapped", `pub fn lane(i: u32) -> i32 {
    1 << i
}

pub fn unmasked(lanes: i32, mask: i32) -> i32 {
    lanes & -mask
}

pub fn negated(lanes: i32) -> i32 {
    -lanes
}

pub fn equal_or_higher(lanes: i32) -> i32 {
    let index = 31 - (lanes as u32).leading_zeros();
    (1 << (index + 1)) - 1
}

pub fn masked(a: i32, b: i32, mask: i32) -> i32 {
    (a + b) & mask
}

pub fn flipped(a: i32, b: i32) -> i32 {
    !(a - b)
}

pub fn doubled(n: u32) -> u32 {
    n << 1
}

pub fn joined(a: i32, b: i32) -> i32 {
    (a | 1) & b
}
`);
  expect(js).toContain("  return 1 << i;\n");
  expect(js).toContain("  return lanes & -mask;\n");
  expect(js).toContain("  return -lanes | 0;\n");
  expect(js).toContain("  const index = (31 - Math.clz32(lanes)) >>> 0;\n  return ((1 << (index + 1)) - 1) | 0;\n");
  expect(js).toContain("  return (a + b) & mask;\n");
  expect(js).toContain("  return ~(a - b);\n");
  expect(js).toContain("  return (n << 1) >>> 0;\n");
  expect(js).toContain("  return (a | 1) & b;\n");
  const min = -2147483648;
  expect([0, 1, 30, 31, 32, 33].map((i) => lib.lane(i))).toEqual([1, 2, 1 << 30, min, 1, 2]);
  expect([0, 6, -1, min, 2147483647].map((n) => lib.unmasked(n, n))).toEqual([0, 2, 1, min, 1]);
  expect([lib.negated(min), lib.negated(5)]).toEqual([min, -5]);
  expect([0, 1, 5, 1 << 30, min, -1].map((n) => lib.equalOrHigher(n))).toEqual([0, 1, 7, 2147483647, 0, 0]);
  expect([lib.masked(2147483647, 1, -1), lib.flipped(min, 1), lib.doubled(0x40000000), lib.joined(2, 3)]).toEqual([min, min, 2147483648, 3]);
});

// ADR 0156, amended: clippy's `manual_isolate_lowest_one` asks for
// `lanes.isolate_lowest_one()`, React's `lanes & -lanes`; std's body is
// `self & self.wrapping_neg()`, lowered of the operators, so an `i32`'s is
// `lanes & -lanes`. `isolate_highest_one()` is std's `self & (MIN_BIT
// .wrapping_shr(self.leading_zeros()))`. What's read twice is read once.
test("isolate_lowest_one and isolate_highest_one are std's operators", async () => {
  const { js, lib } = await compiled("bits-isolated", `pub fn highest_priority_lane(lanes: i32) -> i32 {
    lanes.isolate_lowest_one()
}

pub fn highest_bit(lanes: i32) -> i32 {
    lanes.isolate_highest_one()
}

pub fn highest_bit_u32(lanes: u32) -> u32 {
    lanes.isolate_highest_one()
}

pub fn lowest_of_sum(a: i32, b: i32) -> i32 {
    (a + b).isolate_lowest_one()
}
`);
  expect(js).toContain("  return lanes & -lanes;\n");
  expect(js).toContain("  return lanes & (-2147483648 >> Math.clz32(lanes));\n");
  expect(js).toContain("  return (lanes & (2147483648 >>> Math.clz32(lanes))) >>> 0;\n");
  expect(js).toContain("  const value = (a + b) | 0;\n  return value & -value;\n");
  const min = -2147483648;
  expect([0, 6, -1, min, 12].map((n) => lib.highestPriorityLane(n))).toEqual([0, 2, 1, min, 4]);
  expect([0, 6, -1, min, 2147483647].map((n) => lib.highestBit(n))).toEqual([0, 4, min, min, 1 << 30]);
  expect([0, 6, 0xffffffff].map((n) => lib.highestBitU32(n))).toEqual([0, 4, 2147483648]);
  expect(lib.lowestOfSum(2147483647, 1)).toBe(min);
});
