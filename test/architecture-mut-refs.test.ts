// What a `&mut` is in JS, split from architecture.test.ts at its cap (a test
// file is at most 1500 lines): `mut_ref_form`'s one answer, which each
// operation asks by a method of its own (ADR 1086).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);
const owner = "src/lower/representation/mut_ref.rs";
const lowering = files("src/lower").filter((file) => file.endsWith(".rs") && file !== owner);
/** `file`'s lines of code, its comments left out. */
const code = (file: string) => read(file).split("\n").map((line) => line.replace(/\/\/.*$/, "")).filter((line) => line.trim() !== "");

// What a `&mut` to a type is in JS, a box, a generic code's box, the
// object, a guard's object, a value given as it is or the value itself, is
// `mut_ref_form`'s one answer, a `MutRefForm` (ADR 1086). The predicates
// it's made of are its module's alone: `Cell::from_mut` asked `is_boxable`
// where a parameter asked `is_cell_pointee`, which disagree on a type
// parameter (#614), and `retain_mut` in generic code asked `is_boxable`
// alone, giving a closure that writes `x.value` the item itself (#131).
test("a `&mut`'s JS form is mut_ref_form's alone", () => {
  const asking = lowering.flatMap((file) =>
    read(file).split("\n")
      .filter((line) => /\b(?:fn (?:is_boxable|is_generic_boxed|is_cell_pointee)\(|\.(?:is_boxable|is_generic_boxed|is_cell_pointee|written_in_place)\((?!\)))/.test(line) && !/\bstorage\s*\.written_in_place\(/.test(line))
      .map((line) => `${file}: ${line.trim()}`));
  expect(asking).toEqual([]);
});

// Each operation on a `&mut` asks its form by a method of `MutRefForm`'s,
// whose match names every form, so a new form is a compile error in the
// owner, not a silent default at a site that asked `== MutRefForm::Box`
// and chose for every other form (ADR 1086's step 3, #821): 17 std calls
// asked so, and gave generic code's box the value (#1007). No site outside
// the owner names a form, and none compares one, as it isn't comparable.
// A site that makes a form is listed, with why.
test("a `&mut`'s form is asked by MutRefForm's methods alone, each matching every form", () => {
  const made = [
    // What a trait's method's dictionary entry adapts: generic code's box,
    // whatever its impl's form, as the trait's generic code has it (ADR 0454).
    "src/lower/traits.rs: let form = |_: Ty<'tcx>| super::MutRefForm::GenericBox;",
  ];
  const naming = lowering.flatMap((file) =>
    code(file).filter((line) => /\bMutRefForm::/.test(line)).map((line) => `${file}: ${line.trim()}`));
  expect(naming).toEqual(made);
  const source = read(owner);
  expect(source).not.toMatch(/derive\([^)]*\bPartialEq\b[^)]*\)\]\s*pub\(in crate::lower\) enum MutRefForm\b/);
  // Its own methods match each form by name: no `_` arm, no `matches!`
  // or `==` that leaves the others to a default.
  const methods = source.slice(source.indexOf("impl MutRefForm {"), source.indexOf("\n}\n", source.indexOf("impl MutRefForm {")));
  expect(methods.length).toBeGreaterThan(0);
  expect(methods.split("\n").filter((line) => /\b_\s*=>|matches!\(|==\s*MutRefForm::/.test(line))).toEqual([]);
});
