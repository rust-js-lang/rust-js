// Mutations of src/lower/bindings/nullable.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "optional-field-made-null",
    breaks: "an optional nullable field's read, which may be absent, is taken for `null` or a value, so a nullable field given it stores `undefined` (ADR 0810, amended)",
    file: "src/lower/bindings/nullable.rs",
    find: "    is_nullable(tcx, field) && !is_optional(tcx, field)\n",
    replace: "    is_nullable(tcx, field)\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "an optional nullable field"],
  },
  {
    name: "nullable-refcell-refused",
    breaks: "`#[rust_js::nullable]` of a field's `RefCell<Option<T>>` is refused (ADR 0863)",
    file: "src/lower/bindings/nullable.rs",
    find: "            if (cell(*adt, crate::lower::recognition::StdItem::Cell)\n                || cell(*adt, crate::lower::recognition::StdItem::RefCell))\n",
    replace: "            if cell(*adt, crate::lower::recognition::StdItem::Cell)\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "a nullable RefCell field"],
  },
  {
    name: "nullable-generic-cell-field-refused",
    breaks: "`#[rust_js::nullable]` of a generic struct's `RefCell<T>` or `Cell<T>` field is refused, React's `StackCursor<T>`'s `current` (ADR 0586, amended)",
    file: "src/lower/bindings/nullable.rs",
    find: "            matches!(args.type_at(0).kind(), ty::Param(_))\n",
    replace: "            false\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "a generic struct's nullable field"],
  },
  {
    name: "nullable-thread-local-of-options-refused",
    breaks: "`#[rust_js::nullable]` of a thread-local a generic constructor makes of an `Option`, `StackCursor<Option<Rc<HiddenContext>>>`, is refused (ADR 0586, amended)",
    file: "src/lower/bindings/nullable.rs",
    find: "    matches!(held.kind(), ty::Adt(adt, args) if !tcx.is_lang_item(adt.did(), LangItem::Option) && args.types().any(option))\n",
    replace: "    let _ = (held, option);\n    false\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "a generic constructor makes"],
  },
  {
    name: "nullable-init-of-any-thread-local",
    breaks: "a thread-local that isn't `#[rust_js::nullable]`, made by a generic constructor, is given `null` for `None`, `createCursor(null)` where its JS reads `undefined` (ADR 0586, amended)",
    file: "src/lower/bindings/nullable.rs",
    find: "    tcx.opt_parent(body).is_some_and(|key| nullable_local(tcx, key))\n",
    replace: "    tcx.opt_parent(body).is_some()\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "a generic constructor makes"],
  },
  {
    name: "nullable-option-init-undefined",
    breaks: "a nullable thread-local of an `Option`'s `init` writes its `None` `undefined`, `__DEV__ ? f : undefined`",
    file: "src/lower/bindings/nullable.rs",
    find: "    nullable_result(tcx, body) || tcx.opt_parent(body).is_some_and(|key| nullable_local_option(tcx, key))\n",
    replace: "    nullable_result(tcx, body)\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "a nullable thread-local of an Option is null where it's made"],
  },
];
