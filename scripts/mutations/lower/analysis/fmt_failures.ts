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
];
