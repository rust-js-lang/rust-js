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
    find: "        let form = match own || hint_runs || self.drops(gives) != crate::lower::drops::Drops::Nothing {\n",
    replace: "        let form = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_count_takes_each_inner_count.rs$"],
  },
  {
    name: "flat-map-of-dropped-items-js",
    breaks:
      "`flat_map` of the crate's iterators that give items with a destructor is JS's `flatMap`, whose `last` drops each as the next comes, not each inner one's own `last` (#645)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "        let form = match own || hint_runs || self.drops(gives) != crate::lower::drops::Drops::Nothing {\n",
    replace: "        let form = match own || hint_runs {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_last_takes_each_inner_last.rs$"],
    snapshots: true,
  },
  {
    name: "in-place-somes-filter-of-any-source",
    breaks: "the `Some`s as JS writes them, `.filter((item) => item != null)`, read as chosen of any source, so the net runs them only where std's row is theirs and never where a source's hint runs the crate's code, which a crate-wide fact can't then hide (#2112, the lead's guard)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: '                ("FilterMap", HintRunsNothing),\n',
    replace: '                ("FilterMap", Always),\n',
    unitTests: "lower::iterators::in_place_forms::tests::in_place_forms",
  },
  {
    name: "in-place-somes-filter-flatten-of-any-source",
    breaks: "the `Some`s as JS writes them read as chosen for a flatten of `Option`s of any source, so the net runs them where a crate's `nth` between is asked otherwise by their `try_fold` than by std's `next` (#2205)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: '                ("Flatten<Option>", StepsRunNothing),\n',
    replace: '                ("Flatten<Option>", Always),\n',
    unitTests: "lower::iterators::in_place_forms::tests::in_place_forms",
  },
  {
    name: "in-place-some-items-not-a-flatten",
    breaks: "a flatten's items of its `Option`s named for `FlatMap<Option>` alone, so the net never runs them against `Flatten<Option>`'s row (#2205)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: '                &[("Flatten<Option>", Always), ("FlatMap<Option>", Always)]\n',
    replace: '                &[("FlatMap<Option>", Always)]\n',
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
    find: "                    IterComb::FilterMap => Some(match hint_runs {\n",
    replace: "                    IterComb::FilterMap => Some(match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_counted_asks_no_hint.rs$"],
  },
  {
    name: "somes-of-any-type",
    breaks:
      "the `Some`s of a variable whose type's `size_hint` runs none of the crate's code, `take.map(f)` of a slice's, are `$somes(take.map(f))`, where upstream writes JS's `filter((item) => item != null)` (#2199)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "                    IterComb::FilterMap => Some(match hint_runs {\n",
    replace: "                    IterComb::FilterMap => Some(match true {\n",
    tests: ["test/corpus.test.ts", "-t", "^(adapter_functions_dropped|fold_drops_moved_function_first).rs$"],
    snapshots: true,
  },
  {
    name: "flatten-of-options-a-filter",
    breaks:
      "a flatten of `Option`s, or a `flat_map` to them, made in place over the crate's iterator is a filter of the `Some`s, which steps its source by `try_fold`, so a `skip(1)` between asks the crate's `nth(0)` and a `next` where std's `Flatten` asks `nth(1)`, and a `step_by`'s first step asks no `nth(0)` (#2205)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "                let items = match hint_runs || recognition.own_nth_within(receiver_ty) {\n",
    replace: "                let items = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_steps_its_source_by_next.rs$"],
  },
  {
    name: "flatten-of-options-items-of-any-type",
    breaks:
      "a flatten of `Option`s of a source whose steps run none of the crate's code, a slice's, is `$someItems(..)`, where JS writes `filter((item) => item != null)` (#2205)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "                let items = match hint_runs || recognition.own_nth_within(receiver_ty) {\n",
    replace: "                let items = match true {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_counted_asks_no_hint.rs$"],
    snapshots: true,
  },
  {
    name: "flatten-of-hinting-inners-js-flat-map",
    breaks:
      "a flatten of the crate's iterators with their own `size_hint` is JS's `flatMap`, which has no hint, so a filter's `count` of it never asks theirs, where std's `FlattenCompat::size_hint` does (#2207)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "        let form = match own || hint_runs || self.drops(gives) != crate::lower::drops::Drops::Nothing {\n",
    replace: "        let form = match own || self.drops(gives) != crate::lower::drops::Drops::Nothing {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_asks_its_inner_iterators_hint.rs$"],
  },
  {
    name: "flatten-of-dyn-inners-js-flat-map",
    breaks:
      "a flatten of `Box<dyn Iterator>`s is JS's `flatMap`, which has no hint, so a filter's `count` of it never asks what the boxes hold, the crate's iterators with their own `size_hint` (#2207)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "        let hint_runs = self.recognition().hint_reaches_own_where(made, true);\n",
    replace: "        let hint_runs = self.recognition().hint_reaches_own_where(made, false);\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_asks_its_inner_iterators_hint.rs$"],
  },
  {
    name: "hint-knowing-form-steps-its-back",
    breaks: "the form of a flatten made knowing its hint alone is `$flattenEnds`, which steps what it holds from its back, which a hint asks none of (#2209)",
    file: "src/lower/iterators/in_place_forms.rs",
    find: "        \"Flatten\" | \"FlatMap\" => InPlaceForm::FlatMap,\n",
    replace: "        \"Flatten\" | \"FlatMap\" => InPlaceForm::FlattenEnds,\n",
    unitTests: "lower::iterators::in_place_forms::tests::hint_knowing_forms_stand_for_every_source",
  },
];
