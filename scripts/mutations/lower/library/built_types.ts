// Mutations of src/lower/library/built_types.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "built-template-slot-shifted",
    breaks:
      "a library's manifest names a built type's slot as the next one, so its consumer makes another type, or none, of what its call gives (#737)",
    file: "src/lower/library/built_types.rs",
    find: "        return Some(TypeTemplate::Slot(index));\n",
    replace: "        return Some(TypeTemplate::Slot(index + 1));\n",
    tests: ["test/crates.test.ts", "-t", "built_layout_of_a_library"],
  },
];
