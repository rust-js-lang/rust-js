// Mutations of src/lower/mir/tidy/jumps/branch_ends.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-branch-ends-kept",
    breaks:
      "a `match`'s arms stay labeled blocks, `bb1: { if (c) { s; break bb1; } t }`",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "            ending(body, label);\n",
    replace: "            let _ = (label, body);\n",
    tests: ["test/corpus.test.ts", "-t", "^option_constants_switch.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-inner-block-kept",
    breaks:
      "a block that ends another stays a block of its own, its `break`s labeled",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "        && span.is_none()\n        && marked.is_none_or(|marked| super::continues::latest(body) < marked)\n    {\n        let (inner, mut body)",
    replace: "        && span.is_none()\n        && false\n    {\n        let (inner, mut body)",
    tests: ["test/corpus.test.ts", "-t", "^switch_default_comment_falls.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-let-else-branch-ends",
    breaks:
      "a block that ends in what never goes on takes it as an `else`, not what's after a guard: `if (a || b) { .. } else { for (..) { .. } return; }` where ADR 0805's `or_before_a_returning_loop` writes the loop after the `if`, and a let chain's `else` of reads the same",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "        .filter(|_| !exits)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(or_before_a_returning_loop|let_chain_else_of_reads)\\.rs$"],
    snapshots: true,
  },
  {
    name: "branch-ends-written-block-merged",
    breaks:
      "a block the program labeled that ends a block MIR made is merged into it, `'desc` named by MIR's block (ADR 0745)",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "        } = &mut stmts[at]\n        && span.is_none()\n",
    replace: "        } = &mut stmts[at]\n        && (span.is_none() || true)\n",
    tests: [
      "test/port-performance-track.test.ts",
      "-t",
      "addValueToProperties is a switch whose case breaks where it's written",
    ],
  },
  {
    name: "branch-ends-left-inside-kept",
    breaks:
      "an `if` whose `then` never goes on but breaks out to the block's end inside stays a guard, the block its label (ADR 0745)",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: " if leaves(&ands(then).1, label) || left(then)))\n",
    replace: " if leaves(&ands(then).1, label)))\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "a continue the program wrote in a chain",
    ],
  },
  {
    name: "loop-ending-goes-on",
    breaks:
      "a list that ends in a loop nothing leaves is taken to go on, so an `||` the program wrote before it isn't a guard as written but an `else if` of the arm before, `} else if (indent < 3 || (..)) { desc = ..; break desc; } else { .. }` (ADR 0805)",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "        StmtKind::While { .. } => ends(last),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^or_before_a_returning_loop.rs$"],
    snapshots: true,
  },
  {
    name: "branch-ends-returning-if-not-an-exit",
    breaks:
      "a block MIR made whose rest ends in an `if` each branch of which returns, the returns given back past a block, is made an `else` of a guard before it: addValueToProperties' `if (ArrayBuffer.isView(value)) { ..; break; }` nests what follows in an `else` (rd98)",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "            StmtKind::If(_, then, Some(otherwise)) => {\n                then.last().is_some_and(exits) && otherwise.last().is_some_and(exits)\n",
    replace:
      "            StmtKind::If(_, then, Some(otherwise)) if false => {\n                then.last().is_some_and(exits) && otherwise.last().is_some_and(exits)\n",
    tests: [
      "test/port-performance-track.test.ts",
      "-t",
      "addValueToProperties",
    ],
  },
  {
    name: "ending-block-before-comments-kept",
    breaks: "a block MIR made that ends its block but for the comments after it, a last `_ => {}`'s `// Fallthrough`, keeps its label, so the `switch` is in a labeled block the Rust lacks (rd702)",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "    let last = (stmts.iter()).rposition(|s| !matches!(s.kind, StmtKind::Comments));\n",
    replace: "    let last = stmts.len().checked_sub(1);\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_default_comment_falls.rs$"],
    snapshots: true,
  },
  {
    name: "ending-comments-after-jump",
    breaks: "the comments after a block MIR made are after the jump its statements end with, where nothing reaches them, so they're dropped (rd702)",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "        let jump = (body.last().is_some_and(ends) && !comments.is_empty()).then(|| body.pop());\n",
    replace: "        let jump = (body.last().is_some_and(ends) && comments.is_empty()).then(|| body.pop());\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_default_comment_falls.rs$"],
    snapshots: true,
  },
  {
    name: "ending-merged-past-a-comment-before-code",
    breaks:
      "a block MIR made is merged past a comment written before what it runs, `if a || b || (c && d) { // retain } else { s }`'s, so its last disjunct is an inverted `else if` (rd714)",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "        && marked.is_none_or(|marked| super::continues::latest(body) < marked)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^if_only_comment_or_test.rs$"],
    snapshots: true,
  },
  {
    name: "ending-own-break-after-comments-kept",
    breaks:
      "a block's own `break` after a last `_ => {}`'s comments stops the block before it from being merged, so a `match` with a `return` arm ends each case with the tail (rd702)",
    file: "src/lower/mir/tidy/jumps/branch_ends.rs",
    find: "        && commented(rest)\n",
    replace: "        && commented(rest)\n        && rest.is_empty()\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_default_comment_after_return.rs$"],
    snapshots: true,
  },
];
