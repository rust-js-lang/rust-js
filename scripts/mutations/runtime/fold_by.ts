// Mutations of src/runtime/fold_by.js (#162): a std struct's fold, or a
// value given as it is, not through the crate's own `fold`, or dropping
// what it shouldn't.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rev-advance-by-stepped",
    breaks: "a compiled `Rev`'s `advance_by` of a `Vec`'s items steps each from the back, not its `advance_back_by`'s one slice, front to back (#286)",
    file: "src/runtime/fold_by.js",
    find: "  if (it.items !== undefined && drop !== undefined) return $advanceSlice(it, m, drop, true);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "map-fold-unmapped",
    breaks: "a held map's fold gives its source's items unmapped, so `map(|x| x * 2).peekable().sum()` sums the source's (#264)",
    file: "src/runtime/fold_by.js",
    find: "  return $foldBy(map.iter, init, (acc, x) => g(acc, f(x)), iterator);\n",
    replace: "  return $foldBy(map.iter, init, (acc, x) => g(acc, x), iterator);\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "fuse-fused-try-fold-clears",
    breaks: "a `Fuse` of a `FusedIterator` lets go of its source when a `try_fold` runs it to its end, so `take(1).fuse().any(..)` drops what the take's source holds before `any` answers (#244)",
    file: "src/runtime/fold_by.js",
    find: "  if (!fused && !broke(folded)) {\n",
    replace: "  if (!broke(folded)) {\n",
    tests: ["test/corpus.test.ts", "-t", "fuse_fused_try_fold"],
  },
  {
    name: "fold-by-own-skipped",
    breaks: "a fold through a dictionary steps by `next`, not the crate's own `fold`",
    file: "src/runtime/fold_by.js",
    find: "  if (iterator?.fold !== undefined) return iterator.fold(it, init, f);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "fold-by-undropped",
    breaks: "a fuse's source, stepped to its end by a fold, isn't dropped",
    file: "src/runtime/fold_by.js",
    find: "  drop?.(it);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(own_fold_through_adapters|fused_iterators).rs$"],
  },
  {
    name: "peekable-fold-peeked-lost",
    breaks: "a peekable's fold leaves out what it peeked at",
    file: "src/runtime/fold_by.js",
    find: "    folded = f(folded, $someValue(peeked));\n  }\n  return $foldBy(",
    replace: "  }\n  return $foldBy(",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "peekable-own-fold-unmarked",
    breaks: "a peekable whose source's own fold dropped it is dropped again as it's closed",
    file: "src/runtime/fold_by.js",
    find: "  if (iterator?.fold !== undefined) peekable.ownFold = true;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "fuse-try-fold-by-next",
    breaks: "a compiled `Fuse`'s `try_fold` steps it by `next`, not its source's own `try_fold`, so `step_by(2).fuse().any(..)` asks the crate's own `nth(0)` first (#214)",
    file: "src/runtime/fold_by.js",
    find: "    if (it.tryFold !== undefined) return it.tryFold(init, f, broke);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "fuse-rfold-not-own",
    breaks: "a `fuse()`'s `rfold` steps its source by `next_back`, not the crate's own `rfold` (#224)",
    file: "src/runtime/fold_by.js",
    find: "  if (back?.rfold !== undefined) return back.rfold(iter, init, f);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
];
