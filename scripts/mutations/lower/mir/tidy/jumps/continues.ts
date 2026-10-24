// Mutations of src/lower/mir/tidy/jumps/continues.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-continue-guard-kept",
    breaks: "a loop body ending `if (!c) { a; b }` stays, not `if (c) { continue; } a; b`",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                Some(StmtKind::If(_, _, None)) if tests_continue(guards, body.last().expect(\"matched\").span) => {\n",
    replace: "                Some(StmtKind::If(_, _, None)) if false => {\n",
    tests: ["test/lowering-loops.test.ts", "-t", "if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-continued-block-kept",
    breaks: "a loop body's last labeled block, its `break` the loop's `continue`, stays",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                    if clashes || continues(&mut inner, &block, looped.as_deref(), false).is_none() {\n",
    replace: "                    if true {\n",
    tests: ["test/lowering-loops.test.ts", "-t", "if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-written-continue-nested",
    breaks: "a loop body's last `if` of more than one statement is an early `continue` the program never wrote",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                Some(StmtKind::If(_, _, None)) if tests_continue(guards, body.last().expect(\"matched\").span) => {\n",
    replace: "                Some(StmtKind::If(_, then, None)) if tests_continue(guards, body.last().expect(\"matched\").span) || then.len() > 1 => {\n",
    tests: ["test/lowering-early-exits.test.ts", "-t", "a loop.s early break"],
  },
  {
    "name": "ending-continue-as-else",
    "breaks": "an `if` the program ended with `continue` stays MIR's `if (t) { a } else { .. }`",
    "file": "src/lower/mir/tidy/jumps/continues.rs",
    "find": "                    if tests_continue(guards, body.last().expect(\"matched\").span)\n                        && !declared(otherwise)",
    "replace": "                    if false\n                        && !declared(otherwise)",
    "tests": ["test/lowering-loops.test.ts", "-t", "continues a loop's ifs end with are kept"]
  },
  {
    "name": "mir-tidy-continue-written-negated-only",
    "breaks": "a `continue` the program wrote is an `if (i % 2 !== 0) { \u2026 }` where MIR's test isn't a `!`",
    "file": "src/lower/mir/tidy/jumps/continues.rs",
    "find": "                Some(StmtKind::If(_, _, None)) if tests_continue(guards, body.last().expect(\"matched\").span) => {\n",
    "replace": "                Some(StmtKind::If(test, _, None)) if tests_continue(guards, body.last().expect(\"matched\").span) && matches!(test.kind, ExprKind::Unary(UnaryOp::Not, _)) => {\n",
    "tests": ["test/lowering-numbers.test.ts", "-t", "division by a constant from MIR"]
  },
  {
    "name": "mir-loop-end-block-continues",
    "breaks": "an `else if` at a loop's end from MIR is a `continue` and an `if` after",
    "file": "src/lower/mir/tidy/jumps/continues.rs",
    "find": "                    ending(&mut inner, &block);\n",
    "replace": "",
    "tests": ["test/lowering-loops.test.ts", "-t", "react.dev's llms.txt groups its routes"]
  },
  {
    name: "block-jump-not-forwarded",
    breaks: "a labeled block's `break` to a `continue` of the loop after it stays, `bb3: { .. break bb3; } continue bb1;`",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                StmtKind::Break(Some(l)) if l == label => stmt.kind = jump.clone(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(nested_loop_jumps|interpreter|rc_counted_uses|loop_drops).rs$"],
    snapshots: true,
  },
  {
    name: "outer-continue-kept",
    breaks: "an inner loop's way out, which ends the outer loop's turn, is `continue bb4`, not its `break`",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                StmtKind::Continue(Some(l)) if l == looped => stmt.kind = StmtKind::Break(None),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nested_loop_jumps.rs$"],
    snapshots: true,
  },
  {
    name: "written-continue-unplaced",
    breaks: "a `continue` the program wrote has no place, so the comment before it is gone",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "            last.span = *at;\n",
    replace: "            let _ = at;\n",
    tests: ["test/lowering-written-branches.test.ts", "-t", "a continue the program wrote"],
  },
  {
    name: "else-continue-kept",
    breaks: "an `else` of only the `continue` a loop's body ends with anyway stays, and is left past a `return` as a `continue;` the program didn't write",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                    *otherwise = None;\n",
    replace: "                    let _ = otherwise;\n",
    tests: ["test/lowering-written-branches.test.ts", "-t", "has no continue after it"],
  },
  {
    name: "loop-ending-block-kept",
    breaks: "a loop that ends a block MIR made keeps its `break bb97`, and the block its label",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                    *broken = None;\n",
    replace: "                    let _ = broken;\n",
    tests: ["test/lowering-written-loops.test.ts", "-t", "a loop that ends a block breaks as a loop"],
  },
  {
    name: "loop-ending-block-not-last",
    breaks: "a block's first statement taken for what it ends with, its loop's `break bb97` kept",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "        let Some(last) = stmts.last_mut() else {\n            return;\n        };\n        match &mut last.kind {\n            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {\n                if let Some(Stmt {\n",
    replace: "        let Some(last) = stmts.first_mut() else {\n            return;\n        };\n        match &mut last.kind {\n            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {\n                if let Some(Stmt {\n",
    tests: ["test/lowering-written-loops.test.ts", "-t", "a loop that ends a block breaks as a loop"],
  },
];
