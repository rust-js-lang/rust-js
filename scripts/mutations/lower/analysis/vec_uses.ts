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
  {
    name: "own-reader-shows-no-capacity",
    breaks: "a crate whose reader a library reads into a `Vec` it made keeps no capacity, so the reader sees reads of none (#1297)",
    file: "src/lower/analysis/vec_uses.rs",
    find: "        id.is_local()\n            && tcx.trait_impl_of_assoc(id)",
    replace: "        false\n            && tcx.trait_impl_of_assoc(id)",
    tests: [
      "test/crates.test.ts",
      "-t",
      "crate_readers_across_crates"
    ]
  },
];
