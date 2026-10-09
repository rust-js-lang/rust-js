// Mutations of src/lower/std_types/once.rs (ADR 0317).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "once-get-mut-copy",
    breaks: "`get_mut` of a number gives the number, so a write through it is lost",
    file: "src/lower/std_types/once.rs",
    find: "            OnceOp::GetMut if self.is_boxable(item) => {\n",
    replace: "            OnceOp::GetMut if false && self.is_boxable(item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
  {
    name: "once-take-keeps",
    breaks: "`take` leaves what it took in the cell",
    file: "src/lower/std_types/once.rs",
    find: '                Expr::call(Expr::var("$cellReplace"), vec![arg(), Expr::undefined()])\n',
    replace: '                Expr::member(arg(), "value")\n',
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
];
