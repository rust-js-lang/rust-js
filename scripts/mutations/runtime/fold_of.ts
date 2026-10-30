// Mutations of src/runtime/fold_of.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "collect-dropping-keeps-collected",
    breaks: "a `collect` a panic stops leaves what it had collected undropped (#422)",
    file: "src/runtime/fold_of.js",
    find: "    if (drop !== undefined) $dropItems(items, drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_iteration_drops.rs$"],
  },
  {
    name: "collect-rest-first-ignored",
    breaks: "a `collect` std does otherwise than in place, a panic stopping it, drops what it collected before what's left, where std drops what's left first (#423)",
    file: "src/runtime/fold_of.js",
    find: "  const [first, then] = restFirst ? [rest, collected] : [collected, rest];",
    replace: "  const [first, then] = [collected, rest];",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "fold-to-array-unwinds-nothing",
    breaks: "a compiled std struct's collect, a `zip`'s, a panic stopping it, drops neither what it collected nor what's left (#423)",
    file: "src/runtime/fold_of.js",
    find: "  } catch (error) {\n    $collectUnwound(it, items, unwound ? undefined : drop, restFirst);\n    throw error;\n  }\n}",
    replace: "  } catch (error) {\n    throw error;\n  }\n}",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "fold-dropping-step-unwinds-nothing",
    breaks: "a fold of a JS iterator with no fold of its own, a `flat_map`'s, whose step panics leaves the accumulator undropped (#423)",
    file: "src/runtime/fold_of.js",
    find: "      try {\n        unwind(folded);\n      } finally {\n        it.return?.();\n      }\n",
    replace: "      it.return?.();\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_in_own_next_drops.rs$"],
  },
  {
    name: "fold-dropping-given-to-own-fold-lost",
    breaks: "a fold of an iterator with its own fold gives it no `unwind` (#423)",
    file: "src/runtime/fold_of.js",
    find: "  if (typeof it.fold === \"function\") return it.fold(init, f, unwind);",
    replace: "  if (typeof it.fold === \"function\") return it.fold(init, f);",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "fold-to-array-collected-last",
    breaks: "a collect in place of std's struct, a `map`'s, a stage panicking, drops what's left before what it collected, where std's `InPlaceDrop` drops what it collected first (#423)",
    file: "src/runtime/fold_of.js",
    find: "      unwind,\n    );\n",
    replace: "      undefined,\n    );\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^panic_mid_iteration_drops with"],
  },
  {
    name: "fold-of-unwind-lost",
    breaks: "a dictionary's `fold` of std's struct, `$foldOf`, gives it no `unwind`: `last` of a `flat_map` drops what's left before the one it kept (#423)",
    file: "src/runtime/fold_of.js",
    find: "function $foldOf(it, init, f, unwind) {\n  return it.fold(init, f, unwind);\n",
    replace: "function $foldOf(it, init, f, unwind) {\n  return it.fold(init, f);\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^panic_in_own_next_drops with"],
  },
];
