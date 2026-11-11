// Mutations of src/lower/pipeline/variables.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-statics-boxed",
    breaks: "a thread-local only its module reads and sets stays `{ value }`",
    file: "src/lower/pipeline/variables.rs",
    find: "        super::super::std_types::cell::plain_statics(items, &quiet,",
    replace: "        super::super::std_types::cell::plain_statics(&mut Vec::new(), &quiet,",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "thread-local only its module",
    ],
  },
  {
    name: "exported-box-unrefused",
    breaks:
      "a `pub` thread-local whose module hands on the `&mut` of its `Option` is `export const depth = {}`, which a JS importer reads as the box, never what it holds (ADR 0270, amended)",
    file: "src/lower/pipeline/variables.rs",
    find: "    for name in boxed {\n",
    replace: "    for name in boxed.into_iter().filter(|_| false) {\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an exported cell its module needs the cell of is refused",
    ],
  },
  {
    name: "library-variable-read-as-cell",
    breaks:
      "a consumer reads a library's variable's `value`, `hasBadMapPolyfill.value`, of a boolean (ADR 0270, amended)",
    file: "src/lower/pipeline/variables.rs",
    find: "        .filter(|item| item.calling.has(crate::library::ItemFact::Variable) && item.member.is_none())\n",
    replace:
      "        .filter(|item| item.calling.has(crate::library::ItemFact::Variable) && item.member.is_some())\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a library's exported cell",
    ],
  },
  {
    name: "static-mut-exported-boxed",
    breaks:
      "a `pub static mut` isn't one of its module's variables to `plain_statics`, so it stays its `{ value }` and is refused (ADR 0096, amended)",
    file: "src/lower/pipeline/variables.rs",
    find: "        .chain(&statics)\n        .map(|key| fns[&key.to_def_id()].name.as_str())\n",
    replace:
      "        .map(|key| fns[&key.to_def_id()].name.as_str())\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a pub static mut its module alone writes",
    ],
  },
  {
    name: "static-mut-pointer-export-unrefused",
    breaks:
      "a `pub static mut` a pointer to is given on is `export const TOTAL = { value: 0 }`, which a JS importer reads as the box (ADR 0096, amended)",
    file: "src/lower/pipeline/variables.rs",
    find: "        .chain(&statics)\n        .filter_map(|&key| Some((fns.get(&key.to_def_id())?.name.as_str(), key)))\n",
    replace:
      "        .filter_map(|&key| Some((fns.get(&key.to_def_id())?.name.as_str(), key)))\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a static mut a pointer to is given on",
    ],
  },
  {
    name: "static-mut-variable-unsaid",
    breaks:
      "a `static mut` that's its module's `export let` isn't one of its variables to its importers, which read its `.value`, nor to its `.d.ts` or its library's contract (ADR 0096, amended)",
    file: "src/lower/pipeline/variables.rs",
    find: "        .chain(&statics)\n        .filter_map(|key| {\n",
    replace: "        .filter_map(|key| {\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "static mut"],
  },
  {
    name: "namespaced-library-variable-held",
    breaks:
      "a library's thread-local read through its namespace, `dep::NAME.with_borrow(..)`, is read as its cell, `Dep.name.value`, where it's the library's variable (rd114a, ADR 1018)",
    file: "src/lower/pipeline/variables.rs",
    find: "                js::ExprKind::Var(namespace) if namespaced.contains(&(namespace.as_str(), export)))\n",
    replace:
      "                js::ExprKind::Var(namespace) if false && namespaced.contains(&(namespace.as_str(), export)))\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "library's exported cell",
    ],
  },
  {
    name: "nullable-cells-unnamed",
    breaks:
      "no thread-local of a module is said nullable to `plain_statics`, so one kept as its cell is a `{ value }` that stores what it's written (#541)",
    file: "src/lower/pipeline/variables.rs",
    find: "                && none_form(tcx, NonePlace::LocalCell(key.to_def_id())).stores_null()\n",
    replace: "                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
  },
];
