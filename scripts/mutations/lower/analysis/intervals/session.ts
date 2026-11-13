// Mutations of src/lower/analysis/intervals/session.rs, split from intervals.rs (ADR 1137, #1672).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-mir-only-unreported",
    breaks: "what MIR proves that THIR's facts keep, which M3 leaves out, isn't in the shadow to be triaged",
    file: "src/lower/analysis/intervals/session.rs",
    find: "        (false, true) => &format!(\"mir-only-{kind}\"),\n",
    replace: "        (false, true) => return,\n",
    tests,
  },
];
