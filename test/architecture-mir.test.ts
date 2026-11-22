// The rules of the architecture about what MIR a body's lowering reads, split
// from architecture.test.ts at its cap (a test file is at most 1500 lines).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";
import { compilerRustFiles } from "../scripts/compiler-crates";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// Whether a body's local is a variable the program named is asked one way,
// `analysis::user_variable` (in `mir::locals` until #2299), which answers no of a body whose locals' kinds rustc
// cleared: rustc's own `LocalDecl::is_user_variable` panics of one, a `const
// fn` evaluated for a const generic's argument (#1281), and `back_stepped`
// asked it directly where `self_handles` asked it guarded.
test("a body's local is asked if the program named it by `analysis::user_variable` alone", () => {
  const asking = ["src/lower.rs", ...files("src/lower")]
    .filter((file) => file.endsWith(".rs") && file !== "src/lower/analysis/mir_reads/user_variable.rs")
    .filter((file) => /\.is_user_variable\(\)/.test(read(file)));
  expect(asking).toEqual([]);
  expect(read("src/lower/analysis/mir_reads/user_variable.rs")).toContain("matches!(decl.local_info, mir::ClearCrossCrate::Set(_)) && decl.is_user_variable()");
});

// Whether a call's `Result` is only given on by `?`, so its error may be
// thrown, is asked one way, `analysis::result_use` (#1545, after #1524's
// `given_on`): a `write!` into a `Formatter` and a serde protocol call ask
// it. The files that read `?`'s `Try::branch` may only shrink; a new
// reader of what a call's `Result` is given to asks `result_use`.
test("what a call's Result is given to is asked of `analysis::result_use` alone", () => {
  const reading = ["src/lower.rs", ...files("src/lower")]
    .filter((file) => file.endsWith(".rs") && read(file).includes("TryTraitBranch"));
  expect(reading).toEqual([
    "src/lower/analysis/fmt_failures.rs",
    "src/lower/analysis/mir_reads/result_use.rs",
    "src/lower/mir/dispatch.rs",
    "src/lower/mir/patterns.rs",
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
  expect(asking).toEqual(["src/lower/analysis/mir_reads/arguments_use.rs", "src/lower/mir/given_arguments.rs", "src/lower/mir/std_calls/written_into.rs", "src/lower/mir/std_calls.rs"]);
  expect(read("src/lower/mir/dispatch.rs")).toContain("self.makes_arguments_given_on(state, known, destination)");
});

// rustc's own drop elaboration of a body is taken in one place,
// `analysis/prepared.rs`' `with_elaborated` (#2112's change D), kept on the
// body's `Mir` for the shadow that compares it with rust-js's,
// `mir/elaborated.rs`' `--drops-shadow`, and what will consume it.
test("rustc's elaborated body is taken by `analysis/prepared.rs` alone", () => {
  const reading = ["src/lower.rs", ...files("src/lower")]
    .filter((file) => file.endsWith(".rs") && file !== "src/lower/analysis/prepared.rs")
    .filter((file) => /\bmir_drops_elaborated_and_const_checked\b/.test(read(file)));
  expect(reading).toEqual([]);
});

// What one write of a local in a body is, the one rvalue a statement assigns
// it or the one write, a call's too, that makes it, is asked of
// `body_queries/values.rs` alone (#2211): four answerers of it, each
// counting writes its own way, were three too many.
test("only body_queries/values.rs answers a local's one write", () => {
  const answers = compilerRustFiles()
    .filter((file) => file.endsWith(".rs"))
    .flatMap((file) =>
      [...read(file).matchAll(/\bfn (assigned_once|made_once|the_write|only_write|written_once|assigned_exactly_once)\b/g)].map(
        (m) => `${file}: ${m[1]}`,
      ),
    );
  expect(answers.filter((found) => !found.startsWith("src/lower/body_queries/values.rs"))).toEqual([]);
  expect(answers).toContain("src/lower/body_queries/values.rs: assigned_once");
  expect(answers).toContain("src/lower/body_queries/values.rs: the_write");
});

// Each item of the lowering has one path (#2294's crate split, as #2297's
// has no root re-exports): a module re-exports its own children's, never
// another module's, which would name it twice, at its owner and at the
// re-export. `mir` re-exported what #2299 moved to the base until its commit
// 3; those before the split, of `recognition`'s answers and of
// `representation::nones`', are listed, to go with it, and a new one fails.
const reExports = (path: string, text: string): string[] =>
  text.split("\n").flatMap((line) => /^\s*pub(\([^)]*\))? use (crate|super)::/.test(line) ? [`${path}:${line.trim()}`] : []);
test("the lowering re-exports no other module's item, each item one path", () => {
  expect(reExports("a.rs", "pub(in crate::lower) use crate::lower::analysis::Mir;\npub(super) use super::x::y;\npub use self::child::z;\nuse crate::lower::analysis::cfg;\nmod child;\npub use child::w;"))
    .toEqual(["a.rs:pub(in crate::lower) use crate::lower::analysis::Mir;", "a.rs:pub(super) use super::x::y;"]);
  const found = ["src/lower.rs", ...files("src/lower")]
    .filter((file) => file.endsWith(".rs"))
    .flatMap((file) => reExports(file, read(file)));
  expect(found).toEqual([
    "src/lower/analysis.rs:pub(super) use super::recognition::from_serde_derive;",
    "src/lower/analysis.rs:pub(super) use super::recognition::serde_impl;",
    "src/lower/representation/options.rs:pub(in crate::lower) use super::nones::{NoneForm, NonePlace, Writers, none_form, null_params};",
    "src/lower/representation.rs:pub(super) use super::recognition::ordering_value;",
    "src/lower/serde.rs:pub(super) use super::recognition::Json;",
    "src/lower/serde.rs:pub(super) use super::recognition::serde_trait;",
    "src/lower/traits.rs:pub(super) use super::recognition::operational;",
    "src/lower/traits.rs:pub(super) use super::recognition::implementable;",
    "src/lower/traits.rs:pub(super) use super::recognition::is_operator;",
  ]);
});

// The root, the base's, names nothing of `mir` or of the pipeline, the
// driver's side (#2299's commit 6), whose cut puts them in other crates:
// the driver asks `lower::pipeline` for what names `mir`, a session begun
// with its hooks and tidy's passes. And the root's facade, its `pub use`s
// of the base's modules, is the one path the driver has to each item:
// their modules are `pub` for `mir`, a crate of its own at the cut, so the
// driver's files never name them, `lower::<module>::`.
test("the root names neither mir nor the pipeline, and its facade is the driver's one path", () => {
  const code = (text: string) => text.split("\n").filter((line) => !/^\s*\/\//.test(line)).join("\n");
  const root = code(read("src/lower.rs"));
  expect(root.match(/(?<!rustc_middle::)\b(?:mir|pipeline)::[\w{]+/g) ?? []).toEqual([]);
  const facade = [...root.matchAll(/^pub(?:\(crate\))? use (\w+)::/gm)].map((m) => m[1]!);
  expect(facade.length).toBeGreaterThan(0);
  const driver = files("src").filter((file) => file.endsWith(".rs") && file !== "src/lower.rs" && !file.startsWith("src/lower/"));
  const reached = driver.flatMap((file) =>
    facade.filter((module) => new RegExp(`\\blower::${module}::`).test(code(read(file)))).map((module) => `${file} lower::${module}::`),
  );
  expect(reached).toEqual([]);
});
