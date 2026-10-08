// Mutations of src/lower/analysis/plain_locals.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "plain-locals-boxed",
    breaks: "a thread-local only read and set in its own module keeps its `{ value }`, `const COUNT = { value: 0 }`",
    file: "src/lower/analysis/plain_locals.rs",
    find: "        .filter(|&key| !tcx.visibility(key).is_public())\n",
    replace: "        .filter(|_| false)\n",
    tests: ["test/compiler.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "other-module-local-unboxed",
    breaks: "a thread-local another module reads is its module's `let`, which the importer can't set",
    file: "src/lower/analysis/plain_locals.rs",
    find: "if !accessed.contains(&id) || tcx.parent_module_from_def_id(key) != module {",
    replace: "if !accessed.contains(&id) {",
    tests: ["test/compiler.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "set-local-const",
    breaks: "a thread-local that's `set` is a `const`, which JS can't assign",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                if sets {\n",
    replace: "                if false && sets {\n",
    tests: ["test/compiler.test.ts", "-t", "only read and set is the module's"],
  },
];
