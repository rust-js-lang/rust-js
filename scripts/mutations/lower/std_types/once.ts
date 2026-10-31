// Mutations of src/lower/std_types/once.rs (ADRs 0317, 0847).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "once-get-mut-copy",
    breaks:
      "`get_mut` of a number gives the number, so a write through it is lost",
    file: "src/lower/std_types/once.rs",
    find: "            OnceOp::GetMut if self.is_boxable(item) => {\n",
    replace:
      "            OnceOp::GetMut if false && self.is_boxable(item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
  {
    name: "once-take-keeps",
    breaks: "`take` leaves what it took in the cell",
    file: "src/lower/std_types/once.rs",
    find: "                out.push(StmtKind::Assign(place, Expr::undefined()).at(self.js_span(span)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
  {
    name: "once-set-unchecked",
    breaks:
      "a `set` of a cell that may be set already assigns it as one known empty, so a second `set` replaces what the first set, and is `Ok`",
    file: "src/lower/std_types/once.rs",
    find: "        if empty {\n            let stored = self.once_stored(value, item);\n",
    replace:
      "        if true {\n            let stored = self.once_stored(value, item);\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-set-unboxed",
    breaks:
      "a `()` or a `None` set is stored as it is, `undefined`, so the cell looks unset",
    file: "src/lower/std_types/once.rs",
    find: "            true => self.some(value),\n",
    replace: "            true => value,\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_fields.rs$"],
  },
  {
    name: "once-set-value-made-when-unset",
    breaks:
      'what a `set` whose `Result` nothing reads is given is made only where the cell is empty, so `made("two")` never prints',
    file: "src/lower/std_types/once.rs",
    find: "        let value = match value.reads_same() || (!read && !value.has_effects()) {\n",
    replace: "        let value = match value.reads_same() || !read {\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-set-result-tested-held",
    breaks:
      "a `set` whose `Result` nothing reads still holds its test, `const unset = node.cache === undefined; if (unset) ..`, where it's `if (node.cache === undefined) ..`",
    file: "src/lower/std_types/once.rs",
    find: '        let unset = match read {\n            true => self.spill("unset", unset, out),\n',
    replace:
      '        let unset = match true {\n            true => self.spill("unset", unset, out),\n',
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
    snapshots: true,
  },
];
