// Mutations of src/lower/pipeline/load_cycles.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dictionary-load-cycle-allowed",
    breaks: "a dictionary in an import cycle with a module whose load runs code compiles, and that code may read it before it's made",
    file: "src/lower/pipeline/load_cycles.rs",
    find: "reaches(module, other) && reaches(other, module)",
    replace: "false && reaches(module, other) && reaches(other, module)",
    tests: ["test/lowering-dictionaries.test.ts", "-t", "a dictionary that takes nothing"],
  },
  {
    name: "load-time-calls-unseen",
    breaks: "a module whose load calls a function isn't one that runs code, so a dictionary it may reach through a cycle compiles",
    file: "src/lower/pipeline/load_cycles.rs",
    find: "!pass.dictionary_names.contains(&c.name) && c.value.has_effects())",
    replace: "!pass.dictionary_names.contains(&c.name) && c.value.has_effects() && false)",
    tests: ["test/lowering-dictionaries.test.ts", "-t", "a dictionary that takes nothing"],
  },
  {
    name: "dictionary-cache-runs-code",
    breaks: "a generic dictionary's cache, `new WeakMap()`, is code its module's load runs, so a cycle of dictionaries alone, either's, is refused",
    file: "src/lower/pipeline/load_cycles.rs",
    find: "(consts.iter()).any(|c| !pass.dictionary_names.contains(&c.name) && c.value.has_effects())",
    replace: "(consts.iter()).any(|c| c.value.has_effects())",
    tests: ["test/lowering-dictionaries.test.ts", "-t", "a dictionary that takes nothing"],
  },
  {
    name: "load-cycle-unseen",
    breaks: "a module's load reads another's constant by its name though they import each other, which may load it first, before it's made",
    file: "src/lower/pipeline/load_cycles.rs",
    find: "                .any(|&(from, read)| from == own && made_where_loaded(read) && imports.reaches(fns[&read].module, own))\n",
    replace: "                .any(|&(from, read)| from == own && made_where_loaded(read) && false)\n",
    tests: ["test/lowering-constants.test.ts", "-t", "read by its name where its module loads first"],
  },
];
