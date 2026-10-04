// Mutations of src/runtime/str_pop.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pop-half-a-pair",
    breaks: "`s.pop()` of an emoji takes half its surrogate pair",
    file: "src/runtime/str_pop.js",
    find: "  const size = low >= 0xdc00 && low <= 0xdfff && high >= 0xd800 && high <= 0xdbff ? 2 : 1;",
    replace: "  const size = 1;",
    tests: ["test/corpus.test.ts", "-t", "string_editing"],
  },
];
