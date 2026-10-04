// Mutations of src/runtime/rsplit.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rsplit-overlapping",
    breaks: "`\"aaa\".rsplit(\"aa\")` takes a match overlapping the one after it",
    file: "src/runtime/rsplit.js",
    find: "      at = search - p.length < 0 ? -1 : s.lastIndexOf(p, search - p.length);",
    replace: "      at = s.lastIndexOf(p, search - 1);",
    tests: ["test/corpus.test.ts", "-t", "string_patterns"],
  },
];
