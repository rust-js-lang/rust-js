// Mutations of src/runtime/iterator.js (ADRs 0061, 0421).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "iterator-of-forgotten",
    breaks: "a JS iterator of the crate's iterator doesn't say what it's of, which its drop drops",
    file: "src/runtime/iterator.js",
    find: "  it.of = iterator;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_projection_drops.rs$"],
  },
  {
    name: "iterator-back-forgotten",
    breaks: "a range or the crate's iterator given as a double-ended one has no `back()`",
    file: "src/runtime/iterator.js",
    find: "    it.back = () => step(nextBack(iterator));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "knowing-installs-nothing",
    breaks: "a use site's back and length aren't given the `$iterator` a held adapter steps",
    file: "src/runtime/iterator.js",
    find: "    if (back !== undefined) it.back = () => step(back(iterator));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_enumerate.rs$"],
  },
  {
    name: "iterator-hint-ignored",
    breaks: "a JS iterator given the crate's `size_hint` doesn't answer `$sizeHint` with it",
    file: "src/runtime/iterator.js",
    find: "    it.hint = () => hint(iterator);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_size_hints.rs$"],
  },
  {
    name: "iterator-own-fold-unused",
    breaks: "the crate's iterator's own `fold` isn't called by what std routes through it",
    file: "src/runtime/iterator.js",
    find: "    it.fold = (init, f) => fold(iterator, init, f);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "iterator-own-nth-unused",
    breaks: "the crate's iterator's own `nth` isn't called by `skip`",
    file: "src/runtime/iterator.js",
    find: "  if (nth !== undefined) it.nth = (k) => step(nth(iterator, k));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "given-fold-unpassed",
    breaks: "a value given as it is is stepped by `next`, not its dictionary's own `fold`",
    file: "src/runtime/iterator.js",
    find: "iterator.sizeHint, iterator.fold, iterator.nth);",
    replace: "iterator.sizeHint, undefined, iterator.nth);",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },

];
