// Mutations of src/runtime/own_into_iter.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "own-into-iter-eager",
    breaks: "the crate's own `into_iter` runs where it's given, before generic code calls `into_iter`: `into_iter` printed before `total starts`, where Rust prints it after (#1940)",
    file: "src/runtime/own_into_iter.js",
    find: "  return { [Symbol.iterator]: make };\n",
    replace: "  const made = make();\n  return { [Symbol.iterator]: () => made };\n",
    tests: ["test/corpus.test.ts", "-t", "^own_into_iterator_given_where_any_goes.rs$"],
  },
];
