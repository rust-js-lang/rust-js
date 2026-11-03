// Mutations of src/lower/recursive_fns.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const perModule = ["test/corpus.test.ts", "-t", "^recursive_fns_per_module.rs$"];

export const mutations: Mutation[] = [
  {
    name: "recursive-fn-made-at-each-site",
    breaks:
      "each place that compares a recursive type makes its module's function again, which takes the first one's place, so the places before it call a function no module writes (#517)",
    file: "src/lower/recursive_fns.rs",
    find: "        if module_fns.made.contains_key(&key) {\n",
    replace: "        if false && module_fns.made.contains_key(&key) {\n",
    tests: perModule,
  },
  {
    name: "recursive-fn-of-any-type",
    breaks:
      "inside a recursive type's `==` function, another recursive type's `==`, `HirKind`'s in `Hir`'s, calls the outer one, `eqHir(a.kind, b.kind)` (#511, #517)",
    file: "src/lower/recursive_fns.rs",
    find: "        let key = (self.module, kind, self.tcx.erase_and_anonymize_regions(ty));\n",
    replace: "        let key = (self.module, kind, self.tcx.types.unit);\n",
    tests: ["test/corpus.test.ts", "-t", "^recursive_eq.rs$"],
  },
  {
    name: "recursive-fn-of-any-module",
    breaks:
      "a recursive type's `==` compared in two modules is written in one of them, and the other calls a function it doesn't have (#517)",
    file: "src/lower/recursive_fns.rs",
    find: "        let key = (self.module, kind, self.tcx.erase_and_anonymize_regions(ty));\n",
    replace: "        let key = (rustc_span::def_id::CRATE_MOD_ID, kind, self.tcx.erase_and_anonymize_regions(ty));\n",
    tests: perModule,
  },
  {
    name: "recursive-fn-hidden-by-a-variable",
    breaks:
      "a module's `eqExpr` is named as the variable `let eq_expr = a == b` of the function that makes it is, which hides it there: `const eqExpr = eqExpr(a, b)` throws (#517)",
    file: "src/lower/recursive_fns.rs",
    find: "        taken.extend(self.names.iter().cloned());\n",
    replace: "",
    tests: perModule,
  },
  {
    name: "recursive-fn-imports-unmerged",
    breaks:
      "a module's recursive `==` that calls another module's hand-written `PartialEq`, `setPartialEq_eq`, is written where nothing imports it: what it uses isn't its callers' too (#517)",
    file: "src/lower/recursive_fns.rs",
    find: "    into.references.extend(uses.references.iter().copied());\n",
    replace: "",
    tests: perModule,
  },
  {
    name: "recursive-fn-of-a-generic-type-shared",
    breaks:
      "a generic recursive type's `==`, `Tree<T>`'s in a function given `T`'s dictionary, is its module's function, which reads a dictionary it isn't given (#517)",
    file: "src/lower/recursive_fns.rs",
    find: "        if ty.has_param() || ty.has_aliases() {\n",
    replace: "        if false {\n",
    tests: perModule,
  },
];
