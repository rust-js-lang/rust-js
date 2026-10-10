// Mutations of src/runtime/strip_circumfix.js (ADR 0334).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "strip-circumfix-prefix-only",
    breaks: "`strip_circumfix` keeps the suffix",
    file: "src/runtime/strip_circumfix.js",
    find: "  return rest === undefined ? undefined : $stripSuffix(rest, suffix);",
    replace: "  return rest;",
    tests: ["test/corpus.test.ts","-t","str_circumfix"],
  },
];
