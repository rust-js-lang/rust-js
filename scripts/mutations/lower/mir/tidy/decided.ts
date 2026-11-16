// Mutations of src/lower/mir/tidy/decided.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "written-if-decided-away",
    breaks:
      "an `if` the program wrote that an earlier test decides is folded to its branch, React's `// $FlowFixMe[constant-condition]` lost with its `if (enableGestureTransition)`",
    file: "src/lower/mir/tidy/decided.rs",
    find: "    let wrote = |t: &Expr| !t.span.is_none() && written.iter().any(|c| c.lo < t.span.hi && t.span.lo < c.hi);\n",
    replace: "    let wrote = |_: &Expr| written.is_empty() && false;\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "earlier test decides",
    ],
  },
  {
    name: "decided-tests-kept",
    breaks:
      "a test an earlier one decided stays, `if (option != null) { return option; } .. if (option != null) { drop }` (ADR 0564)",
    file: "src/lower/mir/tidy/decided.rs",
    find: "        if !decidable(test, changing) {\n",
    replace: "        if !decidable(test, changing) || true {\n",
    tests: ["test/std-package.test.ts", "-t", "committed @rust-js/std"],
  },
  {
    name: "decided-test-past-a-call",
    breaks:
      "a test that reads through a field is taken as decided past a call, which may have changed it",
    file: "src/lower/mir/tidy/decided.rs",
    find: "    !test.reads_only_vars() && e.has_effects()\n",
    replace: "    false && !test.reads_only_vars() && e.has_effects()\n",
    tests: ["test/corpus.test.ts", "-t", "^decided_tests_past_a_call.rs$"],
  },
  {
    name: "mir-tidy-decided-part-kept",
    breaks:
      "a part of a chain an earlier test decides is tested again, `.. || Array.isArray(value)` in its `else` (ADR 0743)",
    file: "src/lower/mir/tidy/decided.rs",
    find: "    changed.then_some(folded)\n",
    replace: "    changed.then_some(folded).filter(|_| false)\n",
    tests: [
      "test/lowering-switches.test.ts",
      "-t",
      "a part of a chain an earlier test decides is left out",
    ],
  },
  {
    name: "mir-tidy-decided-part-past-the-if",
    breaks:
      "a part of a chain is left out past the `if` that decides it, where a `switch` of a `match`'s arms reads each test whole (ADR 0743)",
    file: "src/lower/mir/tidy/decided.rs",
    find: "            StmtKind::If(t, ..) if parts && let Some(folded) = decided_part(t, test, holds) => {\n",
    replace:
      "            StmtKind::If(t, ..) if let Some(folded) = decided_part(t, test, holds) => {\n",
    tests: [
      "test/lowering-switches.test.ts",
      "-t",
      "a match of what a value is tells undefined and null",
    ],
  },
  {
    name: "decided-branch-left-empty",
    breaks:
      "an or-pattern's alternative no value reaches, `(_, _, 1) | (_, 1, 1)`, leaves `if (b === 1) {}` once its `c === 1` is decided, and tidy's `values` stage panics, \"no empty if\" (#694); and a guard that fails of some alternatives, `chains`' (#714)",
    file: "src/lower/mir/tidy/decided.rs",
    find: "        if !settled(stmts, i) {\n            i += 1;\n        }\n",
    replace: "        i += 1;\n",
    tests: ["test/corpus.test.ts", "-t", "^or_pattern_(unreachable_alternative|guard_fails_some).rs$"],
  },
  {
    name: "split-test-tested-again",
    breaks: "MIR's split of a `match` of a pair's first option into its two ways keeps the second's test in the `else`, `else if (tracked.contentKeyPath != null && keyPath != null)`, where the `else` says it holds (#1972)",
    file: "src/lower/mir/tidy/decided.rs",
    find: "            split_decided(otherwise, &test, written);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^match_of_a_pair_tests_each_once.rs$"],
    snapshots: true,
  },
  {
    name: "written-test-split-away",
    breaks: "a test the program wrote in a condition of its own, `else if path.is_some() && deep`, is dropped as one an earlier test decides, not as written (#1972, ADR 0564)",
    file: "src/lower/mir/tidy/decided.rs",
    find: "    let one_condition = !t.span.is_none() && condition(t.span).is_some() && condition(t.span) == condition(test.span);\n",
    replace: "    let one_condition = condition(t.span) == condition(test.span) || true;\n",
    tests: ["test/corpus.test.ts", "-t", "^match_of_a_pair_tests_each_once.rs$"],
    snapshots: true,
  },
];
