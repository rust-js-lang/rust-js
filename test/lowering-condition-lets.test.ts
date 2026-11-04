// A block's `let` in an `if`'s condition (#523, ADR 1074, amended): read
// once, by the test right after it, it's read there, in the condition's
// comma expression (the corpus's `condition_block_sequence.rs`, `held`),
// and so is one a later one's value reads, the last read first, a call's
// arguments read in their order; one read twice is set where it was, a
// `let` declared before the `if` (A1, the user's decision), and runs as
// Rust's does.
// Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run } from "./support";

beforeAll(buildReact, 600_000);

test("a condition's let read past a call is read in place, its arguments in order; one read twice is a let set in the condition", async () => {
  const dir = fixture("condition-lets-kept");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

fn bump(flags: &Cell<u32>) -> u32 {
    flags.set(flags.get() + 1);
    flags.get()
}

fn more(a: u32, b: u32) -> bool {
    a > b
}

// \`flags.get()\` is read before \`bump\` changes it, the call's first argument.
pub fn past_a_call(flags: &Cell<u32>, f: u32) -> u32 {
    if f & 1 != 0
        || (f & 2 != 0
            && (more(flags.get(), {
                let d = bump(flags);
                d + 1
            }) || {
                flags.set(f | 8);
                false
            }))
    {
        flags.set(flags.get() + 100);
    }
    flags.get()
}

pub fn read_twice(flags: &Cell<u32>, f: u32) -> u32 {
    if f & 1 != 0
        || (f & 2 != 0
            && (more(f * 4, {
                let d = flags.get();
                d * d
            }) || {
                flags.set(f | 8);
                false
            }))
    {
        flags.set(flags.get() + 100);
    }
    flags.get()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("(more(flags.value, (bump(flags) + 1) >>> 0) || ((flags.value = (f | 8) >>> 0), false))");
  expect(js).toContain("let d;\n  if (");
  expect(js).toContain("((d = flags.value),\n      more((f * 4) >>> 0, Math.imul(d, d) >>> 0)");
  const lib = await import(join(dir, "lib.js"));
  // What native Rust gives, each \`f\` from a flags of 7.
  const pastACall = [1, 2, 4, 50].map((f) => lib.pastACall({ value: 7 }, f));
  expect(pastACall).toEqual([107, 10, 7, 58]);
  const readTwice = [1, 2, 4, 50].map((f) => lib.readTwice({ value: f }, f));
  expect(readTwice).toEqual([101, 102, 4, 58]);
});
