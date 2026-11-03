// Mutations of src/lower/mir/keyed_writes.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "keyed-write-others-kept",
    breaks: "a keyed enum written whole as a variant leaves the other variants' keys, so the old one is still told (#504)",
    file: "src/lower/mir/keyed_writes.rs",
    find: "                for key in keys.iter().filter(|key| !set.iter().any(|(own, _)| own == *key)) {\n",
    replace: "                for key in keys.iter().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_enum.rs$"],
  },
  {
    name: "keyed-write-given-others-kept",
    breaks: "a keyed enum written whole of a value no literal says leaves the old variant's keys (#504)",
    file: "src/lower/mir/keyed_writes.rs",
    find: "                for key in keys {\n                    out.push(delete(key).at(js_span));\n",
    replace: "                for key in keys.iter().filter(|_| false) {\n                    out.push(delete(key).at(js_span));\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_enum.rs$"],
  },
  {
    name: "keyed-write-effects-after-deletes",
    breaks: "a keyed enum's whole write computes its new fields after it deletes the old keys, which what they call may read (#504)",
    file: "src/lower/mir/keyed_writes.rs",
    find: "                    let value = match value.has_effects() {\n",
    replace: "                    let value = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_enum.rs$"],
  },
];
