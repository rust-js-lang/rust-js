// Mutations of src/lower/mir/hints/names.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-booleans-unhinted",
    breaks: "a `bool` variable isn't known as one, `!!a || b`, not `a || b`",
    file: "src/lower/mir/hints/names.rs",
    find: "        booleans.retain(|name| !others.contains(name));\n",
    replace: "        booleans.clear();\n",
    tests: [
      "test/lowering-branches.test.ts",
      "-t",
      "a function.s match of two arms and its if of two values",
    ],
  },
  {
    name: "mir-captured-booleans-unknown",
    breaks:
      "a closure doesn't know a `bool` it captures as one, `!!force || n > 2`",
    file: "src/lower/mir/hints/names.rs",
    find: "                    match ty.peel_refs().is_bool() {\n",
    replace: "                    match false {\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "names its .&x. parameter",
    ],
  },
  {
    name: "mir-bool-ref-unknown",
    breaks:
      "a state's `bool`, a `&bool` local, isn't known as one, `showHint ? hint : undefined`",
    file: "src/lower/mir/hints/names.rs",
    find: "                ty::Ref(_, inner, ty::Mutability::Not) => inner.is_bool(),\n",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "temporary-named-by-keyword",
    breaks: "a temporary is named `mut`, of `as_mut()`, a keyword's name",
    file: "src/lower/mir/hints/names.rs",
    find: "        (plain && !keyword && !USELESS_NAMES.contains(&name)).then(|| name.to_string())\n",
    replace:
      "        (plain && !USELESS_NAMES.contains(&name)).then(|| name.to_string())\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_end_entries.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-named-by-useless-method",
    breaks:
      "a temporary is named after a method that names nothing, `unwrapErr`, not `value`",
    file: "src/lower/mir/hints/names.rs",
    find: "            true if USELESS_METHODS.contains(&name) => return None,\n",
    replace: "",
    tests: ["test/compiler.test.ts", "-t", "the report's JS"],
  },
  {
    name: "temporary-taken-reference-unnamed",
    breaks:
      "a reference MIR takes to a call's result, `format_args!`'s, isn't named after the call",
    file: "src/lower/mir/hints/names.rs",
    find: "                    taken_from.insert(local, from);\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(temporary_names|cell_const|extreme_drops).rs$",
    ],
    snapshots: true,
  },
  {
    name: "temporary-type-unnamed",
    breaks:
      "a temporary holding the program's type isn't named after it, `point` of an `Option<Point>`",
    file: "src/lower/mir/hints/names.rs",
    find: "                    self.held_type_name(mir_body.local_decls[local].ty),\n",
    replace: "                    None,\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-cell-unnamed",
    breaks:
      "a temporary holding a `Cell` of the program's type isn't named after what it holds, `point` of `Cell::new(Some(point))` (rd413)",
    file: "src/lower/mir/hints/names.rs",
    find: "                ty::Adt(_, args) if self.is_std_type(ty, StdItem::Cell) || self.is_value_cell(ty) => {\n",
    replace: "                ty::Adt(_, args) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-result-named-by-ok",
    breaks:
      "a `Result` temporary is named after its `Ok` type, `order.TAG`, as JS holds it `{ TAG, _0 }`",
    file: "src/lower/mir/hints/names.rs",
    find: "                ty::Adt(_, args) if ty.is_box() || self.is_std_type(ty, StdItem::Rc) => args.type_at(0).peel_refs(),\n",
    replace:
      "                ty::Adt(_, args) if ty.is_box() || self.is_std_type(ty, StdItem::Rc) || self.is_std_type(ty, StdItem::Result) => args.type_at(0).peel_refs(),\n",
    tests: ["test/snapshots.test.ts", "-t", "inbox"],
  },
];
