// Mutations of src/lower/mir/tidy/decisions/calls.rs (#221, ADR 0364).
import type { Mutation } from "../../../../../mutations";

const CALLS = ["test/corpus.test.ts", "-t", "^(default_value_exits|guards_that_call).rs$"];

export const mutations: Mutation[] = [
  {
    name: "calls-read-after-unkept",
    breaks: "what a guard's call may change, read after it, `mark(n) && done`, is taken to read as before, and the tree a chain",
    file: "src/lower/mir/tidy/decisions/calls.rs",
    find: "                    && !(trace.iter()).all(|&c| self.made[c].is_some_and(|m| kept.contains(&m)))\n",
    replace: "                    && false\n",
    tests: CALLS,
    snapshots: true,
  },
  {
    name: "calls-nothing-said",
    breaks: "a test what's tested before says, `!odd(b)` once `odd(b)` failed, stays, so the chain calls again and the tree stays",
    file: "src/lower/mir/tidy/decisions/calls.rs",
    find: "                if self.says(steps, &paths[..j], &paths[j][..t], &paths[j][t]) {\n",
    replace: "                if false {\n",
    tests: CALLS,
    snapshots: true,
  },
  {
    name: "calls-no-fewer",
    breaks: "an arm's tests keep what the tree tested on each way to it, `row.length === 2 && below(row[0]) || (row.length >= 1 && row.length < 2 || row.length > 2) && below(row[0])`",
    file: "src/lower/mir/tidy/decisions/calls.rs",
    find: "            paths = fewer;\n",
    replace: "            paths = fewer;\n            break;\n",
    tests: CALLS,
    snapshots: true,
  },
];
