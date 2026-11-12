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

// Whether a call's `Result` is only given on by `?`, so its error may be
// thrown, is asked one way, `mir::result_use` (#1545, after #1524's
// `given_on`): a `write!` into a `Formatter` and a serde protocol call ask
// it. The files that read `?`'s `Try::branch` may only shrink; a new
// reader of what a call's `Result` is given to asks `result_use`.
test("what a call's Result is given to is asked of `mir::result_use` alone", () => {
  const reading = ["src/lower.rs", ...files("src/lower")]
    .filter((file) => file.endsWith(".rs") && read(file).includes("TryTraitBranch"));
  expect(reading).toEqual([
    "src/lower/analysis/fmt_failures.rs",
    "src/lower/mir/dispatch.rs",
    "src/lower/mir/patterns.rs",
    "src/lower/mir/result_use.rs",
    "src/lower/recognition/types.rs",
  ]);
  expect(["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs") && /fn given_on\b/.test(read(file)))).toEqual([]);
  for (const file of ["src/lower/mir/writer.rs", "src/lower/mir/dispatch.rs"]) expect(read(file)).toContain("result_use(");
});
