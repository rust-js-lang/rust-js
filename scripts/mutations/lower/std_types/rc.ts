// Mutations of src/lower/std_types/rc.rs (ADR 0320).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rc-get-mut-shared",
    breaks: "`get_mut` gives a `&mut` while others point at it",
    file: "src/lower/std_types/rc.rs",
    find: "                        Expr::cond(only, given, Expr::undefined())\n",
    replace: "                        Expr::cond(Expr::bool(true), given, Expr::undefined())\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
  {
    name: "rc-make-mut-unassigned",
    breaks: "`make_mut`'s new `Rc` isn't its place's",
    file: "src/lower/std_types/rc.rs",
    find: "                out.push(StmtKind::Assign(place.clone(), made).at(self.js_span(span)));\n",
    replace: "                out.push(StmtKind::Expr(made).at(self.js_span(span)));\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
  {
    name: "rc-new-uncounted",
    breaks: "`Rc::new` of a counted one is its value",
    file: "src/lower/std_types/rc.rs",
    find: "            true => Self::new_rc(value),\n",
    replace: "            true => value,\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
];
