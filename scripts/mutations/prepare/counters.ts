// Mutations of src/prepare/counters.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "counter-scope-unasked",
    breaks:
      "two loops each holding their range's end both name it `end`, two `let end`s of one block, as nothing asks JS's scopes of a counter's word (#2032)",
    file: "src/prepare/counters.rs",
    find: "        .find(|to| !tree.shadows(name.word(), to) && tree.renames(name.word(), to))\n",
    replace: "        .find(|to| !tree.shadows(name.word(), to))\n",
    tests: ["test/lowering-names.test.ts", "-t", "two loops each hold their range's end"],
  },
  {
    name: "counter-shadows-around",
    breaks:
      "a loop's counter is spelled `i` where a variable `i` is in scope around it, `main`'s later `let i`, shadowing it, not `k` (ADR 1042, #2032)",
    file: "src/prepare/counters.rs",
    find: "        .find(|to| !tree.shadows(name.word(), to) && tree.renames(name.word(), to))\n",
    replace: "        .find(|to| tree.renames(name.word(), to))\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_handle.rs$"],
    snapshots: true,
  },
  {
    name: "counter-letters-gone",
    breaks:
      "a loop inside a loop of `i` counts with `i$1`, not `j`, as a person writes it (ADR 1042, #2032)",
    file: "src/prepare/counters.rs",
    find: "        \"i\" => Box::new([\"i\", \"j\", \"k\"].into_iter().map(String::from).chain(apart)),\n",
    replace: "        \"i\" => Box::new([\"i\"].into_iter().map(String::from).chain(apart)),\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_loops.rs$"],
    snapshots: true,
  },
];
