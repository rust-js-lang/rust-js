// Mutations of src/lower/bindings/collects.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "array-from-collected-unrefused",
    breaks:
      "`js::array::from(set)` of what Rust collects itself compiles, a second way to write `set.values().collect()` (ADR 1052)",
    file: "src/lower/bindings/collects.rs",
    find: "            if let Some(way) = collected_by_rust(tcx, thir[given].ty) {\n",
    replace: "            if let Some(way) = collected_by_rust(tcx, thir[given].ty).filter(|_| false) {\n",
    tests: ["test/js-collections.test.ts", "-t", "Array.from is of any iterable"],
  },
  {
    name: "array-from-coercion-unpeeled",
    breaks:
      "`js::array::from` is refused by the `&dyn Iterable` it's given, never by what it was made of, so a `Set` compiles (ADR 1052)",
    file: "src/lower/bindings/collects.rs",
    find: "            while let ExprKind::PointerCoercion { source, .. } = thir[given].kind {\n",
    replace: "            while let ExprKind::PointerCoercion { source, .. } = thir[given].kind && false {\n",
    tests: ["test/js-collections.test.ts", "-t", "Array.from is of any iterable"],
  },
];
