// Mutations of src/lower/analysis/back_stepped.rs (ADR 0545).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "enumerate-of-compiled-struct-stepped-unrecorded",
    breaks:
      "an enumeration of std's compiled `Fuse` of numbers whose own `next_back()` or `rfold` the crate calls isn't recorded stepped from its back: `$enumerate` of it, made knowing neither, is refused (#712)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        return (dropping_items(recognition, source.peel_refs(), &drops) || enumerates_a_compiled_struct(tcx, adapter))\n",
    replace: "        return (dropping_items(recognition, source.peel_refs(), &drops) || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_of_a_reversible_source.rs$"],
  },
  {
    name: "enumerate-of-compiled-struct-asked-unrecorded",
    breaks:
      "an enumeration of a held `peekable()` a `rev()` asks to be double-ended isn't made knowing its back where it's made: `$enumerate` of it is refused (#712)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            || enumerates_a_compiled_struct(tcx, held)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_of_a_reversible_source.rs$"],
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
    find: "    let source = (mapped.or(filtered).or(copied))?.peel_refs();\n",
    replace:
      "    let source = (mapped.or(filtered))?.peel_refs();\n",
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
    find: "                            stepped.extend(back_stepped_adapter(tcx, recognition, held, made_apart, &drops));\n",
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
    find: "                        if held != asked\n                            || given\n                            || held_rev\n                            || held_arg\n                            || dropping_items(recognition, held, &drops)\n",
    replace: "                        if true\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_range.rs$"],
    snapshots: true,
  },
  {
    name: "rev-of-dropping-map-unasked",
    breaks:
      "`rev()` of a map of a `Vec`'s items with a destructor, held in a variable, isn't made knowing its back where it's made, `$mapEnds`, but reversed after, `$rev($closing(..)).map(f)` (#205)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        if held != asked\n                            || given\n                            || held_rev\n                            || held_arg\n                            || dropping_items(recognition, held, &drops)\n",
    replace:
      "                        if held != asked\n                            || given\n                            || held_rev\n                            || held_arg\n",
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
    find: "            sources.push(held);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^back_stepped_adapter_sources.rs$"],
  },
  {
    name: "chain-sides-unstepped",
    breaks:
      "a chain's sides aren't made knowing their back where they're made: `v.into_iter().filter(p).chain(w).rev()` of droppers, whose `a` MIR holds in a variable for `w`'s drop flag, is refused, a `Filter` as a `DoubleEndedIterator` (#718)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            Some((a, b)) => (vec![a, b], chain.is_some()),\n",
    replace: "            Some(_) => (Vec::new(), chain.is_some()),\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_of_adapters_stepped_from_its_back.rs$"],
  },
  {
    name: "chain-side-a-unstepped",
    breaks:
      "a chain's first side isn't made knowing its back where it's made, only its second: a held `filter()` on side `a` of droppers stepped from the back is refused (#718)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            Some((a, b)) => (vec![a, b], chain.is_some()),\n",
    replace: "            Some((_, b)) => (vec![b], chain.is_some()),\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_of_adapters_stepped_from_its_back.rs$"],
  },
  {
    name: "zip-by-index-holding-map-unasked",
    breaks:
      "a zip read by index doesn't record its `b`, a map whose closure holds what drops: the map, held in a variable, is refused as an `ExactSizeIterator` (#775)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            if Some(callee) == zip\n",
    replace: "            if false\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_by_index_of_holding_maps.rs$"],
  },
  {
    name: "zip-by-index-any-map-asked",
    breaks:
      "a zip read by index records its `b` though its closure drops nothing: a map made in place `$mapEnds(..)` where it was `.map(f)`, a change of form (#775)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                if closure_drops(tcx, b, &drops) {\n",
    replace: "                if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^(zip_by_index|zip_by_index_of_holding_maps).rs$"],
    snapshots: true,
  },
  {
    name: "stepped-sources-any-adapter",
    breaks:
      "each held adapter a back-stepped one steps is marked, whatever it is: the crate's own iterator a held `filter` steps is made with its exact length, `$exactLen(..)`, which nothing asks (#218)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            if mapped_source(tcx, held).is_some()\n                || filtered_source(tcx, held).is_some()\n                || filter_mapped_source(tcx, held).is_some()\n                || copied_source(tcx, held).is_some()\n                || cloned_source(tcx, held).is_some()\n                || inspected_source(tcx, held).is_some()\n                || chained_sources(tcx, held).is_some()\n                || sides && counted_source(tcx, held).is_some()\n            {\n",
    replace: "            if true {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(back_stepped_adapter_sources|rev_in_place_through_adapters).rs$",
    ],
    snapshots: true,
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
  {
    name: "filter-map-unstepped-from-back",
    breaks: "a `filter_map` the crate steps from its back isn't one the analysis marks, so one held in a variable before it's stepped, a library's, isn't made knowing its back, and is refused (#473)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    let filtered = filtered_source(tcx, adapter).or_else(|| filter_mapped_source(tcx, adapter));\n",
    replace: "    let filtered = filtered_source(tcx, adapter);\n",
    tests: ["test/corpus.test.ts", "-t", "^filter_map_of_given_from_its_back.rs$"],
  },
  {
    name: "held-given-unrecorded",
    breaks:
      "an adapter held in a variable and given to what steps it from its back, `kept.rev()` of a `let kept = v.into_iter().filter_map(Some)`, isn't made knowing its back where it's made, and is refused (#484)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            || held_arg\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_filter_map_reversed.rs$"],
  },
  {
    name: "held-given-of-items-native",
    breaks:
      "a map of an array's items held in a variable and reversed is taken to step from its back by its own form, which a variable doesn't show, and is refused (#484)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            let made_apart = given || held_rev || held_arg;\n",
    replace: "                            let made_apart = given || held_rev;\n",
    tests: ["test/corpus.test.ts", "-t", "^held_filter_map_reversed.rs$"],
  },
  {
    name: "made-in-place-taken-held",
    breaks:
      "an adapter made where it's reversed, `v.iter().filter(p).rev()`, is taken to be held, and made knowing its back, `$rev($filterEnds(..))`, where its own form reverses the array's items, `v.toReversed().filter(p)` (#484)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                && !matches!(through(arg).kind, ExprKind::Call { .. })\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_filter_map_reversed.rs$"],
    snapshots: true,
  },
  {
    name: "held-through-borrows-unseen",
    breaks:
      "what a call gives, borrowed, `it.by_ref().rev()`, is taken to be held where it's given, and the map `it` holds, std's struct, is made knowing its back besides, `$knowing($knowing(it, ..), ..)` where it was `$knowing($lent(it), ..)` (#484)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                    ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,\n                    _ => break &body.thir[e],\n",
    replace: "                    _ => break &body.thir[e],\n",
    tests: ["test/corpus.test.ts", "-t", "^backs_given_behind_impls.rs$"],
    snapshots: true,
  },
  {
    name: "path-call-uncalled",
    breaks:
      "a function called by its path, `last(doubled)`, isn't found as the call of its callee, so an adapter held in a variable it's given, which it steps from its back, isn't made knowing it, and is refused (#484)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                } => Some((strip(&body.thir, fun), (fn_span, &**args))),\n",
    replace: "                } => Some((fun, (fn_span, &**args))),\n",
    tests: ["test/corpus.test.ts", "-t", "^held_filter_map_reversed.rs$"],
  },
  {
    name: "filter-of-a-rev-backless",
    breaks: "a `filter()` of a reversed `Vec`'s items with a `Drop`, `rev()`ed again and held, is JS's own `filter`, which has no back (#646)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        .map(|source| unreversed(source.peel_refs()))\n",
    replace: "        .map(|source| source.peel_refs())\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_skip_take_and_filter_stepped_from_back.rs$"],
  },
  {
    name: "rev-stepped-from-back-unrecorded",
    breaks:
      "a `rev()` whose own `rfold` or `nth_back` the crate calls isn't recorded stepped from its back: one MIR holds in a variable first, past `String::new()` or lent to `nth_back`, is refused, its form gone (#702)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    if reversed_source(tcx, adapter).is_some() {\n",
    replace: "    if false && reversed_source(tcx, adapter).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_flattened_from_its_back.rs$"],
  },
  {
    name: "chain-of-droppers-asked-unrecorded",
    breaks:
      "a chain of items with a destructor a `rev()` asks to be double-ended isn't made knowing its back where it's made: MIR holds it in a variable to drop them, so its `rev().fold()` is refused (#718)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            && chained_sources(tcx, held).is_some()\n",
    replace: "                            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_made_knowing_its_back.rs$"],
  },
  {
    name: "chain-under-back-stepped-map-unrecorded",
    breaks:
      "a chain a map stepped from its back holds isn't made knowing its back where it's made: MIR holds it in a variable, so the map's `rfold` of a chain of the crate's iterators is refused (#739)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                || chained_sources(tcx, held).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(held_chain_mapped_from_its_back|chain_under_a_held_map_from_its_back).rs$"],
  },
  {
    name: "inspect-of-dropping-items-not-back-stepped",
    breaks: "an `inspect()` of a `Vec`'s items with a destructor, held in a variable to drop them, isn't made knowing its back, and its `rfind` is refused (#751)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        .or_else(|| inspected_source(tcx, adapter))\n        .or_else(|| filtered_source(tcx, adapter))\n",
    replace: "        .or_else(|| filtered_source(tcx, adapter))\n",
    tests: ["test/corpus.test.ts", "-t", "^inspect_of_dropping_items_from_its_back.rs$"],
  },
  {
    name: "zip-not-back-stepped",
    breaks: "a zip held in a variable and stepped from its back isn't made knowing its back where it's made, and its `next_back` is refused (#740)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    if zipped_sources(tcx, adapter).is_some() {\n        return Some(tcx.erase_and_anonymize_regions(adapter));\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_from_its_back.rs$"],
  },
  {
    name: "inspect-of-a-dropping-closure-not-back-stepped",
    breaks: "an `inspect()` whose closure owns a value with a destructor, held in a variable to drop it, isn't made knowing its back, and its `rfind` is refused (#780)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    (mapped_source(tcx, adapter).is_some() || inspected_source(tcx, adapter).is_some())\n",
    replace: "    mapped_source(tcx, adapter).is_some()\n",
    tests: ["test/corpus.test.ts", "-t", "^move_closure_adapter_from_its_back.rs$"],
  },
  {
    name: "zip-sides-not-back-stepped",
    breaks: "a zip stepped from its back doesn't make its sides knowing theirs where they're made, and a `cloned()` side MIR holds in a variable is refused (#792)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        let (held, sides) = match chain.or_else(|| zipped_sources(tcx, ty)) {\n",
    replace: "        let (held, sides) = match chain {\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_cloned_items_whose_clone_runs.rs$"],
  },
  {
    name: "cloned-source-not-recorded",
    breaks: "a `cloned()` a back-stepped adapter holds isn't made knowing its back where it's made (#792)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                || cloned_source(tcx, held).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_cloned_items_whose_clone_runs.rs$"],
  },
  {
    name: "enumerate-of-a-dropping-map-unrecorded",
    breaks: "an enumeration of a map of a `Vec`'s items with a destructor, held in a variable to drop them, isn't made knowing its back where it's made, and its `next_back` is refused (#764)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        return (dropping_items(recognition, source.peel_refs(), &drops) || enumerates_a_compiled_struct(tcx, adapter))\n",
    replace: "        return (false || enumerates_a_compiled_struct(tcx, adapter))\n",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_made_knowing_read_again.rs$"],
  },
  {
    name: "counted-given-held-unrecorded",
    breaks:
      "a `skip(n)` or a `take(n)` given held to what steps it from its back isn't recorded, so it's made with no back: `held.rev()` of a `let held = v.into_iter().skip(1)` is refused (#718)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        return given.then(|| tcx.erase_and_anonymize_regions(adapter));\n",
    replace: "        return false.then(|| tcx.erase_and_anonymize_regions(adapter));\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
  },
  {
    name: "counted-chain-side-unstepped",
    breaks:
      "a chain's `skip(n)` or `take(n)` side, which MIR holds in a variable before the chain is made, isn't made knowing its back: `ds().into_iter().skip(1).chain(w).rev()` is refused (#718)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                || sides && counted_source(tcx, held).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
  },
  {
    name: "held-beneath-unrecorded",
    breaks:
      "an adapter held in a variable, then given to adapters made in place that a `rev()` steps from its back, isn't recorded: `c.map(f).rev()` of a `let c = a.chain(b)` is refused (#790)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                    && let Some(beneath) = held_beneath(hidden)\n",
    replace: "                    && let Some(beneath) = held_beneath(hidden).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^held_beneath_adapters_from_its_back.rs$"],
  },
  {
    name: "held-beneath-chain-as-adapter",
    breaks:
      "a chain held beneath adapters made in place is recorded as a map or a filter would be, which records no chain: `c.map(f).rev()` of a `let c = a.chain(b)` is refused (#790)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                    match chained_sources(tcx, beneath) {\n                        Some(_) => {\n",
    replace: "                    match chained_sources(tcx, beneath).filter(|_| false) {\n                        Some(_) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^held_beneath_adapters_from_its_back.rs$"],
  },
  {
    name: "held-beneath-through-one",
    breaks:
      "what's held beneath adapters made in place is looked for only as what the call is given itself, never through them: `c.map(f).rev()` of a `let c = a.chain(b)` is refused (#790)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            (source, made) = (first, true);\n",
    replace: "                            (source, made) = (first, true);\n                            return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^held_beneath_adapters_from_its_back.rs$"],
  },
  {
    name: "zip-of-droppers-unrecorded",
    breaks:
      "a zip with a side of a `Vec`'s items with a destructor, made in place and held in a variable as it's made, isn't recorded as stepped from its back, so its `rev()` is refused (#812)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        return array_of_drops(a.peel_refs()) || array_of_drops(b.peel_refs());\n",
    replace: "        return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
];
