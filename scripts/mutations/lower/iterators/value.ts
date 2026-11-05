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
    tests: ["test/corpus.test.ts", "-t", "^(lent_iterators|by_ref_lends|by_ref_chains).rs$"],
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
  {
    name: "iter-eager-lent-root-kept",
    breaks: "a chain on `$lent($iter(xs))` just made stays a JS iterator's",
    file: "src/lower/iterators/value.rs",
    find: "        js::ExprKind::Call(_, of) if e.facts.has(Known::IterLent) && of.len() == 1 => &of[0],\n",
    replace: "        js::ExprKind::Call(_, of) if e.facts.has(Known::IterLent) && of.len() == 1 => e,\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "let-else from MIR is a guard",
    ],
  },
  {
    name: "closing-unmarked",
    breaks:
      "a value closed to drop what it hasn't given isn't marked so, so what makes it again knowing its back, `$closing($iterator(it, next), drop)`'s, steps it as the crate's own, unclosed (#223)",
    file: "src/lower/iterators/value.rs",
    find: '            e = made("$closing", args, &[Known::IterClosing]);',
    replace: '            e = made("$closing", args, &[]);',
    tests: ["test/corpus.test.ts", "-t", "^(panic_mid_fold_drops|rfold_and_rev_close_what_they_step|map_into_iter_drops).rs$"],
    snapshots: true,
  },
  {
    name: "in-place-form-peeled",
    breaks:
      "a `rev()` of the crate's iterator made in place is peeled to the iterator it reverses, which a `step_by` then steps forward",
    file: "src/lower/iterators/value.rs",
    find: "    if super::made_in_place(e) {\n        return None;\n    }\n    steps(e).cloned()\n",
    replace: "    steps(e).cloned()\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rev_in_place_through_adapters).rs$",
    ],
  },
  {
    name: "rev-unmarked",
    breaks:
      "`$rev(it)` isn't marked where it's made, so what sees through it, `rev().rev()`'s, a reversed range's count or tidy's for of it, never finds it (#722)",
    file: "src/lower/iterators/value.rs",
    find: "    e.facts = e.facts.with(Known::IterRev);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(rev_of_rev_last_folds|rev_of_rev_folds_its_source|len_made_in_place|rev_range_counted_beside_its_rfold).rs$"],
    snapshots: true,
  },
  {
    name: "own-iterator-unmarked",
    breaks:
      "an iterator of the crate's isn't marked its own steps' `$iterator`, so one stepped till it ends keeps a `size_hint` nothing asks, and one behind a `dyn` keeps its own `nth`",
    file: "src/lower/iterators/value.rs",
    find: "                made(\"$iterator\", args, &[Known::IterStepped, Known::IterOwn])\n",
    replace: "                made(\"$iterator\", args, &[Known::IterStepped])\n",
    tests: ["test/corpus.test.ts", "-t", "^(fused_iterators|double_ended_iterators|own_fold_and_nth|own_steps_behind_adapters_unwind).rs$"],
    snapshots: true,
  },
  {
    name: "knowing-unread",
    breaks:
      "tidy's `stepped_in_place` doesn't see through `$knowing`, so what a `fuse()` std's struct is made knowing its back steps keeps a `size_hint` nothing asks",
    file: "src/lower/iterators/value.rs",
    find: "    e.facts.has(Known::IterKnowing)\n",
    replace: "    let _ = e;\n    false\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterator.rs$"],
    snapshots: true,
  },
  {
    name: "rev-made-knowing-unmarked",
    breaks:
      "a `rev()` made knowing its back isn't marked so by its maker, so where it's stepped from its back, past a variable or a `&mut`, it's refused as one rust-js can't step from its back (#730)",
    file: "src/lower/iterators/value.rs",
    find: "    e.facts = e.facts.with(Known::IterKnowsBack);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(rev_made_knowing_its_back|own_iterator_flattened_from_its_back).rs$"],
  },
  {
    name: "held-iterator-unmarked",
    breaks:
      "an iterator its owner drops, `$held(..)`, loses what its maker marked it, so a `rev()` made knowing its back and dropped as it unwinds is refused where it's stepped from its back (#730)",
    file: "src/lower/iterators/value.rs",
    find: "        true => made_knowing_its_back(held),\n",
    replace: "        true => held,\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_made_knowing_its_back.rs$"],
  },
  {
    name: "map-of-an-iterator-unmarked",
    breaks: "MIR's map of a JS iterator isn't said to be one, so a held map made again where it's stepped from its back is left declared, unread (#805)",
    file: "src/lower/iterators/value.rs",
    find: "    e.facts = e.facts.with(Known::Iterator);\n    e\n",
    replace: "    e\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^unread_map_of_a_held_iterator.rs$"],
  },
  {
    name: "enumeration-source-unread",
    breaks: "an enumerate's form is read as written",
    file: "src/lower/iterators/value.rs",
    find: "                && args[0] == enumerating() =>",
    replace: "                && args[0] != enumerating() =>",
    tests: ["test/corpus.test.ts", "-t", "^returned_slice_iters.rs$"],
  },
  {
    name: "mir-enumerate-swapped",
    breaks: "`enumerate` gives `(item, index)`, not `(index, item)`",
    file: "src/lower/iterators/value.rs",
    find: "    let pair = Expr::array(vec![Expr::var(\"i\"), Expr::var(\"x\")]);\n",
    replace: "    let pair = Expr::array(vec![Expr::var(\"x\"), Expr::var(\"i\")]);\n",
    tests: ["test/corpus.test.ts", "-t", "labeled_blocks"],
  },
  {
    name: "enumerated-knowing-unmarked",
    breaks: "`$enumerate(it)` isn't marked made knowing its back and its length, so a reversed enumeration's `len()` and its `next_back` read it again as what it isn't, and are refused (#764)",
    file: "src/lower/iterators/value.rs",
    find: "    e.facts = e.facts.with(Known::IterEnumerate);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_made_knowing_read_again.rs$"],
  },
];
