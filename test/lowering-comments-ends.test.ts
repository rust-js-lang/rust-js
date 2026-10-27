// Where the compiler writes a comment at the end of a list: after a
// parameter on its line, `start, // DEV-only`, and on lines of its own
// after a struct literal's last field, before its `}` (ADR 0708, amended).
// Each test is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

// React's ReactFiberCacheComponent and ReactPerformanceTrack write
// `startTime: number, // DEV-only` in a parameter list, and a comment
// after an object literal's last property: `{ name, start, // We don't
// know the owner. }`. Its port's comments were the function body's first
// line and above the `return`.
test("a comment after a parameter or a struct literal's last field is where it's written", async () => {
  const { js, lib } = await compiled("comments-at-list-ends", `pub struct Info {
    pub name: &'static str,
    pub start: f64,
}

pub fn track(
    name: &'static str,
    start: f64, // DEV-only
) -> Info {
    Info {
        name,
        start,
        // We don't know the owner.
    }
}

pub fn first(
    name: &'static str, // The name.
    start: f64,
) -> f64 {
    start + name.len() as f64
}

pub fn made(name: &'static str) -> Info {
    Info {
        name,
        // Not yet started.
        start: 0.0,
    }
}
`);
  expect(js).toContain(`export function track(
  name,
  start, // DEV-only
) {
  return {
    name,
    start,
    // We don't know the owner.
  };
}`);
  expect(js).toContain(`export function first(
  name, // The name.
  start,
) {`);
  // One before a field is before it, as before (ADR 0708).
  expect(js).toContain("  return {\n    name,\n    // Not yet started.\n    start: 0,\n  };\n");
  expect(lib.track("a" as never, 2 as never)).toEqual({ name: "a", start: 2 });
  expect(lib.first("ab" as never, 1 as never)).toBe(3);
});

// React's getActivityInstanceFromFiber ends `// TODO: Implement this on
// ActivityComponent.` then `return null;`. The port's tail `None` of a
// nullable result is `return null;`, where the `None`'s comments end: they
// were a block's of only comments there, printed after it (ADR 0862,
// amended).
test("a comment before a tail None is before its return", async () => {
  const { js, lib } = await compiled("comment-before-tail-none", `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn instance(tag: u8) -> Option<u32> {
    if tag == 31 {
        return Some(1);
    }
    // TODO: Implement this on ActivityComponent.
    None
}
`);
  expect(js).toContain("  // TODO: Implement this on ActivityComponent.\n  return null;\n}");
  expect([lib.instance(31 as never), lib.instance(1 as never)]).toEqual([1, null]);
});
