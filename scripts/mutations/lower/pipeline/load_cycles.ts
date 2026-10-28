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
  {
    name: "maker-cycle-unseen",
    breaks: "a thread-local whose maker reads a `const` of a module that imports its own back is made as its module loads, before that module made it: a `ReferenceError`",
    file: "src/lower/pipeline/load_cycles.rs",
    find: "                module != own && imports.reaches(module, own)\n",
    replace: "                false && module != own && imports.reaches(module, own)\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_made_by_constructor.rs$"],
  },
  {
    name: "effects-import-nothing",
    breaks: "a module's `js::import!(\"./setup.js\")` imports none of the crate's modules that file does, so a cycle through it is unseen and the module loaded first reads a constant before it's made",
    file: "src/lower/pipeline/load_cycles.rs",
    find: "    let bound = used.chain(users).chain(effects);\n",
    replace: "    let bound = used.chain(users).chain(effects.take(0));\n",
    tests: ["test/lowering-constants.test.ts", "-t", "a bound JS file imports back"],
  },
  {
    name: "exported-local-cycle-unrefused",
    breaks: "a `pub` thread-local made as its module loads that runs a module in an import cycle with its own compiles, and reads what that module hasn't made yet (ADR 0037, amended)",
    file: "src/lower/pipeline/load_cycles.rs",
    find: "            .find(|&module| module != own && imports.reaches(module, own));\n",
    replace: "            .find(|&module| module != own && imports.reaches(module, own) && false);\n",
    tests: ["test/diagnostics.test.ts", "-t", "a pub thread-local of a module in an import cycle"],
  },
  {
    name: "exported-local-self-read-unrefused",
    breaks: "a `pub` thread-local made as its module loads whose initializer reads itself compiles, and throws reading itself before it's made (ADR 0037, amended)",
    file: "src/lower/pipeline/load_cycles.rs",
    find: "        if reached.contains(&key.to_def_id()) {\n",
    replace: "        if false && reached.contains(&key.to_def_id()) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "a pub thread-local that reads itself"],
  },
];
