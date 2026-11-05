// Mutations of src/lower/mir/hints/calls.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "temporary-named-by-unwritten-call",
    breaks:
      "a temporary is named after a call the source doesn't write by its name, `vec![..]`'s `newUninit`, `a[i]`'s `indexMut`, `f(x)`'s `callMut`",
    file: "src/lower/mir/hints/calls.rs",
    find: "                && written.starts_with(self.tcx.item_name(def_id).as_str())\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(adapter_drops|assignment_order|arm_temporaries).rs$",
    ],
    snapshots: true,
  },
  {
    name: "option-method-names-its-subject",
    breaks: "an `Option`'s own method, `as_deref()` or `filter()`, names the subject held after it, `const deref = props.name`, where the option it gave back is `option` (#784)",
    file: "src/lower/mir/hints/calls.rs",
    find: "            .filter(|&(_, def_id)| !optional(def_id))\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^option_subject_named_by_its_binding.rs$"],
  },
  {
    name: "binding-read-names-its-subject",
    breaks: "a binding read as a property, `get G`, names the subject held after it over the closure's parameter that binds it, `const g = ReactSharedInternals.G`, as if it were a call (#784)",
    file: "src/lower/mir/hints/calls.rs",
    find: "                let read = read(def_id);\n",
    replace: "                let read = false;\n",
    tests: ["test/lowering-moves.test.ts", "-t", "getter's Option a closure binds is named by its parameter"],
  },
];
