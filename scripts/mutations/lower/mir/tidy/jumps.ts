// Mutations of src/lower/mir/tidy/jumps.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-early-breaks-kept",
    breaks: "a loop body ending `if (c) { a; continue; } break;` stays, not `if (!c) { break; } a`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        if !shape {\n",
    replace: "        if true {\n",
    tests: ["test/lowering.test.ts","-t","loops from MIR read"],
  },
  {
    name: "mir-tidy-for-label-kept",
    breaks: "a `for` loop's label nothing names stays",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        | StmtKind::For { label, body, .. } = &mut stmts[i].kind\n",
    replace: "        | StmtKind::For { label, body, .. } = &mut stmts[i].kind\n            && false\n",
    tests: ["test/lowering.test.ts","-t","loops from MIR read"],
  },
  {
    name: "mir-tidy-continue-guard-kept",
    breaks: "a loop body ending `if (!c) { a; b }` stays, not `if (c) { continue; } a; b`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "                    if written(guards, body.last().expect(\"matched\").span) || (negated(test) && then.len() > 1) =>\n",
    replace: "                    if false =>\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-continued-block-kept",
    breaks: "a loop body's last labeled block, its `break` the loop's `continue`, stays",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "                    if clashes || continues(&mut inner, &block, looped.as_deref(), false).is_none() {\n",
    replace: "                    if true {\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
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
    name: "mir-tidy-branch-ends-kept",
    breaks: "a `match`'s arms stay labeled blocks, `bb1: { if (c) { s; break bb1; } t }`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            ending(body, label);\n",
    replace: "            let _ = (label, body);\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-inner-block-kept",
    breaks: "a block that ends another stays a block of its own, its `break`s labeled",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        stmts.pop();\n        stmts.extend(body);\n",
    replace: "        let _ = body;\n        return;\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-or-pattern-negated",
    breaks: "an or-pattern's arm is `if (a !== x && a !== y) { t } else { s }`, not `if (a === x || a === y) { s }`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        (false, true) => StmtKind::If(test.negated_test(), rest, Some(then)).at(span),\n",
    replace: "        (false, true) => StmtKind::If(test, then, Some(rest)).at(span),\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-let-else-branch-ends",
    breaks: "a `let … else`'s block takes its `throw` as an `else`, not a guard",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        .filter(|_| !exits)\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","let-else from MIR"],
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
    name: "mir-tidy-written-continue-nested",
    breaks: "a `continue` the program wrote before one statement is the rest nested, `if (!c) { … }`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "                    if written(guards, body.last().expect(\"matched\").span) || (negated(test) && then.len() > 1) =>\n",
    replace: "                    if negated(test) && then.len() > 1 =>\n",
    tests: ["test/lowering.test.ts","-t","continue.s from MIR are where"],
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
    "name": "let-else-return-not-guard",
    "breaks": "a `let .. else`'s return stays the `else` of a block, not a guard first",
    "file": "src/lower/mir/tidy/jumps.rs",
    "find": "                stmt.kind = StmtKind::If(test.negated_test(), otherwise, Some(then));\n",
    "replace": "                stmt.kind = StmtKind::If(test, then, Some(otherwise));\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "written returns MIR jumps out of a block"
    ]
  },
  {
    "name": "ending-continue-as-else",
    "breaks": "an `if` the program ended with `continue` stays MIR's `if (t) { a } else { .. }`",
    "file": "src/lower/mir/tidy/jumps.rs",
    "find": "                    if written(guards, body.last().expect(\"matched\").span)\n",
    "replace": "                    if false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "continues a loop's ifs end with are kept"
    ]
  },
  {
    "name": "mir-tidy-continue-written-negated-only",
    "breaks": "a `continue` the program wrote is an `if (i % 2 !== 0) { \u2026 }` where MIR's test isn't a `!`",
    "file": "src/lower/mir/tidy/jumps.rs",
    "find": "                    if written(guards, body.last().expect(\"matched\").span) || (negated(test) && then.len() > 1) =>\n",
    "replace": "                    if negated(test) && (then.len() > 1 || written(guards, body.last().expect(\"matched\").span)) =>\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "division by a constant from MIR"
    ]
  },
  {
    "name": "mir-loop-exit-written-return",
    "breaks": "a `return` the program wrote in a function's last loop becomes a `break`, the `return` after the loop",
    "file": "src/lower/mir/tidy/jumps.rs",
    "find": "        && (returns.iter().any(|r| written.lo <= r.lo && r.hi <= written.hi) || valued.contains(written))\n",
    "replace": "        && valued.contains(written)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a loop's break from MIR is a break"
    ]
  },
  {
    "name": "mir-loop-end-block-continues",
    "breaks": "an `else if` at a loop's end from MIR is a `continue` and an `if` after",
    "file": "src/lower/mir/tidy/jumps.rs",
    "find": "                    ending(&mut inner, &block);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "react.dev's llms.txt groups its routes"
    ]
  },
  {
    "name": "mir-loop-tail-continue-kept",
    "breaks": "a branch ending a loop's body from MIR keeps a `continue` that does nothing",
    "file": "src/lower/mir/tidy/jumps.rs",
    "find": "        ends_going_on(body, &looped, &[]);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "react.dev's CodeBlock places its decorators"
    ]
  },
  {
    name: "mir-tidy-valued-loop-exits",
    breaks: "MIR's tail `loop` that `break a` gives its value breaks, then returns",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        && (returns.iter().any(|r| written.lo <= r.lo && r.hi <= written.hi) || valued.contains(written))\n",
    replace: "        && (returns.iter().any(|r| written.lo <= r.lo && r.hi <= written.hi))\n",
    tests: ["test/lowering.test.ts","-t","a tail loop's break"],
  },
  {
    name: "mir-tidy-jump-ends-not-made",
    breaks: "MIR's block that ends in a loop it never leaves isn't seen to end",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())\n        }\n        Some(StmtKind::If(",
    replace: "            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref()) && false\n        }\n        Some(StmtKind::If(",
    tests: ["test/lowering.test.ts","-t","returns in a loop are where"],
  },
  {
    name: "mir-tidy-while-head-rest-jumps",
    breaks: "MIR's loop's rest, moved out, keeps a jump of the loop, which then goes to another",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            && !own_jumps(rest, label.as_deref())\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","returns in a loop are where"],
  },
  {
    name: "mir-tidy-writer-branches-appended",
    breaks: "MIR's writer writing once in each branch appends to `f` in each, not `return s`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "                returned_unwritten(then, name);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","a writer from its MIR returns the string it writes"],
  },
  {
    name: "mir-tidy-writer-branches-unreturned",
    breaks: "MIR's writer writing once in an `if` and its `else` appends to `f` in each, then returns it",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "StmtKind::If(_, then, Some(els)) if returned && let Some(back) = next => {",
    replace: "StmtKind::If(_, then, Some(els)) if false && let Some(back) = next => {",
    tests: ["test/lowering.test.ts","-t","a writer's Debug builder from its MIR is the string its chain writes"],
  },
  {
    name: "mir-tidy-for-mut-binding-unread",
    breaks: "a `for` of a `mut` binding, `row = value` each turn, stays a `while (true)` of `$next`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "                    && parts.is_empty() =>\n",
    replace: "                    && false =>\n",
    tests: [
      "test/lowering.test.ts",
      "-t",
      "a for loop of a mut binding"
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
    name: "lone-exit-written-order",
    breaks: "a `?`'s `if (!value) { return; }` in a loop is put after the rest, as where the block it shares was written",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            !lone\n",
    replace: "            true\n",
    tests: ["test/corpus.test.ts", "-t", "loop_exits"],
    snapshots: true,
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
    name: "returned-in-place-end-kept",
    breaks: "a block's end, `result = b;` before the `return result;` after it, is left there, the block and its `return` kept",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "    if made_at_ends(body, result) {\n        returned_at_ends(body);\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^interpreter.rs$"],
    snapshots: true,
  },
  {
    name: "returned-in-place-unthreaded",
    breaks: "a `break l` after a loop or an `if` isn't pushed into it, so a `?` inside a loop inside a block is `result = r; break;` to a `return result;`",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "    threaded_jumps(&mut threaded, &label, true);\n",
    replace: "    threaded_jumps(&mut threaded, &label, false);\n",
    tests: ["test/corpus.test.ts", "-t", "^calculator.rs$"],
    snapshots: true,
  },
  {
    name: "made-at-ends-jump-goes-on",
    breaks: "a branch ending in a jump past the block is taken to go on to its end, so the block's `return result;` stays",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "        Some(StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Continue(_) | StmtKind::Break(_)) => true,\n",
    replace: "        Some(StmtKind::Return(_) | StmtKind::Throw(_)) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^interpreter.rs$"],
    snapshots: true,
  },
  {
    name: "tried-first-not-after",
    breaks: "`if (r.TAG !== \"Err\") { s; break l; } return r;` keeps its `?` last",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            None => then.last().is_some_and(ends) && returns(stmts.get(i + 1), test),\n",
    replace: "            None => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^calculator.rs$"],
    snapshots: true,
  },
  {
    name: "tried-first-skips-what-it-held",
    breaks: "a `?` inside the statements another's guard put in place is skipped, its `else { return r; }` kept",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            std::iter::once(StmtKind::If(test.negated_test(), exit, None).at(span)).chain(then),\n        );\n",
    replace: "            std::iter::once(StmtKind::If(test.negated_test(), exit, None).at(span)).chain(then.clone()),\n        );\n        i += then.len();\n",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
    snapshots: true,
  },
  {
    name: "tail-continues-written-dropped",
    breaks: "a `continue` the program wrote at a loop body's end is dropped, and its guard turned",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            if (l.is_none() || l == looped) && more && !span.is_some_and(|span| written(kept, span)) =>\n",
    replace: "            if (l.is_none() || l == looped) && more =>\n",
    tests: ["test/corpus.test.ts", "-t", "^loop_exits.rs$"],
    snapshots: true,
  },
  {
    name: "broken-to-inner-loops",
    breaks: "an inner loop's own `break` is taken for the outer loop's, threaded past the block",
    file: "src/lower/mir/tidy/jumps.rs",
    find: "            StmtKind::Break(l) if (own && l.is_none()) || (l.is_some() && l.as_deref() == looped) => {\n",
    replace: "            StmtKind::Break(l) if l.is_none() || (l.is_some() && l.as_deref() == looped) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(interpreter|calculator|returns_in_place).rs$"],
    snapshots: true,
  },
];
