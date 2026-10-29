// Mutations of src/runtime/flat_map.js (ADR 0510): a source and inner
// iterators that both hold what has a destructor, closed source first.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flat-map-inner-closed-first",
    breaks: "`$flatMap` closes the inner iterator before its source, as JS's `flatMap` does, not as Rust drops a `FlatMap`",
    file: "src/runtime/flat_map.js",
    find: "      if (!ended) {\n        ended = true;\n        it.return?.();\n      }\n      inner?.return?.();\n",
    replace: "      inner?.return?.();\n      if (!ended) {\n        ended = true;\n        it.return?.();\n      }\n",
    tests: ["test/corpus.test.ts", "-t", "^flat_map_drops.rs$"],
  },
  {
    name: "flat-map-closed-as-it-throws",
    breaks: "a `flat_map` whose closure panics closes its source before what takes it drops what it holds, a `last`'s kept item, as a generator's `finally` would (#426)",
    file: "src/runtime/flat_map.js",
    find: "        inner = f(outer.value);\n",
    replace: "        try {\n          inner = f(outer.value);\n        } catch (error) {\n          it.return?.();\n          throw error;\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_in_own_next_drops.rs$"],
  },
];
