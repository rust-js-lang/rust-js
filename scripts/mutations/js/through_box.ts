// Mutations of src/js/through_box.rs: a call of a `&mut`'s dictionary
// through its box, folded to the entry of what it's to (#1470).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "through-box-call-unfolded",
    breaks: "a direct call of a `&mut`'s dictionary through its box stays `$throughBox(TAsRefStr, { asRef: [1] }).asRef(held)`, where the entry of what it's to, `TAsRefStr.asRef(held.value)`, does the same (#1470)",
    file: "src/js/through_box.rs",
    find: "    let ([dictionary, entries], Some(\"$throughBox\")) = (made.as_slice(), helper.helper_name()) else {\n",
    replace: "    let ([dictionary, entries], Some(\"$throughBoxNot\")) = (made.as_slice(), helper.helper_name()) else {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_to_its_own_type_parameter_through_std_impls\\.rs$"],
    snapshots: true,
  },
];
