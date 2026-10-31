// Mutations of src/lower/mir/fields_read.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-read-of-the-reference",
    breaks: "a call of a method that gives a field is read as the field of the reference it's given, not of what that's to, `this.counter` of `self.counter()` (rd283)",
    file: "src/lower/mir/fields_read.rs",
    find: "            projection.push(PlaceElem::Deref);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_methods.rs$"],
  },
];
