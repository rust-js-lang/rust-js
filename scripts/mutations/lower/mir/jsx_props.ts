// Mutations of src/lower/mir/jsx_props.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-jsx-shapeless",
    breaks: "a JSX call from MIR has no THIR call for its shape: props in the struct's order",
    file: "src/lower/mir/jsx_props.rs",
    find: "        calls.next().is_none().then_some(call)\n",
    replace: "        calls.next().is_none().then_some(call).filter(|_| false)\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    "name": "mir-written-base-unread-only",
    "breaks": "a struct update reading one field of its base from MIR is each field, `{ title, level: props.level }`, dropping what JS's object held besides",
    "file": "src/lower/mir/jsx_props.rs",
    "find": "            if !field || self.contains_mutated(t) && self.is_copy(t) {\n",
    "replace": "            if true {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "reading one field of its base"
    ]
  },
  {
    "name": "struct-update-index-fields",
    "breaks": "`{..posts[0]}` gives each field by name, `title={posts[0].title}`, not `{...posts[0]}`",
    "file": "src/lower/mir/jsx_props.rs",
    "find": "            thir::ExprKind::Index { .. } if !bindings::has_flatten(self.tcx, ty) => (None, false),\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts",
      "-t",
      "props updated from a reference are spread"
    ]
  },
  {
    "name": "struct-update-deref-fields",
    "breaks": "`{..*unsafe { posts.get_unchecked(0) }}` gives each field by name, not `{...posts[0]}`",
    "file": "src/lower/mir/jsx_props.rs",
    "find": "            {\n                (None, false)\n            }\n            thir::ExprKind::Index",
    "replace": "            {\n                return None;\n            }\n            thir::ExprKind::Index",
    "tests": [
      "test/jsx.test.ts",
      "-t",
      "props updated from a reference are spread"
    ]
  },
  {
    name: "replaced-field-not-a-place",
    breaks: "`mem::replace` of `&mut self.field` holding an iterator given as it is is refused, the field not taken for the place it is",
    file: "src/lower/mir/jsx_props.rs",
    find: "matches!(place, thir::ExprKind::Field { .. })",
    replace: "false",
    tests: ["test/corpus.test.ts", "-t", "^replace_held_given_iterator.rs$"],
    snapshots: true,
  },
];
