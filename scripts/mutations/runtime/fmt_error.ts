// Mutations of src/runtime/fmt_error.js (ADRs 0187, 0461).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fmt-result-err-ok",
    breaks: "a `fmt::Result` given where generic code has a `Result` is `Ok` though it failed",
    file: "src/runtime/fmt_error.js",
    find: "    if (error instanceof $WriteError) return { TAG: \"Err\", _0: error };\n",
    replace: "    if (error instanceof $WriteError) return { TAG: \"Ok\" };\n",
    tests: ["test/corpus.test.ts", "-t", "^formatter_sink.rs$"],
  },
  {
    name: "fmt-of-err-dropped",
    breaks: "a generic `Result`'s `Err` read as a `fmt::Result` is `undefined`, `Ok`",
    file: "src/runtime/fmt_error.js",
    find: "  return result.TAG === \"Ok\" ? undefined : result._0;\n",
    replace: "  return undefined;\n",
    tests: ["test/corpus.test.ts", "-t", "^formatter_sink.rs$"],
  },
  {
    name: "write-expect-io-plain",
    breaks: "`expect` of an `io::Result<()>` that's an `Err` panics with `: Error`, not its error's `{:?}`",
    file: "src/runtime/fmt_error.js",
    find: "  throw new Error(message + \": \" + (result instanceof $IoError ? $ioErrorDebug(result) : \"Error\"));\n",
    replace: "  throw new Error(message + \": Error\");\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result_expect.rs$"],
  },
  {
    name: "write-caught-anything",
    breaks: "a function that gives a value's `Result` takes a panic for its `Err`",
    file: "src/runtime/fmt_error.js",
    find: "function $writeCaught(error) {\n  if (error instanceof $WriteError) return error;\n  throw error;\n",
    replace: "function $writeCaught(error) {\n  return error;\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result_panic.rs$"],
  },
];
