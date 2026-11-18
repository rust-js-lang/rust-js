// Mutations of src/lower/recognition/std_steps.rs (ADR 0093): the one
// catalog of std's steps, what each adapter and wrapper writes and asks of
// what it holds (#2000, #2112).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-steps-sized-fold-default",
    breaks: "a sized `&mut I`'s folds are read as the trait's default, not `I`'s `try_fold`s, so std's compiled `Rev` of `(&mut a)` drops a `take()`'s skipped items 4 then 5, where std drops them 5 then 4 (#2000, #2002)",
    file: "src/lower/recognition/std_steps.rs",
    find: "        } else if each(&format!(\"try_{method}\")) {\n",
    replace: "        } else if each(&format!(\"try_{method}\")) && false {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^lent_take_of_a_vec_folded_from_its_back with"],
  },
  {
    name: "std-steps-box-read-as-a-mut-ref",
    breaks: "a `Box<I>` is read as std's `&mut I`, so it passes on `I`'s own `fold`, by its `try_fold`, which std's `Box<I>` never calls (#2000, #1601)",
    file: "src/lower/recognition/std_steps.rs",
    find: "            Wrapper::BoxSized => \"Box<I>\",\n",
    replace: "            Wrapper::BoxSized => \"&mut I\",\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
  {
    name: "std-steps-unsized-read-as-sized",
    breaks: "an unsized `&mut I`, a `dyn`'s, is read as a sized one's, so its `fold` passes on `I`'s `try_fold`, where std's `&mut dyn Iterator` folds by `next`, the trait's default (#1991)",
    file: "src/lower/recognition/std_steps.rs",
    find: "            Wrapper::MutRefUnsized => \"&mut dyn I\",\n",
    replace: "            Wrapper::MutRefUnsized => \"&mut I\",\n",
    unitTests: "lower::recognition::std_steps::tests::wrappers_pass_on_as_std_does",
  },
];
