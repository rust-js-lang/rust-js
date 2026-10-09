// Mutations of src/runtime/eq.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "eq-counts-keys",
    breaks: "two structs alike but for a `None` one left out aren't equal",
    file: "src/runtime/eq.js",
    find: "  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);\n  return [...keys].every((k) => $eq(a[k], b[k]));",
    replace: "  const keys = Object.keys(a);\n  return keys.length === Object.keys(b).length && keys.every((k) => $eq(a[k], b[k]));",
    tests: ["test/lowering.test.ts", "-t", "literal None is left out"],
  },
];
