// Mutations of src/lower/std_types/result.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "question-box-error-unconverted",
    breaks: "`?` into a `Box<dyn Error>` returns the error as it is, not as a pair",
    file: "src/lower/std_types/result.rs",
    find: "            Conversion::Dyn(dictionary) => Expr::object(vec![\n",
    replace: "            Conversion::Dyn(_) => error,\n            #[allow(unreachable_patterns)]\n            Conversion::Dyn(dictionary) => Expr::object(vec![\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_display"],
  },
  {
    name: "question-fmt-error-dropped",
    breaks: "`?` of a `write!` that fails drops the `fmt::Error`",
    file: "src/lower/std_types/result.rs",
    find: "        if self.is_fmt_result(ty) && self.krate.any_failing {",
    replace: "        if false && self.is_fmt_result(ty) && self.krate.any_failing {",
    tests: ["test/corpus.test.ts","-t","fmt_error_write"],
  },
  {
    name: "question-null-test",
    breaks: "`?` of an Option never falsy tests `item == null`, not `!item`",
    file: "src/lower/std_types/result.rs",
    find: "            Some(inner) => self.absent(subject, inner),\n",
    replace: "            Some(_) => Expr::bin(Op::LooseEq, subject, Expr::null()),\n",
    tests: ["test/lowering.test.ts", "-t", "of an Option never falsy tests its truth"],
  },
];
