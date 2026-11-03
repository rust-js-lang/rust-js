// Mutations of src/lower/pipeline/field_reads.rs (ADR 0093): an untagged
// enum's method that reads each variant's field, lowered before its
// callers (rd317, ADR 0214 amended).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-read-lowered-in-order",
    breaks:
      "an untagged enum's method that reads each variant's field is lowered where it's declared, after a caller before it, which calls it, `Parent.children(parentInstance)` (rd317)",
    file: "src/lower/pipeline/field_reads.rs",
    find: "    work.sort_by_key(|&(id, _, copy)| !(methods.contains(&id) && copy.is_none()));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^untagged_field_of_each_variant.rs$"],
    snapshots: true,
  },
];
