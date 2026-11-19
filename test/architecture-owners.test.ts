// The table of semantic owners, docs/architecture.md's, is the one
// authority on who answers what, and complete for the front end (#2162,
// ADR 1175): one day found four questions each answered twice, an optional
// parameter's, a value made knowing its back, std's own call and a `Vec`'s
// `IntoIter`, while the table's check asked only that the APIs it named
// were there. scripts/owners.ts holds the rules; these run them.
import { expect, test } from "bun:test";
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
  UNCLASSIFIED,
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
// and what's neither is on the ratchet, which only falls (ADR 1175).
test("each answer visible outside its module is a row's API or a helper", () => {
  expect(state.all.length).toBeGreaterThan(1000);
  expect(state.both).toEqual([]);
  const listed = new Set(state.ratchet);
  const left = new Set(state.unclassified);
  // New: name it in a row of docs/architecture.md's "Semantic owners", under
  // its owner, or mark it `// helper: <reason>` above its `fn`.
  expect(state.unclassified.filter((line) => !listed.has(line))).toEqual([]);
  // Classified, renamed or gone: take its line off the ratchet.
  expect(state.ratchet.filter((line) => !left.has(line))).toEqual([]);
  expect(state.ratchet, `${UNCLASSIFIED} is sorted, each line once`).toEqual([...new Set(state.ratchet)].sort());
});

test("each row's primitive is read by its owners alone", () => {
  expect(state.outside).toEqual([]);
  expect(state.unread).toEqual([]);
});

test("a row is four cells, its owners and APIs the names it ticks", () => {
  expect(cells("| Is it `a \\| b`? | `x.rs` | `f`, `T::g` | |")).toEqual(["Is it `a \\| b`?", "`x.rs`", "`f`, `T::g`", ""]);
  const page = ["# A", "| Question | Owner | API | Primitive |", "|---|---|---|---|", "| Is it? | `x.rs`; `y/` | `f`, `T::g` | `tcx.q` |", "", "| not | a | row |"].join("\n");
  expect(ownerTable(page)).toEqual([{ line: 4, question: "Is it?", owners: ["x.rs", "y/"], api: ["f", "T::g"], primitives: ["tcx.q"] }]);
  expect(() => ownerTable(page.replace("| `tcx.q` |", "|"))).toThrow("a row of 3 cells");
  expect(() => ownerTable(page.replace("`x.rs`; `y/`", "the lowering"))).toThrow("names no owner");
  expect(() => ownerTable("# A\n")).toThrow("no table of semantic owners");
});

test("an owner is its module, its submodules and its folder", () => {
  const files = ["src/lower/iterators.rs", "src/lower/iterators/eager.rs", "src/lower/drops/types.rs", "src/names.rs", "src/runtime/steps.js", "src/prepare/shadows.rs", "src/lower/iterators_more.rs"];
  expect(files.filter((file) => ofOwner("iterators.rs", file, files))).toEqual(["src/lower/iterators.rs", "src/lower/iterators/eager.rs"]);
  expect(files.filter((file) => ofOwner("drops/", file, files))).toEqual(["src/lower/drops/types.rs"]);
  expect(files.filter((file) => ofOwner("names.rs", file, files))).toEqual(["src/names.rs"]);
  expect(files.filter((file) => ofOwner("runtime/steps.js", file, files))).toEqual(["src/runtime/steps.js"]);
  expect(files.filter((file) => ofOwner("src/prepare/shadows.rs", file, files))).toEqual(["src/prepare/shadows.rs"]);
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
    "pub(in crate::lower) fn helped() -> R<bool> { Ok(true) }",
    "pub fn made() -> Option<Expr> { None }",
    "// helper: not this one's",
    "",
    "pub fn side() -> Option<Side> { None }",
    "fn private() -> bool { true }",
    "#[cfg(test)]",
    "mod tests { pub fn t() -> bool { true } }",
  ].join("\n");
  const found = candidates("src/lower/x/z.rs", source, new Set(["Side"])).map((c) => ({ ...c, file: "src/lower/x.rs" }));
  expect(found.map(({ name, helper, line }) => [name, helper, line])).toEqual([
    ["asks", false, 1],
    ["helped", true, 6],
    ["side", false, 10],
  ]);
  // A `pub(super)` of a module right under src/lower/ reaches the whole front end.
  expect(candidates("src/lower/x.rs", source, new Set(["Side"])).map((c) => c.name)).toEqual(["asks", "beside", "helped", "side"]);
  const files = ["src/lower/x.rs", "src/lower/y.rs"];
  const row = { line: 9, question: "", owners: ["x.rs"], api: ["asks", "Held::helped"], primitives: [] };
  const elsewhere = { ...found[0]!, file: "src/lower/y.rs" };
  expect(completeness([row], [...found, elsewhere], files)).toEqual({ unclassified: ["src/lower/x.rs side", "src/lower/y.rs asks"], both: ["src/lower/x.rs:6 helped"] });
});

test("a primitive is its owners' alone, and read by one of them", () => {
  expect(primitivePattern("tcx.fn_sig").test("let s = tcx.fn_sig(d);")).toBe(true);
  expect(primitivePattern("fn_sig").test("poly_fn_sig(d)")).toBe(false);
  expect(primitivePattern(".items !== undefined").test("if (x.items  !==  undefined)")).toBe(true);
  expect(primitivePattern("$sliced").test("$sliced2(x)")).toBe(false);
  const code = new Map([
    ["src/lower/x.rs", "tcx.fn_sig(d)"],
    ["src/lower/x/y.rs", "tcx.fn_sig(e)"],
    ["src/lower/z.rs", "let s = tcx.fn_sig(f);"],
  ]);
  const row = (primitives: string[]) => ({ line: 3, question: "", owners: ["x.rs"], api: [], primitives });
  expect(exclusivity([row(["tcx.fn_sig"])], code)).toEqual({ outside: ["`tcx.fn_sig` (line 3) in src/lower/z.rs"], unread: [] });
  expect(exclusivity([row(["tcx.gone"])], code)).toEqual({ outside: [], unread: ["`tcx.gone` (line 3)"] });
});
