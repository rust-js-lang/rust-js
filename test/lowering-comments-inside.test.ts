// Where the compiler writes a comment inside an expression (ADR 0630): a
// block comment after the code it's after, where JS has that code on a line
// of more. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

// React's ReactFiberHotReloading: `if (payload._status === 1 /* Resolved;
// see ReactLazy */) {`. Its port's call, an argument a line as rustfmt
// writes it, `1, /* Resolved; see ReactLazy */`: the comment after its
// argument, inside the test, as upstream has it, not in the `if`'s block
// (ADR 0630, amended); a line comment there is the line's, after it.
test("a block comment after an argument's comma is after the argument", async () => {
  const { js, lib } = await compiled("comment-after-an-argument", `use js::Unknown;

pub fn resolved(payload: Option<&'static Unknown>, count: &mut u32) -> bool {
    if js::strict_equals(
        js::get(payload, "_status"),
        1, /* Resolved; see ReactLazy */
    ) {
        *count += 1;
        return true;
    }
    false
}

pub fn pending(payload: Option<&'static Unknown>) -> bool {
    js::strict_equals(
        js::get(payload, "_status"),
        0, // Pending
    )
}
`);
  expect(js).toContain("  if (payload._status === 1 /* Resolved; see ReactLazy */) {\n    count.value = ");
  expect(js).toContain("  return payload._status === 0; // Pending\n");
  const count = { value: 0 };
  expect([lib.resolved({ _status: 1 } as never, count as never), lib.resolved({ _status: 0 } as never, count as never), count.value]).toEqual([true, false, 1]);
  expect(lib.pending({ _status: 0 } as never)).toBe(true);
});

// React's isSameOriginFrame, `try { // Reading href can throw. return
// typeof iframe.contentWindow.location.href === 'string'; } catch (err) {
// return false; }`: its port's comment, first in the `catch_unwind`
// closure of one value, `unwrap_or(false)`'s, was before the `try` (ADR
// 0444, amended).
test("a comment before a catch_unwind closure's value is in the try", async () => {
  const { js, lib } = await compiled("comment-in-a-closures-value", `use std::panic::{AssertUnwindSafe, catch_unwind};

use js::{Kind, Unknown};

pub fn is_text_href(frame: Option<&'static Unknown>) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        // Reading a cross-origin frame's href throws.
        matches!(js::classify(js::get(js::get(frame, "location"), "href")), Kind::String(_))
    }))
    .unwrap_or(false)
}
`);
  expect(js).toContain(`  try {
    // Reading a cross-origin frame's href throws.
    return typeof frame.location.href === "string";
  } catch {
    return false;
  }`);
  expect([lib.isTextHref({ location: { href: "a" } } as never), lib.isTextHref(undefined as never)]).toEqual([true, false]);
});

// React's beginWork: `oldProps !== newProps || hasLegacyContextChanged() ||
// // Force a re-render if the implementation changed due to hot reload:
// (__DEV__ ? workInProgress.type !== current.type : false)`. Its port's
// comment, before an `||`'s operand of an `if`'s value, was lost (ADR 0444,
// amended). The `if`'s value is the `&&` it means, the user's decision (ADR
// 0879, amended).
test("a comment before an operand that's an if's value is before it", async () => {
  const { js, lib } = await compiled("comment-before-an-if-operand", `const DEV: bool = true;

pub fn changed(a: u8, b: u8, c: bool, t: u8, u: u8) -> bool {
    let mut did = false;
    if a != b
        || c
        // Force a re-render if the implementation changed due to hot reload:
        || (if DEV { t != u } else { false })
    {
        // If props or context changed, mark the fiber as having performed work.
        did = true;
    }
    did
}
`);
  expect(js).toContain(`    c ||
    // Force a re-render if the implementation changed due to hot reload:
    (DEV && t !== u)
  ) {
    // If props or context changed, mark the fiber as having performed work.
    did = true;`);
  expect([lib.changed(1 as never, 1 as never, false as never, 2 as never, 3 as never), lib.changed(1 as never, 1 as never, false as never, 2 as never, 2 as never)]).toEqual([true, false]);
});
