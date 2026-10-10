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
  {
    name: "clone-from-slice-own-unused",
    breaks: "`clone_from_slice` clones each, where the items' own `clone_from` should run",
    file: "src/lower/std_types/slice.rs",
    find: "                    SliceOp::CloneFromSlice if self.recognition().own_clone_from(item()) => {",
    replace: "                    SliceOp::CloneFromSlice if false => {",
    tests: ["test/corpus.test.ts", "-t", "clone_from_slice_own"],
  },
  {
    name: "clone-from-slice-unboxed",
    breaks: "`clone_from_slice`'s own `clone_from` of a fieldless enum writes a copy",
    file: "src/lower/std_types/slice.rs",
    find: "                        let target = match self.is_boxable(item()) {\n                            true => Expr::handle(place),",
    replace: "                        let target = match self.is_boxable(item()) {\n                            true => place,",
    tests: ["test/corpus.test.ts", "-t", "clone_from_slice_own"],
  },
  {
    name: "clone-from-slice-nested-allowed",
    breaks: "`clone_from_slice` of `Vec`s of a type with its own `clone_from` clones each",
    file: "src/lower/std_types/slice.rs",
    find: "                    SliceOp::CloneFromSlice if self.recognition().reaches_clone_from(item()) => {",
    replace: "                    SliceOp::CloneFromSlice if false => {",
    tests: ["test/corpus.test.ts", "-t", "clone_from_slice_nested"],
  },
];
