// Mutations of src/lower/body_queries/ranges/reads.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "utf16-count-impure",
    breaks: "a test of `s.encode_utf16().count()` narrows nothing, as if it changed what it reads, so a counter below it keeps its wrap",
    file: "src/lower/body_queries/ranges/reads.rs",
    find: "                        || makes_utf16_units(tcx, def_id, thir[e].ty)\n",
    replace: "",
    tests: ["test/lowering-strings.test.ts", "-t", "a string's count of UTF-16 units is its length"],
  },
  {
    name: "string-length-impure",
    breaks: "a test of a JS string's `length` isn't read, so `i + 1 < length(s)` shows nothing of `i`",
    file: "src/lower/body_queries/ranges/reads.rs",
    find: "                ) || reads_string_length(tcx, def_id)\n",
    replace: "                ) || false\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "below a JS string"],
  },
  {
    name: "cell-read-impure",
    breaks: "`i < queue.borrow().len()` isn't a test that reads without changing anything, and `queue.borrow()[i]` below it is `$index(queue, i)`",
    file: "src/lower/body_queries/ranges/reads.rs",
    find: "                        || matches!(\n",
    replace: "                        || false && matches!(\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_vec_bounds"],
    snapshots: true,
  },
  {
    name: "string-deref-not-read",
    breaks: "a `String`'s `deref` isn't a read, so a test of `firstDiff < serverText.length` of a `String` narrows nothing, and `firstDiff++` below it keeps its wrap",
    file: "src/lower/body_queries/ranges/reads.rs",
    find: "                        || is_string_deref(tcx, def_id, generic_args)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^ptr_eq_items_and_counts.rs$"],
    snapshots: true,
  },
];
