// What the compiler writes of a loop the program wrote, as it wrote it: a
// `while`'s test as its test, its body's `break` where it was written.
// Each test is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildNode, compiler, fixture, root, run } from "./support";

// ADR 0702: a `while` whose test calls, `!lines[s].contains(..)` of a
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

// ADR 0705 (amended): a `while` in a guard's scope, whose test calls, is
// `while (test) { .. }` once its scope is one `finally`. Its first tidy
// made `while (s < lines.length) { let tmp; try { tmp = ..; } catch ..
// if (tmp) { break; } s++; }`, its call apart in a statement's own `try`;
// tidy's stages run again read `tmp` in place, and the rest of the test
// the program wrote joins the loop's, as `while (true)`'s whole test does
// (ADR 0702). An `if` the program wrote first in the body, of a `break`
// alone, stays where it wrote it.
test("a while in a guard's scope is its test", async () => {
  const dir = fixture("guarded-whiles");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

thread_local! {
    static DROPS: Cell<u32> = const { Cell::new(0) };
}

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        DROPS.set(DROPS.get() + 1);
    }
}

pub fn drops() -> u32 {
    DROPS.get()
}

fn is_root(line: &str) -> bool {
    line.contains("Root")
}

pub fn root(lines: &[&str]) -> usize {
    let _restore = Restore;
    let mut s = 0;
    while s < lines.len() && !is_root(lines[s]) {
        s += 1;
    }
    s
}

pub fn written(lines: &[&str]) -> usize {
    let _restore = Restore;
    let mut s = 0;
    while s < lines.len() {
        if is_root(lines[s]) {
            break;
        }
        s += 1;
    }
    s
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('    while (s < lines.length && !isRoot(lines[s])) {\n      s++;\n    }\n');
  expect(js).toContain('    while (s < lines.length) {\n      if (isRoot(lines[s])) {\n        break;\n      }\n      s++;\n    }\n');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.root(["a", "Root"]), lib.root(["a"]), lib.root([])]).toEqual([1, 1, 0]);
  expect([lib.written(["a", "Root"]), lib.written(["a"]), lib.written([])]).toEqual([1, 1, 0]);
  expect(lib.drops()).toBe(6);
});

// ADR 0892: what can't throw, a `str`'s `includes` of Rust's types
// (`Known::Read`), needs no `finally`, so a guard's scope round a loop that
// only reads is no `try` at all: the loop reads as it would unguarded, its
// drop after it.
test("a guarded loop that only reads is no try", async () => {
  const dir = fixture("guarded-reads");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

thread_local! {
    static DROPS: Cell<u32> = const { Cell::new(0) };
}

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        DROPS.set(DROPS.get() + 1);
    }
}

pub fn drops() -> u32 {
    DROPS.get()
}

pub fn root(lines: &[&str]) -> usize {
    let _restore = Restore;
    let mut s = 0;
    while s < lines.len() && !lines[s].contains("Root") {
        s += 1;
    }
    s
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  while (s < lines.length && !lines[s].includes("Root")) {\n    s++;\n  }\n');
  expect(js).not.toContain("finally");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.root(["a", "Root"]), lib.root(["a"]), lib.root([])]).toEqual([1, 1, 0]);
  expect(lib.drops()).toBe(3);
});

// ADR 0705 (amended): a loop that ends a labeled block MIR made, `bb9: {
// if (..) { .. while (true) { .. break bb9; } } }`, as the last of loops
// each in the one before's way out is once `exits after` puts each after
// it, breaks as a loop does, and the block goes, as React's
// describeNativeComponentFrame matches its two stacks.
test("a loop that ends a block breaks as a loop", async () => {
  const dir = fixture("loop-ends-block");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn common(sample: Option<String>, control: Option<String>) -> String {
    if let Some(sample) = sample
        && let Some(control) = control
    {
        let sample_lines: Vec<&str> = sample.split('\\n').collect();
        let control_lines: Vec<&str> = control.split('\\n').collect();
        let mut s = 0;
        while s < sample_lines.len() && !sample_lines[s].contains("Root") {
            s += 1;
        }
        let mut c = control_lines.len() - 1;
        while s >= 1 {
            if sample_lines[s] != control_lines[c] {
                if s != 1 {
                    loop {
                        s -= 1;
                        if s == 0 || sample_lines[s] != control_lines[c] {
                            return sample_lines[s].to_string();
                        }
                    }
                }
                break;
            }
            s -= 1;
            c = c.saturating_sub(1);
        }
    }
    String::new()
}

fn item(t: Option<&i32>) -> Result<i32, String> {
    t.copied().ok_or_else(|| "end".to_string())
}

pub fn args(tokens: &[i32]) -> Result<Vec<i32>, String> {
    let mut it = tokens.iter().peekable();
    let mut args = Vec::new();
    if it.peek() != Some(&&0) {
        args.push(item(it.next())?);
        while let Some(&&1) = it.peek() {
            it.next();
            args.push(item(it.next())?);
        }
    }
    Ok(args)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    while (s >= 1) {\n");
  expect(js).not.toMatch(/\bbb\d+/);
  // A calculator's arguments: its `?` an early return, and no `continue`
  // where the loop's body ends anyway.
  expect(js).toContain("      args.push(result._0);\n    }\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.common("a\nb\nRoot\nx\ny", "q\nz\ny"), lib.common("a\nRoot", "Root"), lib.common(undefined, "a")]).toEqual(["b", "", ""]);
  expect([lib.args([2, 1, 3]), lib.args([2, 1]), lib.args([0])]).toEqual([{ TAG: "Ok", _0: [2, 3] }, { TAG: "Err", _0: "end" }, { TAG: "Ok", _0: [] }]);
});

// React's shared/ReactComponentStackFrame, as react-in-rust ports it
// (`test/ports/component-stack-frame`, copied with the modules it uses):
// describeNativeComponentFrame's loops, in its guard's `finally`, are
// upstream's, `while (s < sampleLines.length &&
// !sampleLines[s].includes('DetermineComponentFrameRoot')) { s++; }`, with
// no label round them.
test("React's describeNativeComponentFrame's loops are upstream's", () => {
  const node = buildNode();
  const out = fixture("component-stack-frame");
  const crates = ["js", "node", "webapi"].flatMap((lib) => ["--extern", `${lib}=${join(node, `lib${lib}.rmeta`)}`]);
  run([compiler, join(root, "test/ports/component-stack-frame/lib.rs"), "-o", join(out, "lib.js"), "--", ...crates, "-L", node]);
  const js = readFileSync(join(out, "ReactComponentStackFrame.js"), "utf8");
  expect(js).toContain(
    '      while (s < sampleLines.length && !sampleLines[s].includes("DetermineComponentFrameRoot")) {\n        s++;\n      }\n',
  );
  expect(js).toContain(
    '      while (c < controlLines.length && !controlLines[c].includes("DetermineComponentFrameRoot")) {\n        c++;\n      }\n',
  );
  expect(js).toContain("      while (s >= 1 && c >= 0) {\n");
  expect(js).not.toContain("$index");
  expect(js).not.toMatch(/\bbb\d+/);
});
