// Mutations of src/prepare/typeofs.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "typeof-said-asked-again",
    breaks: "an `&&` asks `typeof Symbol !== \"undefined\"` right after `typeof Symbol === \"function\"`, which says it, not React's `typeof Symbol === 'function' && Symbol.for` (ADR 0964)",
    file: "src/prepare/typeofs.rs",
    find: "            && kept.iter().any(|earlier| says_defined(earlier, asked))\n",
    replace: "            && false\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "js::SymbolFor is Symbol.for"],
  },
  {
    name: "typeof-other-read-said",
    breaks: "a test of `typeof Symbol` is taken to say `typeof Symbol.for` is defined, so an `&&` drops what it asks (ADR 0964)",
    file: "src/prepare/typeofs.rs",
    find: "        (ExprKind::Member(x, m), ExprKind::Member(y, n)) => m == n && same_read(x, y),\n",
    replace: "        (ExprKind::Member(x, m), ExprKind::Member(y, n)) => m == n && same_read(x, y),\n        (ExprKind::Var(_), ExprKind::Member(..)) => true,\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "js::SymbolFor is Symbol.for"],
  },
];
