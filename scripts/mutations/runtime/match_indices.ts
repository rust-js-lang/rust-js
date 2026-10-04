// Mutations of src/runtime/match_indices.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "match-indices-in-units",
    breaks: "`match_indices` gives offsets in UTF-16 units, not UTF-8 bytes",
    file: "src/runtime/match_indices.js",
    find: "    bytes += $byteLen(s.slice(unit, at));",
    replace: "    bytes += at - unit;",
    tests: ["test/corpus.test.ts", "-t", "string_patterns"],
  },
];
