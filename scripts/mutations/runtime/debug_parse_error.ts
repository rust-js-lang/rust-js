// Mutations of src/runtime/debug_parse_error.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-from-int-debug-shows-message",
    breaks: "`{:?}` of a `TryFromIntError` shows its message, not its kind, `TryFromIntError(PosOverflow)`",
    file: "src/runtime/debug_parse_error.js",
    find: "  const kind = $parseErrorKind(message);\n",
    replace: "  const kind = name === \"TryFromIntError\" ? message : $parseErrorKind(message);\n",
    tests: ["test/corpus.test.ts", "-t", "std_errors_boxed"],
  },
];
