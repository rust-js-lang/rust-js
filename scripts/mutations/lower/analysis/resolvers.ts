// Mutations of src/lower/analysis/resolvers.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "resolve-of-a-mut-binding",
    breaks:
      "an executor's `mut resolve`, which may be another function by the time it's called, is taken for the promise's resolve, so a borrow held across it isn't counted and never panics `already borrowed` (rd654)",
    file: "src/lower/analysis/resolvers.rs",
    find: "    let hir::PatKind::Binding(hir::BindingMode(hir::ByRef::No, hir::Mutability::Not), _, _, None) = pat.kind else {\n",
    replace: "    let hir::PatKind::Binding(hir::BindingMode(hir::ByRef::No, _), _, _, None) = pat.kind else {\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "promise's resolve is given text"],
  },
];
