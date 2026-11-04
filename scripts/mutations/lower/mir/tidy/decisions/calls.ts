// Mutations of src/lower/mir/tidy/decisions/calls.rs (#221, ADR 0364).
import type { Mutation } from "../../../../../mutations";

const CALLS = [
  "test/corpus.test.ts",
  "-t",
  "^(default_value_exits|guards_that_call).rs$",
];

export const mutations: Mutation[] = [
  {
    name: "calls-nothing-said",
    breaks:
      "a test what's tested before says, `!odd(b)` once `odd(b)` failed, stays, so the chain calls again and the tree stays",
    file: "src/lower/mir/tidy/decisions/calls.rs",
    find: "                if self.says(steps, &paths[..j], &paths[j][..t], &paths[j][t]) {\n",
    replace: "                if false {\n",
    tests: CALLS,
    snapshots: true,
  },
  {
    name: "calls-no-fewer",
    breaks:
      "an arm's tests keep what the tree tested on each way to it, `row.length === 2 && below(row[0]) || (row.length >= 1 && row.length < 2 || row.length > 2) && below(row[0])`",
    file: "src/lower/mir/tidy/decisions/calls.rs",
    find: "            paths = fewer;\n",
    replace: "            paths = fewer;\n            break;\n",
    tests: CALLS,
    snapshots: true,
  },
  {
    name: "calls-alike-refused",
    breaks:
      "a guarded `|`'s guard that calls, MIR's copy in each alternative, is two calls of the model, which refuses the tree: `Some(1) | None if on(..)` is MIR's labeled block (rd136)",
    file: "src/lower/mir/tidy/decisions/calls.rs",
    find: "        if arms.iter().any(|arm| arm.made.iter().any(twice)) {\n",
    replace: "        if unique.len() < calls.len() || arms.iter().any(|arm| arm.made.iter().any(twice)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guarded_or_pattern.rs$"],
    snapshots: true,
  },
  {
    name: "guard-read-after-its-call-refused",
    breaks: "a guard's read of what its own call changes, `mark(*n) && done.get()`, is refused as a read after a call, so the `match` stays MIR's labeled block, `bb30:` (C1, #523)",
    file: "src/lower/mir/tidy/decisions/calls.rs",
    find: "                    && !(trace.iter()).all(|&c| self.made[c].is_some_and(|m| kept.contains(&m)) || in_its_guard(c))\n",
    replace: "                    && !(trace.iter()).all(|&c| self.made[c].is_some_and(|m| kept.contains(&m)) || in_its_guard(c) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^guards_that_call.rs$"],
    snapshots: true,
  },
];
