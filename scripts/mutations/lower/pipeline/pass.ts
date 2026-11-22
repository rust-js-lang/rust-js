// Mutations of src/pipeline/pass.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "default-class-exported-by-name",
    breaks:
      "a class that's its module's default is `export function Complex` too",
    file: "src/pipeline/pass.rs",
    find: "            class.export = false;\n",
    replace: "",
    tests: [
      "test/classes.test.ts",
      "-t",
      "a class is its module's default export",
    ],
  },
  {
    name: "default-constant-read-exported",
    breaks:
      "a `pub` constant the default holds, read in its own module, is exported by its name too, where it's the default alone",
    file: "src/pipeline/pass.rs",
    find: "            constant.export = false;\n",
    replace: "            let _ = &mut constant.export;\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a default constant its own module reads",
    ],
  },
  {
    name: "named-default-exported-by-name",
    breaks:
      "a `pub fn` `js::export_default!(page)` names is `export function page` as well as the default, two exports of one item",
    file: "src/pipeline/pass.rs",
    find: ".filter(|f| f.name.is(name)) {\n            function.export = false;\n",
    replace: ".filter(|f| f.name.is(name)) {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "naming form names as the default alone",
    ],
  },
];
