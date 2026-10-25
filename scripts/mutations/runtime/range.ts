// Mutations of src/runtime/range.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-collect-room-of-its-end",
    breaks: "a range's `collect()` asks room for its end, not its length",
    file: "src/runtime/range.js",
    find: "  if (size !== undefined) $roomFor(length, size);\n",
    replace: "  if (size !== undefined) $roomFor(end, size);\n",
    tests: ["test/runtime-package.test.ts", "-t", "ends where wasm32"],
  },
];
