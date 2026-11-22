// Mutations of crates/rust-js-program/src/runtime/swap.js (ADR 0012).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "deque-swap-site-of-i",
    breaks: "a `VecDeque`'s `swap` of `j` past its end names the line of `i`'s assertion",
    file: "crates/rust-js-program/src/runtime/swap.js",
    find: "$stdSites.vecDequeSwap[1]",
    replace: "$stdSites.vecDequeSwap[0]",
    tests: ["test/binaries.test.ts", "-t", "std raises"],
  },
  {
    name: "deque-swap-slice-message",
    breaks: "a `VecDeque`'s `swap` out of bounds panics with a slice's index message, not its own assertion",
    file: "crates/rust-js-program/src/runtime/swap.js",
    find: "  if (deque && b >= v.length) throw $panic(\"assertion failed: j < self.len()\", $stdSites.vecDequeSwap[1]);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^deque_swap_out_of_bounds.rs$"],
  },
  {
    name: "swap-names-greater-index",
    breaks: "a slice's `swap` of two indices out of bounds names the greater, where std's names the first it checks",
    file: "crates/rust-js-program/src/runtime/swap.js",
    find: "${a >= v.length ? a : b}",
    replace: "${Math.max(a, b)}",
    tests: ["test/corpus.test.ts", "-t", "^slice_swap_first_index.rs$"],
  },
];
