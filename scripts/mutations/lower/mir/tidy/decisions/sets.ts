// Mutations of src/lower/mir/tidy/decisions/sets.rs (ADR 0364).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "decision-integer-converse-apart",
    breaks:
      "an integer comparison's converse, `row.length < 1`, is a set apart from `row.length >= 1`'s complement, so a slice pattern's tree, whose NaN neither passes, stays labeled blocks, not a conditional (ADR 0978, amended)",
    file: "src/lower/mir/tidy/decisions/sets.rs",
    find: "        ExprKind::Binary(Op::Lt | Op::Le, ..) if e.facts.has(js::Known::NeverNaN) => {",
    replace: "        ExprKind::Binary(Op::Lt | Op::Le, ..) if false => {",
    tests: ["test/corpus.test.ts", "-t", "^default_value_exits.rs$"],
    snapshots: true,
  },
];
