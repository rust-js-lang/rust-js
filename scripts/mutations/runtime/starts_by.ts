// Mutations of src/runtime/starts_by.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "ends-by-half-emoji",
    breaks: "`ends_with` tries the last UTF-16 unit, half an emoji, not the last `char`",
    file: "src/runtime/starts_by.js",
    find: "  const at = end ? s.length - (/[\\udc00-\\udfff]$/.test(s) && s.length > 1 ? 2 : 1) : 0;",
    replace: "  const at = end ? s.length - 1 : 0;",
    tests: ["test/corpus.test.ts", "-t", "text_predicates"],
  },
];
