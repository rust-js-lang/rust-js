// Mutations of src/lower/shipped.rs (ADR 0529).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "memrchr-iter-forward",
    breaks: "`memrchr_iter` steps from the front",
    file: "src/lower/shipped.rs",
    find: "                    true => Expr::call_helper(\"$rev\", vec![it]),\n",
    replace: "                    true => it,\n",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
];
