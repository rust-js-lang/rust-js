// Mutations of src/lower/analysis/vec_uses.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "changed-vecs-without-lists",
    breaks: "a clone of a `LinkedList` changed after is the same array, as only a `Vec`'s change is seen",
    file: "src/lower/analysis/vec_uses.rs",
    find: "                && matches!(ty.kind(), ty::Adt(adt, _) if crate::lower::recognition::is_vec_like_def(tcx, adt.did()))",
    replace:
      "                && matches!(ty.kind(), ty::Adt(adt, _) if crate::lower::recognition::is_std_def(tcx, adt.did(), crate::lower::recognition::StdItem::Vec))",
    tests: ["test/corpus.test.ts", "-t", "^linked_list_clone.rs$"],
  },
];
