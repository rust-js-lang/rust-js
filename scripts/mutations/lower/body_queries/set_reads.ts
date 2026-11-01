// Mutations of src/lower/body_queries/set_reads.rs (ADR 0093): a cell read
// back right after it's set to a `Some` (rd331e).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "set-read-past-a-call",
    breaks: "a cell's `get().unwrap()` after a call that runs first in its statement, which may take the `Some` out, is its value, no panic where Rust's is",
    file: "src/lower/body_queries/set_reads.rs",
    find: "            for (i, &operand) in std::iter::once(&fun).chain(args.iter()).enumerate() {\n",
    replace: "            for (i, &operand) in std::iter::once(&fun).chain(args.iter()).enumerate().collect::<Vec<_>>().into_iter().rev() {\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_set_then_read_back.rs$"],
  },
  {
    name: "set-read-of-a-none",
    breaks: "a cell set to `None`, then read back by `get().unwrap()`, is its value, `undefined`, no panic where Rust's is",
    file: "src/lower/body_queries/set_reads.rs",
    find: "    let some = tcx.is_lang_item(adt.adt_def.variant(adt.variant_index).def_id, LangItem::OptionSome);\n",
    replace: "    let some = adt.adt_def.is_enum();\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_set_none_then_read_back.rs$"],
  },
  {
    name: "set-read-of-any-place",
    breaks: "another cell's `get().unwrap()` after a set of one is its value, unchecked",
    file: "src/lower/body_queries/set_reads.rs",
    find: "                && same_place(thir, cell, read)\n",
    replace: "                && cell != read\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^cell_set_then_read_back.rs$"],
  },
  {
    name: "set-read-of-no-let",
    breaks: "a cell's `let x = cell.get().unwrap();` right after its set keeps `$unwrap`",
    file: "src/lower/body_queries/set_reads.rs",
    find: "        } => initializer,\n",
    replace: "        } => initializer.filter(|_| false),\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^cell_set_then_read_back.rs$"],
  },
  {
    name: "set-read-of-no-tail",
    breaks: "a body that gives `cell.get().unwrap()`'s use right after its set keeps `$unwrap`",
    file: "src/lower/body_queries/set_reads.rs",
    find: "                None => block.expr,\n",
    replace: "                None => None,\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^cell_set_then_read_back.rs$"],
  },
  {
    name: "set-read-through-no-block",
    breaks: "`_ = f(cell.get().unwrap())` right after its set keeps `$unwrap`, its `{ let _ = ..; }` unread",
    file: "src/lower/body_queries/set_reads.rs",
    find: "        ExprKind::Block { block } => {\n",
    replace: "        ExprKind::Block { block } if false => {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^cell_set_then_read_back.rs$"],
  },
  {
    name: "set-read-past-no-cast",
    breaks: "a read back inside a cast, `f(cell.get().unwrap()).len() as f64`, keeps `$unwrap`",
    file: "src/lower/body_queries/set_reads.rs",
    find: "        | ExprKind::PointerCoercion { source: arg, .. }\n        | ExprKind::Cast { source: arg } => first_call(thir, arg, None),\n",
    replace: "        | ExprKind::PointerCoercion { source: arg, .. } => first_call(thir, arg, None),\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^cell_set_then_read_back.rs$"],
  },
];
