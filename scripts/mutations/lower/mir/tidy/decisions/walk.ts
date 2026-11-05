// Mutations of src/lower/mir/tidy/decisions/walk.rs (#771, ADR 0364).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "decisions-loop-exit-kept",
    breaks:
      "an arm ending in a loop that leaves the match keeps the match a labeled block",
    file: "src/lower/mir/tidy/decisions/walk.rs",
    find: "                if !loop_breaks(body, label) {\n",
    replace: "                if true {\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(else_break_ends_loop|signed_counter)\\.rs$",
    ],
  },
  {
    name: "declared-between-tests-left",
    breaks: "a `let x;` MIR declares between a tree's tests, past which a test leaves the block, is what a leaf does, so `declared_between`'s `(x, 3) | (3, x) | (x, 0) if h(c, x).is_none() && x > 2` keeps MIR's labeled block (#771)",
    file: "src/lower/mir/tidy/decisions/walk.rs",
    find: "            && leaving\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^(match_trees_chained).rs$"],
    snapshots: true,
  },
  {
    name: "declared-between-tests-of-any-leaf",
    breaks: "a `let x;` between a tree's tests is declared before the chain where what follows it is a leaf too, so a leaf's own tree, `let x$2; if (op === 3) { .. } .. tmp = ..; break bb62;`, is walked into, its arms share `tmp = ..`, and `declared_outside` keeps its labeled block (#771)",
    file: "src/lower/mir/tidy/decisions/walk.rs",
    find: "        let leaving = declares && self.leaf(&frames).is_none();\n",
    replace: "        let leaving = declares;\n",
    tests: ["test/corpus.test.ts", "-t", "^(match_trees_chained).rs$"],
    snapshots: true,
  },
  {
    name: "ways-none-takes-walked",
    breaks: "the walk goes on down each way, `b === 1` once `b >= 1 && b <= 2` failed too, so a run of guarded `if`s each going on to the next is more leaves than a match has and `ways_none_takes` keeps its labeled block (#771)",
    file: "src/lower/mir/tidy/decisions/walk.rs",
    find: "            let taken = |tests: &[Expr]| simplified(tests, false).is_some();\n",
    replace: "            let taken = |_: &[Expr]| true;\n",
    tests: ["test/corpus.test.ts", "-t", "^(match_trees_chained).rs$"],
    snapshots: true,
  },
  {
    name: "given-in-nested-tests-dropped",
    breaks: "what a guard's bindings give is read in its own test only, not in the `if`s MIR nests of its `&&`, `x = l; if (x % 2 === 0) { if (g(c, x, 0)) {`, so `read_twice` keeps its labeled block (#771)",
    file: "src/lower/mir/tidy/decisions/walk.rs",
    find: "            let given = [given, bound].concat();\n",
    replace: "            drop(bound);\n",
    tests: ["test/corpus.test.ts", "-t", "^(match_trees_chained).rs$"],
    snapshots: true,
  },
];
