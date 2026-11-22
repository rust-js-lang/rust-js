// Mutations of crates/rust-js-lower/src/iterators/boundary.rs, an iterator at the JS
// boundary (ADR 1169, #1993, #1994, #1999).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "boundary-back-unread",
    breaks: "an iterator at the JS boundary bound by `DoubleEndedIterator` isn't known to have a back: `upto(4)`'s declared type drops `DoubleEndedIterator<number>` and its JS doesn't know its back, so `lastOf(upto(4))` throws (#1999)",
    file: "crates/rust-js-lower/src/iterators/boundary.rs",
    find: "            back |= is_double_ended(tcx, id);\n",
    replace: "            back |= false;\n",
    tests: ["test/declarations-iterators.test.ts"],
  },
  {
    name: "boundary-len-unread",
    breaks: "an iterator at the JS boundary bound by `ExactSizeIterator` isn't known to have a length: `left(upto(4))` throws `len is not a function` (#1999)",
    file: "crates/rust-js-lower/src/iterators/boundary.rs",
    find: "            len |= is_exact_size(tcx, id);\n",
    replace: "            len |= false;\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "given back to JS"],
  },
  {
    name: "boundary-compiled-struct-converted",
    breaks: "an `impl Iterator` of std's compiled struct, asked neither its back nor its length, is converted though it's a JS iterator already, so a copy of its function is made for the crate's Rust that's no different (#1999)",
    file: "crates/rust-js-lower/src/iterators/boundary.rs",
    find: " || !crate::recognition::is_compiled_std_iterator(tcx, self.revealed)\n",
    replace: " || true\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_clones_of_unseen_callers.rs$"],
    snapshots: true,
  },
  {
    name: "boundary-back-unconverted",
    breaks: "an `impl DoubleEndedIterator` of std's compiled struct, `evens`' `Filter`, is given back to JS unconverted, knowing no back: `lastOf(evens(..))` throws (#1999)",
    file: "crates/rust-js-lower/src/iterators/boundary.rs",
    find: "        self.boundary.back || self.boundary.len",
    replace: "        self.boundary.len",
    tests: ["test/lowering-iterators.test.ts", "-t", "given back to JS"],
  },
];
