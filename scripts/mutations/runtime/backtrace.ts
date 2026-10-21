// Mutations of src/runtime/backtrace.js (ADR 0568).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "backtrace-status-captured",
    breaks: "every backtrace's status is `Captured`, a disabled one's too",
    file: "src/runtime/backtrace.js",
    find: "  return backtrace === undefined ? \"Disabled\" : \"Captured\";",
    replace: "  return \"Captured\";",
    tests: ["test/corpus.test.ts", "-t", "^backtrace.rs$"],
  },
  {
    name: "backtrace-shown-raw",
    breaks: "a disabled backtrace shows as nothing, not `disabled backtrace`",
    file: "src/runtime/backtrace.js",
    find: "  if (backtrace === undefined) return debug ? \"<disabled>\" : \"disabled backtrace\";",
    replace: "  if (backtrace === undefined) return \"\";",
    tests: ["test/corpus.test.ts", "-t", "^backtrace.rs$"],
  },
];
