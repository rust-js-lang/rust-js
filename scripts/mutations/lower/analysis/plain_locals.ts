// Mutations of src/lower/analysis/plain_locals.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "plain-locals-boxed",
    breaks: "a thread-local only read and set in its own module keeps its `{ value }`, `const COUNT = { value: 0 }`",
    file: "src/lower/analysis/plain_locals.rs",
    find: "        .filter(|&key| !tcx.visibility(key).is_public())\n",
    replace: "        .filter(|_| false)\n",
    tests: ["test/modules.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "other-module-local-unboxed",
    breaks: "a thread-local another module reads is its module's `let`, which the importer can't set",
    file: "src/lower/analysis/plain_locals.rs",
    find: "if !accessed.contains(&id) || tcx.parent_module_from_def_id(key) != module {",
    replace: "if !accessed.contains(&id) {",
    tests: ["test/modules.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "set-local-const",
    breaks: "a thread-local that's `set` is a `const`, which JS can't assign",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                if sets {\n",
    replace: "                if false && sets {\n",
    tests: ["test/modules.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "cell-clone-copied",
    breaks: "a clone of a cell that's its function's `let`, `started`, is its value then, `const started = timeout`, so what the listener sets nothing reads",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                        cloned.insert(var, of);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "cell only its function"],
  },
  {
    name: "cell-read-escapes",
    breaks: "a cell only read and set is `{ value }`, as if `get` gave it away",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                    allowed.extend(args.first().and_then(|&a| read(a)).map(|(at, _)| at));\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "cell only its function"],
  },
  {
    name: "cell-captured-escapes",
    breaks: "a cell a closure reads and sets is `{ value }`, as if capturing it gave it away",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                    allowed.extend(closure.upvars.iter().filter_map(|&u| read(u)).map(|(at, _)| at));\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "cell only its function"],
  },
  {
    name: "cell-returned-plain",
    breaks: "a cell returned whole is a `let`, its number returned where the caller wants the cell",
    file: "src/lower/analysis/plain_locals.rs",
    find: "            if !allowed.contains(&id) {\n",
    replace: "            if false && !allowed.contains(&id) {\n",
    tests: ["test/lowering.test.ts", "-t", "cell only its function"],
  },
];
