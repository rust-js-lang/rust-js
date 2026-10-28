// Mutations of src/prepare/stepped_whiles.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const corpus = ["test/corpus.test.ts", "-t", "^while_let_steps.rs$"];

export const mutations: Mutation[] = [
  {
    name: "stepped-while-never",
    breaks: "a count, `let at = n; while (at > 0) { ..; at -= 2; }`, stays a `while`, not upstream's `for (let at = n; at > 0; at -= 2)` (ADR 0974)",
    file: "src/prepare/stepped_whiles.rs",
    find: "    if !matches!(&target.kind, ExprKind::Var(v) if v == name)\n",
    replace: "    if true || !matches!(&target.kind, ExprKind::Var(v) if v == name)\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "stepped-while-continued",
    breaks: "a `while` whose body has a `continue` before its step is a `for`, whose `continue` runs the step the `while`'s skips (ADR 0974)",
    file: "src/prepare/stepped_whiles.rs",
    find: "        || js::continues(rest, label.as_deref())\n",
    replace: "",
    tests: corpus,
  },
  {
    name: "stepped-while-read-after",
    breaks: "a `while` whose variable is read after it is a `for`, which keeps it to itself: `return i` throws a ReferenceError (ADR 0974)",
    file: "src/prepare/stepped_whiles.rs",
    find: "        || js::mentions_in(after, name) > 0\n",
    replace: "",
    tests: corpus,
  },
  {
    name: "stepped-while-body-declared",
    breaks: "a step that reads what the body declares, `i += stride`, is a `for`'s update, which can't see it (ADR 0974)",
    file: "src/prepare/stepped_whiles.rs",
    find: "function_names(value, name) || reads_declared(value, rest))\n",
    replace: "function_names(value, name))\n",
    tests: corpus,
  },
  {
    name: "stepped-while-set-before",
    breaks: "a `while` whose body sets its variable before its last statement, `if (at === 9) { at = 4; }`, is a `for` (ADR 0974)",
    file: "src/prepare/stepped_whiles.rs",
    find: "        || sets(rest, name)\n",
    replace: "",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "stepped-while-untested",
    breaks: "a `while` whose test doesn't read the variable its body counts last, `let prev = 100; while (k > 0) { ..; prev -= 3.5; }`, is a `for` of it (ADR 0974)",
    file: "src/prepare/stepped_whiles.rs",
    find: "        || !cond.mentions_var(name)\n",
    replace: "",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "stepped-while-compound-any",
    breaks: "a step of two, `at -= 2`, is `at--` (ADR 0974)",
    file: "src/prepare/stepped_whiles.rs",
    find: "    let one = |e: &Expr| matches!(e.kind, ExprKind::Num(n) if n == 1.0);\n",
    replace: "    let one = |_: &Expr| true;\n",
    tests: corpus,
  },
  {
    name: "stepped-while-bits",
    breaks: "a `while` that clears its lanes' lowest bit last, `rest &= rest - 1`, is a `for` of it, where React's getEntangledLanes writes a `while` (ADR 0974)",
    file: "src/prepare/stepped_whiles.rs",
    find: "        StmtKind::Compound(op @ (Op::Add | Op::Sub), target, by) =>",
    replace: "        StmtKind::Compound(op, target, by) =>",
    tests: ["test/lowering-loops.test.ts", "-t", "a while that ends an if's branch keeps its test"],
  },
];
