// Mutations of src/runtime/next_if.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "next-if-lazy-keeps-peeked",
    breaks: "`next_if` of a lazy peekable that takes the item leaves it to be given again",
    file: "src/runtime/next_if.js",
    find: "  if (item !== undefined && $unwinds(() => f(item), drop && (() => drop(item)))) return item;\n",
    replace: "  if (item !== undefined && $unwinds(() => f(item), drop && (() => drop(item)))) {\n    peekable.peeked = $some(item);\n    return item;\n  }\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_lazy.rs$"],
  },
  {
    name: "next-if-some-test-boxed",
    breaks: "`next_if`'s test of items that may look like `None` is given the boxed item, so `x.is_none()` of a `None` is false",
    file: "src/runtime/next_if.js",
    find: "f($someValue(item))",
    replace: "f(item)",
    tests: ["test/corpus.test.ts", "-t", "^boxed_option_results.rs$"],
  },
  {
    name: "next-if-item-leaked-as-test-panics",
    breaks: "`next_if` whose test panics leaks the item it asked of, never dropped, where std's drops it as the panic unwinds (#1648)",
    file: "src/runtime/next_if.js",
    find: "  if (item !== undefined && $unwinds(() => f(item), drop && (() => drop(item)))) return item;\n",
    replace: "  if (item !== undefined && f(item)) return item;\n",
    tests: ["test/corpus.test.ts", "-t", "^next_if_drops_its_item_as_its_test_panics.rs$"],
  },
  {
    name: "next-if-some-item-leaked-as-test-panics",
    breaks: "`next_if` of items that may look like `None` whose test panics leaks the item, never dropped (#1648)",
    file: "src/runtime/next_if.js",
    find: "  if (item !== undefined && $unwinds(() => f($someValue(item)), drop && (() => drop($someValue(item))))) return item;\n",
    replace: "  if (item !== undefined && f($someValue(item))) return item;\n",
    tests: ["test/corpus.test.ts", "-t", "^next_if_drops_its_item_as_its_test_panics.rs$"],
  },
];
