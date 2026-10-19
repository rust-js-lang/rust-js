// Mutations of src/runtime/fmt_error.js (ADRs 0187, 0461).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fmt-result-err-ok",
    breaks: "a `fmt::Result` given where generic code has a `Result` is `Ok` though it failed",
    file: "src/runtime/fmt_error.js",
    find: "    if (error instanceof $FmtError) return { TAG: \"Err\", _0: error };\n",
    replace: "    if (error instanceof $FmtError) return { TAG: \"Ok\" };\n",
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
];
