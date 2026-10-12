// Mutations of src/lower/mir/tidy.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tidy-effectful-init-dropped",
    breaks: "`let a = made(1); a = 2;` is `let a = 2;`, and `made(1)` never runs",
    file: "src/lower/mir/tidy.rs",
    find: "is_none_or(|e| !e.has_effects())",
    replace: "is_none_or(|e| true || !e.has_effects())",
    tests: ["test/mir.test.ts","-t","declarations_assigned_later"],
  },
  {
    name: "tidy-declaration-past-read",
    breaks: "a `let` moves to its next assignment past a read of it, which reads it before it's declared",
    file: "src/lower/mir/tidy.rs",
    find: "                    .find(|&j| js::mentions_in(&stmts[j..=j], name) > 0)\n",
    replace: "                    .find(|&j| matches!(stmts[j].kind, StmtKind::Assign(..)) && js::mentions_in(&stmts[j..=j], name) > 0)\n",
    tests: ["test/mir.test.ts","-t","declarations_assigned_later"],
  },
  {
    name: "tidy-closure-assign-unseen",
    breaks: "a variable a closure assigns is a `const`",
    file: "src/lower/mir/tidy.rs",
    find: "            {\n                assigned.insert(name.clone());\n            }\n        }\n",
    replace: "            {\n                let _ = name;\n            }\n        }\n",
    tests: ["test/mir.test.ts","-t","declarations_assigned_later"],
  },
  {
    name: "tidy-handle-assign-unseen",
    breaks: "a variable a handle's setter assigns is a `const`",
    file: "src/lower/mir/tidy.rs",
    find: "        if let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind\n",
    replace: "        if false && let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind\n",
    tests: ["test/mir.test.ts","-t","mut_ref_handle"],
  },
  {
    name: "mir-tidy-alias-kept",
    breaks: "a `const` read once, by another `const` of it, stays a name for a name",
    file: "src/lower/mir/tidy.rs",
    find: "                            && js::mentions_in(&stmts[j..], a) == 1\n",
    replace: "                            && false\n",
    tests: ["test/lowering.test.ts","-t","stepped by hand"],
  },
  {
    name: "mir-tidy-branch-values-kept",
    breaks: "`if (c) { x = a; } else { x = b; }` stays two branches, not `x = c ? a : b`",
    file: "src/lower/mir/tidy.rs",
    find: "        bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-empty-then-kept",
    breaks: "`if (c) {} else { s }` stays, not `if (!c) { s }`",
    file: "src/lower/mir/tidy.rs",
    find: "            StmtKind::If(test, then, Some(otherwise)) if then.is_empty() => StmtKind::If(not(test), otherwise, None),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","branches from MIR read"],
  },
  {
    name: "mir-tidy-inlined-past-changed-reads",
    breaks: "a temporary that does something is written past reads of what it changes, `d.n + reset(d)`",
    file: "src/lower/mir/tidy.rs",
    find: "            matches!(&root.kind, ExprKind::Var(v) if plain.contains(v) && !set.contains(v))\n",
    replace: "            true\n",
    tests: ["test/mir.test.ts","-t","assignment_order"],
  },
  {
    name: "mir-tidy-returned-chain-kept",
    breaks: "`if (a) { if (b) { return t; } } return f;` stays, not `return a && b ? t : f`",
    file: "src/lower/mir/tidy.rs",
    find: "        stmts.push(StmtKind::Return(Some(value)).at(span));\n",
    replace: "        let _ = value;\n        return;\n",
    tests: ["test/lowering.test.ts","-t","branches from MIR read"],
  },
  {
    name: "mir-tidy-or-chain-kept",
    breaks: "`l: { if (!a) { if (!b) { s; break l; } } t }` stays, not `if (a || b) { t } else { s }`",
    file: "src/lower/mir/tidy.rs",
    find: "            StmtKind::Labeled(label, body) if let Some((tests, otherwise, then)) = or_chain(&body, &label) => {\n",
    replace: "            StmtKind::Labeled(label, body) if let Some((tests, otherwise, then)) = or_chain(&body, &label).filter(|_| false) => {\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops"],
  },
  {
    name: "mir-tidy-not-de-morgan",
    breaks: "`!(a && b)` of `bool`s stays, not `!a || !b`",
    file: "src/lower/mir/tidy.rs",
    find: "            Expr::bin(flipped, not(*a), not(*b))\n",
    replace: "            Expr::unary(UnaryOp::Not, Expr::bin(op, *a, *b))\n",
    tests: ["test/lowering.test.ts","-t","branches from MIR read"],
  },
  {
    name: "mir-tidy-loops-kept",
    breaks: "`const iter = $iter(xs); while (true) { const t = $next(iter); … }` stays, not `for (const x of xs)`",
    file: "src/lower/mir/tidy.rs",
    find: "            stmts.splice(i..i + 2, made);\n",
    replace: "            let _ = made;\n",
    tests: ["test/lowering.test.ts","-t","loops from MIR read"],
  },
  {
    name: "mir-tidy-for-range-kept",
    breaks: "a range's `while (true)` with `$rangeNext` stays, not `for (let i = 0; i < k; i++)`",
    file: "src/lower/mir/tidy.rs",
    find: "            if start_key != \"start\" || end_key != \"end\" || mutable {\n",
    replace: "            if true {\n",
    tests: ["test/lowering.test.ts","-t","loops from MIR read"],
  },
  {
    name: "mir-tidy-early-breaks-kept",
    breaks: "a loop body ending `if (c) { a; continue; } break;` stays, not `if (!c) { break; } a`",
    file: "src/lower/mir/tidy.rs",
    find: "        if !shape {\n",
    replace: "        if true {\n",
    tests: ["test/lowering.test.ts","-t","loops from MIR read"],
  },
  {
    name: "mir-tidy-for-label-kept",
    breaks: "a `for` loop's label nothing names stays",
    file: "src/lower/mir/tidy.rs",
    find: "        | StmtKind::For { label, body, .. } = &mut stmts[i].kind\n",
    replace: "        | StmtKind::For { label, body, .. } = &mut stmts[i].kind\n            && false\n",
    tests: ["test/lowering.test.ts","-t","loops from MIR read"],
  },
  {
    name: "mir-tidy-alias-variables-kept",
    breaks: "`const _5 = TName;` stays, not `TName` where `_5` is read",
    file: "src/lower/mir/tidy.rs",
    find: "                    && matches!(&value.kind, ExprKind::Var(of) if !changing.contains(of)) =>\n",
    replace: "                    && false =>\n",
    tests: ["test/lowering.test.ts","-t","what reads the same anywhere"],
  },
  {
    name: "mir-tidy-destructured-kept",
    breaks: "`const _5 = e; const a = _5[0]; const b = _5[1];` stays, not `const [a, b] = e;`",
    file: "src/lower/mir/tidy.rs",
    find: "        let enough = count >= if made || bound { 1 } else { 2 };\n",
    replace: "        let enough = !made && false;\n",
    tests: ["test/lowering.test.ts","-t","bindings from MIR are destructured"],
  },
  {
    name: "mir-tidy-destructured-variable-kept",
    breaks: "`const a = t[0]; const b = t[1];` of a variable stays, not `const [a, b] = t;`",
    file: "src/lower/mir/tidy.rs",
    find: "                (v.to_string(), i)\n",
    replace: "                (v.to_string(), i + stmts.len())\n",
    tests: ["test/lowering.test.ts","-t","bindings from MIR are destructured"],
  },
  {
    name: "mir-tidy-array-parts-kept",
    breaks: "`const [, b] = [x, 6];` stays, not `const b = 6;`",
    file: "src/lower/mir/tidy.rs",
    find: "            stmts.splice(i..first + count, each);\n            continue;\n",
    replace: "            let _ = each;\n            i += 1;\n            continue;\n",
    tests: ["test/lowering.test.ts","-t","bindings from MIR are destructured"],
  },
  {
    name: "mir-tidy-eager-chains-off",
    breaks: "`$iter(xs).map(f).toArray()` stays, not `xs.map(f)`",
    file: "src/lower/mir/tidy.rs",
    find: "    named_temporaries(body);\n    eager_chains(body);\n",
    replace: "    named_temporaries(body);\n",
    tests: ["test/lowering.test.ts","-t","iterator chains from MIR|a lazy chain from MIR spread once"],
  },
  {
    name: "mir-tidy-params-not-destructured",
    breaks: "`(_2) => { const i = _2[0]; const c = _2[1]; … }` stays, not `([i, c]) => …`",
    file: "src/lower/mir/tidy.rs",
    find: "        body.drain(..count);\n        *param = pattern;\n",
    replace: "        let _ = pattern;\n",
    tests: ["test/lowering.test.ts","-t","iterator chains from MIR|a lazy chain from MIR spread once"],
  },
  {
    name: "mir-tidy-falsy-temporary-kept",
    breaks: "a temporary given where what's falsy is skipped stays `test ? value : undefined`, not `test && value`",
    file: "src/lower/mir/tidy.rs",
    find: "                *value = Expr::bin(Op::And, test.tested(), (**then).clone());\n",
    replace: "",
    tests: ["test/bindings.test.ts","-t","skips what's falsy"],
  },
  {
    name: "mir-tidy-cond-false-kept",
    breaks: "`test ? b : false` of a boolean test stays, not `test && b`",
    file: "src/lower/mir/tidy.rs",
    find: "        (_, ExprKind::Bool(false)) => Expr::bin(Op::And, truth(test), yes),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","branches from MIR read"],
  },
  {
    name: "mir-tidy-and-tests-off",
    breaks: "`if (a) { if (b) { s } }` of one condition stays nested, not `if (a && b)`",
    file: "src/lower/mir/tidy.rs",
    find: "    bottom_up(body, &mut |stmts| and_tests(stmts, conditions));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR|a let chain from MIR is one test"],
  },
  {
    name: "mir-tidy-and-tests-any-condition",
    breaks: "`if`s the program nests are merged into one `&&`",
    file: "src/lower/mir/tidy.rs",
    find: "        let one = (conditions.iter()).any(|(first, then)| within(first, span) && within(then, inner.span));\n",
    replace: "        let one = true;\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-broken-chain-unbound",
    breaks: "a chain of `if let`s whose levels bind a value first stays a labeled block",
    file: "src/lower/mir/tidy.rs",
    find: "        ] if bound\n",
    replace: "        ] if bound.is_empty() && bound\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-for-of-assigned-kept",
    breaks: "a loop inside another, whose iterator is a `let` assigned each time, stays `while (true)` stepping it",
    file: "src/lower/mir/tidy.rs",
    find: "                && mentions.get(iter) == Some(&2) =>\n",
    replace: "                && mentions.get(iter) == Some(&99) =>\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-unused-let-kept",
    breaks: "a `let iter;` nothing assigns any more stays declared",
    file: "src/lower/mir/tidy.rs",
    find: "    body.retain(|stmt| !matches!(&stmt.kind, StmtKind::Let(name, None) if !mentions.contains_key(name)));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-and-chain-negated",
    breaks: "`l: { if (a) { if (b) { s; break l; } } t }` is `if (!a || !b) { t } else { s }`, not `if (a && b) { s } else { t }`",
    file: "src/lower/mir/tidy.rs",
    find: "                    (false, false) => StmtKind::If(test, otherwise, Some(then)),\n",
    replace: "                    (false, false) => StmtKind::If(not(test), then, Some(otherwise)),\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-continue-guard-kept",
    breaks: "a loop body ending `if (!c) { a; b }` stays, not `if (c) { continue; } a; b`",
    file: "src/lower/mir/tidy.rs",
    find: "                    if written(guards, body.last().expect(\"matched\").span) || (negated(test) && then.len() > 1) =>\n",
    replace: "                    if false =>\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-continued-block-kept",
    breaks: "a loop body's last labeled block, its `break` the loop's `continue`, stays",
    file: "src/lower/mir/tidy.rs",
    find: "                    if clashes || continues(&mut inner, &block, looped.as_deref(), false).is_none() {\n",
    replace: "                    if true {\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-for-tail-continue-kept",
    breaks: "a `for` loop body's last `continue l;` stays",
    file: "src/lower/mir/tidy.rs",
    find: "            | StmtKind::ForOf {\n                label: Some(label),\n                body,\n                ..\n            }\n            | StmtKind::For {\n                label: Some(label),\n                body,\n                ..\n            } => drop_tail(body, &StmtKind::Continue(Some(label.clone()))),\n",
    replace: "            => drop_tail(body, &StmtKind::Continue(Some(label.clone()))),\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-innermost-jumps-labeled",
    breaks: "a `continue l` of the innermost loop keeps its label, and so the loop",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| innermost_jumps(stmts));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-temporary-ternary-kept",
    breaks: "an `if` the program wrote as a value, of an element, stays a `let` assigned in branches, not a conditional child",
    file: "src/lower/mir/tidy.rs",
    find: "                    && ((small(yes) && small(no)) || is_temporary(x)) =>\n",
    replace: "                    && small(yes) && small(no) =>\n",
    tests: ["test/mir.test.ts","-t","JSX tests"],
  },
  {
    name: "mir-tidy-tail-guard-kept",
    breaks: "`if (!c) { return; } rest` MIR made of `if c { rest }` stays a guard",
    file: "src/lower/mir/tidy.rs",
    find: "StmtKind::If(_, then, None) if then.last().is_some_and(unwritten)))",
    replace: "StmtKind::If(_, then, None) if false && then.last().is_some_and(unwritten)))",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-written-return-ternary",
    breaks: "`if (!g) { return; } return big;`, as the program wrote it, is `return g ? big : undefined`",
    file: "src/lower/mir/tidy.rs",
    find: "        && !written(returns, last.span)\n        && !writes_return(std::slice::from_ref(last_if), returns)\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","value of two branches is returned"],
  },
  {
    name: "mir-tidy-default-part-unstarted",
    breaks: "parts read with a default don't start a pattern, `const className = p.k === undefined ? d : p.k;` stays",
    file: "src/lower/mir/tidy.rs",
    find: "        ExprKind::Cond(_, _, read) => match &read.kind {\n",
    replace: "        ExprKind::Cond(_, _, read) if false => match &read.kind {\n",
    tests: ["test/lowering.test.ts","-t","props field's default is read"],
  },
  {
    name: "mir-tidy-named-temporaries-off",
    breaks: "`const _4 = e; if (_4 != null) { const found = _4; … }` stays, not `const found = e;`",
    file: "src/lower/mir/tidy.rs",
    find: "    named_temporaries(body);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","if-let's binding from MIR keeps"],
  },
  {
    name: "mir-tidy-named-temporary-kept-binding",
    breaks: "the `const found = _4;` a temporary's renaming took stays, `const found = found;`",
    file: "src/lower/mir/tidy.rs",
    find: "    stmts.retain(|s| {\n        !matches!(&s.kind, StmtKind::Const(n, value)\n            if n == b && matches!(&value.kind, ExprKind::Var(v) if v == a))\n    });\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","if-let's binding from MIR keeps"],
  },
  {
    name: "mir-tidy-captured-kept",
    breaks: "what an element made before a branch reads stays a `const className = cn(…)`, not read where it's written",
    file: "src/lower/mir/tidy.rs",
    find: "        let temporary = is_temporary(name) || captured.contains(name);\n",
    replace: "        let temporary = is_temporary(name);\n",
    tests: ["test/mir.test.ts","-t","JSX tests"],
  },
  {
    name: "mir-tidy-guarded-off",
    breaks: "a `let … else` stays a labeled block holding the rest of the function",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut guarded);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","let-else from MIR is a guard"],
  },
  {
    name: "mir-tidy-let-else-return-nested",
    breaks: "a `let … else { return f }` stays `if (a) { if (b) { return t; } } return f;`, not a guard",
    file: "src/lower/mir/tidy.rs",
    find: "        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if))\n",
    replace: "        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if)).filter(|_| false)\n",
    tests: ["test/lowering.test.ts","-t","let-else from MIR is a guard"],
  },
  {
    name: "mir-tidy-range-end-length-kept",
    breaks: "a range's loop to `v.length` of a `v` the body only reads stays `while (true)`",
    file: "src/lower/mir/tidy.rs",
    find: "                    if length == \"length\" && matches!(&of.kind, ExprKind::Var(v) if only_read(&body, v)));\n",
    replace: "                    if false && length == \"length\" && matches!(&of.kind, ExprKind::Var(v) if only_read(&body, v)));\n",
    tests: ["test/lowering.test.ts","-t","early returns and index loops from MIR"],
  },
  {
    name: "mir-tidy-endless-loop-goes-on",
    breaks: "what follows a `while (true)` no `break` leaves is kept, `return args; return result;`",
    file: "src/lower/mir/tidy.rs",
    find: "            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())\n",
    replace: "            false && !breaks(body, label.as_deref())\n",
    tests: ["test/lowering.test.ts","-t","returned after a loop"],
  },
  {
    name: "mir-tidy-boolean-variable-unknown",
    breaks: "a variable of `booleans` isn't a `bool` to `conditional`, `!!expanded`",
    file: "src/lower/mir/tidy.rs",
    find: "        ExprKind::Var(name) => booleans.contains(name),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","value of two branches is returned"],
  },
  {
    name: "mir-tidy-truth-uncoerced",
    breaks: "`o ? true : r` of an object `o` is `o || r`, an object where a `bool` belongs",
    file: "src/lower/mir/tidy.rs",
    find: "        false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, test)),\n",
    replace: "        false => test,\n",
    tests: ["test/lowering.test.ts","-t","value of two branches is returned"],
  },
  {
    name: "mir-tidy-tested-kept",
    breaks: "a test a skips-falsy call reads alone keeps its `!!`, `(!!error || ready) && \"b\"`",
    file: "src/lower/mir/tidy.rs",
    find: "                *value = value.tested();\n",
    replace: "",
    tests: ["test/bindings.test.ts","-t","skips what's falsy"],
  },
  {
    name: "mir-tidy-branch-ends-kept",
    breaks: "a `match`'s arms stay labeled blocks, `bb1: { if (c) { s; break bb1; } t }`",
    file: "src/lower/mir/tidy.rs",
    find: "            ending(body, label);\n",
    replace: "            let _ = (label, body);\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-inner-block-kept",
    breaks: "a block that ends another stays a block of its own, its `break`s labeled",
    file: "src/lower/mir/tidy.rs",
    find: "        stmts.pop();\n        stmts.extend(body);\n",
    replace: "        let _ = body;\n        return;\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-or-pattern-negated",
    breaks: "an or-pattern's arm is `if (a !== x && a !== y) { t } else { s }`, not `if (a === x || a === y) { s }`",
    file: "src/lower/mir/tidy.rs",
    find: "        (false, true) => StmtKind::If(not(test), rest, Some(then)).at(span),\n",
    replace: "        (false, true) => StmtKind::If(test, then, Some(rest)).at(span),\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-implied-guard-kept",
    breaks: "`if (x != null) { if (x === \"a\") { s } }` keeps the guard its chain says",
    file: "src/lower/mir/tidy.rs",
    find: "            StmtKind::If(test, then, None) if !then.is_empty() && then.iter().all(|s| guarded_chain(s, test)) => {\n",
    replace: "            StmtKind::If(test, then, None) if false && !then.is_empty() && then.iter().all(|s| guarded_chain(s, test)) => {\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-implied-by-or-unread",
    breaks: "`a || b` doesn't say what both its sides say, `x != null` of `x === \"d\" || x === \"i\"`",
    file: "src/lower/mir/tidy.rs",
    find: "        (_, ExprKind::Binary(Op::Or, a, b)) => implies(a, test) && implies(b, test),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-guarded-chain-else",
    breaks: "a chain with a last `else` drops its guard, its `else` then run where the guard fails",
    file: "src/lower/mir/tidy.rs",
    find: "                && (otherwise.as_deref()).is_none_or(|rest| matches!(rest, [next] if guarded_chain(next, guard)))\n        }\n        _ => false,\n",
    replace: "                && (otherwise.as_deref()).is_none_or(|rest| matches!(rest, [next] if guarded_chain(next, guard)))\n        }\n        _ => true,\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-let-else-branch-ends",
    breaks: "a `let … else`'s block takes its `throw` as an `else`, not a guard",
    file: "src/lower/mir/tidy.rs",
    find: "        .filter(|_| !exits)\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","let-else from MIR"],
  },
  {
    name: "mir-tidy-tail-return-branch-kept",
    breaks: "`if (c) { s; return; } t` where the program wrote no `return` stays, not `if (c) { s } else { t }`",
    file: "src/lower/mir/tidy.rs",
    find: "        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if then.last().is_some_and(unwritten)))\n",
    replace: "        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if matches!(then.as_slice(), [r] if unwritten(r))))\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-tail-return-else-lost",
    breaks: "`if (c) { s; return; } t` is `if (c) { s }`, `t` lost",
    file: "src/lower/mir/tidy.rs",
    find: "            (false, false) => stmts.push(StmtKind::If(test, then, Some(rest)).at(span)),\n",
    replace: "            (false, false) => stmts.push(StmtKind::If(test, then, None).at(span)),\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-filtered-guard-kept",
    breaks: "a let-else of what `filter` keeps tests `const h = x ? x : undefined` for `null`, not `!x`",
    file: "src/lower/mir/tidy.rs",
    find: "            Op::LooseEq => otherwise.is_none() && then.last().is_some_and(ends),\n",
    replace: "            Op::LooseEq => false,\n",
    tests: ["test/lowering.test.ts","-t","what .filter. keeps"],
  },
  {
    name: "mir-tidy-not-of-booleans-only",
    breaks: "`!(a !== b && c)` of a test not all `bool`s stays, not `a === b || !c`",
    file: "src/lower/mir/tidy.rs",
    find: "        ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => {\n",
    replace: "        ExprKind::Binary(op @ (Op::And | Op::Or), a, b) if is_boolean(&a, &HashSet::new()) && is_boolean(&b, &HashSet::new()) => {\n",
    tests: ["test/lowering.test.ts","-t","De Morgan"],
  },
  {
    name: "mir-tidy-filter-or-kept",
    breaks: "`(x ? x : undefined) ?? y` stays, not `x || y`",
    file: "src/lower/mir/tidy.rs",
    find: "            && truthy_or_undefined(kept)\n",
    replace: "            && false\n",
    tests: ["test/lowering.test.ts","-t","De Morgan"],
  },
  {
    name: "mir-tidy-filter-or-chain",
    breaks: "`(x ? x : undefined) ?? (y ? y : undefined) ?? z` isn't known as one, `x || y || z`",
    file: "src/lower/mir/tidy.rs",
    find: "        ExprKind::Binary(Op::Coalesce, a, b) => truthy_or_undefined(a) && truthy_or_undefined(b),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","De Morgan"],
  },
  {
    name: "mir-tidy-optional-member-not-same",
    breaks: "`a?.b` isn't the same as `a?.b`, its `filter` kept, `a?.b ? a?.b : undefined`",
    file: "src/lower/mir/tidy.rs",
    find: "        | (ExprKind::OptionalMember(x, p), ExprKind::OptionalMember(y, q)) => p == q && same(x, y),\n",
    replace: "        => p == q && same(x, y),\n",
    tests: ["test/lowering.test.ts","-t","De Morgan"],
  },
  {
    name: "mir-tidy-negated-null-test",
    breaks: "`!(x != null && c)` as `not` writes it, `x == null || !c`, isn't known as a negation, a loop's `continue` lost",
    file: "src/lower/mir/tidy.rs",
    find: "                negated(e)\n                    || matches!(&e.kind, ExprKind::Binary(Op::LooseEq, _, null) if matches!(null.kind, ExprKind::Null | ExprKind::Undefined))\n",
    replace: "                negated(e)\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-deref-param-kept",
    breaks: "`|&n|` stays `(_2) => { const n = _2; … }`",
    file: "src/lower/mir/tidy.rs",
    find: "            *param = js::Pattern::Name(name.clone());\n            body.remove(0);\n",
    replace: "            break;\n",
    tests: ["test/lowering.test.ts","-t","names its .&x. parameter"],
  },
  {
    name: "mir-tidy-lets-first",
    breaks: "each `let` stays declared first, not where it's first named",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut sunk_lets);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","declared where it.s first named"],
  },
  {
    name: "mir-tidy-let-not-into-branch",
    breaks: "a `let` one branch alone names is declared before the `if`, not in the branch",
    file: "src/lower/mir/tidy.rs",
    find: "            if let Some(branch) = branch {\n",
    replace: "            if let Some(branch) = branch.filter(|_| false) {\n",
    tests: ["test/lowering.test.ts","-t","declared where it.s first named"],
  },
  {
    name: "mir-tidy-element-read-held",
    breaks: "what an element made ahead of a branch reads stays a `const` where elements before it come first, `className$1`",
    file: "src/lower/mir/tidy.rs",
    find: "            true if element => {\n",
    replace: "            true if false && element => {\n",
    tests: ["test/mir.test.ts","-t","JSX tests"],
  },
  {
    name: "mir-tidy-late-consts-kept",
    breaks: "a `const` declarations makes last stays, `const _27 = e; _24 = _27;`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| guarded_lets(stmts, captured));\n    js::statement_lists(body, &mut |stmts| inlined(stmts, &changed, &set, plain, captured));\n",
    replace: "    js::statement_lists(body, &mut |stmts| guarded_lets(stmts, captured));\n",
    tests: ["test/lowering.test.ts","-t","else if. given as a value"],
  },
  {
    name: "mir-tidy-filtered-some-kept",
    breaks: "`if (h != null) { s }` of what `filter` keeps stays, `h` and its binding `const href = h`",
    file: "src/lower/mir/tidy.rs",
    find: "                otherwise.as_deref().is_none_or(|o| js::mentions_in(o, h) == 0)\n                    && js::mentions_in(&stmts[i + 1..], h) == 0\n",
    replace: "                false\n",
    tests: ["test/mir.test.ts","-t","JSX tests"],
  },
  {
    name: "mir-tidy-result-assigned-kept",
    breaks: "`if (c) { result = a; } else { result = b; } return result;` stays",
    file: "src/lower/mir/tidy.rs",
    find: "    returned_in_branches(body, result);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","branches assign is returned from each"],
  },
  {
    name: "mir-tidy-else-after-return-kept",
    breaks: "`if (c) { return a; } else { s }`, its `else` kept, the let-else not a guard",
    file: "src/lower/mir/tidy.rs",
    find: "        StmtKind::If(test, then, Some(otherwise)) if then.last().is_some_and(ends) => {\n",
    replace: "        StmtKind::If(test, then, Some(otherwise)) if false && then.last().is_some_and(ends) => {\n",
    tests: ["test/lowering.test.ts","-t","branches assign is returned from each"],
  },
  {
    name: "mir-tidy-let-else-of-written-returns",
    breaks: "`if c { return a; } else { return b; }`, both written, is `if (!c) { return b; } return a;`",
    file: "src/lower/mir/tidy.rs",
    find: "        && !body.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(_)) && written(returns, s.span))\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","string's parts from MIR"],
  },
  {
    name: "mir-tidy-inlined-no-backtrack",
    breaks: "of temporaries one after another, only the last is read where it's read, `cn(_8, _14, x)`",
    file: "src/lower/mir/tidy.rs",
    find: "        i = i.saturating_sub(1);\n",
    replace: "",
    tests: ["test/mir.test.ts","-t","JSX tests"],
  },
  {
    name: "mir-tidy-range-end-unheld",
    breaks: "a range loop whose end is a call stays `while (true)` stepping `$rangeNext`",
    file: "src/lower/mir/tidy.rs",
    find: "            let held = (!same).then(|| {\n",
    replace: "            if !same {\n                return None;\n            }\n            let held = (!same).then(|| {\n",
    tests: ["test/lowering.test.ts","-t","whose end is a call holds it first"],
  },
  {
    name: "mir-tidy-chars-arrayed",
    breaks: "a string's `chars()` iterated is `Array.from(code)`",
    file: "src/lower/mir/tidy.rs",
    find: "            iterable: unarrayed(iterated(source.clone())),\n",
    replace: "            iterable: iterated(source.clone()),\n",
    tests: ["test/lowering.test.ts","-t","arguments read out of order"],
  },
  {
    name: "mir-tidy-plain-cells-boxed",
    breaks: "a cell only its function reads and sets stays `{ value }`",
    file: "src/lower/mir/tidy.rs",
    find: "    plain_cells(body);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","only its function reads and sets"],
  },
  {
    name: "mir-tidy-plain-cell-escapes",
    breaks: "a cell given whole, `bump(c)`, is a variable too, and what it's given reads `undefined`",
    file: "src/lower/mir/tidy.rs",
    find: "    let plain: HashSet<&String> = (cells.iter()).filter(|c| reads.get(*c) == kept.get(*c)).collect();\n",
    replace: "    let plain: HashSet<&String> = cells.iter().collect();\n",
    tests: ["test/lowering.test.ts","-t","only its function reads and sets"],
  },
  {
    name: "mir-tidy-inlined-not-past",
    breaks: "a temporary is read in place only by the statement right after it, `const _34 = formatted; const args$1 = args;` kept",
    file: "src/lower/mir/tidy.rs",
    find: "        let past = |at: usize| {\n",
    replace: "        let past = |at: usize| at == 0 && {\n",
    tests: ["test/mir.test.ts","-t","JSX tests"],
  },
  {
    name: "mir-tidy-while-test-kept",
    breaks: "`while (true) { if (!c) { return x; } … }` stays, not `while (c) { … } return x;`",
    file: "src/lower/mir/tidy.rs",
    find: "    bottom_up(body, &mut |stmts| loops(stmts, &mentions));\n    js::statement_lists(body, &mut |stmts| while_tests(stmts, written));\n",
    replace: "    bottom_up(body, &mut |stmts| loops(stmts, &mentions));\n",
    tests: ["test/lowering.test.ts","-t","while. loop from MIR tests"],
  },
  {
    name: "mir-tidy-loop-unrotated",
    breaks: "`while (true) { if (c) { a; continue; } return x; }` stays, not `while (c) { a } return x;`",
    file: "src/lower/mir/tidy.rs",
    find: "            && !goes_on(exit, label.as_deref())\n",
    replace: "            && !goes_on(exit, label.as_deref())\n            && false\n",
    tests: ["test/lowering.test.ts","-t","while. loop from MIR tests"],
  },
  {
    name: "mir-tidy-loop-rotated-unclosed",
    breaks: "a path out of a rotated loop's `if` goes on to its next turn, not out",
    file: "src/lower/mir/tidy.rs",
    find: "                looped.push(StmtKind::Break(None).at(span));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","while. loop from MIR tests"],
  },
  {
    name: "mir-tidy-loop-as-while",
    breaks: "a `loop` the program wrote, its first test leaving, is `while (!t)`",
    file: "src/lower/mir/tidy.rs",
    find: "        if !written_while(stmts[i].span) {\n",
    replace: "        if false && !written_while(stmts[i].span) {\n",
    tests: ["test/lowering.test.ts","-t","while. loop from MIR tests"],
  },
  {
    name: "mir-tidy-written-continue-nested",
    breaks: "a `continue` the program wrote before one statement is the rest nested, `if (!c) { … }`",
    file: "src/lower/mir/tidy.rs",
    find: "                    if written(guards, body.last().expect(\"matched\").span) || (negated(test) && then.len() > 1) =>\n",
    replace: "                    if negated(test) && then.len() > 1 =>\n",
    tests: ["test/lowering.test.ts","-t","continue.s from MIR are where"],
  },
  {
    name: "mir-tidy-callback-unnamed",
    breaks: "a hook's closure a `let` names stays `const tick = () => …; useEffect(tick, …)`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| named_callbacks(stmts, callbacks));\n",
    replace: "",
    tests: ["test/mir.test.ts","-t","JSX tests"],
  },
  {
    name: "mir-tidy-loop-let-outside",
    breaks: "a `let` a loop's body assigns before it's read is declared outside the loop, `let start;`",
    file: "src/lower/mir/tidy.rs",
    find: "            && !looped_reads(&stmts[first], &name)\n",
    replace: "            && false\n",
    tests: ["test/lowering.test.ts","-t","loop.s body makes, from MIR"],
  },
  {
    name: "mir-tidy-guards-kept",
    breaks: "a guard of a handle made for it alone stays `$borrowMut({ get value() { … } }, true)`",
    file: "src/lower/mir/tidy.rs",
    find: "    // A guard of a handle made for it alone: the place it's a handle on.\n    unguarded(body);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","field only ever borrowed for a moment"],
  },
  {
    name: "mir-tidy-rethrow-kept",
    breaks: "`try { s } catch (error) { throw error; }` stays, not `s`",
    file: "src/lower/mir/tidy.rs",
    find: "                && let [rethrow] = handler.as_slice()\n",
    replace: "                && let [rethrow] = handler.as_slice().get(..0).unwrap_or_default()\n",
    tests: ["test/lowering.test.ts","-t","field only ever borrowed for a moment"],
  },
  {
    name: "mir-tidy-inner-let-else-kept",
    breaks: "`if (c) { s } else { return; }`, a let-else's, stays, not the guard `if (!c) { return; } s`",
    file: "src/lower/mir/tidy.rs",
    find: "        && !then.last().is_some_and(ends)\n    {\n        let Some(Stmt {\n            kind: StmtKind::If(test, then, Some(otherwise)),\n",
    replace: "        && false\n    {\n        let Some(Stmt {\n            kind: StmtKind::If(test, then, Some(otherwise)),\n",
    tests: ["test/lowering.test.ts","-t","inside a branch, its .else. a .return."],
  },
  {
    name: "mir-tidy-let-else-binding-after",
    breaks: "a let-else's binding is after its guard, which tests what it's bound to",
    file: "src/lower/mir/tidy.rs",
    find: "                Some(b.clone())\n            }\n            _ => None,\n        };\n        let test = match &bound {",
    replace: "                let _ = b;\n                None::<String>\n            }\n            _ => None,\n        };\n        let test = match &bound {",
    tests: ["test/lowering.test.ts","-t","inside a branch, its .else. a .return."],
  },
  {
    name: "mir-tidy-same-item-unknown",
    breaks: "`xs[i].f` isn't known as the same as `xs[i].f`",
    file: "src/lower/mir/tidy.rs",
    find: "        (ExprKind::Index(x, i), ExprKind::Index(y, j)) => {\n",
    replace: "        (ExprKind::Index(x, i), ExprKind::Index(y, j)) if false => {\n",
    tests: ["test/lowering.test.ts","-t","inside a branch, its .else. a .return."],
  },
  {
    name: "mir-tidy-constant-objects-block",
    breaks: "a call isn't read in place past an object of constants before it, `const _3 = unquoted(..)`",
    file: "src/lower/mir/tidy.rs",
    find: "        ExprKind::Cond(a, b, c) => past(a) && past(b) && past(c),\n        _ => e.is_made_of_constants(),\n",
    replace: "        ExprKind::Cond(a, b, c) => past(a) && past(b) && past(c),\n        _ => e.is_constant(),\n",
    tests: ["test/lowering.test.ts","-t","past an object of constants"],
  },
  {
    name: "mir-tidy-late-tail-returns",
    breaks: "a `return;` the late passes leave at a function's end stays",
    file: "src/lower/mir/tidy.rs",
    find: "    // What those leave at the end, a `return;` the program didn't write.\n    tail_returns(body, returns);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","of .matches!. from MIR"],
  },
  {
    name: "mir-tidy-let-outside-label",
    breaks: "a `let` only a labeled block names stays before it, `let _7;`",
    file: "src/lower/mir/tidy.rs",
    find: "        if alone && let StmtKind::Labeled(_, body) = &mut stmts[first].kind {\n",
    replace: "        if false && alone && let StmtKind::Labeled(_, body) = &mut stmts[first].kind {\n",
    tests: ["test/lowering.test.ts","-t","of .matches!. from MIR"],
  },
  {
    "name": "async-arg-named",
    "breaks": "`__arg0`, an `async fn`'s parameter, is a name the program wrote, kept",
    "file": "src/lower/mir/tidy.rs",
    "find": "        is_temporary(name) || arg.is_some_and(",
    "replace": "        is_temporary(name) || false && arg.is_some_and(",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an async fn's parameter pattern from MIR"
    ]
  },
  {
    "name": "break-test-nested-unread",
    "breaks": "a guard that tests on, `if (k) { if (!m) { .. } }`, isn't one of the `||`",
    "file": "src/lower/mir/tidy.rs",
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
    "file": "src/lower/mir/tidy.rs",
    "find": "Some(Expr::bin(Op::Or, not(test.clone()), inner(then)?)),\n",
    "replace": "Some(Expr::bin(Op::Or, test.clone(), inner(then)?)),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an or of ands from MIR is one test"
    ]
  },
  {
    "name": "late-while-tests-off",
    "breaks": "a `while` whose test MIR held first, `const _7 = m.length;`, stays `while (true)` and a `return`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    // > n)) { .. } }`'s of `const _7 = m.length;`.\n    js::statement_lists(body, &mut |stmts| while_tests(stmts, written));\n",
    "replace": "    // > n)) { .. } }`'s of `const _7 = m.length;`.\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a while in a closure from MIR keeps its test"
    ]
  },
  {
    "name": "builtin-callee-blocks-inline",
    "breaks": "a call's temporary given to `Object.fromEntries` stays a `const`, and no object literal is made of it",
    "file": "src/lower/mir/tidy.rs",
    "find": "\"Object\" | \"Math\"",
    "replace": "\"Object!\" | \"Math\"",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "read in place by a builtin's function"
    ]
  },
  {
    "name": "shown-child-conditional",
    "breaks": "a child shown where a test holds is `shown ? hint : undefined`, not `shown && hint`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    && matches!(none.kind, ExprKind::Undefined)\n                {\n                    let test = match is_boolean(test, booleans) {",
    "replace": "                    && false\n                {\n                    let test = match is_boolean(test, booleans) {",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "table-reads-off",
    "breaks": "a `match` reading a table is MIR's tests of each variant, not `MAP[kind]`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    let within = |s: js::Span| !s.is_none() && tables.iter().any(|t| t.lo <= s.lo && s.hi <= t.hi);",
    "replace": "    let within = |s: js::Span| false && !s.is_none() && tables.iter().any(|t| t.lo <= s.lo && s.hi <= t.hi);",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match reading a table from MIR"
    ]
  },
  {
    "name": "table-reads-returns-kept",
    "breaks": "a `match` of returns, each its variant's own name, stays a return for each, not `return kind;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                stmts.splice(i..=i + returned, [StmtKind::Return(Some(read)).at(span)]);\n",
    "replace": "                let _ = (read, span);\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match reading a table from MIR"
    ]
  },
  {
    "name": "table-read-own-names-off",
    "breaks": "a `match` giving each variant's own name isn't what's matched",
    "file": "src/lower/mir/tidy.rs",
    "find": "        None if names == arms.len() => Some((**key).clone()),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match reading a table from MIR"
    ]
  },
  {
    "name": "table-read-mixed-arms",
    "breaks": "a `match` of a table's fields and a literal reads the table for the literal's variant too",
    "file": "src/lower/mir/tidy.rs",
    "find": "        Some(table) if fields.len() == arms.len() =>",
    "replace": "        Some(table) =>",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match reading a table from MIR"
    ]
  },
  {
    "name": "negated-and-untruthed",
    "breaks": "`!o ? false : f()` is `o && f()`, `undefined` where `o` is, not `false`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        (ExprKind::Bool(false), _) => Expr::bin(Op::And, truth(negate(test)), no),\n",
    "replace": "        (ExprKind::Bool(false), _) => Expr::bin(Op::And, negate(test), no),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a truthy option's is_some given as a value"
    ]
  },
  {
    "name": "negated-value-untruthed",
    "breaks": "`!o ? false : true` is `o`, the object, not `true`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        (ExprKind::Bool(false), ExprKind::Bool(true)) => truth(negate(test)),\n",
    "replace": "        (ExprKind::Bool(false), ExprKind::Bool(true)) => negate(test),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a truthy option's is_some given as a value"
    ]
  },
  {
    "name": "negated-or-untruthed",
    "breaks": "`o ? f() : true` of `is_none()`'s is `o || f()`, the object, not `true`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        (_, ExprKind::Bool(true)) => Expr::bin(Op::Or, truth(negate(test)), yes),\n",
    "replace": "        (_, ExprKind::Bool(true)) => Expr::bin(Op::Or, negate(test), yes),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a truthy option's is_some given as a value"
    ]
  },
  {
    "name": "late-eager-chains-off",
    "breaks": "an array's iterator read in place late stays `$lent($iter(xs)).some(f)`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    // $iter(xs);`: the array's, as THIR's.\n    eager_chains(body);\n",
    "replace": "    // $iter(xs);`: the array's, as THIR's.\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an array's iterator read in place late"
    ]
  },
  {
    "name": "late-not-unsimplified",
    "breaks": "`!matches!(t, \"pre\")` read in place stays `!(t === \"pre\")`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                ..Expr::unary(*op, (**arg).clone())\n",
    "replace": "                kind: ExprKind::Unary(*op, arg.clone()),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "negated matches, string methods"
    ]
  },
  {
    "name": "target-root-not-inlined",
    "breaks": "`const _25 = ref.current; _25[k] = v` stays, not `ref.current[k] = v` (and no `??=` of it)",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    (true, true) => Some(target),\n",
    "replace": "                    (true, true) => None,\n",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "one-field-pattern-kept-apart",
    "breaks": "a `let` pattern of one field of a named variable is `const message = error.message;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "let enough = count >= if made || bound { 1 } else { 2 };",
    "replace": "let enough = count >= if made { 1 } else { 2 };",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let pattern of a named variable takes it apart"
    ]
  },
  {
    "name": "destructured-param-kept",
    "breaks": "an `async fn`'s parameter taken apart in its body stays `(__arg0) { const { params } = __arg0;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "            *param = pattern.clone();\n            body.remove(0);\n            continue;\n",
    "replace": "            let _ = pattern;\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an async fn's parameter pattern from MIR"
    ]
  },
  {
    "name": "read-handles-off",
    "breaks": "a cell field a pattern binds, only read, is a handle, `{ get value() { .. } }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    read_handles(body);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a cell field a let pattern binds"
    ]
  },
  {
    "name": "read-handles-set",
    "breaks": "a cell field a pattern binds and sets is its value read once, its set lost",
    "file": "src/lower/mir/tidy.rs",
    "find": "        !set && js::mentions_in(&rest, name) == 0\n",
    "replace": "        js::mentions_in(&rest, name) == 0\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a cell field a let pattern binds"
    ]
  },
  {
    "name": "block-returns-kept",
    "breaks": "returns MIR jumps out of a block for stay `result = x; break l;` and one `return result;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    returned_from_block(stmts, result);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "written returns MIR jumps out of a block"
    ]
  },
  {
    "name": "let-else-return-not-guard",
    "breaks": "a `let .. else`'s return stays the `else` of a block, not a guard first",
    "file": "src/lower/mir/tidy.rs",
    "find": "                stmt.kind = StmtKind::If(not(test), otherwise, Some(then));\n",
    "replace": "                stmt.kind = StmtKind::If(test, then, Some(otherwise));\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "written returns MIR jumps out of a block"
    ]
  },
  {
    "name": "or-branches-reversed",
    "breaks": "`if a || b { x } else { y }` from MIR is `!a && !b ? y : x`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    true => conditional(not(test.clone()), no.clone(), yes.clone(), booleans),\n",
    "replace": "                    true => conditional(test.clone(), yes.clone(), no.clone(), booleans),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an if of an or from MIR keeps its branches"
    ]
  },
  {
    "name": "plain-field-blocks-inline",
    "breaks": "a prop MIR holds for a borrow stays a `const _14`, read past `meta.tag` of a plain value",
    "file": "src/lower/mir/tidy.rs",
    "find": "            matches!(&root.kind, ExprKind::Var(v) if plain.contains(v) && !set.contains(v))\n",
    "replace": "            false\n",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-import-blocks-inline",
    "breaks": "a prop MIR holds for a borrow stays a `const _13`, read past an imported tag, `<Seo>`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Symbol(_) => true,\n        ExprKind::Arrow(..)",
    "replace": "        ExprKind::Arrow(..)",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-past-borrow-blocks",
    "breaks": "a chain stays a `const _4` ahead of a callback's borrow, `const list$1 = list;`, not the array's own `forEach`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        let past = |at: usize| {\n",
    "replace": "        let past = |at: usize| (!acts || at == 0) && {\n",
    "tests": ["test/mir.test.ts", "-t", "JSX tests"]
  },
  {
    "name": "mir-tidy-passed-on-unread",
    "breaks": "an array method's callback from MIR that passes on what it's given reads `xs.map(parseFloat)`, a JS function's, which may read the index",
    "file": "src/lower/mir/tidy.rs",
    "find": "        && forwarding.contains(&f.span)\n",
    "replace": "        && true\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "only passes its arguments on"
    ]
  },
  {
    "name": "mir-tidy-spread-spreads-kept",
    "breaks": "a copy MIR made of a dictionary spread stays `{ ...base, ...{ ...extra } }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::each_expr_mut(body, &mut spread_spreads);\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-checked-sets-kept",
    "breaks": "an element's place MIR checks then sets stays two statements, `$index(args, 0); args[0] = v;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut checked_sets);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "checked element set is one statement"
    ]
  },
  {
    "name": "mir-tidy-or-chain-negated",
    "breaks": "an `||` chain from MIR is `!(!a && b !== c) || d`, its test negated whole",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Binary(Op::And | Op::Or, ..) => negated_bools(test, booleans),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an or chain"
    ]
  },
  {
    "name": "mir-tidy-bool-field-unknown",
    "breaks": "a `bool` field from MIR isn't known as one, `!!(element.editable || ..)`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Member(..) => field_path(e).is_some_and(|path| booleans.contains(&path)),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "or chain from MIR is written as one"
    ]
  },
  {
    "name": "mir-tidy-chain-first",
    "breaks": "a match's arms as values from MIR chain with the rest first, `variant !== \"foundation\" ? .. : [\"Foundation\", ..]`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    (false, true) => false,\n",
    "replace": "                    (false, true) => true,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arms as values chain in written order"
    ]
  },
  {
    "name": "mir-tidy-or-pattern-blocks",
    "breaks": "an or-pattern's matches! from MIR tests its first pattern alone",
    "file": "src/lower/mir/tidy.rs",
    "find": "                StmtKind::If(or_all(a, b), then, None)\n",
    "replace": "                StmtKind::If(a, then, None)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "or-pattern's matches is an or"
    ]
  },
  {
    "name": "mir-tidy-written-first-unswapped",
    "breaks": "an if-let chain from MIR keeps its first arm last, the panic's test first: `if (kind.TAG !== \"Text\") { .. throw } message = ..`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        js::statement_lists(body, &mut |stmts| written_first(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "if-let chain ending in a panic"
    ]
  },
  {
    "name": "mir-tidy-tail-elses-kept",
    "breaks": "an else-if chain's arm from MIR breaks its block, `if (c) { s; break bb19; } throw ..`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| tail_elses(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "if-let chain ending in a panic"
    ]
  },
  {
    "name": "mir-tidy-typeof-null-kept",
    "breaks": "a typeof test from MIR keeps the null check before it, `given != null && typeof given === \"string\"`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::each_expr_mut(body, &mut typeof_tests);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "typeof test after a null check"
    ]
  },
  {
    "name": "mir-tidy-chained-checks-kept",
    "breaks": "an else-if chain's arm before a panic from MIR is a check in its else, `if (className !== \"language-css\") throw ..`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| chained_checks(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arm before a panic stays an else if"
    ]
  },
  {
    "name": "mir-tidy-sibling-element-blocks",
    "breaks": "a prop MIR holds for a borrow stays a `const _16`, read past a sibling element, `<Search isOpen={show} />`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Jsx(_) => inert_element(e, set),\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-written-chains-kept",
    "breaks": "a match's arms as statements from MIR keep their first arm last, `if (message.TAG !== \"resize\") { .. } else { .. }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| written_chains(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arms as statements chain in written order"
    ]
  },
  {
    "name": "mir-tidy-guarded-lets-kept",
    "breaks": "an element MIR makes only where a test holds stays `let className$2; if (error != null) { .. }`, the props before it held too",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| guarded_lets(stmts, captured));\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-implied-tests-kept",
    "breaks": "a value read where its test holds keeps the test, `error != null && <div className={error != null ? cn(..) : undefined}>`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::each_expr_mut(body, &mut |e| implied_tests(e, &changed));\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-guarded-read-blocked",
    "breaks": "a value made where its test holds isn't read where that test holds, `const className$2 = error != null ? cn(..) : undefined` kept",
    "file": "src/lower/mir/tidy.rs",
    "find": "            if guard.is_some_and(|g| same_test(a, g))",
    "replace": "            if false && guard.is_some_and(|g| same_test(a, g))",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-copied-bindings-kept",
    "breaks": "an if-let's binding of a variable MIR copies stays `const error = raw;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| copied_bindings(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "copy binding of a variable is the variable"
    ]
  },
  {
    "name": "mir-tidy-same-test-truth",
    "breaks": "`!!error` isn't the same test as `error`, so a value made where an object's truth holds stays a `const`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    let (a, b) = (truth(a), truth(b));\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-empty-branches-kept",
    "breaks": "a match arm that does nothing from MIR is `else if (message !== \"refresh\") {}`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    bottom_up(body, &mut empty_branches);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "match arm that does nothing"
    ]
  },
  {
    "name": "mir-tidy-tuple-reads-kept",
    "breaks": "children MIR holds as a tuple stay `const children = [..]`, read `children[0]`, and its `then` closure a `const value`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| tuple_reads(stmts, &changing, captured));\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-tuple-set-through",
    "breaks": "a tuple MIR sets an item of, `t[0] = x`, is read as its items, which crashes printing what's assigned",
    "file": "src/lower/mir/tidy.rs",
    "find": "if sets_through(&stmts[i + 1..], &t) || indexed == 0",
    "replace": "if indexed == 0",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "mut_ref_temporary"
    ]
  },
  {
    "name": "mir-tidy-dyn-pairs-kept",
    "breaks": "dyn values MIR gives JS stay `{ value, impl }` pairs, unwrapped with `.map((item) => item.value)`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| unwrapped_values(stmts));\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-closure-made-blocks",
    "breaks": "a closure's making, `const value = () => ..`, blocks props held before it, `const value$3 = cn(..)`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,\n        ExprKind::Member(of, _) => {\n",
    "replace": "        ExprKind::Member(of, _) => {\n",
    "tests": ["test/mir.test.ts", "-t", "JSX tests"]
  },
  {
    "name": "mir-tidy-choice-blocks",
    "breaks": "a conditional of parameters, `selected ? \"page\" : undefined`, blocks a prop held after it",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Cond(a, b, c) => past(a) && past(b) && past(c),\n",
    "replace": "",
    "tests": ["test/mir.test.ts", "-t", "JSX tests"]
  },
  {
    "name": "mir-tidy-lets-block",
    "breaks": "a `let` no closure sets, `target`, blocks a prop held after it, as any variable set again",
    "file": "src/lower/mir/tidy.rs",
    "find": "    let set = closures_set(body);\n",
    "replace": "    let set = changed.clone();\n",
    "tests": ["test/mir.test.ts", "-t", "JSX tests"]
  },
  {
    "name": "mir-tidy-returned-consts-kept",
    "breaks": "what a function gives from MIR stays `const result = e; return result;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| returned_consts(stmts, result));\n",
    "replace": "",
    "tests": ["test/lowering.test.ts", "-t", "or chain of a getter's bool"]
  },
  {
    "name": "mir-tidy-concise-handlers-kept",
    "breaks": "a held handler of one call from MIR keeps its block, `() => { f(); }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    && concise.contains(&value.span)\n",
    "replace": "                    && false\n",
    "tests": ["test/mir.test.ts", "-t", "JSX tests"]
  },
  {
    "name": "mir-tidy-bool-call-unknown",
    "breaks": "a call giving a `bool` isn't one in a test, `!!isItem(item) && ..`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Call(..) if call_path(e).is_some_and(|path| booleans.contains(&path)) => true,\n",
    "replace": "",
    "tests": ["test/lowering.test.ts", "-t", "call giving a bool is one in a test"]
  },
  {
    "name": "mir-pure-value-held-past-calls",
    "breaks": "what does nothing, of a `let` no closure sets, `showToc && toc.length !== 0`, is held ahead of the calls before its read",
    "file": "src/lower/mir/tidy.rs",
    "find": "unchanged_by_calls || !e.has_effects()",
    "replace": "!e.has_effects()",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "inert-element-chosen-not",
    "breaks": "a child chosen of elements, `open ? <b /> : <i />`, stops classes held for a later child being read in place",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Cond(a, b, c) => [a, b, c].iter().all(|e| inert_element(e, changing)),\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "loop-let-branch-assigned-not-sunk",
    "breaks": "a `let` a loop's branch makes from MIR stays one `let` before the loop, not a `const` in the branch",
    "file": "src/lower/mir/tidy.rs",
    "find": "            alone && !test.mentions_var(name) && assigned(then) && otherwise.as_deref().is_none_or(assigned)",
    "replace": "            false",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let a loop's branch makes is declared in the loop"
    ]
  },
  {
    "name": "spread-items-not-read-past-calls",
    "breaks": "`...first` of an array whose items nothing changes stops a later call's value being read in place",
    "file": "src/lower/mir/tidy.rs",
    "find": " || plain.contains(&format!(\"...{v}\"))",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arrays joined are one array of their items"
    ]
  },
  {
    "name": "array-spread-of-array-kept",
    "breaks": "`[...first, ...[a, b]]` keeps its spread of an array, not `[...first, a, b]`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    if let ExprKind::Array(items) = &mut e.kind {",
    "replace": "    if let ExprKind::Array(items) = &mut e.kind && false {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arrays joined are one array of their items"
    ]
  },
  {
    "name": "ending-continue-as-else",
    "breaks": "an `if` the program ended with `continue` stays MIR's `if (t) { a } else { .. }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    if written(guards, body.last().expect(\"matched\").span)\n",
    "replace": "                    if false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "continues a loop's ifs end with are kept"
    ]
  },
  {
    "name": "let-else-copy-kept",
    "breaks": "a `let`-`else`'s binding of a variable stays a copy, `const current$1 = current`",
    "file": "src/lower/mir/tidy.rs",
    "find": "            && then.last().is_some_and(ends)\n            && let StmtKind::Const(b, value) = &stmts[i + 1].kind",
    "replace": "            && then.last().is_some_and(ends)\n            && false\n            && let StmtKind::Const(b, value) = &stmts[i + 1].kind",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let-else's binding of a variable is the variable"
    ]
  },
  {
    "name": "nested-pattern-flat-only",
    "breaks": "a closure's pattern of a pattern stays `(_2) => { const name = _2[1].name; .. }`, not `([n, { name }]) => ..`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        _ if group.iter().all(|(rest, ..)| !rest.is_empty()) => Some((nested_pattern(group)?, None)),",
    "replace": "        _ if false => Some((nested_pattern(group)?, None)),",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a closure's nested patterns are nested JS patterns"
    ]
  },
  {
    "name": "logical-returns-kept",
    "breaks": "a value written with `&&`, `s != \"en\" && (if t {..} else {..})`, stays MIR's `return` of each branch",
    "file": "src/lower/mir/tidy.rs",
    "find": "        && within(last_if.span)\n",
    "replace": "        && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a value written with && is returned as one"
    ]
  },
  {
    "name": "alias-takes-prop-name",
    "breaks": "a named variable moved into a held prop takes the prop's name, `const code = ..`, and is read away",
    "file": "src/lower/mir/tidy.rs",
    "find": "                if !is_temporary(a) && (captured.contains(&b) || is_temporary(&b)) {",
    "replace": "                if false {",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "forwarded-value-kept",
    "breaks": "a value a `try` makes in each branch stays a temporary's, `children = tmp` after it",
    "file": "src/lower/mir/tidy.rs",
    "find": "            && last_assigns(&stmts[i + 1..=i + 1], t)\n",
    "replace": "            && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a value a try makes in each branch is the variable's there"
    ]
  },
  {
    "name": "array-values-kept-stepped",
    "breaks": "an array's `$iter` an adapter alone steps stays `$iter(xs).filter(..)`, not `xs.values().filter(..)`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        && crate::lower::iterators::is_stepped_items(it)\n        && let ExprKind::Call(_, args) = &mut it.kind\n",
    "replace": "        && false\n        && let ExprKind::Call(_, args) = &mut it.kind\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an array's iterator for an adapter alone is its values"
    ]
  },
  {
    "name": "jsx-child-truth-test-kept",
    "breaks": "a child shown where text is truthy stays `href ? <a /> : undefined`, not `!!href && <a />`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                        false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, (**test).clone())),",
    "replace": "                        false => continue,",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "inlined-pure-not-past-statements",
    "breaks": "what does nothing, of variables no statement between assigns, `version === \"Canary\"`, stays held before a loop",
    "file": "src/lower/mir/tidy.rs",
    "find": "                || (unchanged_by_calls && reads.iter().all(|v| !assigns(&rest[..at], v)))\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "held-prop-named-value",
    "breaks": "what MIR holds for a prop is named `value`, not after the prop, `const href = made(\"h\")`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                        (Some(prop), _) => prop.clone(),\n",
    "replace": "                        (Some(_), _) => \"value\".to_string(),\n",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "jsx-key-read-in-written-order",
    "breaks": "a `key` that does something is read in place, made after the children by JS's JSX runtime, `[1, 3, 2]`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                && let Some(found) = leads(key, name, guard, passes)\n",
    "replace": "                && false\n                && let Some(found) = leads(key, name, guard, passes)\n",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "held-arg-named-value",
    "breaks": "an argument MIR holds, `format!`'s, is named `value`, not `arg` as THIR names it",
    "file": "src/lower/mir/tidy.rs",
    "find": "(None, _) if is_held_arg(name) => \"arg\".to_string(),",
    "replace": "(None, _) if is_held_arg(name) => \"value\".to_string(),",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "format!"
    ]
  },
  {
    "name": "alias-takes-temporary-name",
    "breaks": "a variable the program names, `const props = {..}`, aliased by a temporary, takes the temporary's place and is read away",
    "file": "src/lower/mir/tidy.rs",
    "find": "if !is_temporary(a) && (captured.contains(&b) || is_temporary(&b)) {",
    "replace": "if !is_temporary(a) && captured.contains(&b) {",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "element-branch-not-small",
    "breaks": "a small element chosen, `busy ? <span>\u2026</span> : <b>Save</b>`, stays `let label; if (busy) ..`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Jsx(_) => levels(e) <= 2,\n",
    "replace": "        ExprKind::Jsx(_) => false,\n",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "table-read-after-inlining",
    "breaks": "a match's table read MIR has already read in place, a child's, stays its chain of tests, not `TITLES[kind]`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    // what it's given to reads it in place.\n    js::statement_lists(body, &mut |stmts| table_reads(stmts, tables));\n",
    "replace": "    // what it's given to reads it in place.\n",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "plain-reads-held",
    "breaks": "a field of plain data MIR holds, `const t = q.title`, stays held, destructured, not read in place",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| plain_reads(stmts, plain, captured));\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "rest-parts-apart",
    "breaks": "a `let`'s pattern's rest and what it binds stay two patterns, `const { href: _href, ...rest } = p; const { href } = p;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut rest_parts);\n",
    "replace": "",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "named-callback-reads-base",
    "breaks": "a closure reading what it would be named, `step` of `step$1`, is named it, reading itself",
    "file": "src/lower/mir/tidy.rs",
    "find": "            !reads\n",
    "replace": "            true\n",
    "tests": [
      "test/mir.test.ts",
      "-t",
      "JSX tests"
    ]
  },
  {
    "name": "mir-tidy-continue-written-negated-only",
    "breaks": "a `continue` the program wrote is an `if (i % 2 !== 0) { \u2026 }` where MIR's test isn't a `!`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    if written(guards, body.last().expect(\"matched\").span) || (negated(test) && then.len() > 1) =>\n",
    "replace": "                    if negated(test) && (then.len() > 1 || written(guards, body.last().expect(\"matched\").span)) =>\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "division by a constant from MIR"
    ]
  },
  {
    "name": "mir-enumerate-mapped",
    "breaks": "`iter().enumerate()` of a slice from MIR is `xs.values().map((x, i) => [i, x])`, not `xs.entries()`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        && pairs_index(pair)\n",
    "replace": "        && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "nested loops from MIR read as written"
    ]
  },
  {
    "name": "mir-labels-by-block",
    "breaks": "a loop the program labeled `'rows` from MIR is labeled by its block, `bb2`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| named_labels(stmts, written));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "nested loops from MIR read as written"
    ]
  },
  {
    "name": "mir-for-of-span-lost",
    "breaks": "a `for\u2026of` made of MIR's loop has no span, so its label isn't the program's",
    "file": "src/lower/mir/tidy.rs",
    "find": "    let mut out = vec![looped.at(span)];\n",
    "replace": "    let mut out = vec![looped.at(js::Span::NONE)];\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "nested loops from MIR read as written"
    ]
  },
  {
    "name": "mir-labeled-break-loop-kept",
    "breaks": "a `break 'outer` out of an inner loop from MIR leaves the outer a `while (true)` of `$next`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        StmtKind::Labeled(l, within) if let [only] = within.as_slice() => (only, Some(l.clone())),\n",
    "replace": "        StmtKind::Labeled(_, within) if let [_] = within.as_slice() => (second, None::<String>),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "nested loops from MIR read as written"
    ]
  },
  {
    "name": "mir-for-of-break-after-leaving",
    "breaks": "a `for\u2026of` body from MIR that ends leaving, `continue`, gets a `break` after it",
    "file": "src/lower/mir/tidy.rs",
    "find": "    if falls_out && !inner.last().is_some_and(ends) {\n",
    "replace": "    if falls_out {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "nested loops from MIR read as written"
    ]
  },
  {
    "name": "mir-map-count-handle",
    "breaks": "`*m.entry(k).or_insert(0) += 1` from MIR is a handle on the key's value, `$mutGet(m, k).value = \u2026`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut map_counts);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a count kept in a map from MIR"
    ]
  },
  {
    "name": "mir-chain-tail-folded",
    "breaks": "a variable's value of three arms from MIR has its last two folded, `else { cls = w ? \"warn\" : \"tip\"; }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    && let Some((y, folded)) = assigned_only(otherwise)\n",
    "replace": "                    && let Some((y, folded)) = assigned_only(otherwise).filter(|_| false)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a variable's value of three arms from MIR"
    ]
  },
  {
    "name": "mir-rest-str-of-fresh",
    "breaks": "`w.chars().as_str()` from MIR is `$restStr($iter(Array.from(w)))`, not `Array.from(w).join(\"\")`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        && matches!(&callee.kind, ExprKind::Var(f) if f == \"$restStr\")\n",
    "replace": "        && matches!(&callee.kind, ExprKind::Var(f) if f == \"$none\")\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "the rest of a string's chars from MIR"
    ]
  },
  {
    "name": "mir-closure-values-first",
    "breaks": "a closure's `$iter(g)` from MIR is `g.values()` before its function sees `flatMap` take it",
    "file": "src/lower/mir/tidy.rs",
    "find": "    if !closure {\n",
    "replace": "    if true {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "mir-capital-chars-joined",
    "breaks": "`c.to_uppercase().collect::<String>()` from MIR is `Array.from(c.toUpperCase()).join(\"\")`",
    "file": "src/lower/mir/tidy.rs",
    "find": "if m == \"toUpperCase\" || m == \"toLowerCase\"))",
    "replace": "if m == \"none\"))",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "mir-loop-exit-returned-inside",
    "breaks": "a loop left by `break` before the function's value from MIR returns the value inside the loop",
    "file": "src/lower/mir/tidy.rs",
    "find": "    loop_exits(body, returns, written);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a loop's break from MIR is a break"
    ]
  },
  {
    "name": "mir-loop-exit-written-return",
    "breaks": "a `return` the program wrote in a function's last loop becomes a `break`, the `return` after the loop",
    "file": "src/lower/mir/tidy.rs",
    "find": "        && returns.iter().any(|r| written.lo <= r.lo && r.hi <= written.hi)\n",
    "replace": "        && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a loop's break from MIR is a break"
    ]
  },
  {
    "name": "mir-pattern-tests-kept",
    "breaks": "a range arm from MIR is `if (true) { \u2026 }`, or two `if`s one inside the other",
    "file": "src/lower/mir/tidy.rs",
    "find": "    bottom_up(body, &mut pattern_tests);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range pattern's tests from MIR"
    ]
  },
  {
    "name": "mir-or-tests-nested",
    "breaks": "`\"a\" | \"e\" => \u2026` from MIR is a test failing for each, one inside another, the arm after them",
    "file": "src/lower/mir/tidy.rs",
    "find": "    bottom_up(body, &mut or_tests);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a string's or-pattern from MIR"
    ]
  },
  {
    "name": "mir-set-of-iter",
    "breaks": "`[1, 2].into_iter().collect::<HashSet<_>>()` from MIR is `new Set($iter([1, 2]))`",
    "file": "src/lower/mir/tidy.rs",
    "find": "if c == \"Set\" || c == \"Map\")",
    "replace": "if c == \"None\")",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "sets and maps from MIR"
    ]
  },
  {
    "name": "mir-map-count-literal-key",
    "breaks": "`*m.entry(\"a\").or_insert(0) += 5` from MIR stays a handle, `$mutGet(m, \"a\")`",
    "file": "src/lower/mir/tidy.rs",
    "find": "ExprKind::Str(a), ExprKind::Str(b)) if a == b))",
    "replace": "ExprKind::Str(a), ExprKind::Str(b)) if a == b && false))",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a count kept in a map from MIR"
    ]
  },
  {
    "name": "mir-temp-chain-half-folded",
    "breaks": "a value MIR holds of three branches, its first doing more, is `else { tmp = b ? q : r; }`",
    "file": "src/lower/mir/tidy.rs",
    "find": " || is_temporary(x) && then.len() > 1)",
    "replace": ")",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a variable's value of three arms from MIR"
    ]
  },
  {
    "name": "mir-loop-end-block-continues",
    "breaks": "an `else if` at a loop's end from MIR is a `continue` and an `if` after",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    ending(&mut inner, &block);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an else-if at a loop's end from MIR"
    ]
  },
  {
    "name": "mir-loop-tail-continue-kept",
    "breaks": "a branch ending a loop's body from MIR keeps a `continue` that does nothing",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ends_going_on(body, &looped);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an else-if at a loop's end from MIR"
    ]
  },
];
