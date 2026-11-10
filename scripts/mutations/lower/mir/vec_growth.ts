// Mutations of src/lower/mir/vec_growth.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "extend-of-a-trusted-len-untracked",
    breaks: "in a crate that keeps capacities, an `extend` of a `TrustedLen` iterator keeps its length as its capacity, where std reserves as many as it adds (#1180)",
    file: "src/lower/mir/vec_growth.rs",
    find: "            .then(|| crate::lower::recognition::grows_vec(self.tcx, def_id).or_else(|| extend.then_some(0)))\n",
    replace: "            .then(|| crate::lower::recognition::grows_vec(self.tcx, def_id).or_else(|| extend.then_some(0).filter(|_| false)))\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_capacity_grown.rs$"],
  },
  {
    name: "grown-length-read-after",
    breaks: "an `extend` whose lowering runs statements first, an index loop's, reads the length its growth starts from after them, so it grows by nothing (#1180)",
    file: "src/lower/mir/vec_growth.rs",
    find: "        if !made_out.is_empty() {\n            length = self.spill(\"length\", length, out);\n        }\n        out.extend(made_out);\n",
    replace: "        out.extend(made_out);\n        if !out.is_empty() {\n            length = self.spill(\"length\", length, out);\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_capacity_grown.rs$"],
  },
];
