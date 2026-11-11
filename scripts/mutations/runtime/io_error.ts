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
  {
    name: "os-error-shown-as-its-kind",
    breaks: "an error of the OS's shows its kind's words, `entity not found`, where std's shows its message and number, `No such file or directory (os error 2)` (#1156)",
    file: "src/runtime/io_error.js",
    find: "  if (e.code !== undefined) return `${e.message} (os error ${e.code})`;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fs_file.rs$"],
  },
  {
    name: "io-error-dyn-no-type-id",
    breaks: "an `io::Error` made a `dyn Error` has no `typeId`, so a downcast of it to an `io::Error` finds `None` where Rust finds it",
    file: "src/runtime/io_error.js",
    find: "    source: (e) => e.error?.impl.source(e.error.value),\n    typeId: () => type,\n",
    replace: "    source: (e) => e.error?.impl.source(e.error.value),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_error_downcast.rs$"],
  },
];
