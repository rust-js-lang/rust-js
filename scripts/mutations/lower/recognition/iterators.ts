// Mutations of src/lower/recognition/iterators.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "zip-drops-no-items",
    breaks: "a `zip()` is taken to drop no items as it's stepped, so the panic of the drop of the one it took as its other side ended, or of one it trims from the longer side's back, leaves what it collected and what's left undropped (#707, #711)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "Zip" => [first, types.next()].into_iter().flatten().map(DropsItemsOf).collect(),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "zip-drops-first-side-alone",
    breaks: "a `zip()` is taken to drop items of its first side alone, so the panic of the drop of one it trims from the back of its longer second side leaves what's left undropped (#711)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "Zip" => [first, types.next()].into_iter().flatten().map(DropsItemsOf).collect(),\n',
    replace: '            "Zip" => first.map(DropsItemsOf).into_iter().collect(),\n',
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "chain-drops-no-side",
    breaks: "a `chain()` is taken to drop no side of its as it's stepped, so the panic of the drop of its first side, exhausted, leaves its second undropped (#709)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "Chain" => [first, types.next()]\n',
    replace: '            "Chain" => [None, None]\n',
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "flatten-drops-no-inner",
    breaks: "a `flatten()` is taken to drop no inner iterator it has exhausted, so the panic of its drop leaves the rest undropped (#709)",
    file: "src/lower/recognition/iterators.rs",
    find: "                first.map(DropsExhausted).into_iter().chain(inner).collect()\n",
    replace: "                first.map(DropsExhausted).into_iter().chain(inner.filter(|_| false)).collect()\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "flat-map-drops-no-inner",
    breaks: "a `flat_map()` is taken to drop no inner iterator it has exhausted, so the panic of its drop leaves what it collected undropped (#709)",
    file: "src/lower/recognition/iterators.rs",
    find: "                    .map(DropsExhausted)\n                    .chain(inner)\n",
    replace: "                    .map(DropsExhausted)\n                    .chain(inner.filter(|_| false))\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "cloned-clones-nothing",
    breaks: "a `cloned()` is taken to clone nothing, so the panic of the crate's clone leaves what it collected undropped (#708)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "Cloned" => self.iterator_item(ty).map(Clones).into_iter().collect(),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "cycle-clones-nothing",
    breaks: "a `cycle()` is taken to clone nothing, so the panic of its clone of the crate's iterator leaves what it holds undropped (#708)",
    file: "src/lower/recognition/iterators.rs",
    find: "                .flat_map(|source| [Clones(source), DropsExhausted(source)])\n",
    replace: "                .flat_map(|source| [DropsExhausted(source)])\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "repeat-clones-nothing",
    breaks: "a `repeat()` is taken to clone nothing, so the panic of the crate's clone leaves what its collect collected undropped (#708)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "Repeat" | "RepeatN" => first.map(Clones).into_iter().collect(),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "flat-map-function-unheld",
    breaks:
      "a `flat_map()`'s function with a destructor isn't seen to be held by it, and is never dropped (ADR 0991)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::iter::FlatMap" => Some(args.type_at(2)),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_functions_dropped.rs$"],
  },
  {
    name: "text-escape-not-chars",
    breaks:
      "a held text escape isn't std's iterator of its `char`s' array: `e.next()` of `'\u00e9'.escape_default()` is `$next` of an array, `it.next is not a function` (#379, ADR 0971)",
    file: "src/lower/recognition/iterators.rs",
    find: "            || is_text_escape(self.tcx, ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^escapes_stepped.rs$"],
  },
  {
    name: "linked-list-into-iter-not-owning",
    breaks:
      "a `LinkedList`'s `into_iter()` isn't the array's items it owns: its drop reads std's field, `value.list`, of JS's iterator, `TypeError` (#377, ADR 0970)",
    file: "src/lower/recognition/iterators.rs",
    find: '                    | "std::collections::linked_list::IntoIter"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^linked_list_into_iter.rs$"],
  },
  {
    name: "boxed-array-into-iter-unknown",
    breaks:
      "a `Box<[T; N]>`'s `into_iter()`, 1.99's `BoxedArrayIntoIter`, isn't its array's items: with std's structs alone a `rev()` of one is refused, and `Array.from(names).map(f)` copies what `names.map(f)` reads (ADR 0941)",
    file: "src/lower/recognition/iterators.rs",
    find: '                    | "std::boxed::BoxedArrayIntoIter"\n',
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^boxed_array_iter.rs$"],
  },
  {
    name: "vec-into-iter-unknown",
    breaks: "a clone of a `Vec`'s `into_iter()` is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::vec::IntoIter"\n                    | "std::vec::Drain"\n',
    replace:
      '            "std::vec::Nothing"\n                    | "std::vec::Drain"\n',
    tests: ["test/corpus.test.ts", "-t", "iterator_clones"],
  },
  {
    name: "reverse-by-name",
    breaks:
      "a crate's own `struct Reverse` is taken for std's `cmp::Reverse` by its name, and compares backwards",
    file: "src/lower/recognition/iterators.rs",
    find: '        matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(self.tcx, adt.did(), StdCrate::Core)\n            && self.tcx.item_name(adt.did()).as_str() == "Reverse")',
    replace:
      '        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.item_name(adt.did()).as_str() == "Reverse")',
    tests: [
      "test/recognition.test.ts",
      "-t",
      "library recognition distinguishes user lookalikes and preserves standard behavior",
    ],
  },
  {
    name: "iterator-method-of-any-trait",
    breaks:
      "a method of another trait named as std's iterator method is, `Ord::max`, is taken for `Iterator`'s",
    file: "src/lower/recognition/iterators.rs",
    find: "            _ if !is_std_def(self.tcx, trait_id, StdItem::Iterator) => return None,\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(own_trait_by_ref|code_point_order).rs$",
    ],
  },
  {
    name: "tree-extract-if-an-array",
    breaks:
      "a B-tree's `extract_if()`, which takes its entries as it's stepped, is read as an array, `.slice()` of a generator",
    file: "src/lower/recognition/iterators.rs",
    find: '                && !path.ends_with("::ExtractIf")\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extract_if.rs$"],
  },
  {
    name: "mir-bytes-len-refused",
    breaks:
      "`s.bytes()` isn't known as the array it is: its `len()` and its clone are refused",
    file: "src/lower/recognition/iterators.rs",
    find: "        if self.is_str_bytes(ty) {\n            return Some(true);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^str_bytes.rs$"],
  },
  {
    name: "skip-of-repeat-unclosed",
    breaks:
      "`skip(n)` of a `repeat` isn't closed to drop its value, and is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::iter::Take"\n            | "std::iter::Skip"\n',
    replace: '            "std::iter::Take"\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(fold_back_drops_what_it_takes|own_iterator_dropped_through_adapters).rs$",
    ],
  },
  {
    name: "empty-iter-not-an-array",
    breaks: "a constant of `iter::Empty` is an error, as its clone isn't known",
    file: "src/lower/recognition/iterators.rs",
    find: "owning || self.is_iter_empty(ty))",
    replace: "owning || false && self.is_iter_empty(ty))",
    tests: ["test/corpus.test.ts", "-t", "^std_struct_const.rs$"],
  },
  {
    name: "dyn-double-ended-not-an-iterator",
    breaks: "a `dyn DoubleEndedIterator` isn't a JS iterator, and is refused",
    file: "src/lower/recognition/iterators.rs",
    find: "        .filter(|&id| tcx.is_diagnostic_item(sym::Iterator, id) || is_double_ended(tcx, id))\n",
    replace:
      "        .filter(|&id| tcx.is_diagnostic_item(sym::Iterator, id))\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "drain-not-owning",
    breaks:
      "a vector's `drain` isn't an iterator that owns what it took out, so one of values with destructors is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '                    | "std::vec::Drain"\n                    | "std::vec::Splice"\n',
    replace: '                    | "std::vec::Splice"\n',
    tests: ["test/corpus.test.ts", "-t", "^drain_destructors.rs$"],
  },
  {
    name: "take-of-owned-unclosed",
    breaks:
      "a `take` of an iterator that owns items with a destructor isn't closed when it's dropped",
    file: "src/lower/recognition/iterators.rs",
    find: "        if let Some(held) = self.owned_items(source).or_else(|| self.closed_on_drop(source)) {\n",
    replace:
      "        if let Some(held) = self.owned_items(source).filter(|_| false).or_else(|| self.closed_on_drop(source)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
  {
    name: "heap-into-iter-passed-as-slice",
    breaks:
      "a `BinaryHeap`'s `into_iter()` is taken to drop what it passes as one slice and take its `last` by its back, as a `Vec`'s does, where std's defaults drop each it passes from the back and fold it (#648)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::vec::IntoIter" | "std::array::IntoIter" | "std::collections::vec_deque::IntoIter"\n',
    replace:
      '            "std::vec::IntoIter"\n                | "std::array::IntoIter"\n                | "std::collections::vec_deque::IntoIter"\n                | "std::collections::binary_heap::IntoIter"\n',
    tests: ["test/corpus.test.ts", "-t", "^vec_last_takes_its_back.rs$"],
  },
  {
    name: "heap-drain-not-owning",
    breaks:
      "a heap's `drain()` isn't an iterator that owns what it took out, so one of values with destructors is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '                    | "std::collections::binary_heap::Drain"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^heap_destructors.rs$"],
  },
  {
    name: "rposition-unknown",
    breaks: "`rposition` is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            "rposition" => IterMethod::RPosition,\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "rposition"],
  },
  {
    name: "from-iter-of-values-item",
    breaks:
      "a fallible `collect` into the crate's own collection asks its `FromIterator` of the wrong item, and finds none",
    file: "src/lower/recognition/iterators.rs",
    find: "            self.tcx.mk_args(&[collection.into(), item.into(), values.into()]),\n",
    replace:
      "            self.tcx.mk_args(&[collection.into(), values.into(), values.into()]),\n",
    tests: ["test/corpus.test.ts", "-t", "collect_fallible_into"],
  },
  {
    name: "string-extend-unrecognized",
    breaks: "`s.extend(chars)` of a `String` is refused",
    file: "src/lower/recognition/iterators.rs",
    find: "        if self.is_lang_adt(ty, LangItem::String) {\n            Some(Std::PushStr)\n",
    replace: "        if false {\n            Some(Std::PushStr)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_extend.rs$"],
  },
  {
    name: "rfold-unknown",
    breaks: "`rfold` of a double-ended iterator is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            "rfold" if self.double_ended_iterator() == Some(trait_id) => IterMethod::RFold,\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "into-iter-by-dictionary-of-params",
    breaks:
      "an `I: IntoIterator` on a type parameter itself is taken as given a dictionary",
    file: "src/lower/recognition/iterators.rs",
    find: "        && matches!(self_ty.kind(), ty::Ref(_, inner, _) if known_by_bounds(*inner))\n",
    replace: "        && known_by_bounds(self_ty.peel_refs())\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_method_drops|std_projection_drops|into_iter_of_ref_param).rs$",
    ],
  },
  {
    name: "try-fold-unrecognized",
    breaks: "`try_fold` isn't known, and is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            "try_fold" => IterMethod::TryFold,\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "map-not-closed",
    breaks:
      "a `map` of an iterator of items with a destructor is refused, not closed to drop them",
    file: "src/lower/recognition/iterators.rs",
    find: '            | "std::iter::Map"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "generic-source-unclosed",
    breaks:
      "an adapter of a generic iterator, generic code's `zip`, is refused, not closed by its caller's drop",
    file: "src/lower/recognition/iterators.rs",
    find: "        if generic\n            || matches!",
    replace: "        if false\n            || matches!",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(adapter_drops|generic_skip_of_owned_rev|skip_try_fold|adapter_of_a_value_given_as_it_is|cycled_iterators)\\.rs$",
    ],
  },
  {
    name: "array-iterator-debug-misnamed",
    breaks: "a `Vec`'s `IntoIter` shows as a slice's `Iter`",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::vec::IntoIter" => Some("IntoIter"),\n',
    replace: '            "std::vec::IntoIter" => Some("Iter"),\n',
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
  },
  {
    name: "product-as-sum",
    breaks:
      "std's `Product` is taken for `Sum`, its dictionary's `product` named `sum`",
    file: "src/lower/recognition/iterators.rs",
    find: '        "Product" => Some("product"),\n',
    replace: '        "Product" => Some("sum"),\n',
    tests: ["test/corpus.test.ts", "-t", "^generic_accumulations.rs$"],
  },
  {
    name: "cycle-not-closed",
    breaks:
      "a `Cycle` isn't closed as it's dropped, so what it holds isn't dropped",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::iter::Cycle" => sources.next().filter(|&source| self.array_source(source).is_none()),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cycled_iterators.rs$"],
  },
  {
    name: "splice-unowned",
    breaks:
      "a `splice`'s items, what it took out, aren't its own, and are refused",
    file: "src/lower/recognition/iterators.rs",
    find: '                    | "std::vec::Splice"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^splice_and_inspect_drops.rs$"],
  },
  {
    name: "flat-map-type-unclosed",
    breaks:
      "a `FlatMap` held is a std type refused, not closed to drop what it holds",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::iter::FlatMap" => {\n                let (source, inner) = (sources.next()?, sources.next()?);\n',
    replace:
      '            "std::iter::FlatMapNot" => {\n                let (source, inner) = (sources.next()?, sources.next()?);\n',
    tests: ["test/corpus.test.ts", "-t", "^flat_map_drops.rs$"],
  },
  {
    name: "crate-extension-not-iterator",
    breaks:
      "a type parameter bound by the crate's trait that extends `Iterator` is a JS iterator, which its trait's dictionary steps as the impl's own type",
    file: "src/lower/recognition/iterators.rs",
    find: "    id != iterator && rustc_middle::ty::elaborate::supertrait_def_ids(tcx, id).any(|s| s == iterator)\n",
    replace: "    is_iterator_extension(tcx, id)\n",
    tests: ["test/corpus.test.ts", "-t", "^extension_trait_params.rs$"],
  },
  {
    name: "rev-type-unclosed",
    breaks:
      "a `Rev` of an array's items with a destructor held is a std type refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::iter::Rev" => {\n                let source = sources.next()?;\n',
    replace:
      '            "std::iter::Rev" if false => {\n                let source = sources.next()?;\n',
    tests: ["test/corpus.test.ts", "-t", "^rev_drops.rs$"],
  },
  {
    name: "held-filter-map-refused",
    breaks:
      "a `filter_map` or `map_while` held of items with a destructor is a std type refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            | "std::iter::FilterMap"\n            | "std::iter::MapWhile" => self.closing_source(sources.next()?)?,\n',
    replace: "            => self.closing_source(sources.next()?)?,\n",
    tests: ["test/corpus.test.ts", "-t", "^held_filter_map_drops.rs$"],
  },
  {
    name: "filter-not-back-stepped",
    breaks:
      "a filter of what knows its back isn't seen to know its own, and generic code's is refused from its back",
    file: "src/lower/recognition/iterators.rs",
    find: '                "std::iter::Map"\n                    | "std::iter::Filter"\n                    | "std::iter::FilterMap"\n',
    replace:
      '                "std::iter::Map"\n                    | "std::iter::FilterMap"\n',
    tests: ["test/corpus.test.ts", "-t", "^filter_ends.rs$"],
  },
  {
    name: "filter-map-not-back-stepped",
    breaks: "a `filter_map` of what knows its back isn't seen to know its own",
    file: "src/lower/recognition/iterators.rs",
    find: '                "std::iter::Map"\n                    | "std::iter::Filter"\n                    | "std::iter::FilterMap"\n',
    replace:
      '                "std::iter::Map"\n                    | "std::iter::Filter"\n',
    tests: ["test/corpus.test.ts", "-t", "^filter_ends.rs$"],
  },
  {
    name: "map-back-unknown",
    breaks:
      "a map of an iterator that knows its back doesn't, so `rfind` of a generic one's map is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '                "std::iter::Map"\n                    | "std::iter::Filter"\n                    | "std::iter::FilterMap"\n',
    replace:
      '                "std::iter::Filter"\n                    | "std::iter::FilterMap"\n',
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "enumerate-length-unknown",
    breaks:
      "an `enumerate` of what knows its length isn't seen to know its own",
    file: "src/lower/recognition/iterators.rs",
    find: '                "std::iter::Map" | "std::iter::Copied" | "std::iter::Enumerate" | "std::iter::Rev"\n',
    replace:
      '                "std::iter::Map" | "std::iter::Copied" | "std::iter::Rev"\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(fuse_enumerate_ends|enumerate_of_known_length).rs$",
    ],
  },
  {
    name: "map-length-unknown",
    breaks:
      "a map of an iterator that knows its length doesn't, so its `len()` is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '                "std::iter::Map" | "std::iter::Copied" | "std::iter::Enumerate" | "std::iter::Rev"\n',
    replace:
      '                "std::iter::Copied" | "std::iter::Enumerate" | "std::iter::Rev"\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(exact_size_len|map_stepped_from_its_back|owned_maps_asked_their_length)\\.rs$",
    ],
  },
  {
    name: "list-iter-not-items",
    breaks:
      "a `LinkedList`'s `iter()` is a JS helper of it, which can't be cloned where it is",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::option::Iter" | "std::option::IntoIter" | "std::result::Iter" | "std::collections::linked_list::Iter" | "std::str::CharIndices"\n',
    replace:
      '            "std::option::Iter" | "std::option::IntoIter" | "std::result::Iter" | "std::str::CharIndices"\n',
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
  },
  {
    name: "char-indices-not-items",
    breaks:
      "a string's `char_indices()` is a JS helper of its pairs, which can't be cloned",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::option::Iter" | "std::option::IntoIter" | "std::result::Iter" | "std::collections::linked_list::Iter" | "std::str::CharIndices"\n',
    replace:
      '            "std::option::Iter" | "std::option::IntoIter" | "std::result::Iter" | "std::collections::linked_list::Iter"\n',
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
  },
  {
    name: "option-iter-not-items",
    breaks: "an `Option`'s `iter()` can't be cloned where it is",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::option::Iter" | "std::option::IntoIter" | "std::result::Iter" | "std::collections::linked_list::Iter" | "std::str::CharIndices"\n',
    replace:
      '            "std::result::Iter" | "std::collections::linked_list::Iter" | "std::str::CharIndices"\n',
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
  },
  {
    name: "rev-length-unknown",
    breaks: "a `rev()` of what knows its length isn't seen to know its own",
    file: "src/lower/recognition/iterators.rs",
    find: '                "std::iter::Map" | "std::iter::Copied" | "std::iter::Enumerate" | "std::iter::Rev"\n',
    replace:
      '                "std::iter::Map" | "std::iter::Copied" | "std::iter::Enumerate"\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(std_iterator_clones|len_made_in_place|owned_maps_asked_their_length)\\.rs$",
    ],
  },
  {
    name: "try-rfold-not-from-back",
    breaks: "`try_rfold` isn't seen to step from the back, and is refused",
    file: "src/lower/recognition/iterators.rs",
    find: "                | IterMethod::RFind\n                | IterMethod::TryRFold\n",
    replace: "                | IterMethod::RFind\n",
    tests: ["test/corpus.test.ts", "-t", "^try_rfold.rs$"],
  },
  {
    name: "index-bound-not-index-mut",
    breaks:
      "a bound of `IndexMut<J>` alone on an array is given no `SliceIndex` dictionary, and refused",
    file: "src/lower/recognition/iterators.rs",
    find: "    if !(tcx.is_lang_item(tr.def_id, LangItem::Index) || tcx.is_lang_item(tr.def_id, LangItem::IndexMut)) {",
    replace: "    if !tcx.is_lang_item(tr.def_id, LangItem::Index) {",
    tests: ["test/corpus.test.ts", "-t", "^index_bound_generic.rs$"],
  },
  {
    name: "index-bound-not-of-arrays",
    breaks:
      "a bound of `IndexMut<J>` on an array is given no `SliceIndex` dictionary",
    file: "src/lower/recognition/iterators.rs",
    find: "        ty::Slice(item) | ty::Array(item, _) => item,\n        ty::Adt(adt, args) if tcx.is_diagnostic_item(sym::Vec, adt.did()) => args.type_at(0),",
    replace:
      "        ty::Slice(item) => item,\n        ty::Adt(adt, args) if tcx.is_diagnostic_item(sym::Vec, adt.did()) => args.type_at(0),",
    tests: ["test/corpus.test.ts", "-t", "^index_bound_generic.rs$"],
  },
  {
    name: "index-bound-not-of-vecs",
    breaks:
      "a bound of `Index<J>` on a `Vec<T>` is given no `SliceIndex` dictionary",
    file: "src/lower/recognition/iterators.rs",
    find: "        ty::Adt(adt, args) if tcx.is_diagnostic_item(sym::Vec, adt.did()) => args.type_at(0),\n        _ => return None,",
    replace: "        _ => return None,",
    tests: ["test/corpus.test.ts", "-t", "^index_bound_generic.rs$"],
  },
  {
    name: "index-bound-of-a-known-index",
    breaks: "a bound of `Index<usize>` is given a dictionary nothing reads",
    file: "src/lower/recognition/iterators.rs",
    find: "    if !index.has_param() {\n        return None;\n    }\n    let item = match *tr.self_ty().kind() {",
    replace: "    let item = match *tr.self_ty().kind() {",
    tests: ["test/corpus.test.ts", "-t", "^index_bound_generic.rs$"],
    snapshots: true,
  },
  {
    name: "try-for-each-unknown",
    breaks: "`try_for_each` is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            "try_for_each" => IterMethod::TryForEach,\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^try_for_each.rs$"],
  },
  {
    name: "copied-not-closed",
    breaks:
      "a `copied()` or `cloned()` of an iterator with a destructor isn't closed to drop it, and is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            // A copy or a clone of each it steps (ADR 0547).\n            | "std::iter::Copied"\n            | "std::iter::Cloned"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^consumed_adapter_drops.rs$"],
  },
  {
    name: "copied-holds-no-source",
    breaks:
      "a `copied()` of an iterator with a destructor doesn't hold it closed",
    file: "src/lower/recognition/iterators.rs",
    find: "                | IterMethod::Copied\n                | IterMethod::Cloned\n        )",
    replace: "        )",
    tests: ["test/corpus.test.ts", "-t", "^consumed_adapter_drops.rs$"],
  },
  {
    name: "into-iter-itself-unknown",
    breaks:
      "an iterator's own `IntoIterator`, std's of every iterator, is refused",
    file: "src/lower/recognition/iterators.rs",
    find: "            Ok(ImplSource::UserDefined(imp)) if self.is_std(imp.impl_def_id)\n                && matches!(self.tcx.type_of(imp.impl_def_id)",
    replace:
      "            Ok(ImplSource::UserDefined(imp)) if false && self.is_std(imp.impl_def_id)\n                && matches!(self.tcx.type_of(imp.impl_def_id)",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_held_as_it_is.rs$"],
  },
  {
    name: "compiled-iterator-a-user-iterator",
    breaks:
      "a compiled std iterator, a `Peekable`, is taken for an iterator stepped by its impl's `next`, and wrapped in a `$iterator` where it's stepped, the JS iterator it is already",
    file: "src/lower/recognition/iterators.rs",
    find: " && !is_compiled_std_iterator(self.tcx, ty)\n",
    replace: "\n",
    tests: ["test/corpus.test.ts", "-t", "^std_peekable.rs$"],
    snapshots: true,
  },
  {
    name: "byte-escape-not-stepped",
    breaks:
      "a byte's escape isn't a `$iter` of its bytes, and stepping it from its back or cloning it is refused",
    file: "src/lower/recognition/iterators.rs",
    find: "            || is_byte_escape(self.tcx, ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^byte_escape.rs$"],
  },
  {
    name: "compiled-iterator-stepped-by-library",
    breaks:
      "`Iterator`'s methods of a compiled std iterator, a `Peekable`'s `next`, resolve to the library's compiled impl and are called there, not stepped as the JS iterator its struct is",
    file: "src/lower/recognition/iterators.rs",
    find: "            .is_some_and(|self_ty| is_compiled_std_iterator(tcx, self_ty))\n        && !is_compiled_std(tcx, def_id)\n",
    replace:
      "            .is_some_and(|self_ty| is_compiled_std_iterator(tcx, self_ty))\n        && false\n        && !is_compiled_std(tcx, def_id)\n",
    tests: ["test/corpus.test.ts", "-t", "^std_peekable.rs$"],
    snapshots: true,
  },
  {
    name: "compiled-provided-of-compiled-iterator-not-crates",
    breaks:
      "a compiled provided method of a compiled std iterator, `peekable()` of a `Fuse`, doesn't resolve to the library, and is refused",
    file: "src/lower/recognition/iterators.rs",
    find: "        && !is_compiled_std(tcx, def_id)\n}\n",
    replace: "}\n",
    tests: ["test/corpus.test.ts", "-t", "^held_peekable.rs$"],
  },
  {
    name: "successors-not-closed-on-drop",
    breaks:
      "a `Successors` holding a value with a destructor is refused, as a std type that holds one",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::iter::Repeat" | "std::iter::RepeatN" | "std::iter::Successors" => sources.next(),\n',
    replace:
      '            "std::iter::Repeat" | "std::iter::RepeatN" => sources.next(),\n',
    tests: ["test/corpus.test.ts", "-t", "^successors_drops.rs$"],
  },
  {
    name: "map-into-iter-not-owning",
    breaks:
      "a map's or a set's `into_iter()` of values with a destructor is refused, as a std type holding one",
    file: "src/lower/recognition/iterators.rs",
    find: '                    | "std::collections::btree_map::IntoIter"\n                    | "std::collections::hash_set::IntoIter"\n                    | "std::collections::btree_set::IntoIter"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_into_iter_drops.rs$"],
  },
  {
    name: "rev-of-closing-unclosed",
    breaks:
      "a `rev()` of a chain over items with a destructor isn't closed as it's dropped, so what it never gave isn't dropped",
    file: "src/lower/recognition/iterators.rs",
    find: "                    None => self.closing_source(source)?,\n",
    replace: "                    None => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_drops.rs$"],
  },
  {
    name: "user-iterator-opaque-unrevealed",
    breaks:
      "an `impl Iterator` the crate's iterator is isn't taken for it, so consuming it, `count().sum()`, never drops it",
    file: "src/lower/recognition/iterators.rs",
    find: "        let ty = self.reveal(ty.peel_refs());\n        matches!(ty.kind(), ty::Adt(..)) && self.has_user_impl(iterator, ty)",
    replace:
      "        let ty = ty.peel_refs();\n        matches!(ty.kind(), ty::Adt(..)) && self.has_user_impl(iterator, ty)",
    tests: ["test/corpus.test.ts", "-t", "^drops_of_consumed_iterators.rs$"],
  },
  {
    name: "option-into-iter-unstepped",
    breaks:
      "an `Option`'s `into_iter()` can't be stepped from its back, so `chain(Some(x)).rev()` is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '"std::option::Iter" | "std::option::IntoIter" | ',
    replace: '"std::option::Iter" | ',
    tests: ["test/corpus.test.ts", "-t", "^rev_chain.rs$"],
  },
  {
    name: "chunks-not-made-arrays",
    breaks:
      "a slice's chunks and windows aren't known for the arrays they're made, so stepping one from its back is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '            | "std::slice::Chunks" | "std::slice::ChunksMut" | "std::slice::Windows"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^chunks_from_their_back.rs$"],
  },
  {
    name: "folds-what-it-consumes-not-sum",
    breaks: "`sum()` of an iterator whose own fold drops it drops it again",
    file: "src/lower/recognition/iterators.rs",
    find: '                "sum"\n                    | "product"',
    replace: '                "product"',
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "chain-not-cloned-apart",
    breaks: "cloning a `chain` of what's cloned is refused",
    file: "src/lower/recognition/iterators.rs",
    find: '        "std::iter::Chain" => Parted::Chain,\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cloned_chains_and_cycles.rs$"],
    snapshots: true,
  },
  {
    name: "deque-into-iter-unowned",
    breaks:
      "a `VecDeque`'s `into_iter()` doesn't own its items: behind `fuse()` its drop reads std's `inner`, which throws, and what `take_while` leaves is never dropped (#203)",
    file: "src/lower/recognition/iterators.rs",
    find: '                    | "std::collections::vec_deque::IntoIter"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^deque_into_iter_drops.rs$"],
  },
  {
    name: "held-compiled-no-adapter",
    breaks:
      "a `map()` made in place, which a `rev()` steps from its back, isn't std's adapter, so it isn't made knowing its back",
    file: "src/lower/recognition/iterators.rs",
    find: "        && (!is_compiled_std_type(tcx, id) || super::is_held_compiled_def(tcx, id))\n",
    replace: "        && !is_compiled_std_type(tcx, id)\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rev_of_closed_items|rev_lazy_drops|std_rev).rs$",
    ],
  },
  {
    name: "inspect-holds-no-function",
    breaks:
      "an `Inspect` whose closure took what has a destructor is refused, not dropped after its source as a `Map`'s is",
    file: "src/lower/recognition/iterators.rs",
    find: '            | "std::iter::MapWhile"\n            | "std::iter::Inspect" => Some(args.type_at(1)),\n',
    replace: '            | "std::iter::MapWhile" => Some(args.type_at(1)),\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(std_inspect|adapter_functions_dropped).rs$",
    ],
  },
  {
    name: "closing-source-holding-nothing-unclosed",
    breaks:
      "a zip of a slice's `iter_mut()` or a `chars()` and a `Vec` of items with a destructor is dropped by its fields, `for (const item of iter.b)`, which a `$lazyZip` hasn't: a TypeError (#406)",
    file: "src/lower/recognition/iterators.rs",
    find: "        (self.array_source(source) == Some(false) || self.range_kind(source).is_some() || holds_nothing).then_some(None)\n",
    replace:
      "        (self.array_source(source) == Some(false) || self.range_kind(source).is_some()).then_some(None)\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_borrowed_and_owned.rs$"],
  },
  {
    name: "into-values-unowned",
    breaks:
      "a map's `into_keys()` and `into_values()` own nothing, so `into_values().take(n)` is `Array.from(m.values()).slice(0, n)`, which never drops what it leaves (#432)",
    file: "src/lower/recognition/iterators.rs",
    find: '                    | "std::collections::hash_map::IntoKeys"\n                    | "std::collections::hash_map::IntoValues"\n                    | "std::collections::btree_map::IntoKeys"\n                    | "std::collections::btree_map::IntoValues"\n',
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^collection_taken_drops_what_is_left.rs$",
    ],
  },
  {
    name: "owned-vec-items-unsaid",
    breaks: "a `for` over a `Vec` it owns, `for s in v`, is a `for…of`, where one over its slice counts (ADR 1042)",
    file: "src/lower/recognition/iterators.rs",
    find: "            ) && std_path(self.tcx, adt.did()) == \"std::vec::IntoIter\")\n",
    replace: "            ) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^array_counted_loops.rs$"],
    snapshots: true,
  },
];
