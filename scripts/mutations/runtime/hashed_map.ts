// Mutations of src/runtime/hashed_map.js (ADR 1120).
import type { Mutation } from "../../mutations";

// Not mutated: that keys whose bytes differ are never compared. std's table
// compares such keys only where 7 bits of their random hashes meet, so no
// program can count on how often `eq` is asked of them.
export const mutations: Mutation[] = [
  {
    name: "hashed-keys-eq-unasked",
    breaks: "a search finds the first key held under the same bytes, never asking `eq`: of a `Hash` that hashes less than `==` compares, every key is one (ADR 1120)",
    file: "src/runtime/hashed_map.js",
    find: "    const entry = this.#under.get(bytes)?.find((held) => this.eq(key, held[0]));\n",
    replace: "    const entry = this.#under.get(bytes)?.[0];\n",
    tests: ["test/corpus.test.ts", "-t", "^keys_found_by_their_own_hash.rs$"],
  },
  {
    name: "hashed-keys-removed-still-found",
    breaks: "a key taken out stays under its bytes, so a search finds it after its `remove` (ADR 1120)",
    file: "src/runtime/hashed_map.js",
    find: "    under.splice(under.indexOf(entry), 1);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^keys_found_by_their_own_hash.rs$"],
  },
  {
    name: "hashed-keys-insert-keeps-new-key",
    breaks: "`insert` of a key it holds keeps the key given, where std's keeps the one it holds (ADR 1120)",
    file: "src/runtime/hashed_map.js",
    find: "      found.entry[1] = value;\n      return found.entry;\n",
    replace: "      found.entry[0] = key;\n      found.entry[1] = value;\n      return found.entry;\n",
    tests: ["test/corpus.test.ts", "-t", "^keys_found_by_their_own_hash.rs$"],
  },
  {
    name: "hashed-set-replace-keeps-held",
    breaks: "a hashed set's `replace` keeps the item it held, not the one given (ADR 1120)",
    file: "src/runtime/hashed_map.js",
    find: "    found.entry[0] = key;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^keys_found_by_their_own_hash.rs$"],
  },
  {
    name: "hashed-entry-handles-search",
    breaks: "a hashed map's `values_mut`, `iter_mut` and `retain` give handles that search for each key, running its `hash` where std's walk runs none (#1362)",
    file: "src/runtime/hashed_map.js",
    find: "    entryHandles(entries) {\n      return Array.from(entries, ([key]) => [key, $entryHandle(this.#keys.held(key))]);\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_entry_hashes_its_key_once.rs$"],
  },
];
