// Mutations of src/lower/analysis/intervals/state.rs, split from intervals.rs (ADR 1137, #1672).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-copies-unnarrowed",
    breaks: "a comparison narrows only what's compared, not its copies, so `i += 1` under `i > 100` of `let mut i = start` may wrap",
    file: "src/lower/analysis/intervals/state.rs",
    find: "        for l in [local, root].into_iter().chain(copies) {\n",
    replace: "        for l in [root].into_iter().chain(copies.into_iter().take(0)) {\n",
    tests,
  },
  {
    name: "intervals-join-drops-what-intervals-show",
    breaks: "a join drops a relation one side has and the other's intervals show, `s = 0` before a loop, so `a[s - 1]` of a counter down from a length keeps its check",
    file: "src/lower/analysis/intervals/state.rs",
    find: "                    _ => two.below_by_intervals(x, y),\n",
    replace: "                    _ => None,\n",
    tests: ["test/intervals-shadow.test.ts"],
  },
  {
    name: "intervals-lent-away-narrowed",
    breaks: "a local lent `&mut`, then tested, is taken as what the test showed, so `n - 1` past `n != 0` is JS's own where a raw pointer the borrow became set `n` to 0, and gives `-1` (#1782)",
    file: "src/lower/analysis/intervals/state.rs",
    find: "        if self.lent_away.contains(&root) {\n            return;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointer_written_after_its_owner_reads.rs$"],
  },
];
