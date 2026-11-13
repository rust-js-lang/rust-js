// Mutations of src/lower/analysis/intervals.rs, the MIR intervals run in shadow (ADR 1137, #1672).
import type { Mutation } from "../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-exact-unproved",
    breaks: "no operator is proved in its type's range, so a loop's counter below its bound is logged as one THIR alone leaves out",
    file: "src/lower/analysis/intervals.rs",
    find: "                    proofs.exact.insert(Location {\n                        block,\n                        statement_index: at,\n                    });\n",
    replace: "                    let _ = at;\n",
    tests,
  },
  {
    name: "intervals-copies-unfollowed",
    breaks: "a comparison of a copy, `_7 = copy _3; Lt(move _7, 4)`, narrows the copy alone, so the loop's counter isn't known below its bound",
    file: "src/lower/analysis/intervals.rs",
    find: "            state.copies.insert(local, of);\n",
    replace: "            let _ = of;\n",
    tests,
  },
];
