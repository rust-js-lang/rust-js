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
    name: "question-truth-test",
    breaks: "`?` of an Option of an object returns where it's falsy, `if (!item)`, not where it's `null` or `undefined`",
    file: "src/lower/std_types/result.rs",
    find: "            Some(_) => super::option::absent(subject),\n",
    replace: "            Some(_) => Expr::unary(js::UnaryOp::Not, subject),\n",
    tests: ["test/lowering-options.test.ts", "-t", "of an Option tests it against null"],
  },
];
