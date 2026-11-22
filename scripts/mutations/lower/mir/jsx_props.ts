// Mutations of crates/rust-js-mir/src/jsx_props.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-written-base-unread-only",
    breaks:
      "a struct update reading one field of its base from MIR is each field, `{ title, level: props.level }`, dropping what JS's object held besides",
    file: "crates/rust-js-mir/src/jsx_props.rs",
    find: "            if !field || self.contains_mutated(t) && self.is_copy(t) {\n",
    replace: "            if true {\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "reading one field of its base",
    ],
  },
  {
    name: "struct-update-index-fields",
    breaks:
      "`{..posts[0]}` gives each field by name, `title={posts[0].title}`, not `{...posts[0]}`",
    file: "crates/rust-js-mir/src/jsx_props.rs",
    find: "            thir::ExprKind::Index { .. } if !bindings::has_flatten(self.tcx, ty) => (None, false),\n",
    replace: "",
    tests: [
      "test/jsx-props.test.ts",
      "-t",
      "props updated from a reference are spread",
    ],
  },
  {
    name: "struct-update-deref-fields",
    breaks:
      "`{..*unsafe { posts.get_unchecked(0) }}` gives each field by name, not `{...posts[0]}`",
    file: "crates/rust-js-mir/src/jsx_props.rs",
    find: "            {\n                (None, false)\n            }\n            thir::ExprKind::Index",
    replace:
      "            {\n                return None;\n            }\n            thir::ExprKind::Index",
    tests: [
      "test/jsx-props.test.ts",
      "-t",
      "props updated from a reference are spread",
    ],
  },
  {
    name: "replaced-field-not-a-place",
    breaks:
      "`mem::replace` of `&mut self.field` holding an iterator given as it is is refused, the field not taken for the place it is",
    file: "crates/rust-js-mir/src/jsx_props.rs",
    find: "matches!(place, thir::ExprKind::Field { .. })",
    replace: "false",
    tests: ["test/corpus.test.ts", "-t", "^replace_held_given_iterator.rs$"],
    snapshots: true,
  },
  {
    name: "indexed-item-moved-out-apart",
    breaks:
      "the scheduler's `mem::replace(&mut heap[0], last)` of a generic `Vec<T>` copies the old item, `$apart(heap[0])`, where a write of the item rebinds it (#527)",
    file: "crates/rust-js-mir/src/jsx_props.rs",
    find: "            thir::ExprKind::Deref { arg } => self.indexes_items(arg),\n",
    replace: "            thir::ExprKind::Deref { arg } => self.indexes_items(arg) && false,\n",
    tests: ["test/port-scheduler.test.ts"],
    snapshots: true,
  },
];
