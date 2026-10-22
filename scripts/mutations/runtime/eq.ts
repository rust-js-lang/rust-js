// Mutations of src/runtime/eq.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "eq-counts-keys",
    breaks: "two structs alike but for a `None` one left out aren't equal",
    file: "src/runtime/eq.js",
    find: "  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);\n  return [...keys].every((k) => $eq(a[k], b[k]));",
    replace: "  const keys = Object.keys(a);\n  return keys.length === Object.keys(b).length && keys.every((k) => $eq(a[k], b[k]));",
    tests: ["test/lowering-options.test.ts", "-t", "literal None is left out"],
  },
  {
    name: "eq-of-borrowed-cell",
    breaks: "`==` of a mutably borrowed `RefCell` runs on",
    file: "src/runtime/eq.js",
    find: "  if (a.borrows < 0 || b.borrows < 0) throw new Error(\"RefCell already mutably borrowed\");\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "refcell_eq_while_mut"],
  },
  {
    name: "map-eq-sizes-unchecked",
    breaks: "a map with an entry more than another's is equal to it",
    file: "src/runtime/eq.js",
    find: "function $mapEq(a, b, eq = (x, y) => x === y) {\n  if (a.size !== b.size) return false;\n",
    replace: "function $mapEq(a, b, eq = (x, y) => x === y) {\n",
    tests: ["test/corpus.test.ts", "-t", "^map_eq.rs$"],
  },
  {
    name: "eq-of-held-map-by-keys",
    breaks: "a struct's maps compare by their object keys, none: any two are equal",
    file: "src/runtime/eq.js",
    find: "  if (a instanceof Map) return b instanceof Map && $mapEq(a, b, $eq);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_eq.rs$"],
  },
  {
    name: "eq-of-itself",
    breaks: "a value equals itself, the same JS object, without comparing its fields, so `p == p` of one that holds a NaN is true",
    file: "src/runtime/eq.js",
    find: "  if ((a === b && typeof a !== \"object\") || (a == null && b == null)) {\n",
    replace: "  if (a === b || (a == null && b == null)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nan_self_eq.rs$"],
  },
];
