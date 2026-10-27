// Mutations of src/lower/mir/tidy/value_chains.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chain-negated-arm-kept",
    breaks: "an arm of a value's chain MIR wrote the other way round is `else if (!(selfTime < 100)) { other = \"error\"; } else ..`, not as written",
    file: "src/lower/mir/tidy/value_chains.rs",
    find: "        ExprKind::Unary(UnaryOp::Not, inner) => (&**inner, r, q),\n",
    replace: "        ExprKind::Unary(UnaryOp::Not, _) => (&**t, q, r),\n",
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
  {
    name: "plain-arm-any-value",
    breaks: "a chain with an arm that's more than a value, a sum or a conditional of its own, is one nested conditional, not the `else if` chain (ADR 0969, amended)",
    file: "src/lower/mir/tidy/value_chains.rs",
    find: "        ExprKind::Unary(UnaryOp::Neg, n) => matches!(n.kind, ExprKind::Num(_)),\n        _ => false,\n",
    replace: "        ExprKind::Unary(UnaryOp::Neg, n) => matches!(n.kind, ExprKind::Num(_)),\n        _ => true,\n",
    tests: ["test/lowering-written-branches.test.ts", "-t", "arms from MIR"],
  },
  {
    name: "plain-chain-first-arm-any",
    breaks: "a chain whose first arm is more than a value, `n + 1`, and the rest plain values, is one nested conditional (ADR 0969, amended)",
    file: "src/lower/mir/tidy/value_chains.rs",
    find: "        (false, true) if plain(&yes) => ",
    replace: "        (false, true) => ",
    tests: ["test/lowering-written-branches.test.ts", "-t", "arms from MIR"],
  },
  {
    name: "plain-chain-in-yes-kept",
    breaks: "a match of plain values whose `None` arm comes first is `if (value != null) { align = a ? .. : .. } else { align = \"none\"; }`, half a chain (ADR 0969, amended)",
    file: "src/lower/mir/tidy/value_chains.rs",
    find: "        (true, false) if plain(&no) => ",
    replace: "        (true, false) if false => ",
    tests: ["test/lowering-written-branches.test.ts", "-t", "arms from MIR"],
  },
];
