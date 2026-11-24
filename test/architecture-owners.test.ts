// The table of semantic owners, docs/architecture.md's, is the one
// authority on who answers what, and complete for the front end (#2162,
// ADR 1175): one day found four questions each answered twice, an optional
// parameter's, a value made knowing its back, std's own call and a `Vec`'s
// `IntoIter`, while the table's check asked only that the APIs it named
// were there. scripts/owners.ts holds the rules; these run them.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";
import {
  answers,
  candidates,
  cells,
  completeness,
  exclusivity,
  fieldlessEnums,
  jsCode,
  ofOwner,
  ownerFound,
  ownersState,
  ownerTable,
  primitivePattern,
  returnType,
  rustCode,
  sourceTally,
  unreasoned,
} from "../scripts/owners";

const state = ownersState(root);

// Each row's owners are modules there are and each API it names is
// defined: seventeen of the functions it named had gone when lowering from
// MIR came (the third architecture audit), and two rows of the wrong shape
// were skipped by the check, one with no API cell, one split at a `|` in
// a code span.
test("each owner and API a row names is there", () => {
  expect(state.rows.length).toBeGreaterThan(200);
  expect(state.rows.flatMap((row) => row.owners.filter((owner) => !ownerFound(owner, state.files)).map((owner) => `${row.line}: ${owner}`))).toEqual([]);
  const defined = new Set<string>();
  for (const [file, code] of state.code) {
    const items = file.endsWith(".js") ? /\bfunction\s+([\w$]+)/g : /\b(?:fn|struct|enum|trait|type|const|static)\s+(\w+)/g;
    for (const m of code.matchAll(items)) defined.add(m[1]!);
  }
  const isModule = (name: string) => /\.(?:rs|js)$|\/$/.test(name);
  const missing = state.rows.flatMap((row) =>
    row.api
      .filter((api) => (isModule(api) ? !ownerFound(api, state.files) : !defined.has(api.replace(/\(\)$/, "").replace(/^.*::/, ""))))
      .map((api) => `${row.line}: ${api}`),
  );
  expect(missing).toEqual([]);
});

// Every answer a module beyond its own may ask is a row's or a helper's,
// with no list of exceptions: the sweeps' ratchet reached 0 and went
// (ADR 1175, amended).
test("each answer visible outside its module is a row's API or a helper", () => {
  expect(state.all.length).toBeGreaterThan(1000);
  expect(state.both).toEqual([]);
  // Name it in a row of docs/architecture.md's "Semantic owners", under its
  // owner, or mark it `// helper: <reason>` above its `fn`.
  expect(state.unclassified).toEqual([]);
});

test("each row's primitive is read by its owners alone", () => {
  expect(state.outside).toEqual([]);
  expect(state.unread).toEqual([]);
});

test("a row is five cells, its owners and APIs the names it ticks, its source a class", () => {
  expect(cells("| Is it `a \\| b`? | `x.rs` | `f`, `T::g` | |")).toEqual(["Is it `a \\| b`?", "`x.rs`", "`f`, `T::g`", ""]);
  const page = ["# A", "| Question | Owner | API | Primitive | Source |", "|---|---|---|---|---|", "| Is it? | `x.rs`; `y/` | `f`, `T::g` | `tcx.q` | 1 |", "", "| not | a | row |"].join("\n");
  expect(ownerTable(page)).toEqual([
    { line: 4, question: "Is it?", owners: ["x.rs", "y/"], api: ["f", "T::g"], primitives: ["tcx.q"], guesses: [], source: 1, sourceSaid: "" },
  ]);
  // Its source, the weakest of its answers', and what's that weak (#2200).
  expect(ownerTable(page.replace("`tcx.q` | 1 |", "`tcx.q` | 4: `T::g` |"))[0]).toMatchObject({ source: 4, sourceSaid: "`T::g`" });
  // Every row has one: an empty cell, unclassified, is refused too.
  for (const wrong of ["", "6", "four", "4:", "4 `T::g`"]) expect(() => ownerTable(page.replace("`tcx.q` | 1 |", `\`tcx.q\` | ${wrong} |`))).toThrow("a Source cell");
  expect(sourceTally(ownerTable(page.replace("`tcx.q` | 1 |", "`tcx.q` | 3: `f` |")))).toEqual([0, 0, 1, 0, 0]);
  expect(sourceTally(ownerTable(page))).toEqual([1, 0, 0, 0, 0]);
  // What a primitive replaced, after "instead of": guesses (#2159).
  expect(ownerTable(page.replace("| `tcx.q` |", "| `$mark`, instead of `.items`, `typeof x.len` |"))[0]).toMatchObject({ primitives: ["$mark"], guesses: [".items", "typeof x.len"] });
  expect(ownerTable(page.replace("| `tcx.q` |", "| instead of `.items` |"))[0]).toMatchObject({ primitives: [], guesses: [".items"] });
  expect(() => ownerTable(page.replace("| `tcx.q` |", "|"))).toThrow("a row of 4 cells");
  expect(() => ownerTable(page.replace("`x.rs`; `y/`", "the lowering"))).toThrow("names no owner");
  expect(() => ownerTable("# A\n")).toThrow("no table of semantic owners");
});

// Campaign 3's ratchet (#2200, phase 3): a weak owner row, of a crate-wide
// fact keyed by type, a fact riding a JS node or a guess from JS shape,
// says why its source is enough, after ` — `, or is listed as still to do
// in test/owners-unreasoned.txt, which only shrinks: a new weak row can't
// join it, and a listed one given its reason, or moved to a stronger
// source, comes off. 46 were listed when it began (2026-11-24).
test("a weak owner row says why its source is enough, but those listed still to do, which only shrink", () => {
  const listed = readFileSync(join(root, "test/owners-unreasoned.txt"), "utf8").split("\n").filter((line) => line !== "" && !line.startsWith("//"));
  const now = unreasoned(state.rows);
  // Say why its source is enough after ` — `, or move it to a stronger one.
  expect(now.filter((row) => !listed.includes(row))).toEqual([]);
  // Given its reason, or moved: take its line off test/owners-unreasoned.txt.
  expect(listed.filter((row) => !now.includes(row))).toEqual([]);
  const page = ["# A", "| Question | Owner | API | Primitive | Source |", "|---|---|---|---|---|", "| Is it? | `x.rs` | `f` | | 4: `f` (`Known::F`) |", "| Is it? | `x.rs` | `g` | | 5: `g` (a shape) — reads the tree it built |", "| Is it? | `x.rs` | `h` | | 2 |"].join("\n");
  expect(unreasoned(ownerTable(page))).toEqual(["4: `f` (`Known::F`)"]);
});

test("an owner is its module, its submodules and its folder", () => {
  const files = ["crates/rust-js-lower/src/iterators.rs", "crates/rust-js-lower/src/iterators/eager.rs", "crates/rust-js-lower/src/drops/types.rs", "crates/rust-js-program/src/names.rs", "crates/rust-js-program/src/runtime/steps.js", "crates/rust-js-program/src/prepare/shadows.rs", "crates/rust-js-lower/src/iterators_more.rs"];
  expect(files.filter((file) => ofOwner("iterators.rs", file, files))).toEqual(["crates/rust-js-lower/src/iterators.rs", "crates/rust-js-lower/src/iterators/eager.rs"]);
  expect(files.filter((file) => ofOwner("drops/", file, files))).toEqual(["crates/rust-js-lower/src/drops/types.rs"]);
  expect(files.filter((file) => ofOwner("names.rs", file, files))).toEqual(["crates/rust-js-program/src/names.rs"]);
  expect(files.filter((file) => ofOwner("runtime/steps.js", file, files))).toEqual(["crates/rust-js-program/src/runtime/steps.js"]);
  expect(files.filter((file) => ofOwner("crates/rust-js-program/src/prepare/shadows.rs", file, files))).toEqual(["crates/rust-js-program/src/prepare/shadows.rs"]);
  expect(ownerFound("gone.rs", files)).toBe(false);
});

test("code is the source without its comments, literals and tests", () => {
  const source = ['let a = "x // y"; // z', "let c = '\"'; let l: &'a str = r#\"q \"\" p\"#;", "/* one", "two */ f(b'x', b\"y\")", "#[cfg(test)]", "mod tests { fn t() {} }"].join("\n");
  expect(rustCode(source)).toBe(['let a = ""; ', "let c = ' '; let l: &'a str = \"\";", "", " f(' ', \"\")", ""].join("\n"));
  expect(jsCode("const a = `x\n${y}`; // c\nf('q', \"r\") /* d */")).toBe("const a = ``\n; \nf('', \"\") ");
});

test("an answer is a bool, an Option of what isn't a JS tree, or an enum of answers", () => {
  const enums = fieldlessEnums(["pub enum Side { Front, Back }", "enum Held { One(u8), Two }"]);
  expect([...enums]).toEqual(["Side"]);
  for (const [type, is] of [
    ["bool", true],
    ["R<bool>", true],
    ["Result<Option<usize>, String>", true],
    ["Result<bool, Box<dyn Error>>", true],
    ["Result<(Side, u8), String>", false],
    ["Option<DefId>", true],
    ["Option<Expr>", false],
    ["R<Option<(Ty<'tcx>, Vec<Stmt>)>>", false],
    ["Option<js::Name>", false],
    ["Side", true],
    ["Held", false],
    ["Vec<bool>", false],
  ] as const) {
    expect([type, answers(type, enums)]).toEqual([type, is]);
  }
  const code = "pub fn f<F: Fn() -> bool>(g: impl Fn(u8) -> String) -> Option<(u8, bool)> where F: Copy { 0 }";
  expect(returnType(code, code.indexOf("f<") + 1)).toBe("Option<(u8, bool)>");
  expect(returnType("fn g(x: u8) { }", 4)).toBeUndefined();
});

test("a candidate is visible outside its module, and a helper's mark is above it", () => {
  const source = [
    "pub(crate) fn asks() -> bool { true }",
    "pub(super) fn beside() -> bool { true }",
    "/// Says.",
    "// helper: only its callers' shape",
    "#[inline]",
    "pub(crate) fn helped() -> R<bool> { Ok(true) }",
    "pub fn made() -> Option<Expr> { None }",
    "// helper: not this one's",
    "",
    "pub fn side() -> Option<Side> { None }",
    "fn private() -> bool { true }",
    "#[cfg(test)]",
    "mod tests { pub fn t() -> bool { true } }",
  ].join("\n");
  const found = candidates("crates/rust-js-lower/src/x/z.rs", source, new Set(["Side"])).map((c) => ({ ...c, file: "crates/rust-js-lower/src/x.rs" }));
  expect(found.map(({ name, helper, line }) => [name, helper, line])).toEqual([
    ["asks", false, 1],
    ["helped", true, 6],
    ["side", false, 10],
  ]);
  // A `pub(super)` of a module right under crates/rust-js-lower/src/ reaches the whole front end.
  expect(candidates("crates/rust-js-lower/src/x.rs", source, new Set(["Side"])).map((c) => c.name)).toEqual(["asks", "beside", "helped", "side"]);
  const files = ["crates/rust-js-lower/src/x.rs", "crates/rust-js-lower/src/y.rs"];
  const row = { line: 9, question: "", owners: ["x.rs"], api: ["asks", "Held::helped"], primitives: [], guesses: [], source: 1, sourceSaid: "" };
  const elsewhere = { ...found[0]!, file: "crates/rust-js-lower/src/y.rs" };
  expect(completeness([row], [...found, elsewhere], files)).toEqual({ unclassified: ["crates/rust-js-lower/src/x.rs side", "crates/rust-js-lower/src/y.rs asks"], both: ["crates/rust-js-lower/src/x.rs:6 helped"] });
});

test("a primitive is its owners' alone, and read by one of them", () => {
  expect(primitivePattern("tcx.fn_sig").test("let s = tcx.fn_sig(d);")).toBe(true);
  expect(primitivePattern("fn_sig").test("poly_fn_sig(d)")).toBe(false);
  expect(primitivePattern(".items !== undefined").test("if (x.items  !==  undefined)")).toBe(true);
  expect(primitivePattern("$sliced").test("$sliced2(x)")).toBe(false);
  const code = new Map([
    ["crates/rust-js-lower/src/x.rs", "tcx.fn_sig(d)"],
    ["crates/rust-js-lower/src/x/y.rs", "tcx.fn_sig(e)"],
    ["crates/rust-js-lower/src/z.rs", "let s = tcx.fn_sig(f);"],
  ]);
  const row = (primitives: string[], guesses: string[] = []) => ({ line: 3, question: "", owners: ["x.rs"], api: [], primitives, guesses, source: 1, sourceSaid: "" });
  expect(exclusivity([row(["tcx.fn_sig"])], code)).toEqual({ outside: ["`tcx.fn_sig` (line 3) in crates/rust-js-lower/src/z.rs"], unread: [] });
  expect(exclusivity([row(["tcx.gone"])], code)).toEqual({ outside: [], unread: ["`tcx.gone` (line 3)"] });
  // A guess a primitive replaced is its owners' alone too, though none reads it.
  expect(exclusivity([row([], ["tcx.fn_sig"])], code)).toEqual({ outside: ["`tcx.fn_sig` (line 3) in crates/rust-js-lower/src/z.rs"], unread: [] });
  expect(exclusivity([row([], ["tcx.gone"])], code)).toEqual({ outside: [], unread: [] });
});
