// Mutations of src/runtime/swap.js (ADR 0012).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "deque-swap-slice-message",
    breaks: "a `VecDeque`'s `swap` out of bounds panics with a slice's index message, not its own assertion",
    file: "src/runtime/swap.js",
    find: "  if (deque && b >= v.length) throw new Error(\"assertion failed: j < self.len()\");\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^deque_swap_out_of_bounds.rs$"],
  },
  {
    name: "swap-names-greater-index",
    breaks: "a slice's `swap` of two indices out of bounds names the greater, where std's names the first it checks",
    file: "src/runtime/swap.js",
    find: "${a >= v.length ? a : b}",
    replace: "${Math.max(a, b)}",
    tests: ["test/corpus.test.ts", "-t", "^slice_swap_first_index.rs$"],
  },
];
