// The nullability rules of the architecture, split from
// architecture.test.ts at its cap (a test file is at most 1500 lines): how
// a place holds an `Option`'s `None` is one owner's answer (ADR 1096).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

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
// `Known::NoneNull` and `Known::NoneUndefined`; others only read them. A
// field's kept cell says it stores `null` where `cells.rs` makes it,
// `Known::NullStored`. The shape predicates, `is_optional`, `is_held`,
// `holds_value`, `holds_nested` and `nullable_option`, answer another
// question, a declared payload's shape, and aren't held here.
test("whether a `None` is `null` is none_form's alone", () => {
  const owner = "src/lower/representation/nones.rs";
  const lowering = files("src/lower").filter((file) =>
    file.endsWith(".rs") && file !== owner && file !== "src/lower/bindings.rs" && !file.startsWith("src/lower/bindings/"));
  const code = (file: string) => read(file).split("\n").filter((line) => !/^\s*\/\//.test(line));
  const predicates = "is_nullable|null_where_made|declared_null_or_value|nullable_local|nullable_local_cell|nullable_local_option|null_result|nullable_result|optional_result|nullable_params|takes_nullable|takes_nullable_entries|holds_none_as_null|nullable_init|nulled_param";
  const asking = lowering.flatMap((file) =>
    code(file)
      .filter((line) => new RegExp(`(?<![\\w])(?:${predicates})\\(`).test(line) && !new RegExp(`\\bfn (?:${predicates})\\(`).test(line))
      .map((line) => `${file}: ${line.trim()}`));
  expect(asking).toEqual([]);
  // What's made of them: a reader's match of a storage or a test of a
  // fact reads it; anything else makes it.
  const reads = [/\.has\((?:js::)?Known::/, /== Some\((?:js::)?Known::/, /matches!\(.*Storage::NullForNone/, /Handle\([^)]*Storage::NullForNone\)\s*=>/];
  const making = lowering.flatMap((file) =>
    code(file)
      .filter((line) => /\b(?:Storage::NullForNone|Known::NoneNull|Known::NoneUndefined)\b/.test(line) && !reads.some((read) => read.test(line)))
      .map((line) => `${file}: ${line.trim()}`));
  expect(making).toEqual([]);
  const nullStored = lowering.filter((file) => code(file).some((line) => /\.with\((?:js::)?Known::NullStored\)/.test(line)));
  expect(nullStored).toEqual(["src/lower/representation/cells.rs"]);
});

// What may look like `None` by its type is `options.rs`'s one answer,
// `may_look_like_none` (ADR 0051, amended): a nullable nested field's
// refusal re-derived it with a peel and a catalog of its own, which took a
// `Weak`, a `Pin` of a `()` and a `MaybeUninit` for values and read their
// `Some(None)` as a `Some`, wrong answers, and refused a class of no
// fields (#832). `bindings` asks it, never the shapes it's made of.
test("what may look like None is may_look_like_none's alone", () => {
  const asking = files("src/lower/bindings").concat("src/lower/bindings.rs").filter((file) => file.endsWith(".rs"))
    .flatMap((file) => read(file).split("\n")
      .filter((line) => !/^\s*\/\//.test(line) && /\b(?:holds_nullish|weak_pointee|rc_pointee|known_by_bounds)\(/.test(line))
      .map((line) => `${file}: ${line.trim()}`));
  expect(asking).toEqual([]);
});
