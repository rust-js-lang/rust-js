// Mutations of src/lower/mir/tidy/switches/case_ends.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

const someSwitch = [
  "test/lowering-switch-symbols.test.ts",
  "-t",
  "a match of symbols give is given",
];

export const mutations: Mutation[] = [
  {
    name: "some-switch-default-off",
    breaks:
      "a `match` of an `Option`'s constants whose `_` does something keeps `bb22: { if (x != null) { switch (x) { .. break bb22; } } hit = false; }`, not `default: hit = false;` (#221)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "            some_switch_defaults(body, switches);\n",
    replace: "",
    tests: someSwitch,
    snapshots: true,
  },
  {
    name: "copied-returns-unbroken",
    breaks:
      "MIR's copies, in each case, of the `return` after a `match` that ended a block stay, React's attemptEarlyBailoutIfNoScheduledUpdate's `return bailoutOnAlreadyFinishedWork(..)` in every case (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "        crate::lower::mir::tidy::case_rests::copied_returns_broken(stmts, i, switches);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_ends_function.rs$"],
    snapshots: true,
  },
  {
    name: "switch-default-doing-nothing-kept",
    breaks: "a `_ => {}` is `default: break;`, which does nothing (#268)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "            if default.as_deref().is_some_and(does_nothing) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_does_nothing.rs$"],
    snapshots: true,
  },
  {
    name: "break-past-switch-kept",
    breaks:
      "a block's `break` in a case of a `switch` it ends with stays, `break bb374;`, its label around the `if`",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "            StmtKind::Break(Some(l)) if l.is(label) => stmt.kind = StmtKind::Break(None),\n",
    replace: "",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "past the switch"],
  },
  {
    name: "break-past-switch-in-a-loop",
    breaks:
      "a block's `break` in a loop in a case is the loop's bare `break`, leaving the loop, not the block",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "            StmtKind::Labeled(_, body) => case_breaks(body, label),\n            _ => {}\n",
    replace:
      "            StmtKind::Labeled(_, body) => case_breaks(body, label),\n            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => case_breaks(body, label),\n            _ => {}\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "past the switch"],
  },
  {
    name: "block-ending-case-labeled",
    breaks:
      "a block a case ends with keeps its label, `desc: { .. break desc; }`, not the case's `break;` (ADR 0790)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "                let label = label.clone();\n                case_breaks(body, label.word());\n                ending(body, elses);",
    replace: "                ending(body, elses);",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case",
    ],
  },
  {
    name: "case-if-keeps-unwritten-else",
    breaks:
      "a case's `if` written with no `else`, its `then` leaving, keeps MIR's, `} else if (jsx) { ..; return; } else { .. }` (ADR 0805)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "        unlabeled(stmts);\n        unwritten_else(stmts, elses);\n",
    replace: "        unlabeled(stmts);\n",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case|an \\|\\| before a loop",
    ],
  },
  {
    name: "case-if-drops-written-else",
    breaks:
      "an `if` the program wrote with an `else` loses it in a case, an `else if` chain it wrote broken (ADR 0805)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "    if written(last.span) || ",
    replace: "    if ",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "an else the program wrote in a case",
    ],
  },
  {
    name: "case-if-else-dropped-going-on",
    breaks:
      "an `else` taken out after a `then` that goes on, so both run (ADR 0805)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "    if written(last.span) || !a.last().is_some_and(js::ends) {\n",
    replace: "    if written(last.span) {\n",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "an else the program wrote in a case",
    ],
  },
  {
    name: "none-guard-kept-before-switch",
    breaks:
      "`if (status == null) { break bb4; } switch (status) { .. }` keeps its test of `None`, which no case takes",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "                body.remove(body.len() - 2);\n",
    replace: "",
    tests: ["test/lowering-switches.test.ts", "-t", "resolveThenable"],
  },
  {
    name: "none-guard-around-switch-kept",
    breaks:
      "`if (status != null) { switch (status) { .. } }` keeps its test of `None`, which no case takes",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "            stmts[i] = switch;\n",
    replace: "            let _ = switch;\n",
    tests: ["test/lowering-switches.test.ts", "-t", "resolveThenable"],
  },
  {
    name: "default-break-kept",
    breaks: "a `default` of a `break` alone stays, `default: break;`",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "                *default = None;\n",
    replace: "",
    tests: ["test/lowering-switches.test.ts", "-t", "resolveThenable"],
  },
  {
    name: "guarded-switch-temporary-kept",
    breaks:
      "the temporary a `switch` its guard went from reads stays, `const status = thenable.status; switch (status)`",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "            if discriminant_in_place(stmts, i) {\n",
    replace: "            if false && discriminant_in_place(stmts, i) {\n",
    tests: ["test/lowering-switches.test.ts", "-t", "resolveThenable"],
  },
  {
    name: "guard-break-kept",
    breaks:
      "a case's `if (!onC) { break; } ..` stays MIR's, not the arm's `if (onC) { .. }` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "                guard_held(body, switches);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_shares_next.rs$"],
    snapshots: true,
  },
  {
    name: "case-ending-breaks-kept",
    breaks:
      "a case that goes on past the `switch` keeps a `break;` MIR made ending its `if`'s branch, `case 1: if (n > 3) { break; } break;` of a guarded arm of `continue` (#488)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "                ending_breaks(body);\n            }\n",
    replace: "            }\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_continue_ending_turn.rs$"],
    snapshots: true,
  },
  {
    name: "case-ending-written-breaks-dropped",
    breaks:
      "a `break 'get_tag;` the program wrote, ending an `if` with no `else` in a case, goes, so what follows the `if` is its `else` (#488, ADR 0944)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "            Some(end) if matches!(end.kind, StmtKind::Break(None)) && end.span.is_none() => {\n",
    replace: "            Some(end) if matches!(end.kind, StmtKind::Break(None)) => {\n",
    tests: ["test/lowering-labels.test.ts", "-t", "whose inner switch leaves it"],
  },
  {
    name: "empty-last-case-kept",
    breaks:
      "a last case of nothing, `3 if n > 1 => continue` before `_ => {}` ending a loop's turn, is printed `case 3:` (#488)",
    file: "src/lower/mir/tidy/switches/case_ends.rs",
    find: "            while default.is_none() && cases.last().is_some_and(|case| case.body.is_empty()) {\n",
    replace: "            while false {\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_continue_ending_turn.rs$"],
    snapshots: true,
  },
];
