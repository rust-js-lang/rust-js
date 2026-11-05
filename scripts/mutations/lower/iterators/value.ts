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
  {
    name: "range-from-uncounted",
    breaks:
      "`for i in 0.. { let Some(&x) = v.get(i) else { break }; .. }` stays `for (const i of $iterator({ start: 0 }, ..)) { if (v[i] == null) { break; } .. }`, not `for (let i = 0; i < v.length; i++)` (rd260)",
    file: "src/lower/iterators/value.rs",
    find: "        (_, [_, _]) if e.facts.has(Known::IterRange) => (e, 1),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^stepped_index_to_length.rs$"],
    snapshots: true,
  },
  {
    name: "mir-range-inclusive-lazy",
    breaks:
      "`(1..=n).collect()` from MIR stays `Array.from($iterator(\u2026, $rangeInclusiveNext))`, not `$range(1, n + 1)`",
    file: "src/lower/iterators/value.rs",
    find: "    let (inclusive, back) = [(false, false), (false, true), (true, false), (true, true)]\n",
    replace:
      "    let (inclusive, back) = [(false, false), (false, true), (true, true)]\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "a range mapped and collected",
    ],
  },
  {
    name: "range-unmarked",
    breaks:
      "a range's numbers aren't marked so where they're made, so what reads their bounds, `(0..n).rev().nth(k)`'s `$rangeNthBack`, a `for` that counts, `$lazyRange(a, b)`, never finds them",
    file: "src/lower/iterators/value.rs",
    find: "                made(\"$iterator\", vec![range, step], &[Known::IterStepped, Known::IterRange])\n",
    replace: "                made(\"$iterator\", vec![range, step], &[Known::IterStepped])\n",
    tests: ["test/corpus.test.ts", "-t", "^(range_own_steps|range_steps_known|stepped_index_to_length).rs$"],
    snapshots: true,
  },
  {
    name: "value-facts-dropped-where-remade",
    breaks:
      "a value made again of the same call, `for (x of e)`'s `e`, loses what its maker marked it, so `for (const i of $iterator({ start, end }, $rangeNextBack))` isn't the `for` that counts down",
    file: "src/lower/iterators/value.rs",
    find: "        .filter(|&fact| facts.has(fact))\n",
    replace: "        .filter(|&fact| facts.has(fact) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^(rev_loops|range_rev_from_computed).rs$"],
    snapshots: true,
  },
  {
    name: "steps-of-no-value",
    breaks:
      "what a range's or the crate's value steps isn't read out of its `$iterator`, so a `len()` of a reversed range made in place counts the JS iterator's own fields, not its bounds",
    file: "src/lower/iterators/value.rs",
    find: "        js::ExprKind::Call(_, args) if e.facts.has(Known::IterStepped) => args.first(),\n",
    replace: "        js::ExprKind::Call(_, args) if e.facts.has(Known::IterStepped) && false => args.first(),\n",
    tests: ["test/corpus.test.ts", "-t", "^(len_made_in_place|range_values|collected_counts_ask_room).rs$"],
    snapshots: true,
  },
];
