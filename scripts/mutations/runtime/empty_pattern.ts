// Mutations of src/runtime/empty_pattern.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "empty-pattern-replacement-read-as-pattern",
    breaks: "`$replace`'s replacement's `$&` is what's matched, not the text Rust has",
    file: "src/runtime/empty_pattern.js",
    find: '  if (pattern !== "") return s.replaceAll(pattern, () => replacement);',
    replace: '  if (pattern !== "") return s.replaceAll(pattern, replacement);',
    tests: ["test/lowering.test.ts", "-t", "a replacement is the text it is"],
  },
];
