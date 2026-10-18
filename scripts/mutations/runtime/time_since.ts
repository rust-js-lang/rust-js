// Mutations of src/runtime/time_since.js (ADR 0447).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "time-since-never-err",
    breaks: "`duration_since` of a later `SystemTime` is `Ok`, not a `SystemTimeError`",
    file: "src/runtime/time_since.js",
    find: "a >= b ? { TAG: \"Ok\", _0: a - b }",
    replace: "true ? { TAG: \"Ok\", _0: a - b }",
    tests: ["test/corpus.test.ts", "-t", "^clock.rs$"],
  },
];
