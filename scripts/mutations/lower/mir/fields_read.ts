// Mutations of src/lower/mir/fields_read.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-read-of-the-reference",
    breaks:
      "a call of a method that gives a field is read as the field of the reference it's given, not of what that's to, `this.counter` of `self.counter()` (rd283)",
    file: "src/lower/mir/fields_read.rs",
    find: "            projection.push(PlaceElem::Deref);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_methods.rs$"],
  },
  {
    name: "field-read-uninlined",
    breaks:
      "a call of an untagged enum's method that reads each variant's field stays a call, `Parent.children(parentInstance).push(child)`, where upstream reads `parentInstance.children` (rd317)",
    file: "src/lower/mir/fields_read.rs",
    find: "        let [value] = values else { return Ok(None) };\n",
    replace: "        let [value] = values else { return Ok(None) };\n        if values.len() == 1 {\n            return Ok(None);\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_field_of_each_variant.rs$"],
    snapshots: true,
  },
];
