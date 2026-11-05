// Mutations of src/lower/iterators/value.rs (ADR 0093): an iterator's value,
// the one writer of its JS (ADR 1092, #730).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lent-iterator-closable",
    breaks:
      "a lazy chain lent to `it.take(2)` is closed by it, and its lender's `next()` finds it done",
    file: "src/lower/iterators/value.rs",
    find: '            e = made("$lent", vec![e], &[Known::IterLent]);',
    replace: '            e = Expr::call(Expr::member(Expr::var("Iterator"), "from"), vec![e]);',
    tests: ["test/corpus.test.ts", "-t", "lent_iterators"],
  },
  {
    name: "closing-drops-each-as-one-slice",
    breaks:
      "a `Drain`'s or a B-tree's items closed from their back are dropped as one slice, front to back, where each is dropped as it's passed (#234)",
    file: "src/lower/iterators/value.rs",
    find: "            if passes_each {\n                args.push(Expr::bool(false));\n            }\n",
    replace: "            let _ = passes_each;\n",
    tests: ["test/corpus.test.ts", "-t", "^(nth_passes_a_slice_or_each|rfold_and_rev_close_what_they_step|linked_list_into_iter|skip_drops_what_it_passes_first).rs$"],
  },
];
