// Mutations of src/lower/drops/inlined.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drop-inlined-twice-dropped",
    breaks: "a `Drop` dropped in two places is inlined at the first, and the second calls a function that's gone",
    file: "src/lower/drops/inlined.rs",
    find: "        if mentions != 1 {\n",
    replace: "        if mentions == 0 {\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "guard-kept-whole",
    breaks: "a guard made only to be dropped stays an object, `const _restore = { previous: depth }`",
    file: "src/lower/drops/inlined.rs",
    find: "                if let Some(guard) = given {\n",
    replace: "                if let Some(guard) = given.filter(|_| false) {\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "guard-read-whole-taken-apart",
    breaks: "a guard read whole, given to a function, is taken apart into its fields, and the function reads a variable that's gone",
    file: "src/lower/drops/inlined.rs",
    find: "    if written || reads != js::mentions_in(body, guard) {\n",
    replace: "    if written {\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
];
