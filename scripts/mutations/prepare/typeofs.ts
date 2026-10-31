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
  {
    name: "typeof-of-a-kind-says-defined",
    breaks: "an `&&` asks `typeof performance !== \"undefined\"` right after `typeof performance === \"object\"`, which says it, not upstream's `typeof performance === 'object' && typeof performance.now === 'function'` (rd320, ADR 0942 amended)",
    file: "src/prepare/typeofs.rs",
    find: "    let of_kind = |kind: &str| kind != \"undefined\";\n",
    replace: "    let of_kind = |kind: &str| kind == \"function\";\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "is whether a binding's global is a function"],
  },
];
