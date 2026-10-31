// Mutations of src/lower/mir/tidy/comparisons.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "turned-not-of-text",
    breaks: '`!(c >= "a")` of a `char` stays so, not `c < "a"`',
    file: "src/lower/mir/tidy/comparisons.rs",
    find: "    let text = |x: &Expr| matches!(x.kind, ExprKind::Str(_));\n",
    replace: "    let text = |_: &Expr| false;\n",
    tests: ["test/snapshots.test.ts", "-t", "lexer"],
  },
  {
    name: "turned-of-floats",
    breaks:
      "`!(x < 50)` of a float is `x >= 50`, which a `NaN` makes `false` where Rust's test is `true`",
    file: "src/lower/mir/tidy/comparisons.rs",
    find: "        ExprKind::Var(v) => numbers.contains(v),\n",
    replace: "        ExprKind::Var(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "loop_exits"],
  },
  {
    name: "ordered-floats-of-any-shape",
    breaks:
      "`$partialCmp(a, b) !== -1` of two values of any shape is `!(a < b)`, which runs a `valueOf` once where the helper may twice (ADR 0966)",
    file: "src/lower/mir/tidy/comparisons.rs",
    find: "        ExprKind::Var(v) => floats.contains(v),\n",
    replace: "        ExprKind::Var(_) => true,\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "js::Unknown's < is JS's"],
  },
  {
    name: "ordered-floats-equal-turned",
    breaks:
      "a float's `partial_cmp(&b) != Some(Equal)` is `a === b`, not `a !== b` (ADR 0966)",
    file: "src/lower/mir/tidy/comparisons.rs",
    find: "        (_, Op::Eq) => Expr::bin(Op::Ne, a.clone(), b.clone()),\n",
    replace:
      "        (_, Op::Eq) => Expr::bin(Op::Eq, a.clone(), b.clone()),\n",
    tests: ["test/corpus.test.ts", "-t", "^float_partial_cmp_tests.rs$"],
  },
  {
    name: "index-of-may-be-nan",
    breaks:
      "`!(xs.indexOf(x) >= 0)` stays, not `xs.indexOf(x) < 0`, React's (ADR 0966)",
    file: "src/lower/mir/tidy/comparisons.rs",
    find: '        ExprKind::Call(f, args) => args.len() == 1 && matches!(&f.kind, ExprKind::Member(_, m) if m == "indexOf"),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^float_partial_cmp_tests.rs$"],
    snapshots: true,
  },
];
