// Mutations of src/lower/declarations/cells.rs (ADR 0093): what a field's
// declaration reads of the cell or `Option` it's written with.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cell-option-unwritten",
    breaks:
      "a nullable `RefCell<Option<Trace>>` field is declared as what the alias is, `[string, number][] | null`, not as written, `Trace | null` (rd414)",
    file: "src/lower/declarations/cells.rs",
    find: "                self.written_option(inner()?)\n",
    replace: "                None\n",
    tests: ["test/declarations-cells.test.ts"],
  },
  {
    name: "alias-option-unwritten",
    breaks:
      "an `Option` of an alias, an optional field's, isn't read as written, so it's the alias expanded",
    file: "src/lower/declarations/cells.rs",
    find: "            Res::Def(DefKind::Enum, did) if self.tcx.is_lang_item(did, LangItem::Option) => inner(),",
    replace:
      "            Res::Def(DefKind::Enum, did) if false && self.tcx.is_lang_item(did, LangItem::Option) => inner(),",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "aliases as the Rust names them",
    ],
  },
  {
    name: "cell-field-declared-any",
    breaks: "a `Cell` field is declared `any`, where it's the number it holds",
    file: "src/lower/declarations/cells.rs",
    find: "                if is_std_def(self.tcx, adt.did(), StdItem::Cell)\n                    || field.is_some_and(|field| self.ref_cells.holds_value(self.tcx, field))\n",
    replace:
      "                if field.is_some_and(|field| self.ref_cells.holds_value(self.tcx, field))\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations type what a module exports",
    ],
  },
  {
    name: "plain-refcell-declared-cell",
    breaks: "a field's `RefCell` held as its value is declared `{ value }`",
    file: "src/lower/declarations/cells.rs",
    find: "                    || field.is_some_and(|field| self.ref_cells.holds_value(self.tcx, field))\n                    || self.ref_cells.is_value_cell(self.tcx, ty) =>",
    replace:
      "                    || self.ref_cells.is_value_cell(self.tcx, ty) =>",
    tests: ["test/declarations.test.ts", "test/lowering-field-cells.test.ts"],
  },
];
