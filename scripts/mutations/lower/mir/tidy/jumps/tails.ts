// Mutations of src/lower/mir/tidy/jumps/tails.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "switch-of-booleans-folded",
    breaks:
      "a `match` of strings the program wrote whose arms give `false` and `true` is one boolean, `return tagName !== \"annotation-xml\" && ..`, not its `switch` (rd15, isCustomElement)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        && !switched(first, last_if.span, &program.switches)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^match_of_strings_to_bools_is_a_switch.rs$"],
    snapshots: true,
  },
  {
    name: "alike-branches-of-a-call-returned",
    breaks:
      "a returned `if` of alike values whose test calls is `return mark(n) ? 5 : 5;`, not as written",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        && !(super::super::logicals::alike_branches(&yes, &no) && test.has_effects())\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^alike_branches.rs$"],
    snapshots: true,
  },
  {
    name: "if-jumps-chained-past-the-first",
    breaks:
      "of `if`s written without an `else` whose `break`s MIR joined, only the first ends with its own, the rest an `else if` chain (ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            [inner] => unchained(inner, jump, without_else)?,\n",
    replace: "            [inner] => return None,\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "ifs that each break a loop",
    ],
  },
  {
    name: "if-jumps-of-a-written-else",
    breaks:
      "an `else if` chain the program wrote is unchained, each branch its own `break` (ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        if !without_else(chain) {\n",
    replace: "        if false {\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "ifs that each break a loop",
    ],
  },
  {
    name: "mir-tidy-jump-ends-not-made",
    breaks: "MIR's block that ends in a loop it never leaves isn't seen to end",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            matches!(cond.kind, ExprKind::Bool(true)) && broken_made(body, true, label.as_ref(), result)\n        }\n        Some(StmtKind::If(",
    replace:
      "            matches!(cond.kind, ExprKind::Bool(true)) && broken_made(body, true, label.as_ref(), result) && false\n        }\n        Some(StmtKind::If(",
    tests: ["test/corpus.test.ts", "-t", "^returns_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "loop-breaks-not-returned",
    breaks:
      "a `while (true)` that makes the function's value before each `break` keeps the `break`, its `return` gone: returns_in_place's `?` in a loop of `,`s leaves the loop, not the function, a wrong answer",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        StmtKind::While { body, label, .. } => returned_breaks(body, true, label.as_ref()),\n",
    replace: "        StmtKind::While { .. } => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^returns_in_place.rs$"],
  },
  {
    name: "returned-in-place-end-kept",
    breaks:
      "a block's end, `result = b;` before the `return result;` after it, is left there, the block and its `return` kept",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "    if made_at_ends(body, result) {\n        returned_at_ends(body);\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^interpreter.rs$"],
    snapshots: true,
  },
  {
    name: "made-at-ends-jump-goes-on",
    breaks:
      "a branch ending in a jump past the block is taken to go on to its end, so the block's `return result;` stays",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        Some(StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Continue(_) | StmtKind::Break(_)) => true,\n",
    replace:
      "        Some(StmtKind::Return(_) | StmtKind::Throw(_)) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^interpreter.rs$"],
    snapshots: true,
  },
  {
    name: "broken-to-inner-loops",
    breaks:
      "an inner loop's own `break` is taken for the outer loop's, a `return` of the function's value there",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "                returned_breaks(body, false, looped)\n",
    replace: "                returned_breaks(body, own, looped)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(interpreter|calculator|returns_in_place).rs$",
    ],
    snapshots: true,
  },
  {
    name: "first-written-not-in-try",
    breaks:
      "where a statement in a `try` is written is skipped, so a drop's place says what's written first",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        | StmtKind::Try(body, _)\n        | StmtKind::TryCatch(body, ..) => first_written(body),\n",
    replace: "        => first_written(body),\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_enums.rs$"],
    snapshots: true,
  },
  {
    name: "first-written-if-by-test",
    breaks:
      "an `if`'s place is its test's, a `match`'s written before each arm, so a `Display` match's first arm comes last",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        StmtKind::If(_, then, _) => first_written(then).or((!stmt.span.is_none()).then_some(stmt.span.lo)),\n",
    replace: "        StmtKind::If(_, then, _) => (!stmt.span.is_none()).then_some(stmt.span.lo).or(first_written(then)),\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(app_errors|calculator|interpreter|stock_orders).rs$",
    ],
    snapshots: true,
  },
  {
    name: "tail-if-returns-each",
    breaks:
      "a function's tail `if` of two small values returns each branch, not `return t ? a : b`",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        && elses.iter().any(|(cond, ..)| within(cond, if_stmt.span))\n",
    replace: "        && false\n",
    tests: [
      "test/lowering-branches.test.ts",
      "-t",
      "an if written with an else keeps it",
    ],
  },
  {
    name: "match-made-conditional",
    breaks:
      "a two-arm `match`, no `if` the program wrote, is folded as one, `return t ? a : b`, where it returns each arm",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        && elses.iter().any(|(cond, ..)| within(cond, if_stmt.span))\n",
    replace:
      "        && (true || elses.iter().any(|(cond, ..)| within(cond, if_stmt.span)))\n",
    tests: [
      "test/lowering-branches.test.ts",
      "-t",
      "a function.s match of two arms and its if of two values",
    ],
  },
  {
    name: "written-returns-folded",
    breaks:
      "a `return` the program wrote in each branch of an `if`/`else` is folded into one conditional",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "if small(value) && (!written(returns, *span) || of_if(*span)) =>",
    replace: "if small(value) =>",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "a string's parts from MIR are an array",
    ],
  },
  {
    name: "return-into-branches-never",
    breaks:
      'a `match`\'s `return` some arms go on to and others leave past stays a block MIR made, `bb12: { .. break bb12; .. return "Complex"; }`',
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            at_ends(body, &label);\n",
    replace: "            let _ = (body, label);\n",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a match's return some arms go on to",
    ],
  },
  {
    name: "return-into-branches-break-kept",
    breaks:
      "a path that leaves a block MIR made for a `return` some arms go on to keeps its `break`, so the block stays",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            StmtKind::Break(Some(l)) if l == label => {\n                stmts.pop();\n",
    replace:
      "            StmtKind::Break(Some(l)) if l == label && false => {\n                stmts.pop();\n",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a match's return some arms go on to",
    ],
  },
  {
    name: "return-into-branches-test-unturned",
    breaks:
      "`if (!obj) { break l; }` given the `return` it skips is `if (!obj) { return v; }`, of the paths that left",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "                        *test = std::mem::replace(test, Expr::taken()).negated_test();\n",
    replace:
      "                        *test = std::mem::replace(test, Expr::taken());\n",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a match's return some arms go on to",
    ],
  },
  {
    name: "return-into-branches-of-any-value",
    breaks:
      "a `return` of a call some arms go on to is written in each path, `return complex(array.length);` twice",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "StmtKind::Return(value) if value.as_ref().is_none_or(|e| one_value(e) || call_of_values(e)))\n",
    replace:
      "StmtKind::Return(value) if value.as_ref().is_none_or(|_| true))\n",
    tests: ["test/lowering-labels.test.ts", "-t", "a match's return of a call"],
  },
  {
    name: "returns-into-branches-at-ends-not",
    breaks:
      'a `return` some paths of an `if` go on to, the two ending a branch of the `if` a block ends with, stays after them, `bb272: { if (typeof s === "string") { .. break bb272; .. return; } }`',
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        if let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|s| &mut s.kind) {\n            at_ends(then, label);\n",
    replace:
      "        if false\n            && let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|s| &mut s.kind)\n        {\n            at_ends(then, label);\n",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "guarded arms that return calls fall through in one labeled switch",
    ],
  },
  {
    name: "written-break-dropped-at-end",
    breaks:
      "a `break` the program wrote that ends its block is dropped, so tidy makes the `if` before it an `else` (ADR 0790)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            if !crate::lower::mir::is_block_label(a) && !stmt.span.is_none() =>",
    replace: "            if false =>",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled",
    ],
  },
  {
    name: "written-turned-value-returned-as-mir-has-it",
    breaks:
      "`if a == 0 || a > b { a } else { b }` returns MIR's order, `a !== 0 && !(a > b) ? b : a` (ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            Some(true) => conditional(test.clone().negated_test(), no, yes, booleans, true),\n",
    replace:
      "            Some(true) => conditional(test.clone(), yes, no, booleans, true),\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "a returned value of an if of an",
    ],
  },
  {
    name: "mir-tidy-returned-chain-kept",
    breaks:
      "`if (a) { if (b) { return t; } } return f;` stays, not `return a && b ? t : f`",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        stmts.push(StmtKind::Return(Some(value)).at(span));\n",
    replace: "        let _ = value;\n        return;\n",
    tests: ["test/lowering-branches.test.ts", "-t", "branches from MIR read"],
  },
  {
    name: "mir-tidy-written-return-ternary",
    breaks:
      "`if (!g) { return; } return big;`, as the program wrote it, is `return g ? big : undefined`",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        && !written(returns, last.span)\n        && !writes_return(std::slice::from_ref(last_if), returns)\n",
    replace: "",
    tests: [
      "test/lowering-branches.test.ts",
      "-t",
      "a function.s match of two arms and its if of two values",
    ],
  },
  {
    name: "mir-tidy-let-else-return-nested",
    breaks:
      "a `let … else { return f }` stays `if (a) { if (b) { return t; } } return f;`, not a guard",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if))\n",
    replace:
      "        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if)).filter(|_| false)\n",
    // lowering-matches' let-else test prints the same JS without it; this
    // case's `let .. else { return None; }` doesn't.
    tests: ["test/corpus.test.ts", "-t", "^local_borrow_nested_option.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-let-else-of-written-returns",
    breaks:
      "`if c { return a; } else { return b; }`, both written, is `if (!c) { return b; } return a;`",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        && !body.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(_)) && written(returns, s.span))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^local_consts.rs$"],
    snapshots: true,
  },
  {
    name: "written-continue-guarded",
    breaks:
      "`if a && b { continue; } return None;` before more of the loop's turn is a `let … else`'s guard, `if (!a || !b) { return; } continue;`, its comment gone (at the turn's end, `kept_jumps` turns it back)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        && !tests_continue(continues, last_if.span)\n",
    replace: "",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "a continue the program wrote",
    ],
  },
  {
    name: "returning-chain-keeps-unread-bound",
    breaks:
      "a read a chain's test takes in place stays after it where nothing reads it, `const value = v[i];`",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "    unread_bound(&mut all, bound.len());\n",
    replace: "",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "a read a test takes in place",
    ],
  },
  {
    name: "written-chain-value-folded",
    breaks:
      "`if !js::truthy(v) && n != 0 { f() } else { true }` is `!!v || n === 0 || f()`, not the conditional written (ADR 0879)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            || written_values(last_if.span, (&yes, no), elses);\n",
    replace: "            || false;\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "a value's truth or else a bool is a bool part by part",
    ],
  },
  {
    name: "written-thens-as-mir-has-them",
    breaks:
      "`if DEV { .. } None` is `if (!DEV) { return; } ..`, MIR's guard, not the `if` written (ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        stmts[at].kind = StmtKind::If(test.negated_test(), rest, None);\n        stmts.extend(exit);\n",
    replace:
      "        stmts[at].kind = StmtKind::If(test, exit, None);\n        stmts.extend(rest);\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "keeps its body inside",
    ],
  },
  {
    name: "returned-if-a-statement",
    breaks:
      "`return if c { a } else { b };` is `if (c) { return a; }` and a shared `return b`, not the conditional returned the program wrote (ADR 0657, amended)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        ] if small(value) && (!written(returns, *span) || of_if(*span)) => Some(spanned(value, *span)),\n",
    replace:
      "        ] if small(value) && !written(returns, *span) => Some(spanned(value, *span)),\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "an if of returns the program wrote stays an if",
    ],
  },
  {
    name: "returned-call-not-one-value",
    breaks:
      "React's getTag's arms that return a call keep the blocks MIR made, `break bb127`, no `switch` (ADR 0955)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "    matches!(&stmt.kind, StmtKind::Return(value) if value.as_ref().is_none_or(|e| one_value(e) || call_of_values(e)))\n",
    replace:
      "    matches!(&stmt.kind, StmtKind::Return(value) if value.as_ref().is_none_or(|e| one_value(e) || (call_of_values(e) && false)))\n",
    tests: ["test/lowering-labels.test.ts", "-t", "return calls"],
  },
  {
    name: "held-block-not-made",
    breaks:
      "a `return` from a `match` inside an `if let` holding an `Rc` is `let result;` and a labeled block, `bb24: { .. result = false; break bb24; }`, not the `return false;` React's throwException writes (ADR 0432, amended)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        Some(StmtKind::Labeled(label, body)) => {\n            made_at_ends(body, result)\n",
    replace:
      "        Some(StmtKind::Labeled(label, body)) if false => {\n            made_at_ends(body, result)\n",
    tests: ["test/corpus.test.ts", "-t", "^returns_in_held_branches.rs$"],
    snapshots: true,
  },
  {
    name: "held-block-breaks-unreturned",
    breaks:
      "a block's `break`s that follow the result made aren't returned, so the block stays, `bb24: { .. result = false; break bb24; }` (ADR 0432, amended)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            returned_breaks(body, false, Some(label));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^returns_in_held_branches.rs$"],
    snapshots: true,
  },
  {
    name: "joined-ands-off",
    breaks:
      "a guard of comparisons MIR tests in parts, each failing the same way, `4 if lanes != 1 && lanes != 2 => { // .. }`, is nested ifs and a label, `bb4: { switch .. if (lanes !== 1) { if (lanes !== 2) { } else { break bb4; } ..`, not `if (lanes !== 1 && lanes !== 2) { // ..\n break; }` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            && crate::lower::mir::tidy::decisions::same_stmt(&inner[0], &otherwise[0])\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arm_chains.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-joined-unseen",
    breaks:
      "returns the program wrote in each branch, which MIR joins into one `return;` after a block, keep that block and its `break`s, `bb33: { if (tag === 1) { .. } else { break bb33; } .. } return; }` (rd100, rd98)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        let joins = if joined.is_none() {\n            joined_returns(body, label, returns, starts)\n",
    replace: "        let joins = if joined.is_none() {\n            None\n",
    tests: ["test/corpus.test.ts", "-t", "^returns_joined_past_blocks.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-written-return-copied",
    breaks:
      "a `return` the program wrote once, the last arm's `_ => { // We've found all. return; }`, which a guard's failing goes on to too, is copied to each way, its comment lost (rd100)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        let joins = if joined.is_none() {\n",
    replace: "        let joins = if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^match_arm_guard_first.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-inner-joined-unseen",
    breaks:
      "a block MIR made inside one whose `break`s are the program's `return`s, the `return;` after it given the place of one written in it, keeps its label: React's Fizz renderNode's `bb51: { .. if (out.length <= 1) { break bb51; } .. }` (rd499)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        } else if joined_past.contains(&*label) {\n",
    replace: "        } else if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^let_else_returns_in_arms\\.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-inner-blocks-unsought",
    breaks:
      "the blocks a `break` of a block of joined returns follows aren't sought, so the `return;` that `break` becomes is taken as written after them: React's Fizz renderNode's `bb51: { .. break bb51; .. }` stays (rd499)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "                    && *l == label\n",
    replace: "                    && *l == label && false\n",
    tests: ["test/corpus.test.ts", "-t", "^let_else_returns_in_arms\\.rs$"],
    snapshots: true,
  },
  {
    name: "returned-as-written-let-else-threaded",
    breaks:
      "a `let … else`'s `else`, which MIR puts after the rest, is gone on to from the rest's end, past every `return` written, so its `return;` says no place and stays `if (preamble != null) { .. } else { return; }`, or joins the test before it, `if (root.value === 7 || request.preamble == null)`, its comment outside the closure: React's Fizz flushCompletedQueues (rd499)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "    fits.then_some(e.lo)\n",
    replace: "    fits.then_some(e.lo).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^let_else_returns_in_arms\\.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-inner-taken-counted",
    breaks:
      "a block inside one of joined returns counts the `return`s already a way out of the outer one as its own, so a `return` written once after `if`s whose failing tests go on to it is copied to each: `if (a != null) { if (a > 2) { if (..) { ..; return; } // ..\n return; } return; } return;` (rd499)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: ".filter(|r| !taken.contains(r))",
    replace: ".filter(|r| taken.is_empty() || r.lo > 0)",
    tests: ["test/corpus.test.ts", "-t", "^let_else_returns_in_arms\\.rs$"],
    snapshots: true,
  },
  {
    name: "returned-as-written-left-block-furthest",
    breaks:
      "a block its own `break`s leave, before a `break` of joined returns, is gone on from by the furthest written in it, so the `_` arm's `return` of a `match` guard is lost to the rest nested in `if (target != null && ..) { .. }`, its comment above: React DOM's FormActionEventPlugin (rd499)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            _ if !stmt.span.is_none() => at = at.max(stmt.span.hi),\n",
    replace:
      "            StmtKind::Labeled(own, body) if js::names_jump(body, own) => {\n                js::each_list(body, &mut |list| {\n                    for s in list.iter().filter(|s| !s.span.is_none()) {\n                        at = at.max(s.span.hi);\n                    }\n                });\n            }\n            _ if !stmt.span.is_none() => at = at.max(stmt.span.hi),\n",
    tests: ["test/corpus.test.ts", "-t", "^let_else_returns_in_arms\\.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-joined-anywhere",
    breaks:
      "a block MIR made that a `return;` follows, more of whose ways go on to the function's end than the program wrote `return`s in it, has a `return;` for each, of what the program let go on: `while_let_at_end`'s loop (rd100)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "    (breaks > 0 && written.len() >= breaks).then_some(written)\n",
    replace: "    (breaks > 0).then_some(written)\n",
    tests: ["test/corpus.test.ts", "-t", "^while_let_at_end.rs$"],
    snapshots: true,
  },
  {
    name: "joined-returns-inner-taken",
    breaks: "a block MIR made around a let chain's `if` takes the `return`s written in a block inside it, which a `return;` MIR joined follows too, as its own: React DOM's preload is `if (!href) { // We already have a stylesheet ..\n return; } if (!as) { return; } ..`, not `if (owner != null && href && as) { .. }` with each comment on its own `return`",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        .filter(|r| !inner.iter().any(|&(lo, hi)| lo <= r.lo && r.hi <= hi))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^let_chain_around_early_returns.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-unplaced",
    breaks:
      "a `return` MIR joined, given back to each branch, has no place, so it's taken for one MIR made: the playground's guards one after another, `if (!test) { ..; return; } if (has) { ..; return; }`, become `} else if (!has) { .. } else { .. }` (rd98)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "                (stmt.span, at) = (*place, place.hi);\n",
    replace: "                at = place.hi;\n",
    tests: ["test/corpus.test.ts", "-t", "^returns_joined_past_blocks.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-placed-past-earlier",
    breaks:
      "a `break` MIR joined is placed at the first written `return` after the furthest written statement before it, not after the one just before it: recordLegacyContextWarning's `let … else { log.push(0); return; }`, whose `else` MIR puts after the rest, has no place, and its guard is lost to `if (root != null) { .. } else { log.push(0); }` (#435)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "            at = stmt.span.lo;\n",
    replace: "            at = at.max(stmt.span.lo);\n",
    tests: ["test/corpus.test.ts", "-t", "^let_else_guard_released\\.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-placed-past-branch",
    breaks:
      "a `break` MIR joined after an `if` whose branch leaves is placed at the first written `return` after the branch, not after the test: recordLegacyContextWarning's `if fiber.tag == 3 { return; }`, MIR's `if (fiber.tag !== 3) { ..; return; } break bb;`, has no place, and its guard is lost to `if (fiber.tag !== 3) { .. }` (#435)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "                } else {\n                    test.span.hi\n                };\n",
    replace:
      "                } else {\n                    at.max(stmt.span.hi)\n                };\n",
    tests: [
      "test/lowering-switches.test.ts",
      "-t",
      "written return at a function's end is its case's return",
    ],
  },
  {
    name: "returns-past-blocks-unspanned-test-past-branch",
    breaks:
      "a `break` MIR joined after an `if` whose branch leaves, of a test that says no place, a call's value, is placed after the branch, not where the `if` is written: React's recordLegacyContextWarning's `if (didWarn.has(..)) { return; }` is lost to `if (!didWarn.has(..)) { rest }` (#438)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "                if then.last().is_some_and(ends) && !(test.span.is_none() && stmt.span.is_none()) =>\n",
    replace:
      "                if then.last().is_some_and(ends) && !test.span.is_none() =>\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "written guards joined to its end are each where written",
    ],
  },
  {
    name: "joined-returns-end-at-last-statement",
    breaks:
      "a written `return` after the last of a block's statements isn't the block's, so its `break` becomes a `return;` of no place and the comment or blank line before it is lost (rd108)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        .filter(|&at| at >= hi)\n        .min()\n        .unwrap_or(u32::MAX);\n",
    replace:
      "        .filter(|&at| at >= hi)\n        .min()\n        .map_or(hi, |_| hi);\n",
    tests: ["test/corpus.test.ts", "-t", "^returns_joined_keep_comments.rs$"],
    snapshots: true,
  },
  {
    name: "try-tail-break-kept",
    breaks:
      "a `break` of a block MIR made that ends a `try` at the block's end stays, the block with it, `bb26: { if (..) { try { for (..) { .. } break bb26; } finally { .. } } }` (rd200)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        // finally { $unborrow(nodes); }` (rd200).\n        drop_tail(body, jump);\n",
    replace: "        // finally { $unborrow(nodes); }` (rd200).\n        let _ = body;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_loops.rs$"],
    snapshots: true,
  },
  {
    name: "tail-jump-in-ending-block-kept",
    breaks:
      "a labeled block whose breaks go through drops prints MIR's block around the program's, `bb18: { found: { ..; v = 2; break bb18; } }` (#447)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        // the program wrote there, `continue;` in a `switch`'s case.\n        drop_tail(body, jump);\n",
    replace: "        // the program wrote there, `continue;` in a `switch`'s case.\n        _ = body;\n",
    tests: ["test/lowering-labels.test.ts", "-t", "a break the program wrote through drops is its label's"],
  },
  {
    name: "tail-jump-branch-left-empty",
    breaks:
      "a branch a tail jump was all of stays, empty: React's Fizz `use(usable)` prints `if (usable == null) {} else if (typeof usable === \"object\") { .. }`",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        let last = stmts.pop().expect(\"matched\");\n        if let Some(kind) = emptied_branch(last.kind) {\n            stmts.push(Stmt { kind, ..last });\n        }\n    } else if let Some(StmtKind::Try(body, _))",
    replace: "    } else if let Some(StmtKind::Try(body, _))",
    tests: ["test/corpus.test.ts", "-t", "^emptied_branch_nested\\.rs$"],
    snapshots: true,
  },
  {
    name: "returns-past-blocks-in-try-skipped",
    breaks:
      "a block MIR made of the early returns that ends a drop guard's `try` keeps its label, `try { bb18: { if (..) { run(task); break bb18; } } } finally { .. }` (rd502)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "    tried.push(StmtKind::Return(None).at(js::Span::NONE));\n    returns_past_blocks(tried, written, returns, starts);\n",
    replace: "    let _ = (written, returns, starts);\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_guard_early_returns\\.rs$"],
    snapshots: true,
  },
  {
    name: "written-arm-continue-tail-dropped",
    breaks:
      "an arm's written `continue` that ends a branch of the `if` chain ending a loop's turn is dropped, so its `match` is an `if` chain, not a `switch` (#518)",
    file: "src/lower/mir/tidy/jumps/tails.rs",
    find: "        (StmtKind::Continue(Some(_)), StmtKind::Continue(Some(_))) if !stmt.span.is_none() => false,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_arms_continue.rs$"],
    snapshots: true,
  },
];
