// Mutations of src/lower/body_queries/ranges/reads.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "string-length-impure",
    breaks:
      "a test of a JS string's `length` isn't read, so `i + 1 < length(s)` shows nothing of `i`",
    file: "src/lower/body_queries/ranges/reads.rs",
    find: "                ) || reads_string_length(tcx, def_id)\n",
    replace: "                ) || false\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "below a JS string"],
  },
  {
    name: "cell-read-impure",
    breaks:
      "`i < queue.borrow().len()` isn't a test that reads without changing anything, and `queue.borrow()[i]` below it is `$index(queue, i)`",
    file: "src/lower/body_queries/ranges/reads.rs",
    find: "                        || matches!(\n",
    replace: "                        || false && matches!(\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_vec_bounds"],
    snapshots: true,
  },
  {
    name: "min-max-impure",
    breaks:
      "an integer's `max` reads as a call that may change anything, so a table's count `n.max(2) + 1` has no range (#367)",
    file: "src/lower/body_queries/ranges/reads.rs",
    find: "                        || (ord_max_or_min(tcx, def_id).is_some() && thir[e].ty.is_integral())\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
  {
    name: "if-value-unread",
    breaks:
      "an `if`'s value, `if replace { 1 } else { 0 }`, isn't read as pure, so `&queue[if replace { 1 } else { 0 }..]` isn't shown in bounds and stays `$slice` (rd376)",
    file: "src/lower/body_queries/ranges/reads.rs",
    find: "            } => reads(tcx, thir, cond, vars) && reads(tcx, thir, then, vars) && reads(tcx, thir, otherwise, vars),\n",
    replace: "            } => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^filtered_not_empty.rs$"],
    snapshots: true,
  },
];
