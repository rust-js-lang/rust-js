// Mutations of src/lower/std_types/lazy.rs (ADR 0318).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lazy-deref-of-cell",
    breaks: "`Deref` takes the cell's type for its value's, so `*n += 1` writes a number's field",
    file: "src/lower/std_types/lazy.rs",
    find: "            ty::Adt(_, args) if self.is_std_type(generic_args.type_at(0), StdItem::LazyCell) => args.type_at(0),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "lazy_cells"],
  },
  {
    name: "lazy-force-mut-copy",
    breaks: "`force_mut` of a number gives the number, so a write through it is lost",
    file: "src/lower/std_types/lazy.rs",
    find: "        if op == LazyOp::Force || op == LazyOp::ForceMut && !self.is_boxable(item) {\n",
    replace: "        if op == LazyOp::Force || op == LazyOp::ForceMut {\n",
    tests: ["test/corpus.test.ts", "-t", "lazy_cells"],
  },
  {
    name: "lazy-get-unmade",
    breaks: "`get_mut` gives the cell before its value's made",
    file: "src/lower/std_types/lazy.rs",
    find: "        Ok(Some(Expr::cond(made, given, Expr::undefined())))\n",
    replace: "        Ok(Some(given))\n",
    tests: ["test/corpus.test.ts", "-t", "lazy_cells"],
  },
];
