// Mutations of src/lower/results.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "question-box-error-unconverted",
    breaks: "`?` into a `Box<dyn Error>` returns the error as it is, not as a pair",
    file: "src/lower/results.rs",
    find: "                (None, Some(dictionary)) => Some(Expr::object(vec![",
    replace: "                (None, Some(_)) => Some(error.clone()),\n                (None, Some(dictionary)) => Some(Expr::object(vec![",
    tests: ["test/corpus.test.ts", "-t", "dyn_display"],
  },
  {
    name: "question-fmt-error-dropped",
    breaks: "`?` of a `write!` that fails drops the `fmt::Error`",
    file: "src/lower/results.rs",
    find: "        if self.is_fmt_result(ty) && self.krate.any_failing {",
    replace: "        if false && self.is_fmt_result(ty) && self.krate.any_failing {",
    tests: ["test/corpus.test.ts","-t","fmt_error_write"],
  },
  {
    name: "question-null-test",
    breaks: "`?` of an Option never falsy tests `item == null`, not `!item`",
    file: "src/lower/results.rs",
    find: "                Some(inner) => self.absent(subject, inner),\n",
    replace: "                Some(_) => Expr::bin(Op::LooseEq, subject, Expr::null()),\n",
    tests: ["test/lowering.test.ts", "-t", "of an Option never falsy tests its truth"],
  },
];
