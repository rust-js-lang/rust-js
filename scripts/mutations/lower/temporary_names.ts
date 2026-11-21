// Mutations of src/lower/temporary_names.rs (ADR 0569, #890).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "program-underscore-not-unread",
    breaks:
      "an `async` closure's `|_|`, a parameter the program names `_`, is the program's, kept, `async (_) => 1`, where it takes nothing, `async () => 1`",
    file: "src/lower/temporary_names.rs",
    find: "            Base::Program(name) if name.source() == rustc_span::symbol::kw::Underscore.as_u32() => {\n",
    replace: "            Base::Program(name) if name.source() == u32::MAX => {\n",
    tests: ["test/lowering-async.test.ts", "-t", "an async closure's ignored argument is no parameter"],
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
  {
    name: "every-role-says-nothing",
    breaks:
      "a value spilled for a role that names what it holds, a `Result`'s `result`, is named by its call instead, `parse` of `JSON.parse(..)`, which #769 found reads worse (#890)",
    file: "src/lower/temporary_names.rs",
    find: "        matches!(self, Role::Value | Role::Option)\n",
    replace: "        true\n",
    tests: ["test/corpus.test.ts", "-t", "^interpreter.rs$"],
    snapshots: true,
  },
  {
    name: "program-names-not-the-programs",
    breaks:
      "a local the program named is made as one lowering made, so a name lowering made keeps the spelling the program's wants, and the check of user variables fails (ADR 1150)",
    file: "src/lower/temporary_names.rs",
    find: "            Base::Program(name) => crate::js::Name::program(word, name.source()),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(flattened_state_handed_on|taken_and_returned|std_mixed_enum_mut).rs$"],
    snapshots: true,
  },
  {
    name: "looped-item-named-item",
    breaks: "a drop loop's item that its body loops over in turn, a `Vec<Vec<T>>`'s, is named `item`, so its own loop's is `item$1`, where a person names them `list` and `item` (#1347)",
    file: "src/lower/temporary_names.rs",
    find: "    if !looped {\n        return name;\n    }\n",
    replace: "    if true {\n        return name;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_item_drops\\.rs$"],
    snapshots: true,
  },
];
