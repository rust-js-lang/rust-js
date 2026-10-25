// Mutations of src/lower/bindings/nullable.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "optional-field-made-null",
    breaks: "an optional nullable field's read, which may be absent, is taken for `null` or a value, so a nullable field given it stores `undefined` (ADR 0810, amended)",
    file: "src/lower/bindings/nullable.rs",
    find: "    is_nullable(tcx, field) && !is_optional(tcx, field)\n",
    replace: "    is_nullable(tcx, field)\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "an optional nullable field"],
  },
  {
    name: "nullable-refcell-refused",
    breaks: "`#[rust_js::nullable]` of a field's `RefCell<Option<T>>` is refused (ADR 0863)",
    file: "src/lower/bindings/nullable.rs",
    find: "            if (cell(*adt, crate::lower::recognition::StdItem::Cell)\n                || cell(*adt, crate::lower::recognition::StdItem::RefCell))\n",
    replace: "            if cell(*adt, crate::lower::recognition::StdItem::Cell)\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "a nullable RefCell field"],
  },
];
