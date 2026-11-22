// Mutations of src/lower/analysis/intervals/fixed_points.rs (#2313): the
// fixed points the walks for what callers give share, by body and `Given`.
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fixed-points-kept-by-body-alone",
    breaks: "a walk for what callers give takes the fixed point kept of its body whatever the body was given, so a recursive component's later round, or a closure's maker given more since, reads what an earlier `Given` knew (#2313)",
    file: "src/lower/analysis/intervals/fixed_points.rs",
    find: "        let key = (self.body.source.def_id(), self.from_callers.clone());\n",
    replace: "        let key = (self.body.source.def_id(), Given::default());\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a function that calls itself is given nothing of its outer calls"],
  },
  {
    name: "fixed-points-walks-uncounted",
    breaks: "the walks for what callers give aren't counted, so the shadow says none walked whatever walks, and a change that walks every body, serde's derived thousands of blocks each, goes unseen (#2313)",
    file: "src/lower/analysis/intervals/fixed_points.rs",
    find: "        points.walked.set(points.walked.get() + 1);\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "how many bodies the walks for what callers give walked"],
  },
];
