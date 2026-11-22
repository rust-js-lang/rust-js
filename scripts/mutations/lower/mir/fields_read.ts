// Mutations of crates/rust-js-mir/src/fields_read.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-read-uninlined",
    breaks:
      "a call of an untagged enum's method that reads each variant's field stays a call, `Parent.children(parentInstance).push(child)`, where upstream reads `parentInstance.children` (rd317)",
    file: "crates/rust-js-mir/src/fields_read.rs",
    find: "        let [value] = values else { return Ok(None) };\n",
    replace: "        let [value] = values else { return Ok(None) };\n        if values.len() == 1 {\n            return Ok(None);\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_field_of_each_variant.rs$"],
    snapshots: true,
  },
];
