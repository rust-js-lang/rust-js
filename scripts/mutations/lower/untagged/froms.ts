// Mutations of src/lower/untagged/froms.rs (ADR 0093): a `From` into an
// untagged enum, which calls of one are the value it's given (ADR 0214).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "untagged-from-unchecked",
    breaks:
      "a `From` into an untagged enum that does more than make the variant is accepted, and skipped where called",
    file: "src/lower/untagged/froms.rs",
    find: "    (!variant_of_param).then(|| {\n",
    replace: "    (variant_of_param && !variant_of_param).then(|| {\n",
    tests: ["test/diagnostics.test.ts", "-t", "From into an untagged enum"],
  },
];
