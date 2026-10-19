// Mutations of src/lower/analysis/fmt_failures.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fmt-error-never-made",
    breaks: "a writer that returns `Err(fmt::Error)` is taken as one that never fails",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                {\n                    failing.insert(caller);\n                }",
    replace: "                {}",
    tests: ["test/corpus.test.ts","-t","fmt_error_write"],
  },
  {
    name: "fmt-error-not-passed-on",
    breaks: "a writer that calls one that fails, chrono's, is taken as one that never fails",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "            if fails && failing.insert(*caller) {",
    replace: "            if false && fails && failing.insert(*caller) {",
    tests: ["test/corpus.test.ts","-t","fmt_error_write"],
  },
  {
    name: "fmt-error-formatted-unseen",
    breaks: "`{}` of a value whose `Display` fails isn't one that may fail",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                            if let Some(callee) = resolve(tcx, typing_env, fmt, tcx.mk_args(&[part.into()])) {",
    replace: "                            if let Some(callee) = None::<Callee> {",
    tests: ["test/corpus.test.ts","-t","fmt_error_write"],
  },
  {
    name: "generic-fmt-result-never-fails",
    breaks: "a writer that calls generic code whose `Result<(), S::Error>` is a `fmt::Result` isn't known to fail: its error is thrown out of `write!` to a `String`",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                        calls.push((caller, Callee::Unknown));\n                        continue;\n",
    replace: "                        continue;\n",
    tests: ["test/corpus.test.ts", "-t", "^formatter_sink.rs$"],
  },
  {
    name: "value-result-tried-never-fails",
    breaks: "a function whose only `?` is of a value's `Result` of an `io::Error` isn't known to fail: its error escapes uncaught",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                        && of_write_error(tcx, tried)\n                    {\n                        failing.insert(caller);\n",
    replace: "                        && of_write_error(tcx, tried)\n                    {\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "own-writer-write-all-known",
    breaks: "a function that only gives std's `write_all` of the crate's writer isn't known to fail",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                        calls.push((caller, if own_writer { Callee::Unknown } else { callee }));\n",
    replace: "                        let _ = own_writer;\n                        calls.push((caller, callee));\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
];
