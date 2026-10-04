// Mutations of src/lower/maps.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "value-keys-by-identity",
    breaks: "a `HashMap<Point, _>` is a plain `Map`, which finds a struct key by identity, and never the equal one asked for",
    file: "src/lower/maps.rs",
    find: "        let by_value = key.is_some_and(|key| self.is_value_key(key) && !self.is_js_key(key));",
    replace: "        let by_value = false && key.is_some_and(|key| self.is_value_key(key) && !self.is_js_key(key));",
    tests: ["test/corpus.test.ts", "-t", "value_keys"],
  },
  {
    name: "map-index-message",
    breaks: "`m[&k]` of a missing key panics with `key not found`, not Rust's `no entry found for key`",
    file: "src/lower/maps.rs",
    find: 'vec![value, Expr::str("no entry found for key")]',
    replace: 'vec![value, Expr::str("key not found")]',
    tests: ["test/corpus.test.ts", "-t", "map_index_missing"],
  },
];
