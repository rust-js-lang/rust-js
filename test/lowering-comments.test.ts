// Where the compiler writes a program's comments: a line comment after the
// code it's after where JS has more on that line (ADR 0733), and one in a
// macro before the `fn` it writes before the function (ADR 0735). Each
// test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS,
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// React's addValueToProperties: `desc = '…'; // ellipsis`, and
// `value.slice(0, 1023) + '…' // ellipsis`. The port's branches are
// one conditional, on one line: each comment is after its `"…"`, not
// both after the line, `// ellipsis // ellipsis`.
test("a line comment is after its code where JS has more on the line", async () => {
  const { js, lib } = await compiled("comment-ends-its-line", `pub fn described(value: f64) -> String {
    if value.is_nan() {
        "\\u{2026}".to_string() // ellipsis
    } else {
        (if value > 1024.0 {
            value.to_string() + "\\u{2026}" // ellipsis
        } else {
            value.to_string()
        }) + "!"
    }
}
`);
  expect(js).toContain('    ? "…" // ellipsis\n');
  expect(js).toContain('? $displayF64(value) + "…" // ellipsis\n');
  expect(js).not.toContain("// ellipsis // ellipsis");
  expect([lib.described(NaN as never), lib.described(2048 as never), lib.described(1 as never)]).toEqual(["…", "2048…!", "1!"]);
});

// React's binaryToComparableString: a comment before the macro, and one
// in it before the `fn` it exports, the port's `// GAP:`, are both the
// function's, in order (ADR 0735).
test("a comment in a macro before its fn is the function's", async () => {
  const { js, lib } = await compiled("comment-in-a-macro", `// A license.

// Turns a number into a string
// to compare.
js::export_default! {
    // GAP: upstream takes any view; this takes the one it can name.
    fn compared(value: f64) -> String {
        value.to_string()
    }
}
`);
  expect(js).toContain(
    "// Turns a number into a string\n// to compare.\n// GAP: upstream takes any view; this takes the one it can name.\nexport default function compared(value) {\n",
  );
  expect(lib.default(2 as never)).toBe("2");
});
