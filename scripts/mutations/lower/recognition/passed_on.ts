// Mutations of src/lower/recognition/passed_on.rs (ADR 0093): the one
// catalog of what std's `&mut I` and `Box<I>` pass on (#2000).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "passed-on-sized-fold-default",
    breaks: "the catalog says a sized `&mut I` folds from its back by the trait's default, not `I`'s `try_rfold`, so std's compiled `Rev` of `(&mut a)` drops a `take()`'s skipped items 4 then 5, where std drops them 5 then 4 (#2000, #2002)",
    file: "src/lower/recognition/passed_on.rs",
    find: "        (\"rfold\", [ByTry, Default, Default, Default]),\n",
    replace: "        (\"rfold\", [Default, Default, Default, Default]),\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^lent_take_of_a_vec_folded_from_its_back with"],
  },
  {
    name: "passed-on-box-folds-its-own",
    breaks: "the catalog says a `Box<I>` passes on `I`'s own `fold`, which std's `Box<I>` never calls (#2000, #1601)",
    file: "src/lower/recognition/passed_on.rs",
    find: "        (\"fold\", [ByTry, Default, Default, Default]),\n",
    replace: "        (\"fold\", [ByTry, Default, Own, Own]),\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
];
