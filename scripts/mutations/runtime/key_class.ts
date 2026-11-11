// Mutations of src/runtime/key_class.js (#1346).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "key-class-hashed-part-unhashed",
    breaks: "a tuple with a part found by its own `Hash` and `Eq` is found by value, so two parts its `Eq` finds equal are two keys",
    file: "src/runtime/key_class.js",
    find: '  if (finds.includes("hashed")) return "hashed";\n',
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "key-class-ways"],
  },
  {
    name: "key-class-parts-part-by-value",
    breaks: "a tuple with a part kept under what `parts` keeps it, a path's or one holding a type equal only to itself, is found by its value",
    file: "src/runtime/key_class.js",
    find: '  if (finds.includes("parts")) return "parts";\n',
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "key-class-ways"],
  },
  {
    name: "key-class-own-order-std",
    breaks: "a B-tree keyed by a tuple with a part of its own `Ord` is std's `$btreeMap`, whose search isn't that `Ord`'s",
    file: "src/runtime/key_class.js",
    find: '  if (orders.includes("own")) return "own";\n',
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "key-class-ways"],
  },
  {
    name: "key-class-seen-order-unseen",
    breaks: "a B-tree keyed by a tuple with a part whose order can be seen isn't std's nodes",
    file: "src/runtime/key_class.js",
    find: '  if (orders.includes("tree")) return "tree";\n',
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "key-class-ways"],
  },
  {
    name: "key-class-cmp-first-part",
    breaks: "a tuple key's order is its first part's alone, so keys equal there are one key",
    file: "src/runtime/key_class.js",
    find: "      if (order !== 0) return order;\n",
    replace: "      return order;\n",
    tests: ["test/corpus.test.ts", "-t", "^map_keyed_by_a_tuple_of_a_type_parameter.rs$"],
  },
];
