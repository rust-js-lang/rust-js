// Mutations of src/lower/analysis/back_stepped.rs (ADR 0545).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "given-whole-call-unstepped",
    breaks:
      "what a closure gives back that its caller gives whole to `rfold` isn't made knowing its back there, whose step finds none (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                if !destination.projection.is_empty() || !stepped.contains_key(&at) && !asked.contains(&at) {\n",
    replace: "                if !destination.projection.is_empty() || !stepped.contains_key(&at) {\n",
    tests: ["test/corpus.test.ts", "-t", "^filter_given_back_stepped_from_its_back.rs$"],
  },
  {
    name: "held-rev-source-unmarked",
    breaks:
      "what a `rev()` held in a variable is given, std's `Rev` stepping it by `next_back`, isn't made knowing its back (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                if callee == rev\n",
    replace: "                if false\n",
    tests: ["test/corpus.test.ts", "-t", "^held_revs_of_lazy_sources.rs$"],
  },
  {
    name: "reborrow-lends-nothing",
    breaks:
      "a reborrow of a `&mut` of a stepped local, `&mut (*_r)`, isn't taken for what it lends, so a value written beside it isn't made knowing its back (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            if matches!(of.projection.as_slice(), [mir::ProjectionElem::Deref])\n                                && let Some(&lends) = lent.get(&of.local) =>\n",
    replace: "                            if false\n                                && let Some(&lends) = lent.get(&of.local) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_values_meet.rs$"],
  },
  {
    name: "values-given-beside-unmet",
    breaks:
      "a value given beside a `&mut` of a stepped local, which the callee may write there, isn't made knowing its back (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        if !other_lent && Some(other) != local && other_ty == ty {\n",
    replace: "                        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_values_meet.rs$"],
  },
  {
    name: "struct-held-source-untyped",
    breaks:
      "the source a std struct holds, which asks its dictionary by type alone, isn't recorded as a type's question, and its `DoubleEndedIterator` is refused (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                typed_sources.insert(ty);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^back_stepped_adapter_sources.rs$"],
  },
  {
    name: "return-place-unstepped",
    breaks:
      "what a function or a closure gives back to be stepped from its back isn't made knowing it there (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                    mark(&mut stepped, (made_by, mir::RETURN_PLACE), SteppedLocal::Given);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^filter_given_back_stepped_from_its_back.rs$"],
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
      "a generic function's own adapter of its type parameters it steps from its back, `Filter<I, P>`'s `next_back()`, isn't its callers' to make knowing its back, so each is refused (#388)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                    false => stepped.extend(back_stepped_adapter(tcx, recognition, adapter, false, &drops)),\n",
    replace: "                    false => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^(copied_given_to_generic_backs|adapters_given_to_generic_backs).rs$"],
  },
  {
    name: "held-adapter-not-back-stepped",
    breaks:
      "a filter or a map a type holds, asked to be double-ended by the call that asks it of that type, isn't made knowing its back",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            .filter(|&made| !holds_compiled(tcx, made, &compiled));\n",
    replace: "                            .filter(|_| false);\n",
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
    name: "filter-map-unstepped-from-back",
    breaks: "a `filter_map` the crate steps from its back isn't one the analysis marks, so one held in a variable before it's stepped, a library's, isn't made knowing its back, and is refused (#473)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    let filtered = filtered_source(tcx, adapter).or_else(|| filter_mapped_source(tcx, adapter));\n",
    replace: "    let filtered = filtered_source(tcx, adapter);\n",
    tests: ["test/corpus.test.ts", "-t", "^filter_map_of_given_from_its_back.rs$"],
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
    name: "counted-given-held-unrecorded",
    breaks:
      "a `skip(n)` or a `take(n)` given held to what steps it from its back isn't recorded, so it's made with no back: `held.rev()` of a `let held = v.into_iter().skip(1)` is refused (#718)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            (true, false, true) => mark(&mut stepped, (owner, local), SteppedLocal::Given),\n",
    replace: "                            (true, false, true) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
  },
  {
    name: "counted-chain-side-unstepped",
    breaks:
      "a chain's `skip(n)` or `take(n)` side, which MIR holds in a variable before the chain is made, isn't made knowing its back: `ds().into_iter().skip(1).chain(w).rev()` is refused (#718)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        if ty == source && (chain && made_asked || user || spilled || copy && made_asked) {\n",
    replace: "                        if ty == source && (user || spilled || copy && made_asked) {\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
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
  {
    name: "stepped-local-held-beneath-unasked",
    breaks: "a temporary given whole to what asks its back, a map made in place of a held `skip(n)`, `s.map(f).rev()`, isn't asked, so the variable it holds beneath isn't made knowing its back and its `rev()` is refused (#790, #814)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            (true, false, false) => {\n",
    replace: "                            (true, false, false) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^held_beneath_adapters_from_its_back.rs$"],
  },
  {
    name: "stepped-local-origin-unread",
    breaks: "a temporary moved out of a variable to be given on, `rev(move _x)` of `_x = move held`, is taken for a temporary given whole, so the variable isn't made knowing its back (#814)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                None => (origin(place.local), false, made_at.get(&origin(place.local))),\n",
    replace: "                                None => (place.local, false, made_at.get(&origin(place.local))),\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
  },
  {
    name: "back-stepped-temporary-unrecorded",
    breaks: "a temporary MIR holds a `take(n)` made in place in to lend it to `nth_back` isn't recorded, so it isn't made knowing its back where it's made and its back step is refused (#718)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                if counted || held || zip_length && zips_of_held.contains(&(owner, local)) =>\n",
    replace: "                                if false || zip_length && zips_of_held.contains(&(owner, local)) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_and_take_stepped_from_their_back_in_place.rs$"],
  },
  {
    name: "held-across-nothing-acts",
    breaks: "what an argument runs before a back step, `{ k += 1; k - 2 }`, is never said to act, so a map lent to `nth_back` past it isn't made knowing its back, and is refused (#811)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        if data.statements.get(first..).is_some_and(|rest| rest.iter().any(acts)) {\n",
    replace: "        if data.statements.get(first..).is_some_and(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_stepped_from_its_back_past_an_arguments_statement.rs$"],
  },
  {
    name: "held-across-its-own-step",
    breaks: "an adapter is said held across what acts though its step follows it in the same block, so a map whose `next_back()` reads its form in place is made `$mapEnds`, where its source's back mapped reads better (#811)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        if block == to {\n            return false;\n",
    replace: "        if block == to {\n            return true;\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_stepped_from_its_back_past_an_arguments_statement.rs$"],
    snapshots: true,
  },
  {
    name: "held-lent-unrecorded",
    breaks: "a map lent to `nth_back` across what its argument runs isn't recorded, only a counted adapter, and is refused (#811)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                if counted || held || zip_length && zips_of_held.contains(&(owner, local)) =>\n",
    replace: "                                if counted || zip_length && zips_of_held.contains(&(owner, local)) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_stepped_from_its_back_past_an_arguments_statement.rs$"],
  },
  {
    name: "held-whole-unrecorded",
    breaks: "a map given whole to `rfold` across what its first argument runs isn't recorded, and is refused (#811)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                if held {\n",
    replace: "                                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_stepped_from_its_back_past_an_arguments_statement.rs$"],
  },
  {
    name: "back-stepped-temporary-of-any-adapter-made",
    breaks: "any adapter made in place a temporary holds for a back step is made knowing its back where it's made, a map of an array's items `$mapEnds` where `next_back` steps its source's end itself, `$nextBack(src)` mapped (#718)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                if counted || held || zip_length && zips_of_held.contains(&(owner, local)) =>\n",
    replace: "                                if true =>\n",
    tests: ["test/corpus.test.ts"],
    snapshots: true,
  },
  {
    name: "zip-sides-of-a-zip-unstepped",
    breaks: "a zip's sides, which `$zipEnds` steps from their back, aren't recorded as a chain's are, so a map a zip of items with a destructor holds is refused (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                let chain = chained_sources(tcx, made).is_some() || zipped_sources(tcx, made).is_some();\n",
    replace: "                let chain = chained_sources(tcx, made).is_some();\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_droppers_from_its_back.rs$"],
  },
  {
    name: "zip-by-index-b-unrecorded",
    breaks: "a zip that reads its sides by index asks the length of its `b`, a map whose closure has a destructor, which isn't recorded, so it's refused (#775, #851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                    if crate::lower::recognition::zips_by_index(tcx, typing_env, a, b) && closure_drops(tcx, b, &drops)\n",
    replace: "                    if false && crate::lower::recognition::zips_by_index(tcx, typing_env, a, b) && closure_drops(tcx, b, &drops)\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_by_index_of_holding_maps.rs$"],
    snapshots: true,
  },
  {
    name: "held-closure-map-unrecorded",
    breaks: "a temporary map whose closure has a destructor, held to drop it, given to a back step, isn't recorded, so its form is gone where it's stepped and it's refused (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                || closure_drops(tcx, ty, &drops)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^closing_maps_given_counted.rs$"],
  },
  {
    name: "own-method-receiver-unrecorded",
    breaks: "the receiver of `DoubleEndedIterator`'s own method, a map of the crate's iterator `count().map(f).rfold(..)`, isn't recorded though no form of its own steps it from its back, and it's refused (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                && back_stepped_adapter(tcx, recognition, ty, false, &drops).is_some()\n",
    replace: "                                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^back_stepped_adapter_sources.rs$"],
  },
  {
    name: "asked-adapter-source-unasked",
    breaks: "what an asked adapter made in place is made of isn't asked in turn, so a variable's chain beneath a `filter(..).map(..).rev()` is made without its back and is refused (#790, #851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        } else if ty == source && made_asked && !user {\n",
    replace: "                        } else if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_beneath_adapters_from_its_back|cloned_of_a_chain_stepped_from_its_back|chain_stepped_from_its_back_elsewhere|chain_under_a_held_map_from_its_back|chain_of_adapters_stepped_from_its_back|chain_sides_own_steps|rev_chain|array_cloned_stepped_from_its_back|cloned_items_from_both_ends|copied_given_to_generic_backs|copied_skip_take_and_filter_stepped_from_back|adapters_given_to_generic_backs|map_ends_size_hint|back_folds_of_sources_made_knowing_their_back|consumer_drops_what_is_left|adapters_own_work_that_panics).rs$"],
  },
  {
    name: "cloned-adapter-unrecorded",
    breaks: "a `cloned()` lent to `DoubleEndedIterator`'s own method, `chain.cloned().rfind(p)`, isn't recorded, so it's made JS's `.map(clone)` with no back and throws `it.back is not a function` (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    let copied = copied_source(tcx, adapter).or_else(|| cloned_source(tcx, adapter));\n",
    replace: "    let copied = copied_source(tcx, adapter);\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_beneath_adapters_from_its_back|cloned_of_a_chain_stepped_from_its_back|chain_stepped_from_its_back_elsewhere|chain_under_a_held_map_from_its_back|chain_of_adapters_stepped_from_its_back|chain_sides_own_steps|rev_chain|array_cloned_stepped_from_its_back|cloned_items_from_both_ends|copied_given_to_generic_backs|copied_skip_take_and_filter_stepped_from_back|adapters_given_to_generic_backs|map_ends_size_hint|back_folds_of_sources_made_knowing_their_back|consumer_drops_what_is_left|adapters_own_work_that_panics).rs$"],
  },
  {
    name: "dropping-chain-read-where-made",
    breaks: "a temporary chain of items with a destructor lent to its back step isn't recorded, though MIR holds it to drop them, so its form is gone where it's stepped (#718, #851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                || chained_sources(tcx, ty).is_some()\n                                    && recognition.iterator_item(ty).is_some_and(&drops)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(held_beneath_adapters_from_its_back|cloned_of_a_chain_stepped_from_its_back|chain_stepped_from_its_back_elsewhere|chain_under_a_held_map_from_its_back|chain_of_adapters_stepped_from_its_back|chain_sides_own_steps|rev_chain|array_cloned_stepped_from_its_back|cloned_items_from_both_ends|copied_given_to_generic_backs|copied_skip_take_and_filter_stepped_from_back|adapters_given_to_generic_backs|map_ends_size_hint|back_folds_of_sources_made_knowing_their_back|consumer_drops_what_is_left|adapters_own_work_that_panics).rs$"],
  },
  {
    name: "spilled-source-unmarked",
    breaks: "the source of an adapter stepped from its back whose closure has a destructor, which MIR holds before the adapter is made, isn't marked, so its form is gone and it's made without its back (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        let spilled = made_asked && closure_drops(tcx, made, &drops);\n",
    replace: "                        let spilled = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_beneath_adapters_from_its_back|cloned_of_a_chain_stepped_from_its_back|chain_stepped_from_its_back_elsewhere|chain_under_a_held_map_from_its_back|chain_of_adapters_stepped_from_its_back|chain_sides_own_steps|rev_chain|array_cloned_stepped_from_its_back|cloned_items_from_both_ends|copied_given_to_generic_backs|copied_skip_take_and_filter_stepped_from_back|adapters_given_to_generic_backs|map_ends_size_hint|back_folds_of_sources_made_knowing_their_back|consumer_drops_what_is_left|adapters_own_work_that_panics).rs$"],
  },
  {
    name: "copy-source-unmarked",
    breaks: "what an asked `copied()` or `cloned()` is made of, which it is, isn't marked, so the chain beneath is made without its back (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        let copy = copied_source(tcx, made).or_else(|| cloned_source(tcx, made)).is_some();\n",
    replace: "                        let copy = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_skip_take_and_filter_stepped_from_back.rs$"],
    snapshots: true,
  },
  {
    name: "beside-stepped-field-unmarked",
    breaks: "a value given beside a `&mut` of a field stepped from its back, `replaced(&mut held.0, next)`, isn't made knowing its back, and its `next_back` there throws (#1013)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                let beside: Vec<(Option<mir::Local>, Ty<'tcx>)> =\n                    lent_stepped_locals.chain(lent_stepped_fields).collect();\n",
    replace: "                let beside: Vec<(Option<mir::Local>, Ty<'tcx>)> =\n                    lent_stepped_locals.chain(lent_stepped_fields.filter(|_| false)).collect();\n",
    tests: ["test/corpus.test.ts", "-t", "^iterators_held_in_a_generic_structs_field.rs$"],
  },
  {
    name: "field-reborrow-lends-nothing",
    breaks: "a reborrow of a `&mut` of a field, `&mut (*_r)`, isn't taken for the field it lends, so a value given beside it isn't made knowing its back (#1013)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                && let Some(&field) = lent_field.get(&of.local) =>\n",
    replace: "                                && let Some(&field) = lent_field.get(&of.local).filter(|_| false) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^iterators_held_in_a_generic_structs_field.rs$"],
  },
  {
    name: "library-impl-backs-unmade",
    breaks: "a library's public function that gives a filter behind an `impl DoubleEndedIterator` makes JS's own `.filter`, and its consumer's `next_back` throws `it.back is not a function` (#1024)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                if asked {\n                    typed.extend(given_behind_an_impl(tcx, recognition, hidden, &drops));\n",
    replace: "                if false {\n                    typed.extend(given_behind_an_impl(tcx, recognition, hidden, &drops));\n",
    tests: ["test/crates.test.ts", "-t", "two crates: iterators_stepped_from_their_back_across_crates"],
  },
  {
    name: "library-given-backs-unmade",
    breaks: "a filter of the library's own iterator, or of a `VecDeque`'s items, a crate gives a library's generic function that steps it from its back is JS's own `.filter`, and the library's `next_back` throws (#1024, #1109; a slice's is made knowing its hint, so its back, by hint_asked.rs's rule of what a library is given)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            if recognition.foreign.is_library(callee.krate) {\n",
    replace: "            if false {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: iterators_stepped_from_their_back_across_crates"],
  },
  {
    name: "library-own-params-unstepped",
    breaks: "a library's public generic function given a `Filter<I, P>` of an `I: DoubleEndedIterator`, stepped from its back where no caller of the crate's asks it, is refused, though its consumers make it knowing its back (#1024)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            typed.extend(library_backs(tcx, recognition, body.def_id, identity, &drops));\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: iterators_stepped_from_their_back_across_crates"],
  },
  {
    name: "held-zip-length-unasked",
    breaks: "a zip a side of which MIR holds, whose length `len()` asks, isn't made knowing it, no form left to count it in place, and it's refused (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                    zips_of_held.insert((owner, destination.local));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(zip_of_cloned_items_whose_clone_runs|zip_from_its_back|zip_of_droppers_from_its_back).rs$"],
  },
  {
    name: "shared-borrow-lends-nothing",
    breaks: "a zip given to `len(&self)` by a shared borrow isn't taken for what it lends, so it isn't made knowing its length and it's refused (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        mir::Rvalue::Ref(_, mir::BorrowKind::Shared, of) if of.projection.is_empty() => {\n",
    replace: "                        mir::Rvalue::Ref(_, mir::BorrowKind::Shared, of) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(zip_of_cloned_items_whose_clone_runs|zip_from_its_back|zip_of_droppers_from_its_back).rs$"],
  },
  {
    name: "shared-borrow-unmapped",
    breaks: "a shared borrow of a zip isn't mapped to what it lends where its length is asked, so `len()` of one made knowing it is refused (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "mir::BorrowKind::Mut { .. } | mir::BorrowKind::Shared, of)) =",
    replace: "mir::BorrowKind::Mut { .. }, of)) =",
    tests: ["test/corpus.test.ts", "-t", "^(zip_of_cloned_items_whose_clone_runs|zip_from_its_back|zip_of_droppers_from_its_back).rs$"],
  },
  {
    name: "array-side-zip-made-knowing",
    breaks: "a zip of an array's iterator a variable holds, counted in place as `Math.min(it.end - it.at, ..)`, is made `$zipEnds(..)` for its length instead, which reads worse (#851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "held && recognition.array_source(ty).is_none())",
    replace: "held)",
    tests: ["test/corpus.test.ts", "-t", "^(zip_of_cloned_items_whose_clone_runs|zip_from_its_back|zip_of_droppers_from_its_back).rs$"],
    snapshots: true,
  },
  {
    name: "enumerate-of-compiled-struct-held-unread",
    breaks:
      "an enumeration of std's compiled `Peekable` or `Fuse`, held where the crate steps it from its back, isn't marked made knowing its back, as no form of `$enumerate` steps from its back: its `rev()` or `next_back()` is refused (#712, #851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                || enumerates_a_compiled_struct(tcx, ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_of_a_reversible_source.rs$"],
  },
  {
    name: "map-length-asked-unrecorded",
    breaks: "a map whose closure has a destructor, given where its length is asked, `lens(v.iter().map(move |x| ..))`, isn't recorded made knowing it, so it's refused as an `ExactSizeIterator` (#385, #851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                    let map_length = length && closure_drops(tcx, asked_ty, &drops);\n",
    replace: "                    let map_length = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^closing_maps_given_counted.rs$"],
  },
  {
    name: "stepped-sources-unmarked",
    breaks:
      "a type's question isn't asked of its source too, so a `copied()` beneath a filter given to generic code that steps it from its back isn't made knowing its back, and is refused (#388, #851)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    with_stepped_sources(tcx, typed)\n",
    replace: "    typed\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_given_to_generic_backs.rs$"],
  },
  {
    name: "cloned-chain-asked-unmarked",
    breaks: "a chain cloned apart beneath a map stepped from its back isn't made knowing its back, so the map is refused beside a case that clones that chain's type (#1002)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                if !made_stepped && cloned_chain(owner, made) {\n",
    replace: "                if !made_stepped && cloned_chain(owner, made) && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_chain_stepped_from_its_back.rs$"],
  },
  {
    name: "chain-made-here-backless",
    breaks:
      "a chain made where it's lent to its own `nth_back`, `next_back` or `rfind`, `a.chain(b).nth_back(1)`, isn't made knowing its back, so it's refused, in generic code and of ranges alike (#1118)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "    if !given && chained_sources(tcx, adapter).is_some() {\n",
    replace: "    if false && !given && chained_sources(tcx, adapter).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_made_here_stepped_from_its_back.rs$"],
  },
  {
    name: "compiled-behind-an-impl-made-knowing",
    breaks: "std's struct of an adapter a function gives behind an `impl`, `(0..n).rev()`, is taken for one made knowing its back wherever it's made, so `(1..6).rev()` in `main` is made a JS iterator and given back unmade, `$rangeFirst($rev($iterator(range, ..)).of)`, where it's the range itself; the answer is the same since #1226's fix, so the form holds it (#1109)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "        .filter(|&made| !holds_compiled(tcx, made, &compiled))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^impl_trait_adapter_is_its_value.rs$"],
    snapshots: true,
  },
  {
    name: "flattened-options-source-unstepped",
    breaks: "a flatten of `Option`s, made a filter of the `Some`s, doesn't step its source from its back as a filter does, so a map a variable holds beneath it isn't made knowing that, and is refused (#1342)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        .or_else(|| recognition.flattened_options(made).map(|(source, ..)| source))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^as_is_map_stepped_from_its_back.rs$"],
  },
  {
    name: "enumerated-source-uncounted",
    breaks: "a local an enumeration stepped from its back is made of isn't said to be counted there, so a range's map it holds isn't made knowing its length, and is refused (#1354)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "enumerated_source(tcx, made).is_some() {\n                                counted.insert((owner, local));\n",
    replace: "enumerated_source(tcx, made).is_some() {\n                                let _ = local;\n",
    tests: ["test/corpus.test.ts", "-t", "^map_held_then_enumerated_from_its_back.rs$"],
  },
  {
    name: "grown-zip-length-unasked",
    breaks: "a zip a `Vec` is grown by in a crate that keeps capacities, exact-sized but not `TrustedLen`, held in a temporary, isn't made knowing its length, so the collect is refused, \"can't count\" (#1394)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                if keeps_capacity {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_an_inspect_counted_where_capacity_is_read.rs$"],
  },
  {
    name: "own-param-items-dropping-unasked",
    breaks: "a map of a type parameter's items a caller gives a drop, stepped from its back where it's made, isn't made knowing its back, its closed source has no back, and `next_back()` is refused (#1406)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                        given_a_drop_of(tcx, owner, item, (params_may_drop, given_a_drop))\n",
    replace: "                                        false && given_a_drop_of(tcx, owner, item, (params_may_drop, given_a_drop))\n",
    tests: ["test/corpus.test.ts", "-t", "^map_as_it_is_stepped_from_its_back.rs$"],
  },
  {
    name: "owning-source-item-unread",
    breaks: "of generic code's `vec::IntoIter<T>`, whose item the crate's typing environment can't normalize, no item is found, so a map of its items a caller gives a drop isn't made knowing its back, and is refused (#1406)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "(recognition.iterator_item(source).or_else(owned))",
    replace: "(recognition.iterator_item(source).or_else(|| None.or_else(owned).filter(|_| false)))",
    tests: ["test/corpus.test.ts", "-t", "^map_as_it_is_stepped_from_its_back.rs$"],
  },
  {
    name: "grown-take-uncounted",
    breaks: "a `take(n)` of a map's values a crate that reads a `Vec`'s capacity grows one by, held in a temporary, isn't counted where it's made, and is refused (#1317)",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                                    if !user_variable(decl) && erased(decl.ty) == ty {\n",
    replace: "                                    if false && !user_variable(decl) && erased(decl.ty) == ty {\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_adapters_where_capacity_is_read.rs$"],
  },
  {
    name: "held-rev-source-type-unmarked",
    breaks: "the `rev()` of a `filter_map` a variable holds, given to std's compiled `fuse()`, is std's compiled `Rev`, whose dictionary of what it holds is by type, and that type isn't made knowing its back: refused, the `DoubleEndedIterator` of a `FilterMap`",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                    typed_sources.insert(erased(mir.local_decls[origin(local)].ty));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^reversed_filter_map_given_to_a_compiled_adapter.rs$"],
  },
];
