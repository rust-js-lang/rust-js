// Mutations of src/lower/mir.rs (ADR 0364).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-fold-out-of-order",
    breaks: "temporaries that do something are folded where they're read, not in the order they were made",
    file: "src/lower/mir.rs",
    find: "        let ordered = acting.windows(2).all(|w| w[0] < w[1]);\n",
    replace: "        let ordered = true;\n",
    tests: ["test/mir.test.ts", "-t", "order"],
  },
  {
    name: "mir-assert-inverted",
    breaks: "a MIR `Assert` throws when its condition holds, not when it fails",
    file: "src/lower/mir.rs",
    find: "                    true => Expr::unary(js::UnaryOp::Not, cond),\n",
    replace: "                    true => cond,\n",
    tests: ["test/mir.test.ts", "-t", "bitwise_operators"],
  },
];
