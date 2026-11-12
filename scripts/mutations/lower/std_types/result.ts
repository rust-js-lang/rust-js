// Mutations of src/lower/std_types/result.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "question-box-error-unconverted",
    breaks:
      "`?` into a `Box<dyn Error>` returns the error as it is, not as a pair",
    file: "src/lower/std_types/result.rs",
    find: "            Conversion::Dyn(dictionary) => Expr::object(vec![\n",
    replace:
      "            Conversion::Dyn(_) => error,\n            #[allow(unreachable_patterns)]\n            Conversion::Dyn(dictionary) => Expr::object(vec![\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_display"],
  },
  {
    name: "question-from-unanswered",
    breaks: "`?` whose error a `From` converts is refused, its conversion asked of no impl (#1420)",
    file: "src/lower/std_types/result.rs",
    find: "        self.compiled_conversion(method, args, span)\n",
    replace: "        let _ = (method, args, span);\n        Ok(None)\n",
    tests: ["test/crates.test.ts", "-t", "instance_converts_and_transmutes"],
  },
];
