// Mutations of src/lower/temporary_names.rs (ADR 0569, #890).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "temporary-name-taken-suffixed",
    breaks:
      "a temporary whose suggested names are taken is given one apart, `point$1`, not `value`",
    file: "src/lower/temporary_names.rs",
    find: "        .find(|name| !taken.contains(name))\n",
    replace: "        .next()\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-named-by-keyword",
    breaks: "a temporary is named `mut`, of `as_mut()`, a keyword's name",
    file: "src/lower/temporary_names.rs",
    find: "    (plain && !keyword && !USELESS_NAMES.contains(&name)).then(|| name.to_string())\n",
    replace: "    (plain && !USELESS_NAMES.contains(&name)).then(|| name.to_string())\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_end_entries.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-named-by-useless-method",
    breaks:
      "a temporary is named after a method that names nothing, `unwrapErr`, not `value`",
    file: "src/lower/temporary_names.rs",
    find: "        true if USELESS_METHODS.contains(&name) => return None,\n",
    replace: "",
    tests: ["test/compiler.test.ts", "-t", "the report's JS"],
  },
  {
    name: "temporary-cell-unnamed",
    breaks:
      "a temporary holding a `Cell` of the program's type isn't named after what it holds, `point` of `Cell::new(Some(point))` (rd413)",
    file: "src/lower/temporary_names.rs",
    find: "                ty::Adt(_, args) if self.is_std_type(ty, StdItem::Cell) || self.is_value_cell(ty) => {\n",
    replace: "                ty::Adt(_, args) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-result-named-by-ok",
    breaks:
      "a `Result` temporary is named after its `Ok` type, `order.TAG`, as JS holds it `{ TAG, _0 }`",
    file: "src/lower/temporary_names.rs",
    find: "                ty::Adt(_, args) if ty.is_box() || self.is_std_type(ty, StdItem::Rc) => args.type_at(0).peel_refs(),\n",
    replace:
      "                ty::Adt(_, args) if ty.is_box() || self.is_std_type(ty, StdItem::Rc) || self.is_std_type(ty, StdItem::Result) => args.type_at(0).peel_refs(),\n",
    tests: ["test/snapshots.test.ts", "-t", "inbox"],
  },
];
