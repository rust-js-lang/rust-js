// Mutations of src/js/applied.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "applied-return-not-broken",
    breaks: "a closure's `return;` in its loop isn't that loop's `break`, so one called before the function's end stays a closure",
    file: "src/js/applied.rs",
    find: "                stmt.kind = StmtKind::Break(None);\n                true\n",
    replace: "                false\n",
    tests: ["test/lowering-cells.test.ts", "-t", "closure called as a statement is its body"],
  },
  {
    name: "applied-tail-returns-refused",
    breaks: "a closure whose `return;` ends the function, called last, stays a closure",
    file: "src/js/applied.rs",
    find: "    if valued || any && !tail && !returns_broken(&mut body) {\n",
    replace: "    if valued || any && !returns_broken(&mut body) {\n",
    tests: ["test/lowering-cells.test.ts", "-t", "closure called as a statement is its body"],
  },
  {
    name: "applied-param-unread",
    breaks: "a closure's parameter, called with what's another name, is left unread, so its body reads a variable that isn't",
    file: "src/js/applied.rs",
    find: "        let whole_body = substituted(&mut body, &|v| (v == param).then(|| arg.clone()));\n",
    replace: "        let whole_body = true;\n",
    tests: ["test/lowering-cells.test.ts", "-t", "closure called as a statement is its body"],
  },
  {
    name: "named-call-never-applied",
    breaks: "a `Drop`'s one drop calls its function, `restoreDrop_drop(_restore)`, where it's its body",
    file: "src/js/applied.rs",
    find: "    let call = StmtKind::Expr(Expr::call(Expr::arrow(params.to_vec(), body.to_vec()), args.clone())).at(stmt.span);\n    block_of(&call, whole, false)\n",
    replace: "    let call = StmtKind::Expr(Expr::call(Expr::arrow(params.to_vec(), body.to_vec()), args.clone())).at(stmt.span);\n    block_of(&call, whole, false).filter(|_| false)\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
];
