// Mutations of src/lower/mir/tidy/value_chains.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chain-negated-arm-kept",
    breaks: "an arm of a value's chain MIR wrote the other way round is `else if (!(selfTime < 100)) { other = \"error\"; } else ..`, not as written",
    file: "src/lower/mir/tidy/value_chains.rs",
    find: "        ExprKind::Unary(UnaryOp::Not, inner) => (&**inner, r, q),\n",
    replace: "",
    tests: ["test/lowering-written-branches.test.ts", "-t", "four arms"],
  },
  {
    name: "chain-rest-one-arm",
    breaks: "a value's chain of four arms has only its next arm its own, the rest a conditional, `else if (b) { x = q; } else { x = c ? r : s; }`",
    file: "src/lower/mir/tidy/value_chains.rs",
    find: "    let rest = match chained_arms(x, r, at) {\n",
    replace: "    let rest = match None::<Vec<Stmt>> {\n",
    tests: ["test/lowering-written-branches.test.ts", "-t", "four arms"],
  },
];
