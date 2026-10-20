// Mutations of src/runtime/flat_map.js (ADR 0510): a source and inner
// iterators that both hold what has a destructor, closed source first.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flat-map-inner-closed-first",
    breaks: "`$flatMap` closes the inner iterator before its source, as JS's `flatMap` does, not as Rust drops a `FlatMap`",
    file: "src/runtime/flat_map.js",
    find: "    if (!ended) it.return?.();\n    inner?.return?.();\n",
    replace: "    inner?.return?.();\n    if (!ended) it.return?.();\n",
    tests: ["test/corpus.test.ts", "-t", "^flat_map_drops.rs$"],
  },
];
