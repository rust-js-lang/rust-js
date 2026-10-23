// What the compiler writes of a loop the program wrote, as it wrote it: a
// `while`'s test as its test, its body's `break` where it was written.
// Each test is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// ADR 0698: a `while` whose test calls, `!lines[s].contains(..)` of a
// `Vec`'s index, is `while (test) { .. }`, its test as written (ADR 0683),
// not `while (true) { if (s >= lines.length || ..) { break; } .. }`; and
// one whose body leaves by a `break` in an `if`, the rest stepping on, is
// `while (test) { if (t) { ..; break; } s--; }`, as React's
// describeNativeComponentFrame matches two stacks.
test("a while is its test, its break where it was written", async () => {
  const dir = fixture("written-whiles");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn root(lines: Vec<&str>) -> usize {
    let mut s = 0;
    while s < lines.len() && !lines[s].contains("Root") {
        s += 1;
    }
    s
}

pub fn differ(a: &[i32], b: &[i32]) -> usize {
    let mut s = a.len().min(b.len());
    let mut found = 0;
    while s >= 1 {
        if a[s - 1] != b[s - 1] {
            if s > 1 {
                found = s;
            }
            break;
        }
        s -= 1;
    }
    found
}

pub fn first_diff(a: &[i32], b: &[i32]) -> usize {
    let mut s = a.len().min(b.len());
    let mut found = 0;
    while s >= 1 {
        if a[s - 1] == b[s - 1] {
            s -= 1;
        } else {
            found = s;
            break;
        }
    }
    found
}

pub fn scan(rows: &[Vec<i32>]) -> i32 {
    let mut total = 0;
    'rows: for row in rows {
        let mut i = 0;
        while i < row.len() {
            if row[i] < 0 {
                i += 1;
                continue;
            }
            total += row[i];
            i += 1;
        }
        if total > 0 {
            break 'rows;
        }
    }
    total
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  while (s < lines.length && !lines[s].includes("Root")) {\n    s++;\n  }\n');
  expect(js).toContain(
    "  while (s >= 1) {\n    if ($index(a, s - 1) !== $index(b, s - 1)) {\n      if (s > 1) {\n        found = s;\n      }\n      break;\n    }\n    s--;\n  }\n",
  );
  // The program's `if` and its `else`, the `break` the `else`'s.
  expect(js).toContain(
    "  while (s >= 1) {\n    if ($index(a, s - 1) === $index(b, s - 1)) {\n      s--;\n    } else {\n      found = s;\n      break;\n    }\n  }\n",
  );
  const lib = await import(join(dir, "lib.js"));
  expect([lib.root(["a", "Root"]), lib.root(["a"]), lib.root([])]).toEqual([1, 1, 0]);
  expect([lib.differ([1, 2, 3], [1, 5, 3]), lib.differ([4, 2], [1, 2]), lib.differ([1], [1])]).toEqual([2, 0, 0]);
  expect([lib.firstDiff([1, 2, 3], [1, 5, 3]), lib.firstDiff([4, 2], [1, 2]), lib.firstDiff([1], [1])]).toEqual([2, 1, 0]);
  expect([lib.scan([[-1, 2], [3]]), lib.scan([[-1], [-2], [4, 1]]), lib.scan([])]).toEqual([2, 5, 0]);
});
