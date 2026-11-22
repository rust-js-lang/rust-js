// The nullability rules of the architecture, split from
// architecture.test.ts at its cap (a test file is at most 1500 lines): how
// a place holds an `Option`'s `None` is one owner's answer (ADR 1096).
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

// Whether a place's `None` is `null`, and what a reader may trust of it,
// is `none_form`'s one answer, a `NoneForm` of a `NonePlace`, and
// `NoneForm::trust`'s (ADR 1096, #819). Nullability had the highest
// regression share of any area, 7 of 24 issues (#443 #477 #519 #539 #541
// #570 #571), each a writer and a reader, or two readers, asking
// `#[rust_js::nullable]`'s declarations apart. So the predicates that
// answer "is `None` `null` here" are asked only by the owner, which
// `bindings` defines them for, and only the owner makes what a write
// stores, `Storage::NullForNone`, and what a reader trusts,
// `js::OnlyNone::Null` and `js::OnlyNone::Undefined`; others only read them. A
// field's kept cell says it stores `null` where `cells.rs` makes it,
// `Known::NullStored`. The shape predicates, `is_optional`, `is_held`,
// `holds_value`, `holds_nested` and `nullable_option`, answer another
// question, a declared payload's shape, and aren't held here.
test("whether a `None` is `null` is none_form's alone", () => {
  const owner = "crates/rust-js-lower/src/representation/nones.rs";
  const lowering = [...files("crates/rust-js-lower/src").filter((f) => f !== "crates/rust-js-lower/src/lib.rs"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) =>
    file.endsWith(".rs") && file !== owner && file !== "crates/rust-js-lower/src/bindings.rs" && !file.startsWith("crates/rust-js-lower/src/bindings/"));
  // Its code, not its comments, nor its unit tests, whose fixtures make facts.
  const code = (file: string) => read(file).split("#[cfg(test)]")[0].split("\n").filter((line) => !/^\s*\/\//.test(line));
  const predicates = "is_nullable|null_where_made|declared_null_or_value|nullable_local|nullable_local_cell|nullable_local_option|null_result|nullable_result|optional_result|nullable_params|takes_nullable|takes_nullable_entries|holds_none_as_null|nullable_init|nulled_param";
  const asking = lowering.flatMap((file) =>
    code(file)
      .filter((line) => new RegExp(`(?<![\\w])(?:${predicates})\\(`).test(line) && !new RegExp(`\\bfn (?:${predicates})\\(`).test(line))
      .map((line) => `${file}: ${line.trim()}`));
  expect(asking).toEqual([]);
  // What's made of them: a reader's match of a storage or a test of an
  // answer, `matches!(none, js::OnlyNone::Null)` too, reads it; anything
  // else makes it.
  const reads = [/\.has\((?:js::)?Known::/, /== Some\((?:js::)?OnlyNone::/, /matches!\(\w+, (?:js::)?OnlyNone::(?:Null|Undefined)\)/, /matches!\(.*Storage::NullForNone/, /Handle\([^)]*Storage::NullForNone\)\s*=>/];
  const making = lowering.flatMap((file) =>
    code(file)
      .filter((line) => /\b(?:Storage::NullForNone|OnlyNone::Null|OnlyNone::Undefined)\b/.test(line) && !reads.some((read) => read.test(line)))
      .map((line) => `${file}: ${line.trim()}`));
  expect(making).toEqual([]);
  const nullStored = lowering.filter((file) => code(file).some((line) => /\.with\((?:js::)?Known::NullStored\)/.test(line)));
  expect(nullStored).toEqual(["crates/rust-js-lower/src/representation/cells.rs"]);
});

// What may look like `None` by its type is `options.rs`'s one answer,
// `may_look_like_none` (ADR 0051, amended): a nullable nested field's
// refusal re-derived it with a peel and a catalog of its own, which took a
// `Weak`, a `Pin` of a `()` and a `MaybeUninit` for values and read their
// `Some(None)` as a `Some`, wrong answers, and refused a class of no
// fields (#832). `bindings` asks it, never the shapes it's made of.
test("what may look like None is may_look_like_none's alone", () => {
  const asking = files("crates/rust-js-lower/src/bindings").concat("crates/rust-js-lower/src/bindings.rs").filter((file) => file.endsWith(".rs"))
    .flatMap((file) => read(file).split("\n")
      .filter((line) => !/^\s*\/\//.test(line) && /\b(?:holds_nullish|weak_pointee|rc_pointee|known_by_bounds)\(/.test(line))
      .map((line) => `${file}: ${line.trim()}`));
  expect(asking).toEqual([]);
});

// What a test proved of an `Option` where it's read is MIR's one answer,
// `NoneForms::presence`, asked where each use is lowered (#2112 C S4): an
// `unwrap()`'s receiver (`Given::receiver`), a value stored
// (`stored_given`). It was a fact stamped on the read's JS,
// `Known::Proved`/`ProvedDefined`, which copies carried past the test and
// tidy had to forget as it began (#1942); no such fact is made again.
test("what a test proved of a read is presence's, asked where it's used, never carried on its JS", () => {
  const sources = compilerRustFiles();
  const lines = (pattern: RegExp) => sources.flatMap((file) =>
    read(file).split("\n").filter((line) => !/^\s*\/\//.test(line) && pattern.test(line)).map(() => file));
  expect(lines(/Known::Proved|\.proved\(\)|\.proved_defined\(\)|forget_proved/)).toEqual([]);
  // The proofs are asked by `presence` alone, outside the analysis that
  // makes them.
  const askers = lines(/\.(?:proved_present|proved_defined|present_item_read|capture_read_proved|field_read_proved|ended_present)\(/)
    .filter((file) => !file.startsWith("crates/rust-js-mir/src/nulled"));
  expect([...new Set(askers)]).toEqual(["crates/rust-js-mir/src/none_forms.rs"]);
});

// That a test says an `Option` is there is MIR's alone to say (#1933):
// tidy reads none off the printed JS, no `x ?? d` nor `$unwrap(x)` it drops
// under a test of `x`, no `x ?? null` of a place an `&&`'s part shows, and
// no decision of its own counts such an answer. Its decided tests, its
// `present_in`, and its `nulled_as_they_are` went once MIR proved what
// they answered, React's port and every corpus case the same (ADR 0620,
// amended). What's left of tidy's is MIR's facts, kept where it moves a
// read (`alias_variables`' `only_none`).
test("tidy decides no Option is there from a test of it", () => {
  const tidy = files("crates/rust-js-mir/src/tidy").concat("crates/rust-js-mir/src/tidy.rs").filter((file) => file.endsWith(".rs"));
  const found = tidy.flatMap((file) =>
    read(file).split("\n")
      .filter((line) => !/^\s*\/\//.test(line) && /\bfn (?:present_in|nulled_as_they_are|shown_present|shown_defined|unnulled)\b|implies\(&known, &Expr::bin\(Op::LooseNe/.test(line))
      .map((line) => `${file}: ${line.trim()}`));
  expect(found).toEqual([]);
  expect(read("crates/rust-js-program/src/decisions.rs")).not.toMatch(/name: "tidy-(?:present-in|nulled-as-they-are)"/);
});

// What an `Option`'s `None` can be is MIR's answer (`NoneForms::place_none`,
// a `js::OnlyNone`), asked where a test or a store is lowered, never a
// fact riding a JS node (#2112 C S5, Campaign 3's class 4, #2200): the read
// stamps `Known::NoneNull` and `Known::NoneUndefined` decided tests and
// stores from what lowering wrote on the node, and are gone, with the
// `Facts` that read them.
test("no fact a JS node carries says what an Option's None is", () => {
  const known = read("crates/rust-js-program/src/js/known.rs");
  expect(known).not.toMatch(/^\s*None(?:Null|Undefined),/m);
  const facts = read("crates/rust-js-program/src/js.rs");
  expect(facts).not.toMatch(/\bfn (?:of_presence|only_none|same_none)\(/);
  const named = compilerRustFiles().filter((file) => file.endsWith(".rs") && /\bKnown::None(?:Null|Undefined)\b/.test(read(file)));
  expect(named).toEqual([]);
});


// A `Some` made where a test holds is a node of its own, `SomeWhere`, so
// what reads it, `presence`'s fold and a nullable place's write, asks its
// kind, not a fact (#2112 C S3c); a write through a handle carries MIR's
// answer on its statement (S4b). The mark that said a value was there,
// `Known::Present`, the one that said it was never `undefined`,
// `Known::Defined` (S4d), and the answer that asked the marks,
// `Undefined::ByFacts`, are gone (S4c): none comes back, nor a writer of a
// mark or an asker of the answer.
test("no mark of presence, nor an answer that asks marks", () => {
  const named = compilerRustFiles()
    .filter((file) => /\bKnown::(?:Present|Defined)\b|\bUndefined::ByFacts\b|\.present\(\)|\.defined\(\)|\bwith_none\(/.test(read(file)));
  expect(named).toEqual([]);
});

// What a nullable place's write is told of whether what it's given may be
// `undefined` (`js::Undefined`): lowering's writes ask MIR's answer,
// `Never`, or the form alone, `ByForm`; `store_again`'s re-store of every
// item's `x ?? null`, of what tidy joins, asks the form alone too, so a
// decision lowering made without MIR's answer shows as a move, never
// rescued by a mark (#2112). No pass asks a mark of a write: a write through a
// handle, whose place `tidy::values::handles` and `js::applied` learn past
// lowering, carries MIR's answer on its statement (`StmtFacts::written`,
// set by `Stmt::written`, #2112 S4b). A `Drop`
// guard's inlined body asks MIR's `nulled` answer of each field, its
// literal's layout's (`ObjectLayout::nulled`, `ArrayLayout::nulled`, #2112
// S4a, ADR 0841's reader 8). A trailing `Option` a binding call is given asks MIR's proof and
// its form alone too (`binding_calls.rs`' `surely_there`, #2112 S3f); and
// tidy's `tested_option` asks the `SomeWhere`'s `there` (S3d).
test("the marks are asked only where MIR isn't at hand", () => {
  expect(read("crates/rust-js-lower/src/drops/inlined.rs")).not.toMatch(/Known::(?:Defined|Present)/);
  expect(read("crates/rust-js-lower/src/std_types/cell/variables.rs")).toContain("store_again(js::Undefined::ByForm)");
});

