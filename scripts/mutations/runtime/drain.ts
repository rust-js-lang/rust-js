// Mutations of crates/rust-js-program/src/runtime/drain.js (ADR 0063).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drain-unchecked",
    breaks: "`drain(a..)` with `a` past the end takes nothing, not a panic",
    file: "crates/rust-js-program/src/runtime/drain.js",
    find: "  $checkRange(start, end, items.length, raisedAt, inclusive);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","drain_from_past_len"],
  },
];
