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
    find: "            || self.recognition().is_vacant_entry(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","map_entry_match"],
  },
  {
    name: "try-lock-error-unsupported",
    breaks: "a `TryLockError` value is refused",
    file: "src/lower/support.rs",
    find: "            ty::Adt(_, args) if self.recognition().is_try_lock_error(ty) => {\n                return self.unsupported_in(args.type_at(0), seen);\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","try_lock"],
  },
];
