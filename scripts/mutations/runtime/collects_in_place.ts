// Mutations of src/runtime/collects_in_place.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "collects-in-place-any-size",
    breaks:
      "generic code's `src.map(f).enumerate().collect()` of a `Vec`'s `IntoIter`, whose `(usize, D)`s are bigger than its `D`s, is taken to be collected in place, so a panic drops what it collected first (#672)",
    file: "src/runtime/collects_in_place.js",
    find: " && source[0] >= size;",
    replace: ";",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "collects-in-place-never",
    breaks:
      "generic code's collect of a `Vec`'s `IntoIter`, which std collects in place, a panic stopping it, drops what's left first (#672)",
    file: "src/runtime/collects_in_place.js",
    find: "  return source !== undefined && ",
    replace: "  return false && ",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
];
