// Mutations of src/runtime/time_debug.js (ADR 0447).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "time-debug-before-epoch-unborrowed",
    breaks:
      "a time before the epoch shows its nanoseconds below zero, `tv_sec: -1, tv_nsec: -500000000`, not unix's `tv_sec: -2, tv_nsec: 500000000`",
    file: "src/runtime/time_debug.js",
    find: "  if (past < 0n) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^time_debug_as_unix\\.rs$"],
  },
  {
    name: "time-debug-hex-unread",
    breaks: "`{:x?}` of a time shows its fields in decimal, where std's `debug_struct` gives each integer the hex flag",
    file: "src/runtime/time_debug.js",
    find: "    $formatted($debugInt(String(n), BigInt.asUintN(bits, n).toString(16), options), options, true);\n",
    replace: "    $formatted(String(n), options, true);\n",
    tests: ["test/corpus.test.ts", "-t", "^time_debug_as_unix\\.rs$"],
  },
];
