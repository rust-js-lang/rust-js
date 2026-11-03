// Mutations of src/lower/body_queries/ranges/keys.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

const corpus = ["test/corpus.test.ts", "-t", "^map_index_of_a_key_held\\.rs$"];

export const mutations: Mutation[] = [
  {
    name: "map-index-held-unjudged",
    breaks:
      "every `m[k]` is taken as of a key the map holds, so `counts[other]` under `counts.contains_key(key)` reads `undefined` where Rust panics",
    file: "src/lower/body_queries/ranges/keys.rs",
    find: "            *self.indexes.entry(fun).or_insert(true) &= k.keys.contains(&held);\n",
    replace: "            *self.indexes.entry(fun).or_insert(true) &= k.keys.iter().any(|(map, _)| *map == held.0);\n",
    tests: corpus,
  },
  {
    name: "map-key-of-any-map",
    breaks:
      "a key one map holds is taken as another's, so `others[key]` under `counts.contains_key(key)` reads `undefined` where Rust panics",
    file: "src/lower/body_queries/ranges/keys.rs",
    find: "            *self.indexes.entry(fun).or_insert(true) &= k.keys.contains(&held);\n",
    replace: "            *self.indexes.entry(fun).or_insert(true) &= k.keys.iter().any(|(_, key)| *key == held.1);\n",
    tests: corpus,
  },
];
