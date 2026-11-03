// Mutations of src/runtime/assign.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "assign-keeps-old-fields",
    breaks: "an enum replaced by another variant in place keeps the old one's fields, so `==` of it is false",
    file: "src/runtime/assign.js",
    find: "    for (const key of Object.keys(target)) if (!(key in value)) delete target[key];\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "replace_through_mut"],
  },
  {
    name: "assign-itself-cleared",
    breaks: "a map given back in its own box is cleared, then filled from itself, which is empty",
    file: "src/runtime/assign.js",
    find: "  if (target === value) return;\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","generic_mut_map"],
  },
  {
    name: "assign-std-struct-methods",
    breaks:
      "`$assign` of std's compiled struct copies the value's methods, which step the value: what it knew of its back is lost (#537)",
    file: "src/runtime/assign.js",
    find: "    target.$become(value);\n",
    replace: "    for (const key of Object.keys(target)) if (!(key in value)) delete target[key];\n    Object.assign(target, value);\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_values_meet.rs$"],
  },
];
