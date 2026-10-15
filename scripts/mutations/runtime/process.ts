// Mutations of src/runtime/process.js (ADR 0391).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "process-ungated",
    breaks: "in a browser, `env::args()` throws JS's own `TypeError`, not a panic naming what needed Node",
    file: "src/runtime/process.js",
    find: "  if (process?.argv === undefined) {\n",
    replace: "  if (false) {\n",
    tests: ["test/process.test.ts", "-t", "in a browser"],
  },
  {
    name: "args-with-node",
    breaks: "`env::args()` begins with Node's own path, not the program's",
    file: "src/runtime/process.js",
    find: "argv.slice(1)",
    replace: "argv.slice(0)",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
  },
];
