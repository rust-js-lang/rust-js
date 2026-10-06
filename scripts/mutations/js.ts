// Mutations of src/js.rs.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "index-of-one-item-array-kept",
    breaks: "`[x][0]` is written as it is, not `x`",
    file: "src/js.rs",
    find: "            && let [item] = &items[..]\n",
    replace: "            && let [item, ..] = &items[..]\n            && false\n",
    tests: ["test/corpus.test.ts", "-t", "wrapping_type"],
    snapshots: true,
  },
  {
    name: "closure-reads-unseen",
    breaks: "a helper only a closure calls isn't imported, and the module throws when the closure runs",
    file: "src/js.rs",
    find: "            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => visit_stmts(body, read),\n",
    replace: "            ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "char_from_code"],
  },
  {
    name: "strings-read-as-vars",
    breaks: "a string that spells a helper's name imports it, as scanning the printed text did",
    file: "src/js.rs",
    find: "            ExprKind::Var(name) => read(name),\n",
    replace: "            ExprKind::Var(name) | ExprKind::Str(name) => read(name),\n",
    tests: ["test/link.test.ts", "-t", "spells a helper"],
  },
];
