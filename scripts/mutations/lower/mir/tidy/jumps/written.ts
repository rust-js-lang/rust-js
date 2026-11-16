// Mutations of src/lower/mir/tidy/jumps/written.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "written-else-of-a-made-return-unturned",
    breaks:
      "an `if … else { break; }` of the loop a function ends with, MIR's `if (!t) { return; } rest`, isn't turned back, `if (x <= 3) { break; } out.push(x);` (ADR 0684, amended)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                            || matches!(e.kind, StmtKind::Return(None)) && made(e)) =>\n",
    replace: ") =>\n",
    tests: ["test/corpus.test.ts", "-t", "^written_jumps_kept.rs$"],
    snapshots: true,
  },
  {
    name: "written-else-of-a-lone-jump-guarded",
    breaks:
      "a chain's last arm of the `break` the program wrote, which `early breaks` made `if (!b) { break; }`, stays a guard in the `else`, `} else { if (!(..)) { break; } .. }`, not `} else if (..) { .. } else { break; }` (ADR 0237, amended)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                        && matches!(&exit[..], [e] if jumps(e) && (e.span.is_none() || within(cond, e.span))\n                            || matches!(e.kind, StmtKind::Return(None)) && made(e)) =>\n",
    replace: "                        && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^else_break_ends_loop.rs$"],
    snapshots: true,
  },
  {
    name: "written-else-of-a-while-test",
    breaks:
      "a `while let`'s own test, `if (next == null) { break; } rest`, is taken for a written `else`, its rest the branch, and `while tests` leaves `break; break;` (ADR 0237, amended)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                    if otherwise.lo >= then.hi\n",
    replace: "                    if true\n",
    tests: ["test/corpus.test.ts", "-t", "^(else_break_ends_loop|loop_over_a_thread_local).rs$"],
    snapshots: true,
  },
  {
    name: "written-else-jump-span-kept",
    breaks:
      "the `break` `early breaks` put where the test is keeps the test's span, so `let else first` takes the written `else { break; }` for a `let … else`'s and makes it a guard again (ADR 0237, amended)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                    e.span = js::Span::NONE;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^else_break_ends_loop.rs$"],
    snapshots: true,
  },
  {
    name: "written-else-left-a-guard",
    breaks:
      "an `if` the program wrote with an `else` that returns is a guard, `if (!c) { return; } …`, and its `else if` an `else { if }` with a `continue`",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                (false, true) => branch(then, Some(cond)).map(|k| (true, k)),\n",
    replace: "",
    tests: [
      "test/lowering-branches.test.ts",
      "-t",
      "an if written with an else keeps it",
    ],
  },
  {
    name: "written-else-binding-not-then",
    breaks:
      "a let chain's binding, written in its condition, isn't its `then`'s, so its `else if` is an `else { if (c) { …; break bb27; } throw … }` in a labeled block",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: " || made.is_some_and(|c| within(c, at(s))))\n",
    replace: " || made.is_some_and(|_| false))\n",
    tests: ["test/lowering-branches.test.ts", "-t", "binding kept in its arm"],
  },
  {
    name: "lone-jumps-given-an-else",
    breaks:
      "`if (t) { return a; } return b;`, a lone jump each way, is given an `else`, `if (a === 0 || a > b) { return a; } else { return b; }`, not the conditional the program's `if .. else` is (lane_bits' lowerEventPriority, tested_options' named)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                _ if plain(exit, rest, linked) => None,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(lane_bits|tested_options).rs$"],
    snapshots: true,
  },
  {
    name: "if-let-link-kept-an-else",
    breaks:
      "an `else if let` chain is a staircase, `} else { const y = ..; if (y != null) { .. } else { .. } }`, where its binding keeps it from being a JS `else if`",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "        ) && else_ifs.contains(otherwise)\n",
    replace: "        ) && false\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "an else after a branch that returns stays",
    ],
  },
  {
    name: "guarded-else-unmarked",
    breaks:
      "an `else` made of what the program wrote in it, `if (a) { return n; } else { const n = 2; .. }`, isn't marked its, so preparation takes it away again",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                if !linking {\n                    stmts[at].facts = stmts[at].facts.with(js::Known::Else);\n                }\n",
    replace: "                let _ = linking;\n",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "a block's const reuses a name",
    ],
  },
  {
    name: "both-branches-else-unmarked",
    breaks:
      "an `if` MIR left with both branches the program wrote, `if (key === 2) { return 0; } else if (key === 3) { .. }`, loses its `else` to ADR 0237's (the user's decision, 2026-10-28)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "            stmts[at].facts = stmts[at].facts.with(js::Known::Else);\n        }\n        at += 1;\n",
    replace: "        }\n        at += 1;\n",
    tests: ["test/corpus.test.ts", "-t", "^returns_in_held_branches.rs$"],
    snapshots: true,
  },
  {
    name: "written-else-keeps-what-follows",
    breaks:
      "`} else if (tags.length === 0) { return 7; } return 9;` is `} else { if (..) { return 7; } return 9; }`, what the program wrote after the `if` in its `else`",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                stmts.splice(at + 1..at + 1, after);\n                let declared = bindings::declared_before(stmts, at, &mut kept_in, &kept);\n",
    replace: "                kept_in.extend(after);\n                let declared = bindings::declared_before(stmts, at, &mut kept_in, &kept);\n",
    tests: ["test/corpus.test.ts", "-t", "^returns_in_held_branches.rs$"],
    snapshots: true,
  },
  {
    name: "else-if-link-not-a-link",
    breaks:
      '`} else if (key === 2) { return 20; } else { return key + 1; }`, a chain\'s links of values, `} else if (flag) { return "never"; } else { .. }`, are `} else { if (flag) { .. } return ..; }`',
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                linked_elses(&mut rest, (elses, else_ifs), returns, !turned && !linking);\n",
    replace:
      "                linked_elses(&mut rest, (elses, else_ifs), returns, false);\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(option_bool_tested|unit_given_to_js).rs$",
    ],
    snapshots: true,
  },
  {
    name: "plain-jumps-marked-written",
    breaks:
      "`if (flaky.n > 10) { return; } return flaky.n;`, of a value, keeps an `else` of MIR's lone `return`s",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "            && !plain(then, otherwise, linked)\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(asked_length_overflow|refcell_vec_bounds).rs$",
    ],
    snapshots: true,
  },
  {
    name: "written-else-branches-as-mir-turned",
    breaks:
      "`if !c { a } else { b }` stays `if (c) { b } else { a }`, MIR's order, not the program's (ADR 0879)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "            stmts[at].kind = StmtKind::If(test.negated_test(), second, Some(first));\n",
    replace:
      "            stmts[at].kind = StmtKind::If(test, first, Some(second));\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(branches_as_written|interpreter|generic_mut_ref_kept).rs$",
    ],
    snapshots: true,
  },
  {
    name: "written-else-branches-turned-untested",
    breaks:
      "`if (c) { b } else { a }` turned as written keeps its test, `if (c) { a } else { b }`, each branch taken where the other was",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "            stmts[at].kind = StmtKind::If(test.negated_test(), second, Some(first));\n",
    replace:
      "            stmts[at].kind = StmtKind::If(test, second, Some(first));\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(branches_as_written|interpreter|generic_mut_ref_kept).rs$",
    ],
  },
  {
    name: "written-if-value-of-mir-branches",
    breaks:
      "a value written with an `if` isn't seen as written, so one of what isn't a `bool`, `c ? o : false`, is the `&&` MIR makes of the same branches, not the conditional written (ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "        match (of(then, a) && of(otherwise, b), of(then, b) && of(otherwise, a)) {\n",
    replace: "        match (false, false) {\n",
    tests: [
      "test/lowering-bool-ternaries.test.ts",
      "-t",
      "a written if or match of a bool literal branch is && or \\|\\|",
    ],
  },
  {
    name: "written-else-of-a-while-too",
    breaks:
      "a `while let`'s desugared `if let .. { body } else { break }` is an `else` the program wrote, so the loop stays `while (true) { const x = v.pop(); if (x != null) { .. } else { break; } }` (ADR 0879)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "        // A `while`'s desugaring's, its `else` the whole loop, isn't one.\n        otherwise.lo >= then.hi\n            && within(cond, stmt.span)\n",
    replace: "        within(cond, stmt.span)\n",
    tests: ["test/corpus.test.ts", "-t", "^(loop_exits|iter_mut_kept).rs$"],
    snapshots: true,
  },
  {
    name: "written-else-only-of-the-whole-rest",
    breaks:
      "an `else` the program wrote is made only of every statement after the guard, so `if (a) { return; } else if (b) { return; } more` stays two `if`s (ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                let more = rest[k..].iter().any(|s| !place(s).is_none());\n",
    replace: "                let more = false;\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "an else after a branch that returns stays",
    ],
  },
  {
    name: "written-else-return-at-its-span",
    breaks:
      "a `return` MIR made of the function's value is placed where the function ends, not where its value is, so the chain before it takes it in",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "        StmtKind::Return(Some(v)) if s.span.is_none() => v.span,\n",
    replace:
      "        StmtKind::Return(Some(v)) if s.span.is_none() && false => v.span,\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "an else-if chain keeps its else",
    ],
  },
  {
    name: "late-let-made-the-branchs",
    breaks: "a late `let x;` before a written `if c { return 0; } else if let .. { x = .. } else { x = .. }` is made the `else`'s, out of the scope of `x * 2` after it, which throws `x is not defined` (rd578)",
    file: "src/lower/mir/tidy/jumps/written.rs",
    find: "                bindings::kept_declarations(moved, after).map(|kept| (turned, k, kept))\n",
    replace: "                Some((turned, k, Vec::<String>::new()))\n",
    tests: ["test/corpus.test.ts", "-t", "^late_let_before_written_else.rs$"],
    snapshots: true,
  },
];
