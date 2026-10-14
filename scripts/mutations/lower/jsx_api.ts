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
];
