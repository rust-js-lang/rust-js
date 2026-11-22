// Mutations of crates/rust-js-program/src/runtime/main.js (ADR 1184).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "binary-hook-never-set",
    breaks: "a binary's panic writes no hook's text, as a library's doesn't",
    file: "crates/rust-js-program/src/runtime/main.js",
    find: "  if (!browser) $onPanic = $hook;\n",
    replace: "",
    tests: ["test/binaries.test.ts", "-t", "where it's raised"],
  },
  {
    name: "backtrace-note-each-panic",
    breaks: "std's backtrace note is written after each panic, not the process's first alone",
    file: "crates/rust-js-program/src/runtime/main.js",
    find: "  $noted = true;\n",
    replace: "",
    tests: ["test/binaries.test.ts", "-t", "where it's raised"],
  },
];
