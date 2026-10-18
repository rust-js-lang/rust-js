// Mutations of src/runtime/system_now.js (ADR 0447).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "system-now-microseconds",
    breaks: "`SystemTime::now()` counts JS's milliseconds as microseconds, a time in 1970",
    file: "src/runtime/system_now.js",
    find: "BigInt(Date.now()) * 1000000n",
    replace: "BigInt(Date.now()) * 1000n",
    tests: ["test/corpus.test.ts", "-t", "^clock.rs$"],
  },
];
