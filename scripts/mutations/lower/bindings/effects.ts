// Mutations of src/lower/bindings/effects.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const refused = ["test/lowering-cell-flow.test.ts", "-t", "a collection's mark is refused"];

export const mutations: Mutation[] = [
  {
    name: "collection-mark-of-a-body",
    breaks:
      "`#[rust_js::keeps_in_collection]` on a function that isn't a binding, whose body is the program's, is taken (ADR 1045)",
    file: "src/lower/bindings/effects.rs",
    find: "        let refused = match function && super::is_binding(tcx, def) {",
    replace: "        let refused = match function {",
    tests: refused,
  },
  {
    name: "collection-mark-runs-anything",
    breaks:
      "`#[rust_js::keeps_in_collection]` on a binding that may run code, which may keep what it's given anywhere, is taken (ADR 1045)",
    file: "src/lower/bindings/effects.rs",
    find: "            true if !crate::lower::analysis::binding_runs_nothing(tcx, def) => Some(",
    replace: "            true if false => Some(",
    tests: refused,
  },
  {
    name: "collection-mark-of-no-collection",
    breaks:
      "`#[rust_js::keeps_in_collection]` on a binding with no JS object it's called on or gives is taken (ADR 1045)",
    file: "src/lower/bindings/effects.rs",
    find: "    recognition.is_js_object(held).then_some(held)",
    replace: "    Some(held)",
    tests: refused,
  },
];
