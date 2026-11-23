// Mutations of crates/rust-js-program/src/runtime/json_codec.js (ADR 0459).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-codec-swallows-errors",
    breaks: "a hand-written impl's `Err` is read as its value, and serde_json's error never comes",
    file: "crates/rust-js-program/src/runtime/json_codec.js",
    find: "    if (result.TAG === \"Ok\") return result._0;\n",
    replace: "    return result._0;\n",
    tests: ["test/serde.test.ts", "-t", "hand-written"],
  },
  {
    name: "json-present-reads-missing",
    breaks: "a field's own reader is called for a field that isn't there, so `deserialize_with = \"Option::deserialize\"` reads `{}` as `None` (#2382)",
    file: "crates/rust-js-program/src/runtime/json_codec.js",
    find: "    if (json instanceof $JsonMissing) throw $jsonError(`missing field \\`${json.name}\\``);\n",
    replace: "",
    tests: ["test/serde-attributes.test.ts", "-t", "its own reader that isn't there"],
  },
];
