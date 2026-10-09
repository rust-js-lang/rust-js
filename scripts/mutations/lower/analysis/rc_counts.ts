// Mutations of src/lower/analysis/rc_counts.rs (ADR 0320).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rc-generic-uncounted",
    breaks: "a generic function's `Rc<T>` isn't counted where its caller's is",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    let all = types.iter().any(|t| t.has_non_region_param()) || !types.is_empty() && generic_rc;\n",
    replace: "    let all = false && generic_rc;\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
];
