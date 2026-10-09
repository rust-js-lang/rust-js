// Mutations of src/lower/std_types/slice.rs (ADR 0324).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "is-sorted-nan",
    breaks: "`is_sorted` of a NaN says it's sorted",
    file: "src/lower/std_types/slice.rs",
    find: "                            Some(_) => Expr::bin(Op::Le, a, b),\n",
    replace: "                            Some(_) => Expr::unary(crate::js::UnaryOp::Not, Expr::bin(Op::Gt, a, b)),\n",
    tests: ["test/corpus.test.ts", "-t", "slice_methods"],
  },
  {
    name: "retain-mut-no-handles",
    breaks: "`retain_mut` of numbers gives each as a value, so what `f` writes is lost",
    file: "src/lower/std_types/slice.rs",
    find: "                        let handles = self.is_boxable(item());\n",
    replace: "                        let handles = false;\n",
    tests: ["test/corpus.test.ts", "-t", "cell_deque_char_methods"],
  },
];
