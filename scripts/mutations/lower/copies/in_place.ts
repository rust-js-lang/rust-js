// Mutations of src/lower/copies/in_place.rs, the one constructor of a write in place (#1665, #1672).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "in-place-unguarded",
    breaks: "a write in place of a `Copy` type whose copies are left out is made, not refused, so every name sharing the value changes (#1665)",
    file: "src/lower/copies/in_place.rs",
    find: "        if self.copies_left_out(written) {\n",
    replace: "        if false && self.copies_left_out(written) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "naming #1665"],
  },
];
