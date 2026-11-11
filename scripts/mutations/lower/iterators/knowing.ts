// Mutations of src/lower/iterators/knowing.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "back-stepped-range-map-length-unknown",
    breaks:
      "a range's map whose closure has a destructor, a zip's `b` read by index, isn't read as knowing its length where it's made knowing it: refused as an `ExactSizeIterator` (#775)",
    file: "src/lower/iterators/knowing.rs",
    find: "                || self.range_kind(source.peel_refs()).is_some()\n",
    replace: "                || false\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_by_index_of_holding_maps.rs$"],
  },
  {
    name: "back-stepped-range-map-length-always",
    breaks:
      "every range's map stepped from its back is made knowing its length, though its closure drops nothing and nothing asks it: `$rangeLen` where it wasn't (#775)",
    file: "src/lower/iterators/knowing.rs",
    find: "                    && (matches!(ty.kind(), ty::Adt(_, args) if self.drops(args.type_at(1)) != crate::lower::drops::Drops::Nothing)\n                        || self.made_counted_from_its_back())\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(map_stepped_from_its_back|range_mapped_by_a_function).rs$"],
    snapshots: true,
  },
  {
    name: "compiled-source-map-length-unknown",
    breaks:
      "a map whose closure has a destructor, of a `fuse()`, std's compiled struct, isn't read as knowing its length where it's made knowing it: refused as an `ExactSizeIterator` (#385)",
    file: "src/lower/iterators/knowing.rs",
    find: "                    && compiled_exact()\n",
    replace: "                    && false\n",
    tests: ["test/corpus.test.ts", "-t", "^closing_maps_given_counted.rs$"],
  },
  {
    name: "inspect-made-here-backless",
    breaks:
      "an `inspect(f)` made in place and `rev()`'d isn't stepped from its back as the map it's made as: refused",
    file: "src/lower/iterators/knowing.rs",
    find: '            .or_else(|| crate::lower::recognition::inspected_source(self.tcx, ty))\n            && let js::ExprKind::Call(callee, args) = &value.kind\n            && let js::ExprKind::Member(source_value, method) = &callee.kind\n            && method == "map"\n            && let [f] = args.as_slice()\n        {\n            let source_value = self.as_double_ended(',
    replace:
      '            .or_else(|| None)\n            && let js::ExprKind::Call(callee, args) = &value.kind\n            && let js::ExprKind::Member(source_value, method) = &callee.kind\n            && method == "map"\n            && let [f] = args.as_slice()\n        {\n            let source_value = self.as_double_ended(',
    tests: ["test/corpus.test.ts", "-t", "^inspect_from_back.rs$"],
  },
  {
    name: "back-stepped-enumerate-unknown",
    breaks:
      "an enumeration the crate steps from its back, made knowing it, isn't read as knowing it: refused (#383)",
    file: "src/lower/iterators/knowing.rs",
    find: "                || self.stepped_from_its_back(ty);\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^(enumerate_of_droppers_from_back|enumerate_made_knowing_read_again|enumerate_of_a_reversible_source|enumerate_made_in_place_from_back).rs$"],
    snapshots: true,
  },
  {
    name: "stepped-range-restepped",
    breaks:
      "a range stepped from its start already, an `enumerate()`'s source, is stepped as a range again where it's made knowing its length: `(1..5).enumerate()` given to code stepping its back is `(false, 0)`, not `(true, 3)` (#385)",
    file: "src/lower/iterators/knowing.rs",
    find: "            // source's, the range it steps (#385).\n            let value = stepped_value(&value).unwrap_or(value);\n",
    replace: "            // source's, the range it steps (#385).\n",
    tests: ["test/corpus.test.ts", "-t", "^lengths_and_backs_given_on.rs$"],
  },
  {
    name: "made-skipping-reread",
    breaks:
      "a `$skipping` `as_exact_size` made already, a `rev()`'s source, is refused as an `ExactSizeIterator`: `first((0..10).skip(6).rev())` (#385)",
    file: "src/lower/iterators/knowing.rs",
    find: '            && args.len() == 2\n            && callee.helper_name() == Some(if skip { "$skipping" } else { "$taking" })\n',
    replace:
      '            && args.len() == 99\n            && callee.helper_name() == Some(if skip { "$skipping" } else { "$taking" })\n',
    tests: ["test/corpus.test.ts", "-t", "^lengths_and_backs_given_on.rs$"],
  },
  {
    name: "enumerate-back-of-arrays-only",
    breaks:
      "an `enumerate()` made in place of what isn't an array's items, a `fuse()` of owned items, is refused where it's stepped from its back (#383)",
    file: "src/lower/iterators/knowing.rs",
    find: "                false => self.as_exact_size(source_value.clone(), source.peel_refs(), true, span)?,\n",
    replace:
      '                false => return Err(self.unsupported(span, "a mutated enumerate")),\n',
    tests: ["test/corpus.test.ts", "-t", "^(enumerate_of_a_fuse_from_back|back_steps_beside_a_live_drop|lengths_and_backs_given_on).rs$"],
  },
  {
    name: "double-ended-rev-unread",
    breaks:
      "a `rev()` made in place, given where generic code steps it from its back, is refused: `ends((1..4).rev())` (ADR 0987)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if let Some(source) = crate::lower::recognition::reversed_source(self.tcx, ty)\n            && let Some(knowing) = self.reversal_knowing(&value, source, span)?\n",
    replace:
      "        if let Some(source) = crate::lower::recognition::reversed_source(self.tcx, ty).filter(|_| false)\n            && let Some(knowing) = self.reversal_knowing(&value, source, span)?\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_given_to_generic_code.rs$"],
  },
  {
    name: "double-ended-reversed-range-forward",
    breaks:
      "a reversed range made in place, given where generic code steps it, is stepped forward: `ends((1..4).rev())` is `(Some(3), Some(1), ..)`, not `(Some(1), Some(3), ..)` (ADR 0987)",
    file: "src/lower/iterators/knowing.rs",
    find: "                let range = self.as_double_ended(range.clone(), source, span)?;\n                crate::lower::iterators::reversed_by(range)\n",
    replace: "                let range = self.as_double_ended(range.clone(), source, span)?;\n                range\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_given_to_generic_code.rs$"],
  },
  {
    name: "exact-size-rev-unread",
    breaks:
      "a `rev()` made in place, given where its length is asked, is refused: `count((1..4).rev())` (ADRs 0986, 0987)",
    file: "src/lower/iterators/knowing.rs",
    find: "            && let Some(reversal) = self.reversal_of(&value, source)\n        {\n            return match reversal {\n                Reversal::Stepped(it) | Reversal::Range(it) => {\n",
    replace:
      "            && let Some(reversal) = self.reversal_of(&value, source).filter(|_| false)\n        {\n            return match reversal {\n                Reversal::Stepped(it) | Reversal::Range(it) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "exact-size-reversed-items-unread",
    breaks:
      "an array's items reversed where they're made, or a map of them, given where their length is asked, are refused: `count(v.iter().map(f).rev())` (ADRs 0986, 0987)",
    file: "src/lower/iterators/knowing.rs",
    find: "                Reversal::Chain => self.as_exact_size(value.clone(), source, back, span),\n",
    replace:
      '                Reversal::Chain => Err(self.unsupported(span, "a mutated reversal")),\n',
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "knowing-try-rfold-ungiven",
    breaks:
      "a held `skip()` over a `take()` stepped by `rfind` steps each from its back, not by std's `Take::try_rfold`, one `nth_back` of the crate's iterator (ADR 0930)",
    file: "src/lower/iterators/knowing.rs",
    find: "            let own = [rfold, nth_back, try_rfold];\n",
    replace:
      "            let own = [rfold, nth_back, try_rfold.filter(|_| false)];\n",
    tests: ["test/corpus.test.ts", "-t", "^held_back_folds_by_try_rfold.rs$"],
  },
  {
    name: "double-ended-held-knows-its-back",
    breaks:
      "a generic iterator held as it is, stepped by its `Iterator` dictionary, is stepped by `$nextBack`, which it has no back for",
    file: "src/lower/iterators/knowing.rs",
    find: "            ty::Param(param) => !given_as_it_is(self.tcx, id, *param),\n",
    replace: "            ty::Param(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^given_iterator_extensions.rs$"],
  },
  {
    name: "double-ended-range-no-back",
    breaks: "a range as a double-ended iterator has no `nextBack`",
    file: "src/lower/iterators/knowing.rs",
    find: "false, Some(Expr::helper(back))));",
    replace: "false, None));",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "map-made-unstepped",
    breaks:
      "`rfind` of an array's map made in place, `v.iter().map(f)`, is refused",
    file: "src/lower/iterators/knowing.rs",
    find: '            && method == "map"\n            && let [f] = args.as_slice()\n        {\n            let source_value = self.as_double_ended(',
    replace:
      '            && method == "unmapped"\n            && let [f] = args.as_slice()\n        {\n            let source_value = self.as_double_ended(',
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "associated-double-ended-unknown",
    breaks:
      "`rev()` of an `I::IntoIter` a bound says is double-ended is refused",
    file: "src/lower/iterators/knowing.rs",
    find: "            ty::Alias(..) => true,\n            _ => false,\n        };\n",
    replace: "            _ => false,\n        };\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(associated_double_ended|backs_given_behind_impls).rs$",
    ],
  },
  {
    name: "filter-made-here-backless",
    breaks:
      "a filter made where it's stepped from its back, `xs.iter().filter(p).rfold(..)`, is refused",
    file: "src/lower/iterators/knowing.rs",
    find: '                (js::ExprKind::Member(source_value, method), [p]) if method == "filter" => {\n',
    replace: '                (js::ExprKind::Member(source_value, method), [p]) if method == "filtered" => {\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(filter_ends|rev_given_to_generic_code|user_iterator_ends|returned_slice_iters|own_back_through_adapters)\\.rs$",
    ],
  },
  {
    name: "enumerate-back-unknown",
    breaks:
      "an `enumerate` of what knows its back and length isn't seen to know its back",
    file: "src/lower/iterators/knowing.rs",
    find: "            return (self.knows_its_back(source) || self.user_double_ended(source))\n",
    replace:
      "            return false && (self.knows_its_back(source) || self.user_double_ended(source))\n",
    tests: ["test/corpus.test.ts", "-t", "^(fuse_enumerate_ends|own_back_through_adapters|filter_folds_by_its_sources_fold).rs$"],
    snapshots: true,
  },
  {
    name: "memchr-iter-backless",
    breaks:
      "memchr's iterator isn't seen to know its back, and `next_back` of it is refused",
    file: "src/lower/iterators/knowing.rs",
    find: "        if self.recognition().is_memchr_iter(ty) || self.recognition().is_vec_splice(ty) {\n",
    replace: "        if self.recognition().is_vec_splice(ty) {\n",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
  {
    name: "double-ended-of-a-js-iterator-again",
    breaks:
      "a map's source made a JS iterator, `$iterator(range, $rangeNext)`, is stepped as a range again, and gives nothing",
    file: "src/lower/iterators/knowing.rs",
    find: "        }\n        let value = stepped_value(&value).unwrap_or(value);\n        // One a struct holds, which keeps its source (ADR 0542): of one given\n",
    replace:
      "        }\n        // One a struct holds, which keeps its source (ADR 0542): of one given\n",
    tests: ["test/corpus.test.ts", "-t", "^range_mapped_by_a_function.rs$"],
  },
  {
    name: "held-enumerate-not-knowing",
    breaks:
      "a held `Enumerate` of a source given as it is isn't given this use's back and length, and its back is refused",
    file: "src/lower/iterators/knowing.rs",
    find: "        // was made; of one made knowing them, an array's or a range's, itself.\n        if let Some(knowing) = self.knowing_held(value.clone(), ty, span)? {\n",
    replace:
      "        // was made; of one made knowing them, an array's or a range's, itself.\n        if let Some(knowing) = self.knowing_held(value.clone(), ty, span)?.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^held_enumerate.rs$"],
  },
  {
    name: "back-stepped-unknowing",
    breaks: "a map the crate steps from its back isn't made knowing it",
    file: "src/lower/iterators/knowing.rs",
    find: "            return self.knows_its_back(source) || self.stepped_from_its_back(ty);\n",
    replace: "            return self.knows_its_back(source);\n",
    tests: ["test/corpus.test.ts", "-t", "^filters_held_double_ended.rs$"],
  },
  {
    name: "compiled-iterator-not-knowing",
    breaks:
      "a compiled std iterator stepped from its back by a JS helper, `rev()`'s, isn't given its compiled `next_back` with this use's dictionaries",
    file: "src/lower/iterators/knowing.rs",
    find: "                    Some(trait_id) if self.compiled_step(ty, trait_id, method) => {\n",
    replace:
      "                    Some(trait_id) if false && self.compiled_step(ty, trait_id, method) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^fuse_enumerate_ends.rs$"],
  },
  {
    name: "dyn-double-ended-forgets-its-back",
    breaks:
      "a `dyn DoubleEndedIterator` is a JS iterator that can't step from its back, and is refused",
    file: "src/lower/iterators/knowing.rs",
    find: "            return crate::lower::recognition::is_double_ended(self.tcx, id);\n",
    replace: "            return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "copied-not-double-ended",
    breaks:
      "a `Copied` of a slice's iterator, given as a `DoubleEndedIterator`, is refused",
    file: "src/lower/iterators/knowing.rs",
    find: "        // A copy of one, which it is (ADR 0480).\n        if let Some(source) = crate::lower::recognition::copied_source(self.tcx, ty) {\n            return self.as_double_ended(value, source, span);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^associated_double_ended.rs$"],
  },
  {
    name: "held-hint-of-lazy-adapter",
    breaks:
      "`len()` of a `Peekable` of a `map` made here asks the JS iterator's hint, `(0, None)`, and its exact length panics",
    file: "src/lower/iterators/knowing.rs",
    find: "                .all(|held| self.held_hint_known(held))\n",
    replace: "                .all(|_| true)\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "the size hint of a peekable filter made here",
    ],
  },
  {
    name: "held-hint-of-range-unknown",
    breaks:
      "`len()` of a `Peekable` of a range is refused, though the range knows how many it has left",
    file: "src/lower/iterators/knowing.rs",
    find: "            || self.range_kind(ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^peekable_range_len.rs$"],
  },
  {
    name: "rev-take-as-skip",
    breaks:
      "`take(n).rev()` steps from the back as `skip(n)` does, giving what it should leave",
    file: "src/lower/iterators/knowing.rs",
    find: '            let name = if skip { "$skipping" } else { "$taking" };\n',
    replace:
      '            let name = if skip || true { "$skipping" } else { "$taking" };\n',
    tests: ["test/corpus.test.ts", "-t", "^rev_counted.rs$"],
  },
  {
    name: "exact-size-map-unknown",
    breaks:
      "a `map` of an `ExactSizeIterator` can't be counted, so `map(f).skip(n).rev()` is refused",
    file: "src/lower/iterators/knowing.rs",
    find: '            && method == "map"\n            && let [f] = args.as_slice()\n        {\n            let source_value = stepped_value(source_value).unwrap_or_else(|| (**source_value).clone());\n            let source_value = self.as_exact_size(',
    replace:
      '            && method == "map" && false\n            && let [f] = args.as_slice()\n        {\n            let source_value = stepped_value(source_value).unwrap_or_else(|| (**source_value).clone());\n            let source_value = self.as_exact_size(',
    tests: ["test/corpus.test.ts", "-t", "^rev_counted.rs$"],
  },
  {
    name: "exact-size-inclusive-unknown",
    breaks:
      "an inclusive range can't be counted, so `(1..=10u8).skip(7).rev()` is refused",
    file: "src/lower/iterators/knowing.rs",
    find: '            Some(crate::lower::std_types::range::RangeKind::Inclusive) => Some("$rangeInclusiveLen"),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rev_counted.rs$"],
  },
  {
    name: "chain-made-unstepped",
    breaks: "`rev()` of a chain made in place is refused",
    file: "src/lower/iterators/knowing.rs",
    find: '            && callee.helper_name() == Some("$lazyChain")\n',
    replace: '            && callee.helper_name() == Some("$unchained")\n',
    tests: ["test/corpus.test.ts", "-t", "^rev_chain.rs$"],
  },
  {
    name: "flatten-not-double-ended",
    breaks:
      "a flatten stepped from its back is refused, not made `$flattenEnds`",
    file: "src/lower/iterators/knowing.rs",
    find: '                (js::ExprKind::Member(source_value, method), [each]) if method == "flatMap" => {\n',
    replace: '                (js::ExprKind::Member(source_value, method), [each]) if method == "never" => {\n',
    tests: ["test/corpus.test.ts", "-t", "^flatten_from_its_back.rs$"],
  },
  {
    name: "enumerate-of-array-no-back",
    breaks: "an enumerate of an array's items made here steps from its back",
    file: "src/lower/iterators/knowing.rs",
    find: "        // One made so already, itself (#764).\n        if let Some(source) = crate::lower::recognition::enumerated_source(self.tcx, ty)\n",
    replace: "        // One made so already, itself (#764).\n        if let Some(source) = None::<ty::Ty<'tcx>>\n",
    tests: ["test/corpus.test.ts", "-t", "^returned_slice_iters.rs$"],
  },
  {
    name: "exact-size-enumerate-unmade",
    breaks: "an enumerate of an array's items made here is exact-sized",
    file: "src/lower/iterators/knowing.rs",
    find: "        // Of another's, its source made knowing its length (#1215).\n        if let Some(source) = crate::lower::recognition::enumerated_source(self.tcx, ty)\n",
    replace: "        // Of another's, its source made knowing its length (#1215).\n        if let Some(source) = None::<ty::Ty<'tcx>>\n",
    tests: ["test/corpus.test.ts", "-t", "^rposition_of_lazy.rs$"],
  },
  {
    name: "rposition-of-user-refused",
    breaks:
      "`rposition` of the crate's double-ended, exact-sized iterator steps it unknowing",
    file: "src/lower/iterators/knowing.rs",
    find: "            return self.user_iterator_knowing(value, ty, (back, true), span);\n",
    replace: "            return Ok(value);\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(user_iterator_ends|rposition_of_lazy).rs$",
    ],
  },
  {
    name: "mapped-array-length-unknown",
    breaks: "a map of an array's items made knowing its back knows its length",
    file: "src/lower/iterators/knowing.rs",
    find: "                || self.steps_an_array(source) && self.knows_its_back(ty)\n",
    replace: "\n",
    tests: ["test/corpus.test.ts", "-t", "^(cloned_items_from_both_ends|backs_given_behind_impls).rs$"],
    snapshots: true,
  },
  {
    name: "knowing-rfold-unasked",
    breaks:
      "a compiled struct stepped from its back isn't given std's compiled `rfold`, so `rev()` of a `fuse()` folds the crate's iterator by `next_back`, not by its own `rfold` (#224, ADR 0981)",
    file: "src/lower/iterators/knowing.rs",
    find: "                Some(back) if std_order || self.recognition().reaches_its_own(ty, OwnStep::Rfold, Reach::May) => {\n",
    replace: "                Some(back) if false && (std_order || self.recognition().reaches_its_own(ty, OwnStep::Rfold, Reach::May)) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "knowing-rfold-only-its-owns",
    breaks:
      "a compiled `peekable()` of what drops is given std's compiled `rfold` only where it may reach the crate's own, so its `rev().fold(..)` of an iterator that writes none steps it by `next_back` and drops its source after what it peeked at, where std's `Peekable::rfold` drops it before (#649)",
    file: "src/lower/iterators/knowing.rs",
    find: "            let std_order = self.drops(ty) != crate::lower::drops::Drops::Nothing;\n",
    replace: "            let std_order = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_steps_a_type_parameter_may_reach.rs$"],
  },
  {
    name: "knowing-fuse-nth-back-unasked",
    breaks: "a compiled `fuse()` isn't given its compiled `nth_back` (#224)",
    file: "src/lower/iterators/knowing.rs",
    find: '                Some(back) if self.compiled_step(ty, back, "nth_back") => {\n',
    replace:
      '                Some(back) if false && self.compiled_step(ty, back, "nth_back") => {\n',
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "reversed-unknowing-length",
    breaks:
      "an in-place `rev()` of the crate's iterator asked its length past a `skip` isn't made knowing it: refused",
    file: "src/lower/iterators/knowing.rs",
    find: "            return self.reversed_user_knowing(value, source, (back, true), span);\n",
    replace: '            return Err(self.unsupported(span, ""));\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rev_in_place_through_adapters).rs$",
    ],
  },
  {
    name: "reversed-unknowing-back",
    breaks:
      "an in-place `rev()` of the crate's iterator asked its back past a `step_by` isn't made knowing it: refused",
    file: "src/lower/iterators/knowing.rs",
    find: "            return self.reversed_user_knowing(value, source, (true, false), span);\n",
    replace: '            return Err(self.unsupported(span, ""));\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rev_in_place_through_adapters).rs$",
    ],
  },
  {
    name: "flatten-of-iterators-unknowing",
    breaks:
      "a flatten of an `Option`'s slice iterator, or of an associated `IntoIterator`'s items, stepped from its back, is a JS `flatMap` that has no back: it throws (#260, #1355)",
    file: "src/lower/iterators/knowing.rs",
    find: "                    if let Some(into_iter) = self.recognition().flattened_inner(ty)\n",
    replace: "                    if let Some(into_iter) = None::<ty::Ty<'tcx>>\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(flatten_of_iterators_from_back|associated_into_iterator_flattened_from_its_back).rs$",
    ],
  },
  {
    name: "flatten-of-iterators-each-called",
    breaks:
      "a flatten of ranges made knowing its back takes what `each` gives as a range, and steps its JS iterator by its bounds",
    file: "src/lower/iterators/knowing.rs",
    find: "                    let knowing = self.as_double_ended(taken, into_iter, span)?;\n",
    replace:
      "                    let knowing = self.as_double_ended(Expr::call(each.clone(), vec![Expr::var(&item)]), into_iter, span)?;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(flatten_of_iterators_from_back).rs$",
    ],
  },
  {
    name: "held-default-len-unknown",
    breaks:
      "a held `skip()` reversed isn't given its length, std's default of its exact `size_hint` (ADR 0918)",
    file: "src/lower/iterators/knowing.rs",
    find: '                    Some(trait_id) if method == "len" => self.default_len(ty, trait_id),\n',
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^held_adapters_by_ref.rs$"],
  },
  {
    name: "filter-map-ends-unmade",
    breaks: "a `filter_map` made of a source given as it is, stepped from its back, isn't made knowing it: itertools' `self.iter.filter_map(..).rfold(..)` is refused, \"can't step from its back\" (#473)",
    file: "src/lower/iterators/knowing.rs",
    find: "                let some = Expr::call_helper(\"$filterEnds\", vec![mapped, present.clone()]);\n",
    replace: "                let some = Expr::call_helper(\"$filterEnds\", vec![mapped, present.clone()]);\n                return Err(self.unsupported(span, \"a mutated filter_map\"));\n",
    tests: ["test/corpus.test.ts", "-t", "^filter_map_of_given_from_its_back.rs$"],
  },
  {
    name: "given-filter-map-ends-boxed",
    breaks: "a `filter_map` made knowing its back keeps each kept `Some` in its box, where it looks like `None`, and folds `{ value }`s (#473)",
    file: "src/lower/iterators/knowing.rs",
    find: "                    Some(unbox) => Expr::call_helper(\"$mapEnds\", vec![some, unbox.clone()]),\n",
    replace: "                    Some(_) => some,\n",
    tests: ["test/corpus.test.ts", "-t", "^filter_map_of_given_from_its_back.rs$"],
  },
  {
    name: "flat-map-helper-backless",
    breaks: "a `$flatMap(src, each)` stepped from its back isn't made `$flattenEnds`, so an `rfold` of a mapped `Vec`'s flatten of what has a destructor folds it from its front (#644)",
    file: "src/lower/iterators/knowing.rs",
    find: "                (_, [source_value, each]) if callee.helper_name() == Some(\"$flatMap\") => Some((source_value, each)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^flatten_closed_part_way_drops_source_first.rs$"],
  },
  {
    name: "rev-made-knowing-of-own-unknowing",
    breaks:
      "a `rev()` of the crate's iterator the crate steps from its back is made by its forms, not by its own `next`: its `$iterator(it, nextBack)` has no back, refused where it's made (#702)",
    file: "src/lower/iterators/knowing.rs",
    find: "            return self\n                .reversed_user_knowing(value, source, (true, true), span)\n                .map(made_knowing_its_back);\n",
    replace: "            return Ok(value);\n",
    tests: ["test/corpus.test.ts", "-t", "^(own_iterator_flattened_from_its_back|std_rev).rs$"],
    snapshots: true,
  },
  {
    name: "rev-of-a-parameter-not-made-knowing",
    breaks:
      "a type parameter's `rev()` stepped from its back isn't made knowing its back where it's made, so MIR's variable of it, lent to `nth_back`, is refused (#718)",
    file: "src/lower/iterators/knowing.rs",
    find: "            !self.is_unknown(source) || matches!(source.kind(), ty::Param(_))\n",
    replace: "            !self.is_unknown(source)\n",
    tests: ["test/corpus.test.ts", "-t", "^(rev_made_knowing_its_back|rev_of_a_type_parameter_held_as_it_is).rs$"],
  },
  {
    name: "adapted-hint-of-params",
    breaks: "`size_hint()` of a `filter_map()` of what generic code is given is `(0, None)` where its caller gives an array's iterator, not refused (#733)",
    file: "src/lower/iterators/knowing.rs",
    find: "                !self.is_unknown(held.peel_refs())\n                    && (self.adapted_hint_known(held)",
    replace: "                true\n                    && (self.adapted_hint_known(held)",
    tests: ["test/diagnostics.test.ts", "-t", "size_hint of a filter_map of a type parameter's"],
  },
  {
    name: "adapted-hint-of-arrays",
    breaks: "`size_hint()` of JS's own `filter` of a slice's items is `(0, None)`, not refused (#733)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if self.array_source(ty).is_some() {\n            return false;\n        }\n        if self.held_hint_known(ty) {\n",
    replace: "        if self.held_hint_known(ty) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "the size hint of a peekable filter made here"],
  },
  {
    name: "dropping-skip-uncounted",
    breaks: "a `skip(n)` of items with a destructor, `$skipping(source, n, drop)`, isn't made knowing its back and its length, so an enumeration of the crate's iterator's `skip()` folded from its back is refused (#778), and so is a `Vec`'s `skip(1).rev()` (#718)",
    file: "src/lower/iterators/knowing.rs",
    find: '        (_, [source, n, drop]) if skip && callee.helper_name() == Some("$skipping") => {\n',
    replace: '        (_, [source, n, drop]) if skip && callee.helper_name() == Some("$skipping") && false => {\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(back_folds_of_sources_made_knowing_their_back|skip_take_of_droppers_from_back).rs$",
    ],
  },
  {
    name: "cloned-map-uncounted",
    breaks: "a `cloned()` of what isn't a number, `src.map(clone)`, isn't made knowing its back and its length, so an enumeration of it folded from its back is refused (#778)",
    file: "src/lower/iterators/knowing.rs",
    find: "            .or_else(|| crate::lower::recognition::cloned_source(self.tcx, ty))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^back_folds_of_sources_made_knowing_their_back.rs$"],
  },
  {
    name: "zip-not-made-knowing-its-back",
    breaks: "a zip made here asked its back isn't made `$zipEnds`, and its `rev()` and `rfind` are refused (#740)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if let Some(zipped) = self.zip_knowing(&value, ty, true, span)? {\n            return Ok(zipped);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_from_its_back.rs$"],
  },
  {
    name: "zip-back-trims-undropped",
    breaks: "what a zip's back trims of its second side isn't dropped (#740)",
    file: "src/lower/iterators/knowing.rs",
    find: "        let drop_b = item_drop(self, b_ty)?;\n",
    replace: "        let drop_b = None;\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_from_its_back.rs$"],
  },
  {
    name: "zip-not-counted-held",
    breaks: "a zip the crate steps from its back, held in a variable, doesn't know its length, and its `len()` reads an array's (#740)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if crate::lower::recognition::zipped_sources(self.tcx, ty).is_some() && self.knows_its_back(ty) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_from_its_back.rs$"],
  },
  {
    name: "size-hint-of-projection-refused",
    breaks:
      "`size_hint()` of a generic `I::IntoIter` is refused, where a type parameter's is its JS value's",
    file: "src/lower/iterators/knowing.rs",
    find: "            || self.is_unknown(ty)\n            || self.explicit_adapter(ty)\n",
    replace: "            || matches!(ty.kind(), ty::Param(_))\n            || self.explicit_adapter(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_into_iter.rs$"],
  },
  {
    name: "compiled-iterator-size-hint-refused",
    breaks:
      "`size_hint()` of a compiled std iterator, a `Fuse`'s, is refused, as if its struct carried no hint",
    file: "src/lower/iterators/knowing.rs",
    find: "            || self.struct_hint_known(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fuse_size_hint.rs$"],
  },
  {
    name: "held-size-hint-refused",
    breaks: "a held adapter's size hint is refused",
    file: "src/lower/iterators/knowing.rs",
    find: "            || self.explicit_adapter(ty)\n            || self.range_kind(ty).is_some()\n",
    replace: "            || self.range_kind(ty).is_some()\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_cycle|held_explicit_adapters_hint).rs$"],
  },
  {
    name: "dropping-take-dropless",
    breaks:
      "a `take(n)` of items with a destructor made knowing its back isn't given their drop, `$taking(source, n)`, so what its `rfold` trims and what an `nth_back` past its end finds are never dropped (#718)",
    file: "src/lower/iterators/knowing.rs",
    find: "                None => match self.iterator_item(ty) {\n",
    replace: "                None => match self.iterator_item(ty).filter(|_| skip) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(skip_take_of_droppers_from_back|enumerate_of_take_from_back).rs$"],
  },
  {
    name: "zip-ends-not-to-the-shorter",
    breaks:
      "a zip of droppers made knowing its back isn't told std folds it to the shorter's count of pairs, so its fold steps its first side once more (#812)",
    file: "src/lower/iterators/knowing.rs",
    find: "        let to_shorter = crate::lower::recognition::zip_folds_to_the_shorter(self.tcx, ty) == Some(false);\n",
    replace: "        let to_shorter = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_droppers_from_its_back.rs$"],
  },
  {
    name: "step-by-made-uncounted",
    breaks: "a `step_by(n)` made here isn't made knowing its length where it's counted, and an `enumerate()` of it folded from its back is refused (#718)",
    file: "src/lower/iterators/knowing.rs",
    find: "        // A `step_by(n)` made here, as `as_double_ended` makes it (#718).\n        if let Some(stepped) = self.step_by_knowing(&value, ty, span)? {\n",
    replace: "        // A `step_by(n)` made here, as `as_double_ended` makes it (#718).\n        if let Some(stepped) = None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "^step_by_stepped_from_its_back.rs$"],
  },
  {
    name: "knowing-form-made-again",
    breaks: "a value whose form knows its back already, `$mapEnds` of an array's, isn't taken as it is but made knowing it again, which refuses what no arm makes (#851)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if self.knows_its_back_of(&value, ty) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(backs_given_behind_impls|held_adapters_by_ref|zip_from_its_back).rs$"],
  },

  {
    name: "zip-length-of-any",
    breaks: "a zip's length is known of its type alone, not of whether it was made knowing its back, so `zip(..).len()` of one made forward, `$trustedZip`, calls a `len` it hasn't and throws (#851)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if crate::lower::recognition::zipped_sources(self.tcx, ty).is_some() && self.knows_its_back(ty) {\n",
    replace: "        if crate::lower::recognition::zipped_sources(self.tcx, ty).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^(zip_of_cloned_items_whose_clone_runs|zip_from_its_back|zip_of_droppers_from_its_back).rs$"],
    snapshots: true,
  },
  {
    name: "length-of-a-value-unmarked",
    breaks: "a map made knowing its length, held, `$held($mapEnds($rev(mapped), f))`, isn't taken to know it by its mark, so `mapped.rev().map(f).len()` is refused (#851)",
    file: "src/lower/iterators/knowing.rs",
    find: "            || crate::lower::iterators::was_made_knowing_its_length(value)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(owned_maps_asked_their_length|zip_by_index_of_holding_maps).rs$"],
  },

  {
    name: "length-of-what-knows-its-back-unread",
    breaks: "a copy of an array's items made knowing its back, held where a zip is made, isn't taken to know its length, so `a.iter().cloned().zip(b.iter().cloned()).rev()` is refused as an `ExactSizeIterator` (#851)",
    file: "src/lower/iterators/knowing.rs",
    find: "            || self.kept_over_an_array(ty) && self.knows_its_back_of(value, ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_cloned_items_whose_clone_runs.rs$"],
  },
  {
    name: "map-of-what-knows-its-length-unmade",
    breaks: "a map of a `rev()` of a map stepped from its back elsewhere isn't made knowing its length, `$mapEnds`, so its `len()` is refused (#851)",
    file: "src/lower/iterators/knowing.rs",
    find: "        !self.steps_an_array(ty) && held_knowing(source) && self.knows_its_length_of(source, ty)\n",
    replace: "        false\n",
    tests: ["test/corpus.test.ts", "-t", "^owned_maps_asked_their_length.rs$"],
  },
  {
    name: "map-of-what-knows-its-length-in-place",
    breaks: "a map of what's made knowing its back in place, `$rev($enumerate(..))`, is made knowing its length though nothing asks it, `$mapEnds` where JS's own `.map` reads (#851)",
    file: "src/lower/iterators/knowing.rs",
    find: "        !self.steps_an_array(ty) && held_knowing(source) && self.knows_its_length_of(source, ty)\n",
    replace: "        !self.steps_an_array(ty) && self.knows_its_length_of(source, ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_made_in_place_from_back.rs$"],
    snapshots: true,
  },
  {
    name: "map-of-an-arrays-own-knowing-its-length",
    breaks: "a map of what an array's own iterator is, `rows.iter().map(Vec::as_slice)`'s `$iter(rows)`, is made knowing its length, `$mapEnds`, where `len_in_place` counts it as it is and JS's own `.map` reads (#851)",
    file: "src/lower/iterators/knowing.rs",
    find: "                js::ExprKind::Var(_) => {\n                    crate::lower::iterators::was_made_knowing_its_back(e)\n",
    replace: "                js::ExprKind::Var(_) | js::ExprKind::Call(..) => {\n                    true || crate::lower::iterators::was_made_knowing_its_back(e)\n",
    tests: ["test/corpus.test.ts", "-t", "^rest_function_values.rs$"],
    snapshots: true,
  },
  {
    name: "exact-size-of-a-value-by-its-type",
    breaks: "what's given as an `ExactSizeIterator` is taken to know its length by its type alone, not its value's form or mark, so a copy of an array's items made knowing its back, a zip's side, is refused (#851)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if self.knows_its_length_of(&value, ty)\n",
    replace: "        if self.knows_its_length(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_cloned_items_whose_clone_runs.rs$"],
  },
  {
    name: "chunks-hint-unknown",
    breaks:
      "a slice's chunks, `$iter` of the array of them, aren't taken to know their length where a zip holds them, so `len()` of rand_core's `dest.by_ref().zip(src.by_ref())` is refused (#1100)",
    file: "src/lower/iterators/knowing.rs",
    find: "            || self.recognition().iterates_items(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_len_of_chunks.rs$"],
  },
  {
    name: "splice-length-unknown",
    breaks: "a `Splice`'s `len()` is refused, as no rule says `$splice`'s iterator knows its length (#1149)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if self.recognition().is_vec_splice(ty) {\n            return true;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^splice_drops_and_fills_as_std.rs$"],
  },
  {
    name: "splice-backless",
    breaks: "a `Splice`'s `next_back()` is refused, as no rule says `$splice`'s iterator knows its back (#1149)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if self.recognition().is_memchr_iter(ty) || self.recognition().is_vec_splice(ty) {\n",
    replace: "        if self.recognition().is_memchr_iter(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^splice_drops_and_fills_as_std.rs$"],
  },
  {
    name: "decode-utf16-hint-of-held-unknown",
    breaks: "`decode_utf16`'s `size_hint()` of an array's items is refused, as std adapters of them are JS's own, though `$decodeUtf16` asks its source's hint (#1151)",
    file: "src/lower/iterators/knowing.rs",
    find: "                    && (self.adapted_hint_known(held) || decodes && self.held_hint_known(held))\n",
    replace: "                    && self.adapted_hint_known(held)\n",
    tests: ["test/corpus.test.ts", "-t", "^decode_utf16_steps_as_asked.rs$"],
  },
  {
    name: "exact-size-iter-of-an-array-uncounted",
    breaks: "an array's `$iter`, a hash set's `$iter(Array.from(set))`, given where its length is asked, isn't taken as knowing it, so a capacity-reading crate's `h.iter().skip(1).collect()` is refused (#1200)",
    file: "src/lower/iterators/knowing.rs",
    find: "            || super::fresh_iter_source(value).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^capacity_exact_size_adapters.rs$"],
  },
  {
    name: "enumerate-of-a-hash-set-uncounted",
    breaks: "a hash set's iterator enumerated, given where its length is asked, isn't made knowing its source's length, so a crate that reads a capacity refuses its collect (#1215)",
    file: "src/lower/iterators/knowing.rs",
    find: "                false => self.as_exact_size(stepped_value(&value).unwrap_or(value), source, back, span)?,\n",
    replace: "                false => return Err(self.unsupported(span, \"an enumeration\")),\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_what_std_hints_less.rs$"],
  },
  {
    name: "generic-not-double-ended-refused",
    breaks: "a `map` of an `I: Iterator` made for its hint is refused as a `DoubleEndedIterator` rust-js can't step from its back, though nothing can step it so (#1240)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if self.recognition().never_double_ended(ty) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_made_in_generic_code_asked_its_hint\\.rs$"],
  },
  {
    name: "flatten-each-by-name-unknowing",
    breaks: "a flatten stepped from its back whose `each` is a function given by its name, `flat_map(pairs)`, is refused: its items aren't made knowing their backs (#1235)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if !named {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^flat_map_of_given_adapters_from_its_back.rs$"],
  },
  {
    name: "flatten-each-with-a-body-unknowing",
    breaks: "a flatten stepped from its back whose closure does more than return its iterator, `|n| owned(v(n))`, is refused (#1235)",
    file: "src/lower/iterators/knowing.rs",
    find: "        StmtKind::Return(Some(value)) if plain => Some((param, before, value)),\n",
    replace: "        StmtKind::Return(Some(value)) if plain && before.is_empty() => Some((param, before, value)),\n",
    tests: ["test/corpus.test.ts", "-t", "^flat_map_of_given_adapters_from_its_back.rs$"],
  },
  {
    name: "counted-part-made-knowing-uncounted",
    breaks: "a `skip(n)` made `$skipping` for a hint asked of it isn't taken to know its length, so a zip of it is refused (#1216)",
    file: "src/lower/iterators/knowing.rs",
    find: "        if crate::lower::recognition::counted_source(self.tcx, ty).is_some() && self.made_knowing_its_hint(ty) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_chains_of_exact_size_adapters.rs$"],
  },
  {
    name: "flattened-options-unknowing",
    breaks: "a flatten of `Option`s made in place, a filter of the `Some`s, isn't made knowing its back, so `rfold` asks its source's `back`, which JS's own filter hasn't: a `TypeError` (#1326)",
    file: "src/lower/iterators/knowing.rs",
    find: "            .or_else(|| (self.recognition().flattened_options(ty)).map(|(source, _, mapped)| (source, mapped)));\n",
    replace: "            .or_else(|| (self.recognition().flattened_options(ty)).map(|(source, _, mapped)| (source, mapped)).filter(|_| false));\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_from_its_back.rs$"],
  },
  {
    name: "flattened-options-source-unread",
    breaks: "a flatten of `Option`s, `src.filter(present)`, is read as a `filter_map`'s, `src.map(f).filter(present)`, which it isn't, so it's refused from its back (#1326)",
    file: "src/lower/iterators/knowing.rs",
    find: "                    (false, _) => Some((source_value, None)),\n",
    replace: "                    (false, _) => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_from_its_back.rs$"],
  },
  {
    name: "as-is-map-counted-as-made",
    breaks: "a map whose JS is its source's is counted by that source with the map's marks, made in place, so its range is stepped as a JS iterator, never as a range: nothing (#1342)",
    file: "src/lower/iterators/knowing.rs",
    find: "            return self.as_exact_size(source_value, source.peel_refs(), back, span);\n",
    replace: "            return self.as_exact_size(value, source.peel_refs(), back, span);\n",
    tests: ["test/corpus.test.ts", "-t", "^as_is_map_stepped_from_its_back.rs$"],
  },
  {
    name: "as-is-map-backed-as-made",
    breaks: "a map whose JS is its source's is made knowing its back by that source with the map's marks, made in place, so its range is stepped as a JS iterator, never as a range: nothing (#1342)",
    file: "src/lower/iterators/knowing.rs",
    find: "            return self.as_double_ended(source_value, source.peel_refs(), span);\n",
    replace: "            return self.as_double_ended(value, source.peel_refs(), span);\n",
    tests: ["test/corpus.test.ts", "-t", "^as_is_map_stepped_from_its_back.rs$"],
  },
  {
    name: "counted-range-map-length-unknown",
    breaks: "a range's map a variable holds for an enumeration stepped from its back isn't made knowing its length, so its index from the back is refused, \"as an `ExactSizeIterator`, which rust-js can't count\" (#1354)",
    file: "src/lower/iterators/knowing.rs",
    find: "                        || self.made_counted_from_its_back())\n",
    replace: "                        || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^map_held_then_enumerated_from_its_back.rs$"],
  },
];
