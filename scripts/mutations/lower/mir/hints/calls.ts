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
];
