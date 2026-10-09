// Mutations of src/lower/std_types/cell.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "atomic-fetch-new-value",
    breaks: "an atomic's `fetch_add` and the like give the new value, not the old",
    file: "src/lower/std_types/cell.rs",
    find: "                out.push(StmtKind::Assign(slot, next).at(js_span));\n                previous\n",
    replace: "                out.push(StmtKind::Assign(slot.clone(), next).at(js_span));\n                slot\n",
    tests: ["test/corpus.test.ts", "-t", "atomics"],
  },
  {
    name: "drop-of-nothing-kept",
    breaks: "`drop(guard)` keeps an unused `const value = ..`: right, but not the JS a person writes",
    file: "src/lower/std_types/cell.rs",
    find: "                if !self.has_drops(ty) {",
    replace: "                if false {",
    tests: ["test/corpus.test.ts", "-t", "^locks"],
    snapshots: true,
  },
  {
    name: "cell-take-undefined",
    breaks: "`Cell::take()` of a number leaves `undefined`, not its default, `0`",
    file: "src/lower/std_types/cell.rs",
    find: "                    Std::CellTake => (cell, self.default_value(item, span)?),",
    replace: "                    Std::CellTake => (cell, Expr::undefined()),",
    tests: ["test/corpus.test.ts", "-t", "collection_methods"],
  },
  {
    name: "replace-with-unboxed",
    breaks: "`replace_with` of a `String` gives its closure the string, which reads it as a box's `value`",
    file: "src/lower/std_types/cell.rs",
    find: "                        let given = if self.is_object(item) {",
    replace: "                        let given = if true || self.is_object(item) {",
    tests: ["test/corpus.test.ts", "-t", "collection_methods"],
  },
  {
    name: "local-with-closure-called",
    breaks: "`START.with(|s| s.get())` calls the closure it's given, `((s) => s.value)(START)`, not `START.value` in place",
    file: "src/lower/std_types/cell.rs",
    find: "                crate::lower::calls::apply_in(f, vec![key], out)\n",
    replace: "                Expr::call(f, vec![key])\n",
    tests: ["test/compiler.test.ts", "-t", "thread-locals are module variables"],
  },
  {
    name: "with-statement-called",
    breaks: "a thread-local's `with` of a one-statement closure calls it in place, `((it) => { .. })(LOGGER)`",
    file: "src/lower/std_types/cell.rs",
    find: "                crate::lower::calls::apply_in(f, vec![key], out)\n",
    replace: "                apply(f, vec![key])\n",
    tests: ["test/lowering.test.ts", "-t", "with of a one-statement closure"],
  },
  {
    name: "cell-get-mut-copy",
    breaks: "a number `Cell`'s `get_mut` gives the number, so a write through it is lost",
    file: "src/lower/std_types/cell.rs",
    find: "                true => arg(),\n                false => Expr::member(arg(), \"value\"),\n",
    replace: "                true => Expr::member(arg(), \"value\"),\n                false => Expr::member(arg(), \"value\"),\n",
    tests: ["test/corpus.test.ts", "-t", "cell_deque_char_methods"],
  },
];
