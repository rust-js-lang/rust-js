// Mutations of crates/rust-js-program/src/runtime/location.js (ADR 0427, amended by ADR 1184).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "location-split-from-its-start",
    breaks: "a `Location` of a file whose path holds a `:` is split at its first, its file and line wrong",
    file: "crates/rust-js-program/src/runtime/location.js",
    find: "  const line = location.lastIndexOf(\":\", column - 1);\n",
    replace: "  const line = location.indexOf(\":\");\n",
    tests: ["test/binaries.test.ts", "-t", "has a `:`"],
  },
];
