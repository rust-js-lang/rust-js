// Mutations of src/lower/mir/tidy/jumps.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-early-breaks-kept",
    breaks: "a loop body ending `if (c) { a; continue; } break;` stays, not `if (!c) { break; } a`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        if tests.is_empty() || !leaves {\n",
    replace: "        if true {\n",
    tests: ["test/lowering.test.ts","-t","loops from MIR read"],
  },
  {
    name: "mir-tidy-tail-guard-kept",
    breaks: "`if (!c) { return; } rest` MIR made of `if c { rest }` stays a guard",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "StmtKind::If(_, then, None) if then.last().is_some_and(unwritten)))",
    replace: "StmtKind::If(_, then, None) if false && then.last().is_some_and(unwritten)))",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-tail-return-branch-kept",
    breaks: "`if (c) { s; return; } t` where the program wrote no `return` stays, not `if (c) { s } else { t }`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if then.last().is_some_and(unwritten)))\n",
    replace: "        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if matches!(then.as_slice(), [r] if unwritten(r))))\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-tail-return-else-lost",
    breaks: "`if (c) { s; return; } t` is `if (c) { s }`, `t` lost",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            (false, false) => stmts.push(StmtKind::If(test, then, Some(rest)).at(span)),\n",
    replace: "            (false, false) => stmts.push(StmtKind::If(test, then, None).at(span)),\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-inner-let-else-kept",
    breaks: "`if (c) { s } else { return; }`, a let-else's, stays, not the guard `if (!c) { return; } s`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        && !then.last().is_some_and(ends)\n    {\n        let Some(Stmt {\n            kind: StmtKind::If(test, then, Some(otherwise)),\n",
    replace: "        && false\n    {\n        let Some(Stmt {\n            kind: StmtKind::If(test, then, Some(otherwise)),\n",
    tests: ["test/lowering.test.ts","-t","inside a branch, its .else. a .return."],
  },
  {
    name: "mir-tidy-let-else-binding-after",
    breaks: "a let-else's binding is after its guard, which tests what it's bound to",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "                Some(b.clone())\n            }\n            _ => None,\n        };\n        let test = match &bound {",
    replace: "                let _ = b;\n                None::<String>\n            }\n            _ => None,\n        };\n        let test = match &bound {",
    tests: ["test/lowering.test.ts","-t","inside a branch, its .else. a .return."],
  },
  {
    "name": "break-test-nested-unread",
    "breaks": "a guard that tests on, `if (k) { if (!m) { .. } }`, isn't one of the `||`",
    "file": "src/lower/mir/tidy/jumps.rs",
    "find": "        None => Some(Expr::bin(Op::And, test.clone(), inner(then)?)),\n",
    "replace": "        None => None,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an or of ands from MIR is one test"
    ]
  },
  {
    "name": "break-test-else-unnegated",
    "breaks": "`if (!m) { .. } else { break l; }` is read as `!m || ..`, breaking where `m` doesn't hold",
    "file": "src/lower/mir/tidy/jumps.rs",
    "find": "Some(Expr::bin(Op::Or, test.clone().negated_test(), inner(then)?)),\n",
    "replace": "Some(Expr::bin(Op::Or, test.clone(), inner(then)?)),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an or of ands from MIR is one test"
    ]
  },
  {
    name: "loop-exit-before-what-goes-on",
    breaks: "`if (c) { a } return x;` is `if (!c) { return x; } a`, where `a` goes on round the loop instead of returning",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "    if !in_else && !goes_on && !then.last().is_some_and(ends) {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "loop_exits"],
  },
  {
    name: "jumps-exit-each-way-turned",
    breaks: "a loop's `if (c) { return 1; } return 2;` is turned, and turned back, without end: its compile never finishes",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "    if !in_else && matches!(then.as_slice(), [e] if exits(e)) {",
    replace: "    if false && matches!(then.as_slice(), [e] if exits(e)) {",
    tests: ["test/corpus.test.ts", "-t", "^loop_break_choice.rs$"],
  },
  {
    name: "early-breaks-nested-tests-kept",
    breaks: "a `while let`'s `if (d != null) { if (c) { a; continue; } } break;` stays, not `if (d == null || !c) { break; } a`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        if tests.len() > 1 {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^calculator.rs$"],
    snapshots: true,
  },
  {
    name: "early-breaks-not-nested",
    breaks: "a loop body ending `if (a) { x; if (b) { y; continue; } } break;` stays, not `if (!a) { break; } x; if (!b) { break; } y`, its `while (a)` lost",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            true => leaves_early(then),\n",
    replace: "            true => then.last().is_some_and(ends),\n",
    tests: ["test/lowering.test.ts", "-t", "a .while. loop from MIR tests where"],
  },
];
