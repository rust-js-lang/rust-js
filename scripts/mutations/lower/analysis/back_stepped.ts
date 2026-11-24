// Mutations of crates/rust-js-lower/src/analysis/back_stepped.rs (ADR 0545).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-copied-back-unstepped",
    breaks:
      "a copy given to generic code as `Copied<I>` and stepped from its back there isn't made knowing its back: `next_back()` of a lazy or generic `Copied` is refused (#385)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    let source = (mapped.or(filtered).or(copied))?.peel_refs();\n",
    replace:
      "    let source = (mapped.or(filtered))?.peel_refs();\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_given_to_generic_backs.rs$"],
  },
  {
    name: "generic-back-steps-uncarried",
    breaks:
      "a filter given to generic code as `Filter<I, P>` and stepped from its back there isn't made knowing its back where its caller makes it: `TypeError: it.back is not a function` (#388)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    for _ in 0..32 {\n",
    replace: "    for _ in 0..0 {\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_given_to_generic_backs.rs$"],
    snapshots: true,
  },
  {
    name: "filter-of-items-native",
    breaks:
      "a filter of an array's items stepped from its back is taken to have a form of its own that does it, and is refused: `v.iter().filter(p).next_back()` (ADR 0990)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
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
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "                    false => stepped.extend(back_stepped_adapter(tcx, recognition, adapter, false, &drops)),\n",
    replace: "                    false => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^(copied_given_to_generic_backs|adapters_given_to_generic_backs).rs$"],
  },
  {
    name: "held-adapter-not-back-stepped",
    breaks:
      "a filter or a map a type holds, asked to be double-ended by the call that asks it of that type, isn't made knowing its back",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
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
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
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
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    if flattened_source(tcx, adapter).is_some() {",
    replace: "    if false {",
    tests: ["test/corpus.test.ts", "-t", "^flatten_from_its_back.rs$"],
  },
  {
    name: "given-impl-not-back-stepped",
    breaks:
      "an adapter a function gives behind an impl is made knowing its back",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "                let given = hidden != asked;",
    replace: "                let given = false;",
    tests: ["test/corpus.test.ts", "-t", "^returned_impl_ends.rs$"],
  },





  {
    name: "filter-map-unstepped-from-back",
    breaks: "a `filter_map` the crate steps from its back isn't one the analysis marks, so one held in a variable before it's stepped, a library's, isn't made knowing its back, and is refused (#473)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    let filtered = filtered_source(tcx, adapter).or_else(|| filter_mapped_source(tcx, adapter));\n",
    replace: "    let filtered = filtered_source(tcx, adapter);\n",
    tests: ["test/corpus.test.ts", "-t", "^filter_map_of_given_from_its_back.rs$"],
  },





  {
    name: "filter-of-a-rev-backless",
    breaks: "a `filter()` of a reversed `Vec`'s items with a `Drop`, `rev()`ed again and held, is JS's own `filter`, which has no back (#646)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "        .map(|source| unreversed(source.peel_refs()))\n",
    replace: "        .map(|source| source.peel_refs())\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_skip_take_and_filter_stepped_from_back.rs$"],
  },
  {
    name: "rev-stepped-from-back-unrecorded",
    breaks:
      "a `rev()` whose own `rfold` or `nth_back` the crate calls isn't recorded stepped from its back: one MIR holds in a variable first, past `String::new()` or lent to `nth_back`, is refused, its form gone (#702)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    if reversed_source(tcx, adapter).is_some() {\n",
    replace: "    if false && reversed_source(tcx, adapter).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_flattened_from_its_back.rs$"],
  },


  {
    name: "inspect-of-dropping-items-not-back-stepped",
    breaks: "an `inspect()` of a `Vec`'s items with a destructor, held in a variable to drop them, isn't made knowing its back, and its `rfind` is refused (#751)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "        .or_else(|| inspected_source(tcx, adapter))\n        .or_else(|| filtered_source(tcx, adapter))\n",
    replace: "        .or_else(|| filtered_source(tcx, adapter))\n",
    tests: ["test/corpus.test.ts", "-t", "^inspect_of_dropping_items_from_its_back.rs$"],
  },
  {
    name: "zip-not-back-stepped",
    breaks: "a zip held in a variable and stepped from its back isn't made knowing its back where it's made, and its `next_back` is refused (#740)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    if zipped_sources(tcx, adapter).is_some() {\n        return Some(tcx.erase_and_anonymize_regions(adapter));\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_from_its_back.rs$"],
  },
  {
    name: "inspect-of-a-dropping-closure-not-back-stepped",
    breaks: "an `inspect()` whose closure owns a value with a destructor, held in a variable to drop it, isn't made knowing its back, and its `rfind` is refused (#780)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    (mapped_source(tcx, adapter).is_some() || inspected_source(tcx, adapter).is_some())\n",
    replace: "    mapped_source(tcx, adapter).is_some()\n",
    tests: ["test/corpus.test.ts", "-t", "^move_closure_adapter_from_its_back.rs$"],
  },


  {
    name: "zip-of-droppers-unrecorded",
    breaks:
      "a zip with a side of a `Vec`'s items with a destructor, made in place and held in a variable as it's made, isn't recorded as stepped from its back, so its `rev()` is refused (#812)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "        return array_of_drops(a.peel_refs()) || array_of_drops(b.peel_refs());\n",
    replace: "        return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "hand-written-of-no-other-value",
    breaks: "the held flip's policy keeps no chain hand-written for another value of its type stepped from its back, so a held `map` of `Ends` beside one is std's compiled struct (#2158)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "                .any(|&(of, local)| of == owner && erased(mir.local_decls[local].ty) == ty)\n",
    replace: "                .any(|_| false)\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^held_chain_beside_another_value_stepped_from_its_back.rs$"],
  },
  {
    name: "cloned-adapter-unrecorded",
    breaks: "a `cloned()` lent to `DoubleEndedIterator`'s own method, `chain.cloned().rfind(p)`, isn't recorded, so it's made JS's `.map(clone)` with no back and throws `it.back is not a function` (#851)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    let copied = copied_source(tcx, adapter).or_else(|| cloned_source(tcx, adapter));\n",
    replace: "    let copied = copied_source(tcx, adapter);\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_beneath_adapters_from_its_back|cloned_of_a_chain_stepped_from_its_back|chain_stepped_from_its_back_elsewhere|chain_under_a_held_map_from_its_back|chain_of_adapters_stepped_from_its_back|chain_sides_own_steps|rev_chain|array_cloned_stepped_from_its_back|cloned_items_from_both_ends|copied_given_to_generic_backs|copied_skip_take_and_filter_stepped_from_back|adapters_given_to_generic_backs|map_ends_size_hint|back_folds_of_sources_made_knowing_their_back|consumer_drops_what_is_left|adapters_own_work_that_panics).rs$"],
  },
  {
    name: "library-impl-backs-unmade",
    breaks: "a library's public function that gives a filter behind an `impl DoubleEndedIterator` makes JS's own `.filter`, and its consumer's `next_back` throws `it.back is not a function` (#1024)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "                if asked {\n                    let given = given_behind_an_impl(tcx, recognition, hidden, &drops);\n",
    replace: "                if false {\n                    let given = given_behind_an_impl(tcx, recognition, hidden, &drops);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: iterators_stepped_from_their_back_across_crates"],
  },
  {
    name: "library-given-backs-unmade",
    breaks: "a filter of the library's own iterator, or of a `VecDeque`'s items, a crate gives a library's generic function that steps it from its back is JS's own `.filter`, and the library's `next_back` throws (#1024, #1109; a slice's is made knowing its hint, so its back, by hint_asked.rs's rule of what a library is given)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "            if recognition.foreign.is_library(callee.krate) {\n",
    replace: "            if false {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: iterators_stepped_from_their_back_across_crates"],
  },
  {
    name: "library-own-params-unstepped",
    breaks: "a library's public generic function given a `Filter<I, P>` of an `I: DoubleEndedIterator`, stepped from its back where no caller of the crate's asks it, is refused, though its consumers make it knowing its back (#1024)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "            typed.extend(backs.into_iter().map(|ty| typed_key(tcx, body.def_id, ty)));\n            // Its regions erased, those it binds too.\n",
    replace: "            let _ = backs;\n            // Its regions erased, those it binds too.\n",
    tests: ["test/crates.test.ts", "-t", "two crates: iterators_stepped_from_their_back_across_crates"],
  },
  {
    name: "stepped-sources-unmarked",
    breaks:
      "a type's question isn't asked of its source too, so a `copied()` beneath a filter given to generic code that steps it from its back isn't made knowing its back, and is refused (#388, #851)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    with_stepped_sources(tcx, typed)\n",
    replace: "    typed\n",
    tests: ["test/corpus.test.ts", "-t", "^flat_map_of_given_adapters_from_its_back.rs$"],
    snapshots: true,
  },
  {
    name: "chain-made-here-backless",
    breaks:
      "a chain made where it's lent to its own `nth_back`, `next_back` or `rfind`, `a.chain(b).nth_back(1)`, isn't made knowing its back, so it's refused, in generic code and of ranges alike (#1118)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "    if !given && chained_sources(tcx, adapter).is_some() {\n",
    replace: "    if false && !given && chained_sources(tcx, adapter).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_made_here_stepped_from_its_back.rs$"],
  },
  {
    name: "compiled-behind-an-impl-made-knowing",
    breaks: "std's struct of an adapter a function gives behind an `impl`, `(0..n).rev()`, is taken for one made knowing its back wherever it's made, so `(1..6).rev()` in `main` is made a JS iterator and given back unmade, `$rangeFirst($rev($iterator(range, ..)).of)`, where it's the range itself; the answer is the same since #1226's fix, so the form holds it (#1109)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "        .filter(|&made| !holds_compiled(tcx, made, &compiled))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^impl_trait_adapter_is_its_value.rs$"],
    snapshots: true,
  },
  {
    name: "owning-source-item-unread",
    breaks: "of generic code's `vec::IntoIter<T>`, whose item the crate's typing environment can't normalize, no item is found, so a map of its items a caller gives a drop isn't made knowing its back, and is refused (#1406)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "(recognition.iterator_item(source).or_else(owned))",
    replace: "(recognition.iterator_item(source).or_else(|| None.or_else(owned).filter(|_| false)))",
    tests: ["test/corpus.test.ts", "-t", "^map_as_it_is_stepped_from_its_back.rs$"],
  },
  {
    name: "held-length-flips-held",
    breaks: "a variable asked its length makes the held flip (`keeps_hand_written`) keep its type hand-written, so a held `step_by` of a reversed range, std's struct, is made in place, a change of form where none is asked (#1795)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "                        .filter(|(_, how)| **how == LengthAsked::Counted)\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^held_step_by_of_a_reversed_range_grows_as_std.rs$"],
  },
  {
    name: "hand-written-of-hinted-unkept",
    breaks: "a chain of a type made knowing its hint wherever it's made isn't kept hand-written, so the held flip makes it std's compiled struct (#2209)",
    file: "crates/rust-js-lower/src/analysis/back_stepped.rs",
    find: "            || hinted.is_typed(tcx, owner, ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^given_iterator_of_droppers_drops_what_it_passes.rs$"],
    snapshots: true,
  },
];
