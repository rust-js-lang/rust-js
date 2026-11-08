// Mutations of src/lower/body_queries/total_orders.rs (ADR 0036, #936).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "orders-totally-any-call",
    breaks: "a comparator of `partial_cmp(..).unwrap_or(Equal)` is taken as a total order, so its sort of NaNs is JS's, which leaves another order and doesn't panic where std's does (#936)",
    file: "src/lower/body_queries/total_orders.rs",
    find: "                        self.order(args[0], depth + 1) && total_fn(thir[strip(thir, args[1])].ty)\n                    }\n                    _ => false,\n",
    replace: "                        self.order(args[0], depth + 1) && total_fn(thir[strip(thir, args[1])].ty)\n                    }\n                    _ => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^sort_not_total.rs$"],
  },
  {
    name: "orders-totally-written-then-unasked",
    breaks: "an `Ord` that writes `then` out, `if o != Equal { o } else { p }`, is taken as maybe not total, so its sort is `$sortStable`, not JS's (#936)",
    file: "src/lower/body_queries/total_orders.rs",
    find: "                self.same_var(o, kept) && self.order(o, depth + 1) && self.order(next, depth + 1)\n",
    replace: "                false && self.same_var(o, kept) && self.order(o, depth + 1) && self.order(next, depth + 1)\n",
    tests: ["test/corpus.test.ts", "-t", "^sort_total_written_out.rs$"],
    snapshots: true,
  },
  {
    name: "orders-totally-written-then-kept-unasked",
    breaks: "an `if o != Equal { p } else { o }`, which isn't `then`, is taken as a total order, so a sort by one that isn't is JS's (#936)",
    file: "src/lower/body_queries/total_orders.rs",
    find: "                self.same_var(o, kept) && self.order(o, depth + 1) && self.order(next, depth + 1)\n",
    replace: "                self.order(o, depth + 1) && self.order(next, depth + 1)\n",
    tests: ["test/corpus.test.ts", "-t", "^sort_total_written_out.rs$"],
    snapshots: true,
  },
  {
    name: "orders-totally-match-unasked",
    breaks: "an `Ord` that writes `then` out as a `match o { Equal => p, o => o }` is taken as maybe not total, so its sort is `$sortStable`, not JS's (#936)",
    file: "src/lower/body_queries/total_orders.rs",
    find: "                first.guard.is_none()\n",
    replace: "                false && first.guard.is_none()\n",
    tests: ["test/corpus.test.ts", "-t", "^sort_total_written_out.rs$"],
    snapshots: true,
  },
];
