// Mutations of src/runtime/io_error.js (ADR 0462).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "io-error-display-kind",
    breaks: "`{}` of an `io::Error` made of a message shows its kind's words, not the message",
    file: "src/runtime/io_error.js",
    find: "  if (e.error !== undefined) return e.error.impl.Display().fmt(e.error.value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_error_values.rs$"],
  },
  {
    name: "io-error-debug-custom-as-kind",
    breaks: "`{:?}` of an `io::Error` made of a message is `Kind(..)`",
    file: "src/runtime/io_error.js",
    find: "  if (e.error === undefined) return options?.alternate",
    replace: "  if (true) return options?.alternate",
    tests: ["test/corpus.test.ts", "-t", "^io_error_values.rs$"],
  },
  {
    name: "io-error-debug-never-pretty",
    breaks: "`{:#?}` of an `io::Error` is on one line",
    file: "src/runtime/io_error.js",
    find: "  return options?.alternate\n    ? $pretty(",
    replace: "  return false\n    ? $pretty(",
    tests: ["test/corpus.test.ts", "-t", "^io_error_values.rs$"],
  },
  {
    name: "io-error-source-itself",
    breaks: "an `io::Error`'s `source()` is the error it holds, not that one's source",
    file: "src/runtime/io_error.js",
    find: "    source: (e) => e.error?.impl.source(e.error.value),\n",
    replace: "    source: (e) => e.error,\n",
    tests: ["test/corpus.test.ts", "-t", "^io_error_values.rs$"],
  },
  {
    name: "io-result-debug-err-plain",
    breaks: "`{:?}` of an `io::Result<()>` that's an `Err` shows `Err(Error)`",
    file: "src/runtime/io_error.js",
    find: "  const error = $ioErrorDebug(result, options);\n",
    replace: "  const error = \"Error\";\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
];
