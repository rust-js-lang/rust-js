// Mutations of src/lower/bindings/indexed.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "indexed-list-unread",
    breaks:
      "a `NodeList`, `#[rust_js::indexed]`, is no indexed list, and a `for` over `node_list::iter(list)` stays the `for…of` Closure's ES5 build polyfills, not upstream's `for (let i = 0; i < list.length; i++)` (ADR 1042)",
    file: "src/lower/bindings/indexed.rs",
    find: "    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if marked(tcx, adt.did()))\n",
    replace: "    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if marked(tcx, adt.did()) && false)\n",
    tests: ["test/lowering-index-loops.test.ts", "-t", "NodeList's items is an index loop"],
  },
  {
    name: "indexed-of-a-function-taken",
    breaks: "`#[rust_js::indexed]` of a function, which says nothing of a list, is taken silently (ADR 1042)",
    file: "src/lower/bindings/indexed.rs",
    find: "    (marked(tcx, def_id) && tcx.def_kind(def_id) != DefKind::Struct).then_some(\n",
    replace: "    (marked(tcx, def_id) && tcx.def_kind(def_id) != DefKind::Struct && false).then_some(\n",
    tests: ["test/lowering-index-loops.test.ts", "-t", "anything but a struct is refused"],
  },
];
