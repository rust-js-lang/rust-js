// Mutations of src/lower/mir/tidy/logicals/skipped.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "skipped-tests-or-flipped",
    breaks:
      "a test whose `then` goes on where a test holds and whose `else` always does is `t && x`, not `!t || x` (ADR 0946)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "                    (Truth::Test(x), Truth::Always) => Truth::Test(Expr::bin(Op::Or, test.negated_test(), x)),\n",
    replace:
      "                    (Truth::Test(x), Truth::Always) => Truth::Test(Expr::bin(Op::And, test, x)),\n",
    tests: ["test/lowering-written-branches.test.ts", "-t", "one test"],
  },
  {
    name: "skipped-tests-and-negated",
    breaks:
      "a test whose `then` may go on and whose `else` never does is `!t && x` (ADR 0946)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "                    (Truth::Test(x), Truth::Never) => Truth::Test(Expr::bin(Op::And, test, x)),\n",
    replace:
      "                    (Truth::Test(x), Truth::Never) => Truth::Test(Expr::bin(Op::And, test.negated_test(), x)),\n",
    tests: ["test/lowering-written-branches.test.ts", "-t", "one test"],
  },
  {
    name: "skipped-tests-leaf-unnegated",
    breaks:
      "a test whose `then` leaves the block and whose `else` goes on is `t`, not `!t` (ADR 0946)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "                    (Truth::Never, Truth::Always) => Truth::Test(test.negated_test()),\n",
    replace:
      "                    (Truth::Never, Truth::Always) => Truth::Test(test),\n",
    tests: ["test/lowering-written-branches.test.ts", "-t", "one test"],
  },
  {
    name: "skipped-tests-of-the-outer-test-alone",
    breaks:
      "`if (p) { tree; t }` is `if (tree) { t }`, its first test lost (ADR 0946)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "    Some((Expr::bin(Op::And, first.clone(), rest), then, *span))\n",
    replace: "    Some((rest, then, *span))\n",
    tests: ["test/lowering-written-branches.test.ts", "-t", "one test"],
  },
  {
    name: "chain-binding-between-tests-kept",
    breaks:
      "a let chain's binding between its tests keeps the chain a labeled block, `bb11: { if (lanes == null) { break bb11; } .. }`, not React's `if (lanes != null && root != null) { .. } else { .. }` (ADR 0946, amended)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "            StmtKind::Const(_, value) if rereadable(value) && body.get(k + 1).is_some_and(guard) => {\n",
    replace:
      "            StmtKind::Const(_, value) if false && rereadable(value) && body.get(k + 1).is_some_and(guard) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^let_chain_else_of_reads.rs$"],
    snapshots: true,
  },
  {
    name: "condition-call-unsequenced",
    breaks: "a block's call on a way through a condition, `check(f) || { log(f); false }`, leaves the `if` labeled blocks and `break`s the Rust doesn't have (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "            StmtKind::Expr(e) => {\n                done.push(e.clone());\n",
    replace: "            StmtKind::Expr(e) if false => {\n                done.push(e.clone());\n",
    tests: ["test/corpus.test.ts", "-t", "^condition_block_sequence.rs$"],
    snapshots: true,
  },
  {
    name: "condition-field-set-unsequenced",
    breaks: "a block's field set on a way through a condition, `{ flags.set(f & !32); false }`, leaves the `if` labeled blocks and `break`s (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "                    ExprKind::Member(..) | ExprKind::Index(..) | ExprKind::Var(_)\n",
    replace: "                    ExprKind::Var(_)\n",
    tests: ["test/corpus.test.ts", "-t", "^condition_block_sequence.rs$"],
    snapshots: true,
  },
  {
    name: "condition-local-set-unsequenced",
    breaks: "a local's set in a condition's block, `{ seen = 5; false }`, is no part of the test, so the `if` stays MIR's labeled blocks (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "                    ExprKind::Member(..) | ExprKind::Index(..) | ExprKind::Var(_)\n",
    replace: "                    ExprKind::Member(..) | ExprKind::Index(..)\n",
    tests: ["test/corpus.test.ts", "-t", "^condition_block_sequence.rs$"],
    snapshots: true,
  },
  {
    name: "condition-lets-read-first-first",
    breaks: "a condition's block's `let`s are read in from the first, so one read past a later one, `more(flags.get(), { let d = bump(flags); d + 1 })`'s, is no test's and the `if` stays MIR's labeled blocks (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "            .rposition(|s| matches!(s.kind, StmtKind::Const(..)))\n",
    replace: "            .position(|s| matches!(s.kind, StmtKind::Const(..)))\n",
    tests: ["test/lowering-condition-lets.test.ts"],
  },
  {
    name: "skipped-tests-outer-first",
    breaks: "a block MIR made around one of a condition's, `bb10: { bb7: { .. } seen += 10; }`, is read before the one it holds is a test, so each way returns, `seen += 10; return seen; } return seen;` (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "        skipped_tests(body, logicals, lets);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^condition_block_sequence.rs$"],
    snapshots: true,
  },
  {
    name: "condition-sequence-leaving-goes-on",
    breaks: "a way through a condition that does something and then leaves, `(s, false)`, is `(s, true)`, so the `if` is taken where Rust's isn't (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "                Truth::Never => Expr::bool(false),\n",
    replace: "                Truth::Never => Expr::bool(true),\n",
    tests: ["test/corpus.test.ts", "-t", "^condition_block_sequence.rs$"],
  },
  {
    name: "condition-sequence-of-the-ifs-own",
    breaks: "what an `if`'s own branch does, not a block in its condition, is put in the condition's sequence, `if (on && (a == null || (log.push(a), false))) { log.push(0); }` of a let chain's `else` (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "        StmtKind::Expr(_) | StmtKind::Assign(..) => within(s.span),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^let_chain_else_inside_if.rs$"],
    snapshots: true,
  },
  {
    name: "condition-let-kept",
    breaks: "a block's `let` in a condition, `{ let deps = *deps.borrow(); deps.unwrap() }`, read once by the test after it, is no part of the test, so the `if` stays MIR's labeled blocks (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "    if stmts.iter().any(|s| matches!(s.kind, StmtKind::Const(..))) {\n",
    replace: "    if false && stmts.iter().any(|s| matches!(s.kind, StmtKind::Const(..))) {\n",
    tests: ["test/corpus.test.ts", "-t", "^condition_block_sequence.rs$"],
    snapshots: true,
  },
  {
    name: "condition-let-unread",
    breaks: "a block's `let` in a condition is dropped where the test after it doesn't read it once in its place, so the test reads a variable nothing declares (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "            if read_again(&mut read, k) {\n                continue;\n            }\n",
    replace: "            if read_again(&mut read, k) || {\n                read.remove(k);\n                true\n            } {\n                continue;\n            }\n",
    tests: ["test/lowering-condition-lets.test.ts"],
  },
  {
    name: "condition-let-never-hoisted",
    breaks: "a block's `let` a condition reads twice, `{ let d = bump(flags); d * d }`, is never set in the condition, so the `if` stays MIR's labeled blocks (A1, #523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "        if !self.on {\n            return None;\n        }\n",
    replace: "        if true {\n            return None;\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^condition_block_sequence.rs$"],
    snapshots: true,
  },
  {
    name: "condition-let-of-a-held-block-kept",
    breaks: "a `let` a block it held declared before its test keeps the block around it from being one test, `bb12: { let d$1; if (..) { break bb12; } n += d; }`, so each way returns (#523)",
    file: "src/lower/mir/tidy/logicals/skipped.rs",
    find: "            StmtKind::Let(name, None) if tree.is_empty() && bound.is_empty() && hoisting.on => {\n",
    replace: "            StmtKind::Let(name, None) if false && tree.is_empty() && bound.is_empty() && hoisting.on => {\n",
    tests: ["test/corpus.test.ts", "-t", "^condition_block_sequence.rs$"],
    snapshots: true,
  },
];
