// Mutations of src/lower/std_types/values.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "option-as-mut-same",
    breaks: "`list.next.as_mut()` of a handle on `list.next` is the handle, not the `Option`",
    file: "src/lower/std_types/values.rs",
    find: "            Std::Pointee => self.through_refs(arg(), tys[0]).0,\n",
    replace: "            Std::Pointee => arg(),\n",
    tests: ["test/corpus.test.ts","-t","generic_mut_ref_kept"],
  },
];
