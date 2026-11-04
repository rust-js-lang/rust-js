// Mutations of src/lower/mir/tidy/decided.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "present-return-unseen",
    breaks: "React's resolveCurrentlyRenderingComponent returns `$unwrap(currentlyRenderingComponent)` after its `== null` test threw (rd443)",
    file: "src/lower/mir/tidy/decided.rs",
    find: "        | StmtKind::Expr(value)\n        | StmtKind::Return(Some(value)) => present(value),\n",
    replace: "        | StmtKind::Expr(value) => present(value),\n",
    tests: ["test/lowering-early-exits.test.ts", "-t", "module variable returned after its test"],
  },
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
    name: "present-coalesce-kept",
    breaks:
      '`return x ?? null` stays where a test it\'s under says `x` is there, `typeof x === "function"`',
    file: "src/lower/mir/tidy/decided.rs",
    find: "                    present_in(&mut stmts[j], test, holds, false);\n",
    replace: "",
    tests: ["test/lowering-options.test.ts", "-t", "a test says is there"],
  },
  {
    name: "present-coalesce-failed-test-unread",
    breaks: "after `if (x == null) { return null; }`, `x ?? null` stays",
    file: "src/lower/mir/tidy/decided.rs",
    find: "        false => test.clone().negated_test(),\n",
    replace: "        false => test.clone(),\n",
    tests: ["test/lowering-options.test.ts", "-t", "a test says is there"],
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
    name: "field-test-undecided-where-its-variable-changes",
    breaks:
      "a test through a field of a variable the loop assigns is never decided, so `$unwrap(node.sibling)` stays where `node.sibling != null` holds",
    file: "src/lower/mir/tidy/decided.rs",
    find: "    if test.reads_only_vars() {\n        test.visit_vars(&mut |v| stable &= !changing.contains(v));\n",
    replace:
      "    if true {\n        test.visit_vars(&mut |v| stable &= !changing.contains(v));\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "an unwrap of a field a test says is there",
    ],
  },
  {
    name: "unwrap-of-tested-kept",
    breaks: "an `$unwrap(p)` where the test says `p` is there stays",
    file: "src/lower/mir/tidy/decided.rs",
    find: "            Some((_, option, _)) if option.reads_same() && there(option) => *e = option.clone().or_at(e.span),\n",
    replace:
      "            Some((_, option, _)) if false && there(option) => *e = option.clone().or_at(e.span),\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "an unwrap of a field a test says is there",
    ],
  },
  {
    name: "unwrap-past-a-call",
    breaks:
      "an `$unwrap(p)` after a call that may change `p` is `p`, though the call cleared it",
    file: "src/lower/mir/tidy/decided.rs",
    find: "                    present_in(&mut stmts[j], test, holds, true);\n                    return;\n",
    replace:
      "                    present_in(&mut stmts[j], test, holds, true);\n                    j += 1;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "an unwrap of a field a test says is there",
    ],
  },
];
