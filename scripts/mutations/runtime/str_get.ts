// Mutations of src/runtime/str_get.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "str-get-inside-char",
    breaks: "`get` of a range inside a `char` is a piece of it, not `None`",
    file: "src/runtime/str_get.js",
    find: "  return from === undefined || to === undefined ? undefined : s.slice(from, to);",
    replace: "  return s.slice(from ?? 0, to ?? s.length);",
    tests: ["test/corpus.test.ts", "-t", "text_predicates"],
  },
];
