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

// Whether a writer's `Debug` builders and derived `Debug` are written step
// by step, std's builders' model, or assembled at `finish`, is one fact,
// `prints.rs`' `through_sink` (#1533, #1557): its writers that may print,
// and those whose builder's step may fail, `fmt_failures`' answer, which it
// reads, never recomputes. The lowering reads it through
// `writes_through_a_sink` alone, where a builder's made, and only
// `display/streamed.rs` names the builders' helpers.
test("a builder is streamed where the sink fact says, and the fact is `prints.rs`' alone", () => {
  const lower = ["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs"));
  expect(lower.filter((file) => /\.through_sink\b/.test(read(file)))).toEqual(["src/lower/analysis/prints.rs"]);
  expect(lower.filter((file) => /prints\.writes_through_sink\(/.test(read(file)))).toEqual(["src/lower/display/printing.rs"]);
  expect(lower.filter((file) => /Taken::Streamed\(/.test(read(file)))).toEqual(["src/lower/mir/writer.rs"]);
  expect(lower.filter((file) => /"\$debug(List|Set|Map|Struct|Tuple)"/.test(read(file)))).toEqual(["src/lower/display/streamed.rs"]);
  const writer = read("src/lower/mir/writer.rs");
  const made = writer.indexOf("let taken = Taken::Streamed(builder);");
  expect(writer.lastIndexOf("if self.writes_through_a_sink(state) {", made)).toBeGreaterThan(writer.lastIndexOf("fn ", made));
  const prints = read("src/lower/analysis/prints.rs");
  expect(prints).toContain("failing.contains(&body.def_id)");
  expect(prints).not.toMatch(/failing_fns|makes_err|fmt_failures::/);
});

// Whether an `fmt::Arguments` a body makes is its text or a function of it
// is one fact from MIR, `mir::arguments_use` (#1565): the lowering asks it
// where the `Arguments` is made and where std is given one, and nowhere
// reconstructs it from the JS a `format_args!` became.
test("an Arguments' text or function is `mir::arguments_use`'s answer alone", () => {
  const lower = ["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs"));
  const asking = lower.filter((file) => /\barguments_use\(|\bholds_text\(/.test(read(file)));
  expect(asking).toEqual(["src/lower/mir/arguments_use.rs", "src/lower/mir/given_arguments.rs", "src/lower/mir/std_calls/written_into.rs", "src/lower/mir/std_calls.rs"]);
  expect(read("src/lower/mir/dispatch.rs")).toContain("self.makes_arguments_given_on(state, known, destination)");
});
