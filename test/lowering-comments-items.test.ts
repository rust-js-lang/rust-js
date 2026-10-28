// Where the compiler writes a comment among a module's items: before the
// next item that has JS, past one that has none, a `pub type`'s (ADR 0444,
// amended). Each test is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

// React's ReactFiberConfigWithNoHydration heads a section, `// Hydration
// (when unsupported)`, whose first lines are types, then its constants:
// Flow's JS keeps the heading before `export const supportsHydration =
// false;`. A `pub type` writes no JS, so the comment was lost.
test("a comment before an item with no JS is before the next item that has", async () => {
  const { js, lib } = await compiled("comment-before-a-type", `pub fn first() -> u8 {
    1
}

// Hydration (when unsupported)
pub type ActivityInstance = u8;
#[allow(dead_code)]
pub(crate) type SuspenseInstance = u8;
pub const SUPPORTS_HYDRATION: bool = false;
`);
  expect(js).toContain("// Hydration (when unsupported)\nexport const SUPPORTS_HYDRATION = false;");
  expect(lib.SUPPORTS_HYDRATION).toBe(false);
});

// React's getAttributeAlias, `new Map([['acceptCharset', 'accept-charset'],
// // HTML and SVG attributes, .. ['crossOrigin', 'crossorigin'], ..])`: a
// comment between an array's items is before the item after it, as it's
// written, where it was lost, or before the statement (ADR 0708, amended).
test("a comment between an array's items is before the next item", async () => {
  const { js, lib } = await compiled("comment-between-items", `use std::collections::HashMap;

thread_local! {
    static ALIASES: HashMap<&'static str, &'static str> = HashMap::from([
        ("acceptCharset", "accept-charset"),
        // HTML and SVG attributes, but the SVG attribute is case sensitive.
        ("crossOrigin", "crossorigin"),
        ("xHeight", "x-height"), // after an item
    ]);
}

pub fn alias(name: &str) -> String {
    ALIASES.with(|aliases| aliases.get(name).copied().unwrap_or(name).to_string())
}

pub fn pairs(x: u8) -> Vec<(u8, u8)> {
    vec![
        (x, 1),
        // Before the second.
        (x, 2),
    ]
}
`);
  expect(js).toContain(`  ["acceptCharset", "accept-charset"],
  // HTML and SVG attributes, but the SVG attribute is case sensitive.
  ["crossOrigin", "crossorigin"],
  ["xHeight", "x-height"], // after an item
]);`);
  expect(js).toContain("  return [\n    [x, 1],\n    // Before the second.\n    [x, 2],\n  ];");
  expect([(lib.alias as (n: string) => string)("crossOrigin"), (lib.pairs as (x: number) => number[][])(3)]).toEqual(["crossorigin", [[3, 1], [3, 2]]]);
});

// React's SchedulerMock re-exports its priorities last, `pub use
// crate::scheduler_priorities::IDLE_PRIORITY as unstable_IdlePriority;`,
// as upstream's export list is. Printed first (ADR 0240), at where it's
// written, the comments of a block of only comments before it, an empty
// `else`'s and `function forceFrameRate() { // No-op }`'s, printed above
// it.
test("a re-export printed first takes no comment of a block before it", async () => {
  const { js, lib } = await compiled("reexport-takes-no-comment", `use std::cell::Cell;

pub mod priorities {
    pub const IDLE_PRIORITY: u8 = 5;
}

thread_local! {
    static SHOULD_YIELD: Cell<bool> = const { Cell::new(false) };
}

pub fn after_continuation() -> bool {
    if SHOULD_YIELD.get() {
        return true;
    } else {
        // We keep flushing synchronously.
    }
    false
}

pub fn force_frame_rate() {
    // No-op
}

/// The idle priority, by upstream's name.
pub use crate::priorities::IDLE_PRIORITY as UNSTABLE_IDLE_PRIORITY;
`);
  expect(js).toContain("export function forceFrameRate() {\n  // No-op\n}");
  // The `else`'s, in its function, as it is without the re-export.
  expect(js.indexOf("// We keep flushing synchronously.")).toBeGreaterThan(js.indexOf("export function afterContinuation"));
  expect(js).toContain('/** The idle priority, by upstream\'s name. */\nexport { IDLE_PRIORITY as UNSTABLE_IDLE_PRIORITY } from "./priorities.js";');
  expect([lib.UNSTABLE_IDLE_PRIORITY, (lib.afterContinuation as () => boolean)()]).toEqual([5, false]);
});
