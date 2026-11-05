// Mutations of src/lower/iterators/in_place.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fuse-of-drops-elided",
    breaks:
      "a `fuse()` stepped in place given its source's drop is elided with it, so what a `Vec` of items with a destructor didn't give is never dropped (ADR 0983)",
    file: "src/lower/iterators/in_place.rs",
    find: "        _ if fused && given.len() <= 3 && given.first().is_some_and(is_js_iterator) => given[0].clone(),\n",
    replace:
      "        _ if fused && given.first().is_some_and(is_js_iterator) => given[0].clone(),\n",
    tests: ["test/corpus.test.ts", "-t", "^(owned_items_through_fuse|compiled_adapters_closed_as_a_collect_unwinds).rs$"],
  },

  {
    name: "does-f64-max-given-acts",
    breaks: "`$f64Max` given as it is acts, so a chain it folds stays lazy",
    file: "src/lower/iterators/in_place.rs",
    find: "        js::ExprKind::Var(_) => !f.helper_name().is_some_and(crate::runtime::is_pure),\n",
    replace: "",
    tests: ["test/traits.test.ts", "-t", "MIR dictionaries are explicit"],
  },
  {
    name: "into-iter-for-kept",
    breaks:
      "a `for` of a generic `into_iter()` steps `$intoIter(xs)`, where a `for` steps `xs` as it is",
    file: "src/lower/iterators/in_place.rs",
    find: '        Some("$intoIter") if by_for => given[0].clone(),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_into_iter_values.rs$"],
    snapshots: true,
  },
  {
    name: "fuse-kept-in-place",
    breaks:
      "`fuse()` of a JS iterator stepped till it ends keeps its `iterator_fuse` call, which steps the same",
    file: "src/lower/iterators/in_place.rs",
    find: "        _ if fused && given.len() <= 3 && given.first().is_some_and(is_js_iterator) => given[0].clone(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterator.rs$"],
    snapshots: true,
  },
  {
    name: "consumed-iterator-keeps-hint",
    breaks:
      "an iterator of the crate's stepped till it ends keeps a `size_hint` nothing asks, `$iterator(x, next, false, undefined, undefined, hint)`",
    file: "src/lower/iterators/in_place.rs",
    find: "    if own_steps {\n",
    replace: "    if false && own_steps {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(fused_iterators|double_ended_iterators).rs$",
    ],
    snapshots: true,
  },
  {
    name: "closure-own-assignment-does",
    breaks:
      'a closure that only sets what it declares, `let sign; if (x < 0) { sign = "-"; } ..`, is taken to do something, so its chain is `Array.from(xs.values().map(..))`, not `xs.map(..)` (rd187)',
    file: "src/lower/iterators/in_place.rs",
    find: "                !matches!(&target.kind, js::ExprKind::Var(name) if declared.contains(name)) || acts(e)\n",
    replace: "                true\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "where order can't be told",
    ],
  },
  {
    name: "never-unwinding-predicate-may-throw",
    breaks:
      "a predicate that's one of the crate's functions that never unwinds is judged one that may panic, so an owning `map(tagged)` keeps its `$closing`, not the array's own `ds(..).map(tagged)`",
    file: "src/lower/iterators/in_place.rs",
    find: "        _ => !f.facts.has(js::Known::NeverThrows),",
    replace: "        _ => true,",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
    snapshots: true,
  },
  {
    name: "helper-drop-unseen",
    breaks:
      "a helper stepping all is taken to be given no drop of its items, so a `$iter` of a borrowed `Vec` it's given is made the array, which it takes as its own and drops what's left of as a panic stops it (#423)",
    file: "src/lower/iterators/in_place.rs",
    find: "        .filter(|drop| !matches!(drop.kind, js::ExprKind::Undefined))\n",
    replace: "        .filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "knowing-of-items-kept",
    breaks:
      "a `$knowing` tidy's `stepped_in_place` left an array's iterator inside, a `fuse()` of one dropped to its source, stays `$knowing($iter(xs), ..)` where the array's iterator knows its back and length already",
    file: "src/lower/iterators/in_place.rs",
    find: "        _ if knowing && is_stepped_items(&given[0]) => given[0].clone(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterator.rs$"],
    snapshots: true,
  },
];
