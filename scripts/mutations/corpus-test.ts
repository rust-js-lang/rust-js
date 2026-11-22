// The corpus's run of each case (test/corpus.test.ts): where a panic was raised (#2292).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "located-otherwise-matching-unreported",
    breaks: "a case `located-otherwise` says panics elsewhere than native's, once its panic is native's, isn't reported, so the list never shrinks (#2292)",
    file: "test/corpus.test.ts",
    find: "    problems.push(`its panic is raised where native Rust's is now: remove \\`located-otherwise: ${want.locatedOtherwise}\\``);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_function_value_panic.rs$|a panic raised elsewhere than native"],
  },
];
