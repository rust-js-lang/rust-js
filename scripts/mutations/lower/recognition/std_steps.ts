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
  {
    name: "asks-of-one-run-alone",
    breaks:
      "what std's adapter asks of what it holds is read from the catalog's first run alone, so a step asked only where its source is longer, a `skip`'s `last`'s `nth`, is taken as unasked (#2112)",
    file: "src/lower/recognition/std_steps.rs",
    find: "        .into_iter()\n        .flatten()\n    {\n",
    replace: "        .into_iter()\n        .flatten()\n        .take(1)\n    {\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_hint_held.rs$"],
    snapshots: true,
  },
  {
    name: "asks-of-a-default-step",
    breaks:
      "a step std's adapter doesn't write, the trait's default, is taken to ask what its own steps ask of its source, so a `map()`'s `last`, its `fold`'s, reaches the crate's own `fold` as its `last` (#2112)",
    file: "src/lower/recognition/std_steps.rs",
    find: "    if !writes(name, step) {\n        return asked;\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
    snapshots: true,
  },
  {
    name: "lending-of-the-outer-ref-only",
    breaks: "a `&mut` of a `&mut dyn Iterator` lends as a sized `&mut I`, by its first referent alone, so `(&mut d).find(..)` of a `skip(1)` reaches its own `try_fold`, which asks the crate's `nth(0)` where std's default asks `nth(1)` (#2180)",
    file: "src/lower/recognition/std_steps.rs",
    find: "        while let ty::Ref(_, to, _) = *held.kind() {\n            held = to;\n        }\n",
    replace: "        if let ty::Ref(_, to, _) = *held.kind() {\n            held = to;\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_dyn_lent_steps_as_std_does.rs$"],
  },
  {
    name: "lent-dyn-as-sized",
    breaks: "a `&mut dyn` iterator is lent as a sized one's, `$lent(it)`, so its `rfold` is its iterator's `try_rfold`, where std's unsized `&mut I` takes the default (#1991; `Wrapper::lending`'s since #2180)",
    file: "src/lower/recognition/std_steps.rs",
    find: "            false => Wrapper::MutRefUnsized,\n",
    replace: "            false => Wrapper::MutRefSized,\n",
    tests: ["test/corpus.test.ts", "-t", "^lent_iterator_folded_as_std_passes_it_on.rs$"],
  },
];
