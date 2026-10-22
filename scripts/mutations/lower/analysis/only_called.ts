// Mutations of src/lower/analysis/only_called.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "named-as-value-only-called",
    breaks: "a function named as a value, `map(double)`, is taken as only its calls give it values, so `n * 2` doesn't wrap",
    file: "src/lower/analysis/only_called.rs",
    find: "                hir::ExprKind::Path(ref qpath) => self.values.extend(self.named(qpath, e.hir_id)),",
    replace: "                hir::ExprKind::Path(_) => {}",
    tests: ["test/lowering.test.ts", "-t", "only calls reach"],
  },
  {
    name: "exported-only-called",
    breaks: "a function a JS caller sees is taken as only the crate's calls give it values, so `half(0)` doesn't wrap",
    file: "src/lower/analysis/only_called.rs",
    find: "        def_id.as_local().is_some_and(|local| !visibilities.is_exported(local))",
    replace: "        def_id.as_local().is_some_and(|_| visibilities.is_exported(def_id.expect_local()) || true)",
    tests: ["test/lowering.test.ts", "-t", "only calls reach"],
  },
  {
    name: "only-called-walks-type-system-consts",
    breaks: "the analysis of what calls reach a function typechecks a const argument in a signature, which rustc can't before it lowers it, and crashes",
    file: "src/lower/analysis/only_called.rs",
    find: "        .filter(|&owner| !super::is_type_system_const(tcx, owner))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^const_argument_in_a_signature.rs$"],
  },
];
