// Two places set the same variable or constant one after the other, `b =
// v; a = v;`, are one statement, `a = b = v`, where each is a variable or a
// plain field (ADR 0968); `test/corpus/assignments_chained.rs` holds what
// they do against native Rust. A binding's setter, which may do anything,
// stays a statement of its own. Each test is a program of its own,
// compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

test("a binding's setters set the same value as statements of their own", async () => {
  const dir = fixture("chained-setters");
  writeFileSync(
    join(dir, "lib.rs"),
    `#[cfg_attr(rust_js, rust_js::link_name = "set globalThis.top")]
fn set_top(_value: f64) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set globalThis.left")]
fn set_left(_value: f64) {
    unreachable!()
}

pub fn scroll(to: f64) {
    set_top(to);
    set_left(to);
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  globalThis.top = to;\n  globalThis.left = to;\n");
  const lib = await import(join(dir, "lib.js"));
  lib.scroll(3);
  expect([(globalThis as any).top, (globalThis as any).left]).toEqual([3, 3]);
});
