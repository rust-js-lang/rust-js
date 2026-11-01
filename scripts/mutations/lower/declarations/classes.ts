// Mutations of src/lower/declarations/classes.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "class-to-string-undeclared",
    breaks: "a class's `toString` is left out of its declaration (#358)",
    file: "src/lower/declarations/classes.rs",
    find: "        if crate::lower::classes::has_to_string(self.tcx, def_id) {\n",
    replace: "        if false {\n",
    tests: ["test/classes.test.ts", "-t", "Display is its prototype"],
  },  {
    name: "class-type-alone-exported",
    breaks:
      "a `pub` class whose `new` isn't `pub` is declared `export declare class Wrapper`, a value TypeScript lets its importer construct, which its JS doesn't export (ADR 0761, amended)",
    file: "src/lower/declarations/classes.rs",
    find: "    if crate::lower::classes::exports_its_value(tcx, class) || declared[\"exported\"] != true {\n",
    replace: "    if true {\n",
    tests: ["test/classes.test.ts", "-t", "exports its type alone"],
  },
  {
    name: "subclass-declared-extending-nothing",
    breaks: "a class component's declaration extends nothing, so TypeScript has no `props` or `setState` of it",
    file: "src/lower/declarations/classes.rs",
    find: "                class[\"extends\"] = self.written(trait_ref.def_id, &written, trait_ref.args);\n",
    replace: "                drop(written);\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "an ES class React renders"
    ]
  },
];
