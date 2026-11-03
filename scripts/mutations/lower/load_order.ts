// Mutations of src/lower/load_order.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "bound-file-imports-nothing",
    breaks:
      "a module that binds a JS file of the program's own imports none of the crate's modules that file imports, so a cycle through it is unseen: React's `NoEventPriority = NoLane` throws where ReactFiberLane loads first",
    file: "src/lower/load_order.rs",
    find: "                Some(reached) => to.extend(reached.iter().map(|&at| modules[at].0)),\n",
    replace: "                Some(_) => {}\n",
    tests: [
      "test/lowering-constants.test.ts",
      "test/lowering-dictionaries.test.ts",
      "-t",
      "a bound JS file imports back|a dictionary that takes nothing",
    ],
  },
  {
    name: "unread-bound-file-imports-nothing",
    breaks:
      "a bound JS file that isn't found, or imports one that isn't, imports none of the crate's modules, where it may import any",
    file: "src/lower/load_order.rs",
    find: "                None => to.extend(modules.iter().map(|&(module, _)| module)),\n",
    replace: "                None => {}\n",
    tests: [
      "test/lowering-constants.test.ts",
      "-t",
      "a bound JS file imports back",
    ],
  },
  {
    name: "imports-through-cycle-taken",
    breaks:
      "an import through a module that re-exports it is taken where that module's load reaches the importer, `import { helper } from \"./lib.js\"` of a module the root re-exports, a load cycle (rd650)",
    file: "src/lower/load_order.rs",
    find: "        if from == to || self.reaches(to, from) {\n",
    replace: "        if from == to {\n",
    tests: ["test/reexports-through.test.ts", "-t", "a use through a module"],
  },
];
