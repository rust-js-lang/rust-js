// Mutations of src/lower/mir/tidy/loop_labels.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "framed-case-not-continued",
    breaks:
      "a framed `switch`'s case that goes on to the loop's next turn falls to the `break;` after the `switch`, leaving the loop (#389)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "    for case in cases.iter_mut().filter(|case| !case.falls && goes_on(&case.body)) {\n",
    replace: "    for case in cases.iter_mut().filter(|_| false) {\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "no label"],
  },
  {
    name: "framed-default-missing",
    breaks:
      "a framed `switch` of no `default`, a value of none of its cases falls to the `break;` after it, leaving the loop (#389)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "        None => *default = Some(vec![StmtKind::Continue(None).at(js::Span::NONE)]),\n",
    replace: "        None => {}\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "no label"],
  },
  {
    name: "guarded-loops-kept",
    breaks:
      "a loop a `switch` of guarded arms that fall through breaks keeps tidy's `loop:` label and `break loop;`, a label the program didn't write (#389b)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "            chained_in(body, &name, &locals);\n",
    replace: "",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "guarded arms"],
  },
  {
    name: "guarded-chain-forgets-fallen",
    breaks:
      "an arm's test leaves out the values of the guarded arms that fall to it, so `2 | 3 | 4 => ..` of a 2 whose guards failed runs none of it (#389b)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "            fallen = tested.clone();\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(switch_fallthrough|guarded_arms_leave_their_loop).rs$",
    ],
  },
  {
    name: "guarded-chain-guard-dropped",
    breaks:
      "a guarded arm's test leaves out its guard, so `2 | 3 if on(three) => break` breaks the loop whatever its guard says (#389b)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "                Some(guard.into_iter().fold(any, |any, guard| Expr::bin(Op::And, any, guard)))\n",
    replace: "                Some(any)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(switch_fallthrough|guarded_arms_leave_their_loop).rs$",
    ],
  },
  {
    name: "guarded-subject-not-held",
    breaks:
      "a `switch` of guarded arms that fall through, of a subject that does something, `c.nextCode()`, is read in place, so it keeps tidy's `loop:` label the program didn't write (#389c)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "    cases.iter().any(|case| case.falls) && labels.iter().any(|l| tidy_made(l))\n",
    replace: "    false\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "held once"],
  },
  {
    name: "guarded-chain-break-not-an-else",
    breaks:
      "a case whose `if` leaves the `switch` before what follows it, `if (c) { a; break; } b`, keeps tidy's `loop:` label, the `switch` no `if` chain (#389c)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "        elses_of_breaks(&mut body);\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(guarded_arms_nested_break|loop_exit_tuple).rs$",
    ],
    snapshots: true,
  },
  {
    name: "guarded-chain-else-of-a-going-branch",
    breaks:
      "an `if` whose branch goes on, not leaving the `switch`, is given what follows as its `else`, so that runs only where its test fails (#389c)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "    let leaves = |branch: &[Stmt]| matches!(branch.last().map(|s| &s.kind), Some(StmtKind::Break(None)));\n",
    replace: "    let leaves = |_: &[Stmt]| true;\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arms_nested_break.rs$"],
  },
  {
    name: "guarded-chain-ending-breaks-kept",
    breaks:
      "a `break;` that ends a branch of the `if` a case ends with stays, so the `switch` is no `if` chain and keeps tidy's `loop:` label (#389c)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "        super::switches::ending_breaks(&mut body);\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(guarded_arms_nested_break|loop_exit_tuple).rs$",
    ],
    snapshots: true,
  },
  {
    name: "guarded-chain-empty-link-kept",
    breaks:
      "a link of no statements, `b' ' => {}`'s, is `else if (c) {} else { .. }` (#389c)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "                vec![StmtKind::If(test.negated_test(), after, None).at(span)]\n",
    replace:
      "                vec![StmtKind::If(test, Vec::new(), Some(after)).at(span)]\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "empty arm"],
  },
  {
    name: "copy-read-in-place-kept",
    breaks:
      "an `if` chain of guarded arms reads its subject through MIR's copy of the index, `const value = i; if (codes[value] === 97 && ..)`, not `codes[i]` as written (#389c)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "    let Some(at) = k.checked_sub(1) else {\n",
    replace: "    let Some(at) = k.checked_sub(1).filter(|_| false) else {\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arms_index_copy.rs$"],
    snapshots: true,
  },
];
