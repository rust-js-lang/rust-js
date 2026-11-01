// Which of the program's comments the JS leaves out (ADR 0444, amended): a
// `// SAFETY:` comment, which says why an `unsafe` block holds, as clippy
// asks of one, where JS has no `unsafe`. Each test is a program of its own,
// compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS, its `.d.ts` and its module. */
async function compiled(
  name: string,
  rust: string,
): Promise<{ js: string; declarations: string; lib: Record<string, unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  return { js, declarations, lib: await import(join(dir, "lib.js")) };
}

// The user's decision (2026-11-01): a `// SAFETY:` comment, its lines after
// it, and one at a line's end, aren't in the JS; the comments around them
// are, and the blank line before them is the next code's.
test("a SAFETY comment is left out of the JS, the comments around it kept", async () => {
  const { js, declarations, lib } = await compiled("safety-comments", `use std::num::NonZeroU32;

pub fn resolved(dispatcher: Option<u32>) -> u32 {
    // Will result in a null access error if accessed outside render phase.
    // SAFETY: every caller is in render, where the dispatcher is set.
    unsafe { dispatcher.unwrap_unchecked() }
}

pub fn halved(n: u32) -> u32 {
    let doubled = n * 2;

    // SAFETY: \`doubled\` is twice a number, and \`| 1\` keeps it from zero,
    // so it's never zero.
    let nonzero = unsafe { NonZeroU32::new_unchecked(doubled | 1) };
    // Halved again.
    nonzero.get() / 2
}

pub fn kept(n: u32) -> u32 {
    let one = unsafe { NonZeroU32::new_unchecked(1) }; // SAFETY: one isn't zero.
    // Not a SAFETY: comment, which only one that begins so is.
    n + one.get()
}

// Of a number its caller gives.
// SAFETY: every caller gives one that isn't zero.
pub fn given(n: u32) -> u32 {
    unsafe { NonZeroU32::new_unchecked(n) }.get()
}
`);
  expect(js).not.toContain("SAFETY: every");
  expect(js).toContain(
    "export function resolved(dispatcher) {\n  // Will result in a null access error if accessed outside render phase.\n  return dispatcher;\n}",
  );
  expect(js).toContain(
    "export function halved(n) {\n  const doubled = (n * 2) >>> 0;\n\n  const nonzero = (doubled | 1) >>> 0;\n  // Halved again.\n",
  );
  expect(js).toContain("const one = 1;\n  // Not a SAFETY: comment, which only one that begins so is.\n");
  expect(js).toContain("// Of a number its caller gives.\nexport function given(n) {");
  expect(js).not.toMatch(/\/\/ SAFETY|so it's never zero|one isn't zero/);
  expect(declarations).toContain("// Of a number its caller gives.\nexport function given(n: number): number;");
  expect(declarations).not.toContain("SAFETY");
  expect([lib.resolved(3 as never), lib.halved(4 as never), lib.kept(2 as never)]).toEqual([3, 4, 3]);
});
