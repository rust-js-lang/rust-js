// Mutations of src/lower/body_queries/in_place.rs (ADR 0093): which of
// `HELD_COMPILED`'s adapters a body makes in place (ADR 0871).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "rev-never-in-place",
    "breaks": "every `rev()` is std's compiled `Rev`, a chain stepped where it's made no longer `v.toReversed()`",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            if steps_as_js(callee, index, &found, at, (made, maker)) {\n",
    "replace": "            if false && steps_as_js(callee, index, &found, at, (made, maker)) {\n",
    "snapshots": true,
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^rev_drops.rs$"
    ]
  },
  {
    "name": "rev-borrowed-held",
    "breaks": "a `rev()` borrowed for what steps it, `rev().nth(n)`, is taken as held",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "                ExprKind::Borrow { arg, .. } => strip(thir, arg),\n                _ => strip(thir, arg),\n            };\n            let ExprKind::Call { fun, fn_span, .. } = thir[arg].kind else {",
    "replace": "                _ => strip(thir, arg),\n            };\n            let ExprKind::Call { fun, fn_span, .. } = thir[arg].kind else {",
    "snapshots": true,
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^range_own_steps.rs$"
    ]
  },
  {
    "name": "rev-own-impl-in-place",
    "breaks": "`rev().next_back()`, `Rev`'s own compiled impl's, is given what the recognizers make of it",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            crate::lower::iterators::bound_as_iterator(tcx, callee, param)\n                && (std ||",
    "replace": "            (std || crate::lower::iterators::bound_as_iterator(tcx, callee, param))\n                && (std ||",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^(std_rev|iterator_values_meet).rs$"
    ]
  },
  {
    "name": "holder-param-steps-js",
    "breaks": "a `map()` given to the crate's `T` bound by nothing, `set_get(&mut it, (5..8).map(f))`'s, is made a JS iterator, which the caller's std struct then reads as its own (#537)",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            crate::lower::iterators::bound_as_iterator(tcx, callee, param)\n                && (std ||",
    "replace": "            (callee.is_local() || crate::lower::iterators::bound_as_iterator(tcx, callee, param))\n                && (std ||",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^iterator_values_meet.rs$"
    ]
  },
  {
    "name": "rev-given-js-held",
    "breaks": "a `rev()` given to the crate's generic function as a JS iterator is std's struct",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            && (std || callee.is_local() && !crate::lower::iterators::given_as_it_is(tcx, callee, param))\n",
    "replace": "            && (std || false && callee.is_local() && !crate::lower::iterators::given_as_it_is(tcx, callee, param))\n",
    "snapshots": true,
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^iterables_given_to_generics.rs$"
    ]
  },
  {
    "name": "local-adapter-held",
    "breaks": "an `enumerate()` a variable holds that only `next()` steps is std's struct, not `entries()`",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            if !found.contains(&at)\n                && all > 0\n",
    "replace": "            if false\n                && all > 0\n",
    "snapshots": true,
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^(iter_by_ref|rev_drops).rs$"
    ]
  },
  {
    "name": "local-adapter-lent-held",
    "breaks": "a variable's `enumerate()` lent by `by_ref()` to what steps it from its back, `next_back()`, is taken to be stepped as a JS iterator",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "                _ if by_ref => lent.push((e, arg)),\n",
    "replace": "",
    "snapshots": true,
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^(iter_by_ref|std_enumerate).rs$"
    ]
  },
  {
    "name": "local-adapter-dropped-held",
    "breaks": "a variable's `enumerate()` that's dropped is std's struct",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "                _ if by_iterator || crate::lower::recognition::is_mem_drop(tcx, callee) => js_steps.push((arg, at)),\n",
    "replace": "                _ if by_iterator => js_steps.push((arg, at)),\n",
    "snapshots": true,
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^(adapter_drops).rs$"
    ]
  },
  {
    "name": "adapter-chain-held",
    "breaks": "an adapter given to another made in place, `rev().rev()`'s first, is std's struct, which the other steps from a back it hasn't",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            return in_place.contains(&at);\n",
    "replace": "            return false;\n",
    "snapshots": true,
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^(rev_of_closed_items|own_iterator_dropped_through_adapters).rs$"
    ]
  },
  {
    "name": "local-adapter-made-of-held",
    "breaks": "a variable's `enumerate()` read only by a `map()` made in place of it is std's struct, not `entries()`",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "                _ if crate::lower::recognition::makes_held_compiled(tcx, callee) => makers.push((arg, at)),\n",
    "replace": "",
    "snapshots": true,
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^(iter_by_ref|std_map).rs$"
    ]
  },
  {
    "name": "local-adapter-made-of-held-maker",
    "breaks": "a variable read by a `rev()` that's held is taken to be in place, a JS iterator the held `rev()`'s struct steps",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "                && of.iter().all(|maker| found.contains(maker))\n",
    "replace": "",
    "snapshots": true,
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^(std_map|std_rev|std_enumerate|iter_by_ref).rs$"
    ]
  },
  {
    "name": "flat-map-result-held",
    "breaks": "a `map()` a closure gives `flat_map` is std's struct, `iterator_map(..)`, not `g.map(f)`",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "        given.insert(closure.closure_id.to_def_id());\n",
    "replace": "        given.insert(rustc_span::def_id::CRATE_DEF_ID.to_def_id());\n",
    "tests": [
      "test/lowering-iterators.test.ts",
      "-t",
      "iterator chains from MIR are array methods where order"
    ]
  },
  {
    "name": "size-hint-in-place",
    "breaks": "a variable's flatten that `size_hint()` asks of is the recognizers' JS iterator, sized `(0, None)`",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            && callee != size_hint;\n",
    "replace": ";\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^std_flatten.rs$"
    ]
  },
  {
    "name": "cycle-source-in-place",
    "breaks": "a `rev()` a `cycle()` holds is made in place, a JS iterator, which `cycle()` clones as std's struct, so `(1..4u8).rev().cycle()` throws (#308, ADR 0915)",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            let held_to_clone = (callee == cycle || cloned_apart(thir[e].ty)) && !cloned_apart(thir[arg].ty);\n",
    "replace": "            let held_to_clone = (false || cloned_apart(thir[e].ty)) && !cloned_apart(thir[arg].ty);\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^cycled_reversed_ranges.rs$"
    ]
  },
  {
    "name": "cloned-apart-part-in-place",
    "breaks": "a `rev()` a chain cloned apart holds is made in place, a JS iterator, which the chain's clone clones as std's struct, so a cycled zigzag throws (#308, ADR 0915)",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            let held_to_clone = (callee == cycle || cloned_apart(thir[e].ty)) && !cloned_apart(thir[arg].ty);\n",
    "replace": "            let held_to_clone = (callee == cycle || false) && !cloned_apart(thir[arg].ty);\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^cycled_reversed_ranges.rs$"
    ]
  },
  {
    "name": "cloned-apart-part-held",
    "breaks": "a `map()` itself cloned apart, part of a `filter()` cloned apart, is held as std's struct, which the filter's maker clones as cloned apart, `$clone`, and throws (ADR 0915)",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "            let held_to_clone = (callee == cycle || cloned_apart(thir[e].ty)) && !cloned_apart(thir[arg].ty);\n",
    "replace": "            let held_to_clone = (callee == cycle || cloned_apart(thir[e].ty));\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^std_map.rs$"
    ]
  },
  {
    "name": "held-flat-map-results-in-place",
    "breaks": "what the closure of a `flat_map()` held in a variable gives is made in place, a JS `.map()` chain, which std's compiled `FlatMap` steps from its back as its struct and throws (#726)",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "                && set.contains(&fn_span)\n",
    "replace": "",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^held_flat_map_of_maps_from_its_back.rs$"
    ]
  },
  {
    "name": "zip-folding-apart-held-from-its-back",
    "breaks": "a zip std folds otherwise than it steps, of a `Vec`'s items with a destructor, given to `rev()` is std's compiled `Zip` with its dictionaries, not `$rev($zipEnds(..))` (#812)",
    "file": "src/lower/body_queries/in_place.rs",
    "find": "                && (!crate::lower::recognition::folds_apart(tcx, thir[arg].ty) || from_its_back(callee))\n",
    "replace": "                && !crate::lower::recognition::folds_apart(tcx, thir[arg].ty)\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^zip_of_droppers_from_its_back.rs$"
    ],
    "snapshots": true
  },
  {
    name: "flatten-wanted-made-in-place",
    breaks: "a flatten given where its hint is asked is made in place, JS's own `flatMap`, which knows no items' constant size: `(0, None)` (#1323)",
    file: "src/lower/body_queries/in_place.rs",
    find: "            if flattens(maker) && wanted.contains(&made) {\n",
    replace: "            if flattens(maker) && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_given_where_its_hint_is_asked.rs$"],
  },
  {
    name: "hint-wanted-not-through-adapters",
    breaks: "a flatten under another adapter given where its hint is asked, `flat_map(f).filter(g)`, is made in place, its hint `(0, None)` (#1323)",
    file: "src/lower/body_queries/in_place.rs",
    find: "                    _ if adapts && index == 0 => wanted.contains(&at),\n",
    replace: "                    _ if adapts && index == 0 => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_given_where_its_hint_is_asked.rs$"],
  },
  {
    name: "hint-of-a-library-unasked",
    breaks: "a flatten under a `filter` given to a library's function, whose body the crate can't see, is made in place, though the library may ask its hint: `(0, None)` (#1323)",
    file: "src/lower/body_queries/in_place.rs",
    find: "                    Some(_) => callee == size_hint || hint_asked.contains(&callee) || library(callee),\n",
    replace: "                    Some(_) => callee == size_hint || hint_asked.contains(&callee),\n",
    tests: ["test/crates.test.ts", "-t", "flatten given to a library's function knows its hint"],
  },
  {
    name: "collect-hint-seen-made-in-place",
    breaks: "a flatten collected where the program sees the hint std's collect asks is made in place, JS's own `flatMap`, which knows no items' constant size: its source's printing `size_hint` asked three times where std asks once (#1327)",
    file: "src/lower/body_queries/in_place.rs",
    find: "                    _ if observed((callee, index), thir[arg].ty) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^flatten_collected_whose_own_hint_runs.rs$"],
  },
  {
    name: "std-iterators-ignored",
    breaks:
      "`--std-iterators` builds as a plain build does, adapters made in place, so the differential job checks the recognizers' forms, not std's structs",
    file: "src/lower/body_queries/in_place.rs",
    find: "    if std_iterators {\n",
    replace: "    if false && std_iterators {\n",
    tests: ["test/iterator-forms.test.ts"],
  },
  {
    name: "collect-hint-seen-unsized-held",
    breaks:
      "a generic flatten of a library's trait's items, whose constant size its manifest says no dictionary carries, is held where a collect sees its hint, and refused, \"which no caller gives\", where it compiled made in place (#1327, #1341)",
    file: "src/lower/body_queries/in_place.rs",
    find: "            recognition.collect_hint_seen(at, given) && flattens_sized(here, caller, given)\n",
    replace: "            recognition.collect_hint_seen(at, given)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_trait_items_flattened_where_its_hint_is_seen"],
  },
  {
    name: "flattens-sized-unwalked",
    breaks:
      "only the flatten a collect is given is asked its items' constant size, not one beneath a `map()`, which is held and refused (#1327)",
    file: "src/lower/body_queries/in_place.rs",
    find: "        .is_none_or(|held| held.into_iter().all(|held| flattens_sized(recognition, caller, held)))\n",
    replace: "        .is_none_or(|_| true)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_trait_items_flattened_where_its_hint_is_seen"],
  },
];
