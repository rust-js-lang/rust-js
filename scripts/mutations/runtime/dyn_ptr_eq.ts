// Mutations of src/runtime/dyn_ptr_eq.js (ADR 0562).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dyn-ptr-eq-ignores-value",
    breaks: "`==` of two `dyn` pointers to two values of one type is true",
    file: "src/runtime/dyn_ptr_eq.js",
    find: "  return p.value === q.value;\n",
    replace: "  return true;\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
];
