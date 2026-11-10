// The rules of the architecture about what MIR a body's lowering reads, split
// from architecture.test.ts at its cap (a test file is at most 1500 lines).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// Whether a body's local is a variable the program named is asked one way,
// `mir::locals::user_variable`, which answers no of a body whose locals' kinds rustc
// cleared: rustc's own `LocalDecl::is_user_variable` panics of one, a `const
// fn` evaluated for a const generic's argument (#1281), and `back_stepped`
// asked it directly where `self_handles` asked it guarded.
test("a body's local is asked if the program named it by `mir::locals::user_variable` alone", () => {
  const asking = ["src/lower.rs", ...files("src/lower")]
    .filter((file) => file.endsWith(".rs") && file !== "src/lower/mir/locals.rs")
    .filter((file) => /\.is_user_variable\(\)/.test(read(file)));
  expect(asking).toEqual([]);
  expect(read("src/lower/mir/locals.rs")).toContain("matches!(decl.local_info, mir::ClearCrossCrate::Set(_)) && decl.is_user_variable()");
});
