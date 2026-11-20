// The forms std's adapters and consumers are made as in place, and the std
// type each stands for (src/lower/iterators/in_place_forms.rs, #2112):
// the one decision of each form chosen between several.
import type { Mutation } from "../../../mutations";
export const mutations: Mutation[] = [
  {
    name: "flat-map-js-close-order",
    breaks:
      "`flatten` of inner iterators that drop, of a `map`'s source that drops too, closes the inner one first, as JS's `flatMap` does (#644)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "        let form = match own || self.drops(gives) != crate::lower::drops::Drops::Nothing {\n",
    replace: "        let form = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_count_takes_each_inner_count.rs$"],
  },
  {
    name: "flat-map-of-dropped-items-js",
    breaks:
      "`flat_map` of the crate's iterators that give items with a destructor is JS's `flatMap`, whose `last` drops each as the next comes, not each inner one's own `last` (#645)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "        let form = match own || self.drops(gives) != crate::lower::drops::Drops::Nothing {\n",
    replace: "        let form = match own {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_last_takes_each_inner_last.rs$"],
    snapshots: true,
  },
  {
    name: "in-place-somes-filter-of-any-source",
    breaks: "the `Some`s as JS writes them, `.filter((item) => item != null)`, read as chosen of any source, so the net runs them only where std's row is theirs and never where a source's hint runs the crate's code, which a crate-wide fact can't then hide (#2112, the lead's guard)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "            InPlaceForm::SomesFilter => Where::HintRunsNothing,\n",
    replace: "",
    unitTests: "lower::iterators::in_place_forms::tests::in_place_forms",
  },
  {
    name: "in-place-somes-not-a-flatten",
    breaks: "the `Some`s' forms named for `FilterMap` and `FlatMap<Option>` alone, so the net never runs them against `Flatten<Option>`'s row, whose `next` is its source's `next` (#2205)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: '                &["FilterMap", "Flatten<Option>", "FlatMap<Option>"]\n',
    replace: '                &["FilterMap", "FlatMap<Option>"]\n',
    unitTests: "lower::iterators::in_place_forms::tests::in_place_forms",
  },
  {
    name: "step-by-past-a-skip-filtered",
    breaks:
      "a `step_by` whose first step is a `skip`'s `nth` of a reversed `Vec` is a `filter`, whose fold reaches `Skip::fold`, dropping back to front (#294)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "            && !recognition.steps_past_a_slice(receiver_ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^step_by_compiled_source_nth.rs$"],
  },
  {
    name: "nth-own-by-drop",
    breaks: "nth of what reaches the crate's own nth is its nth",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "                match self\n                    .recognition()\n                    .reaches_its_own(receiver_ty, OwnStep::Nth, Reach::Sure)\n                {\n",
    replace: "                match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "step-by-of-slice-filtered",
    breaks:
      "`rev().step_by(n)` of a `Vec` is a filter dropping each it steps over from the back, not `$stepBy` asking its `nth`'s slice (#234)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "            && !recognition.nths_a_slice(receiver_ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_skipped_from_back_drops.rs$"],
  },
  {
    name: "step-by-slice-in-order-filtered",
    breaks: "a `step_by(n)` of a `Vec`'s items whose drop may panic is a filter by index, each it passes dropped as it's rejected, so a panic there drops the fold's accumulator before the rest of std's slice (#668)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "                && recognition.nths_a_slice_in_order(receiver_ty))\n",
    replace: "                && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^nth_passed_drops_unwind.rs$"],
  },
  {
    name: "step-by-in-place-of-panicking-steps",
    breaks: "a `step_by()` of a source whose steps may panic is made in place as a filter by index, whose first step is inside its source's fold, so a panic there drops the accumulator before the consumer's closure, where std's `StepBy` drops the closure first (#773)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "            && !self.steps_may_panic(receiver_ty.peel_refs())\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(skip_while_first_step_unwinds|own_fold_and_count_where_adapters_call_them).rs$"],
  },
  {
    name: "step-by-of-unbounded-as-filter",
    breaks:
      "a `step_by(n)` of what no array's items bound is a filter of every `n`th, stepped by `next` where std's `StepBy` steps it by `nth`: a source that gives more after a `None` goes on, the crate's own `nth` is never asked, a range steps each number, a zip's or a flatten's drops come in another order, and its hint, a filter's, asks no room where std panics `capacity overflow` (#161, #214, #231, #654, #668, #1046)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "        recognition.bounded_by_items(receiver_ty)\n",
    replace: "        true\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(counted_iterators_held_ask_room|restarting_iterators|own_nth_through_adapters|range_steps_anywhere|reversed_vec_rest_through_zip_and_enumerate).rs$",
    ],
    snapshots: true,
  },
  {
    name: "somes-never",
    breaks:
      "the `Some`s of a source whose `size_hint` runs the crate's code are JS's filter, whose own `count` in the runtime is `Filter::count`'s, so generic code counting a `filter_map` of the crate's iterator asks its `size_hint`, which std's `FilterMap` never asks (#2199)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "                let somes = match self.recognition().hint_reaches_own_where(receiver_ty, true) {\n",
    replace: "                let somes = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_counted_asks_no_hint.rs$"],
  },
  {
    name: "somes-of-any-type",
    breaks:
      "the `Some`s of a variable whose type's `size_hint` runs none of the crate's code, `take.map(f)` of a slice's, are `$somes(take.map(f))`, where upstream writes JS's `filter((item) => item != null)` (#2199)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "                let somes = match self.recognition().hint_reaches_own_where(receiver_ty, true) {\n",
    replace: "                let somes = match true {\n",
    tests: ["test/corpus.test.ts", "-t", "^(adapter_functions_dropped|fold_drops_moved_function_first).rs$"],
    snapshots: true,
  },
];
