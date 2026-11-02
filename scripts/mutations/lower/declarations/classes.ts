// Mutations of src/lower/declarations/classes.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "state-keys-undeclared",
    breaks: "a class's flattened state's keys, `status`, `value` and `reason`, are left out of its declaration (rd380)",
    file: "src/lower/declarations/classes.rs",
    find: "        members.splice(0..0, self.state_keys(def_id));\n",
    replace: "",
    tests: ["test/declarations-flatten.test.ts", "-t", "flattened state declares its keys"],
  },
  {
    name: "state-key-of-a-variant-without-it-never",
    breaks: "a flattened state's key a variant has no field at is declared as only what the others hold, though its object may have none there (rd380)",
    file: "src/lower/declarations/classes.rs",
    find: "                            None => keyword(\"undefined\"),\n",
    replace: "                            None => keyword(\"never\"),\n",
    tests: ["test/declarations-flatten.test.ts", "-t", "flattened state declares its keys"],
  },
  {
    name: "state-key-of-any-value-a-union",
    breaks: "a flattened state's key one variant holds any value at is declared as a union with `unknown` in it, which says no more than `unknown` (rd380)",
    file: "src/lower/declarations/classes.rs",
    find: "                    (Some(top), _) => keyword(top),\n",
    replace: "                    (Some(_), _) => union(types),\n",
    tests: ["test/declarations-flatten.test.ts", "-t", "flattened state declares its keys"],
  },
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
