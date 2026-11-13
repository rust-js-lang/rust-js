// Mutations of src/lower/mir/dispatch/passed_on.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "passed-on-receiver-not-read-through",
    breaks:
      "a method std's impl for a `&mut W` passes on to `W`'s is given its receiver, the `&mut &mut W`, not the `&mut W` it's of: a `&mut &mut String`'s `write_str` replaces the cell its local holds with text, and a `&mut &mut Vec<u8>`'s `write_all` throws (#1309)",
    file: "src/lower/mir/dispatch/passed_on.rs",
    find: "            node: Operand::Copy(self.tcx.mk_place_deref(receiver)),",
    replace: "            node: Operand::Copy(receiver),",
    tests: ["test/corpus.test.ts", "-t", "^std_method_of_a_mut_to_a_held_mut.rs$"],
  },
];
