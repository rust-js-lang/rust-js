// Mutations of src/lower/analysis/intervals/shadowed.rs, the effects query's
// shadow beside the MIR intervals' forgetting rules (#2112 B).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "effects-shadow-keeps-unsaid",
    breaks:
      "the shadow writes no `effects-keeps` line, so a fact the forgetting rules forget at a write that the query would keep, a hole of the query's, goes unseen (#2112 B)",
    file: "src/lower/analysis/intervals/shadowed.rs",
    find: '                (false, true) => "effects-keeps",\n',
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "keeps nothing the forgetting rules forget"],
  },
];
