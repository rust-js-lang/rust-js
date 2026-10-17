// Mutations of src/lower/jsx_api.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unexpanded-jsx-unrefused",
    breaks: "a `jsx!` by a path to it, `react::jsx!`, is refused as a call of `__jsx`, which says nothing of how it's written",
    file: "src/lower/jsx_api.rs",
    find: "        if super::bindings::is_unexpanded_jsx(self.tcx, def) {\n",
    replace: "        if false {\n",
    tests: ["test/jsx.test.ts", "-t", "a jsx! by a path to it is refused"],
  },
  {
    name: "element-builder-allowed",
    breaks: "an element builder called directly, `react::element(..)`, compiles to what no `jsx!` wrote",
    file: "src/lower/jsx_api.rs",
    find: "        } else if is_element_builder(self.tcx, def) {\n",
    replace: "        } else if false {\n",
    tests: ["test/jsx.test.ts", "-t", "direct element builder"],
  },
  {
    name: "element-attribute-any-case",
    breaks: "an element's attribute in Rust's case, `<button on_click>`, is taken as `onClick`",
    file: "src/lower/jsx_api.rs",
    find: "                        self.spelled(segment.ident.span, &prop);\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "attribute in Rust's case"],
  },
  {
    name: "component-prop-any-case",
    breaks: "a component's prop in Rust's case, `<Badge label_text>`, is taken as `labelText`",
    file: "src/lower/jsx_api.rs",
    find: "            hir::ExprKind::Block(block, _) => self.component_props(block),\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "names a component's prop as JSX"],
  },
  {
    name: "component-prop-js-name-refused",
    breaks: "a component's prop as JSX names it, `labelText`, is refused",
    file: "src/lower/jsx_api.rs",
    find: "            if fields.iter().any(|f| super::bindings::field_key(self.tcx, f) == text) {\n",
    replace: "            if false {\n",
    tests: ["test/jsx.test.ts", "-t", "names a component's prop as JSX"],
  },
];
