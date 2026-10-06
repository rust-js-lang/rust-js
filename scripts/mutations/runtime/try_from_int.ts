// Mutations of src/runtime/try_from_int.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-from-int-one-message",
    breaks: "a `TryFromIntError` below its target's range says it's too large, where 1.99 says too small",
    file: "src/runtime/try_from_int.js",
    find: '  if (x < lo) return { TAG: "Err", _0: "number too small to fit in target type" };',
    replace: '  if (x < lo) return { TAG: "Err", _0: "number too large to fit in target type" };',
    tests: ["test/compiler.test.ts", "-t", "wide.report"],
  },
];
