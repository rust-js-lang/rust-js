// Mutations of src/runtime/fold_by.js (#162): a std struct's fold, or a
// value given as it is, not through the crate's own `fold`, or dropping
// what it shouldn't.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
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
    find: "    folded = f(folded, $someValue(peeked));\n",
    replace: "",
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
];
