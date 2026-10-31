// Mutations of src/lower/analysis/back_stepped.rs (ADR 0545).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "enumerate-of-droppers-unrecorded",
    breaks:
      "an enumeration of items with a destructor whose `next_back()` the crate calls isn't recorded stepped from its back: `rev()` of one beside it, flushed into a variable, is refused (#383)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        return item\n            .is_some_and(&drops)\n",
    replace: "        return item\n            .is_some_and(|_| false)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^enumerate_of_droppers_from_back.rs$",
    ],
  },
  {
    name: "closing-map-asked-unrecorded",
    breaks:
      "a map whose closure has a destructor, held in a variable, given where generic code asks its back and length isn't made knowing them where it's made: refused as an `ExactSizeIterator` (#385)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            || closure_drops(tcx, held, &drops)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^closing_maps_given_counted.rs$"],
  },
  {
    name: "closing-map-length-unasked",
    breaks:
      "a map whose closure has a destructor given where generic code asks its length alone, `lens(v.iter().map(f))`, isn't made knowing it: refused (#385)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                if length && !closure_drops(tcx, asked, &drops) {\n",
    replace: "                if length {\n",
    tests: ["test/corpus.test.ts", "-t", "^closing_maps_given_counted.rs$"],
  },
  {
    name: "generic-copied-back-unstepped",
    breaks:
      "a copy given to generic code as `Copied<I>` and stepped from its back there isn't made knowing its back: `next_back()` of a lazy or generic `Copied` is refused (#385)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    let source = (mapped.or_else(|| filtered_source(tcx, adapter)).or(copied))?.peel_refs();\n",
    replace:
      "    let source = (mapped.or_else(|| filtered_source(tcx, adapter)))?.peel_refs();\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_given_to_generic_backs.rs$"],
  },
  {
    name: "generic-back-steps-uncarried",
    breaks:
      "a filter given to generic code as `Filter<I, P>` and stepped from its back there isn't made knowing its back where its caller makes it: `TypeError: it.back is not a function` (#388)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    for _ in 0..32 {\n",
    replace: "    for _ in 0..0 {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^adapters_given_to_generic_backs.rs$",
    ],
  },
  {
    name: "filter-of-items-native",
    breaks:
      "a filter of an array's items stepped from its back is taken to have a form of its own that does it, and is refused: `v.iter().filter(p).next_back()` (ADR 0990)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    let native = (mapped.is_some() || copied.is_some())\n        && !given\n",
    replace: "    let native = !given\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(next_back_lazy|held_adapters_by_ref).rs$",
    ],
  },
  {
    name: "back-stepped-none",
    breaks:
      "no map the crate steps from its back is seen, and one held before it's stepped is refused",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                false => stepped.extend(back_stepped_adapter(tcx, recognition, adapter, false, &drops)),\n",
    replace: "                false => {}\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(back_stepped_adapter_sources|flatten_from_its_back).rs$",
    ],
  },
  {
    name: "held-adapter-not-back-stepped",
    breaks:
      "a filter or a map a type holds, asked to be double-ended by the call that asks it of that type, isn't made knowing its back",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            stepped.extend(back_stepped_adapter(tcx, recognition, held, given || held_rev, &drops));\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(filters_held_double_ended|returned_impl_ends).rs$",
    ],
  },
  {
    name: "held-adapter-opaque",
    breaks:
      "what an `impl DoubleEndedIterator` is isn't seen, so a filter behind one isn't made knowing its back",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                let asked = recognition.reveal(hidden);\n",
    replace: "                let asked = hidden;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(filters_held_double_ended|returned_impl_ends).rs$",
    ],
  },
  {
    name: "held-adapter-through-adapters",
    breaks:
      "what a std adapter holds, `filter(p).map(f).rev()`'s filter, is made knowing its back where it's made, not by its `rev()`",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        walk.skip_current_subtree();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_range.rs$"],
    snapshots: true,
  },
  {
    name: "held-adapter-asked-itself",
    breaks:
      "the adapter a `rev()` asks to be double-ended is made knowing its back where it's made, not by its `rev()`",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        if held != asked\n                            || given\n                            || held_rev\n                            || dropping_items(recognition, held, &drops)\n",
    replace: "                        if true\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_range.rs$"],
    snapshots: true,
  },
  {
    name: "rev-of-dropping-map-unasked",
    breaks:
      "`rev()` of a map of a `Vec`'s items with a destructor, held in a variable, isn't made knowing its back where it's made, `$mapEnds`, but reversed after, `$rev($closing(..)).map(f)` (#205)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        if held != asked\n                            || given\n                            || held_rev\n                            || dropping_items(recognition, held, &drops)\n",
    replace:
      "                        if held != asked\n                            || given\n                            || held_rev\n",
    tests: ["test/corpus.test.ts", "-t", "^(rev_lazy_drops|std_rev).rs$"],
    snapshots: true,
  },
  {
    name: "flatten-back-stepped-unseen",
    breaks:
      "a flatten held in a variable, then stepped from its back, is made as it is, not knowing its back",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    if flattened_source(tcx, adapter).is_some() {",
    replace: "    if false {",
    tests: ["test/corpus.test.ts", "-t", "^flatten_from_its_back.rs$"],
  },
  {
    name: "given-impl-not-back-stepped",
    breaks:
      "an adapter a function gives behind an impl is made knowing its back",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                let given = hidden != asked;",
    replace: "                let given = false;",
    tests: ["test/corpus.test.ts", "-t", "^returned_impl_ends.rs$"],
  },
  {
    name: "stepped-sources-unmarked",
    breaks:
      "a map held in a variable that a back-stepped map steps isn't made knowing its back, and is refused (#218)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    with_stepped_sources(tcx, stepped)\n",
    replace: "    stepped\n",
    tests: ["test/corpus.test.ts", "-t", "^back_stepped_adapter_sources.rs$"],
  },
  {
    name: "stepped-sources-one-deep",
    breaks:
      "a map two adapters inside a back-stepped one isn't made knowing its back (#218)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            source = back_stepped_source(tcx, held);\n",
    replace: "            source = None;\n",
    tests: ["test/corpus.test.ts", "-t", "^back_stepped_adapter_sources.rs$"],
  },
  {
    name: "stepped-sources-any-adapter",
    breaks:
      "a held `filter_map` a back-stepped map steps is marked, and steps a JS iterator with no back, which throws, where it's refused (#218)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            if mapped_source(tcx, held).is_some()\n                || filtered_source(tcx, held).is_some()\n                || copied_source(tcx, held).is_some()\n                || inspected_source(tcx, held).is_some()\n            {\n",
    replace: "            if true {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(back_stepped_adapter_sources|held_filter_map_back_refused).rs$",
    ],
  },
  {
    name: "held-rev-source-unstepped",
    breaks:
      "the source of a held `rev()` isn't made knowing its back where it's made",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            let held_rev = Some(callee) == rev",
    replace: "            let held_rev = false && Some(callee) == rev",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(range_mapped_by_a_function|held_revs_of_lazy_sources).rs$",
    ],
  },
];
