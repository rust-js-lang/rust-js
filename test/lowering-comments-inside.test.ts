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
