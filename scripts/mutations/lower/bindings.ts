// Mutations of src/lower/bindings.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "assoc-this-not-method",
    breaks: "a type's own function whose first parameter is `this`, `Mouse::widen(this)`, isn't a method: rust-js refuses it",
    file: "src/lower/bindings.rs",
    find: "        DefKind::AssocFn => tcx.associated_item(def_id).is_method() || named_this(),",
    replace: "        DefKind::AssocFn => tcx.associated_item(def_id).is_method(),",
    tests: ["test/jsx.test.ts", "-t", "tag's element"],
  },
  {
    name: "camel-case-unread",
    breaks: "`js::camel_case!();` names nothing the JS way",
    file: "src/lower/bindings.rs",
    find: "    marks(tcx, CRATE_MOD_ID, \"camel_case\").next().is_some()\n",
    replace: "    marks(tcx, CRATE_MOD_ID, \"none\").next().is_some()\n",
    tests: ["test/react.test.ts","-t","camel_case crate"],
  },
  {
    name: "route-marks-written",
    breaks: "\`js::directive!\`'s and \`js::export_default!\`'s \`const _\` is written to the JS",
    file: "src/lower/bindings.rs",
    find: '    ["import", "camel_case", "directive", "export_default"]\n',
    replace: '    ["import", "camel_case"]\n',
    tests: ["test/compiler.test.ts", "-t", "make a module a Next.js route"],
  },
  {
    name: "flatten-not-a-rest",
    breaks: "a flattened field is a field, `{ size, children, anchor }`, not `...anchor`",
    file: "src/lower/bindings.rs",
    find: "        .is_some_and(|field| is_flatten(tcx, field) || is_rest(tcx, field.ty(tcx, args).skip_normalization()))",
    replace: "        .is_some_and(|field| is_rest(tcx, field.ty(tcx, args).skip_normalization()))",
    tests: ["test/jsx.test.ts", "-t", "flattened struct"],
  },
];
