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
    find: "            out.push(StmtKind::Assign(place.clone(), made).at(self.js_span(span)));\n",
    replace: "            out.push(StmtKind::Expr(made).at(self.js_span(span)));\n",
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
  {
    name: "pin-shown-as-pointer-box",
    breaks: "`{}` of a pinned `&mut` to a number shows its box",
    file: "src/lower/std_types/rc.rs",
    find: "            if let Some(pointer) = self.recognition().pinned(ty) {",
    replace: "            if let Some(pointer) = self.recognition().pinned(ty).filter(|_| false) {",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
  },
  {
    name: "make-mut-drop-unknown",
    breaks: "`make_mut` that drops the last `Rc` leaves what it points at undropped",
    file: "src/lower/std_types/rc.rs",
    find: "            let mut given = vec![place.clone(), self.clone_arg(item, span)?];\n            given.extend(self.drop_function(item, span)?);\n",
    replace: "            let given = vec![place.clone(), self.clone_arg(item, span)?];\n",
    tests: ["test/corpus.test.ts", "-t", "rc_clone_sees_owners"],
  },
  {
    name: "unwrap-or-clone-drop-unknown",
    breaks: "`unwrap_or_clone` that drops the last `Rc` leaves what it points at undropped",
    file: "src/lower/std_types/rc.rs",
    find: "                        let mut given = vec![rc, self.clone_arg(item, span)?];\n                        given.extend(self.drop_function(item, span)?);\n",
    replace: "                        let given = vec![rc, self.clone_arg(item, span)?];\n",
    tests: ["test/corpus.test.ts", "-t", "rc_clone_sees_owners"],
  },
];
