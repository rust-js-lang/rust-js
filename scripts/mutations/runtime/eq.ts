// Mutations of src/runtime/eq.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "eq-counts-keys",
    breaks: "two structs alike but for a `None` one left out aren't equal",
    file: "src/runtime/eq.js",
    find: "  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);\n  return Array.from(keys).every((k) => $eq(a[k], b[k], true));",
    replace: "  const keys = Object.keys(a);\n  return keys.length === Object.keys(b).length && keys.every((k) => $eq(a[k], b[k], true));",
    tests: ["test/lowering-options.test.ts", "-t", "literal None is left out"],
  },
  {
    name: "eq-of-borrowed-cell",
    breaks: "`==` of a mutably borrowed `RefCell` runs on",
    file: "src/runtime/eq.js",
    find: "  if (cells && (a.borrows < 0 || b.borrows < 0)) throw new Error(\"RefCell already mutably borrowed\");\n",
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
    find: "  if (a instanceof Map) return b instanceof Map && $mapEq(a, b, (x, y) => $eq(x, y, true));\n",
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
  {
    name: "eq-of-values-borrowed",
    breaks: "what a `RefCell` of an object holds, compared through its guard, is borrowed to compare, and found borrowed by that guard (ADR 0863)",
    file: "src/runtime/eq.js",
    find: "  if (cells && (a.borrows < 0 || b.borrows < 0)) throw new Error(\"RefCell already mutably borrowed\");\n",
    replace: "  if (a.borrows < 0 || b.borrows < 0) throw new Error(\"RefCell already mutably borrowed\");\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_eq_through_guard.rs$"],
  },
  {
    name: "eq-of-parts-unborrowed",
    breaks: "a `RefCell` part of what's compared, mutably borrowed, is compared without a borrow, where Rust panics",
    file: "src/runtime/eq.js",
    find: "    return Array.isArray(b) && a.length === b.length && a.every((x, i) => $eq(x, b[i], true));\n",
    replace: "    return Array.isArray(b) && a.length === b.length && a.every((x, i) => $eq(x, b[i]));\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_eq_through_guard.rs$"],
  },
];
