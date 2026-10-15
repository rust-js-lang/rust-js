// Mutations of src/lower/support.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dyn-node-unsupported",
    breaks: "`Box<dyn ReactNode>` is refused, though it's the node itself, and a list of text and elements has no type",
    file: "src/lower/support.rs",
    find: "                    .is_some_and(|t| super::bindings::is_jsx_node(self.tcx, t)) =>",
    replace: "                    .is_some_and(|t| false && super::bindings::is_jsx_node(self.tcx, t)) =>",
    tests: ["test/jsx.test.ts", "-t", "dyn ReactNode is the node"],
  },
  {
    name: "vacant-entry-unsupported",
    breaks: "a `VacantEntry` value is refused",
    file: "src/lower/support.rs",
    find: " || self.recognition().is_vacant_entry(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","map_entry_match"],
  },
  {
    name: "never-unsupported",
    breaks: "a function that never returns, `-> !`, is refused",
    file: "src/lower/support.rs",
    find: "            || ty.is_never()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^never_returns.rs$"],
  },
  {
    name: "union-unrepresentable",
    breaks: "a union's values are refused",
    file: "src/lower/support.rs",
    find: "            (ty::Adt(adt, args), _) if adt.is_enum() || adt.is_union() => {\n",
    replace: "            (ty::Adt(adt, args), _) if adt.is_enum() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^unions.rs$"],
  },
];
