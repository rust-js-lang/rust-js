// Mutations of src/lower/recognition/iterators.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "filter-drops-no-items",
    breaks:
      "a `filter()` or a `take_while()` is taken to drop no items as it's stepped, so where its closure can't panic, the panic of the drop of the item it rejects leaves what's left undropped (#701)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "Skip" | "StepBy" | "SkipWhile" | "Take" | "Filter" | "TakeWhile" => {\n',
    replace: '            "Skip" | "StepBy" | "SkipWhile" | "Take" => {\n',
    tests: ["test/corpus.test.ts", "-t", "^closure_stages_that_never_unwind_past_adapters.rs$"],
  },
  {
    name: "fuse-drops-nothing",
    breaks:
      "a `fuse()` is taken to drop nothing as it's stepped, so where its stages can't panic, the panic of the drop of the iterator it has exhausted leaves what it collected undropped (#701)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "Fuse" => first.map(DropsExhausted).into_iter().collect(),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^closure_stages_that_never_unwind_past_adapters.rs$"],
  },
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
    name: "flat-map-function-unheld",
    breaks:
      "a `flat_map()`'s function with a destructor isn't seen to be held by it, and is never dropped (ADR 0991)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::iter::FlatMap" => return Some((args.type_at(2), HeldFolded::Alone)),\n',
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
    tests: ["test/corpus.test.ts", "-t", "^generic_rev_nths_its_own_back.rs$"],
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
    find: '            "std::vec::IntoIter"\n                    | "std::vec::Drain"\n',
    replace: '            "std::vec::IntoIter"\n',
    tests: ["test/corpus.test.ts", "-t", "^drain_destructors.rs$"],
  },
  {
    name: "take-of-owned-unclosed",
    breaks:
      "a `take` of an iterator that owns items with a destructor isn't closed when it's dropped, but each item left in its source dropped by hand, `for (const item of iter.iter)`, where closing it, `iter.return()`, drops the same: since the one `Take` keeps its source as `iter` (#830), only how it reads",
    file: "src/lower/recognition/iterators.rs",
    find: "        if let Some(held) = self.owned_items(source).or_else(|| self.closed_on_drop(source)) {\n",
    replace:
      "        if let Some(held) = self.owned_items(source).filter(|_| false).or_else(|| self.closed_on_drop(source)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
    snapshots: true,
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
    find: "            self.tcx.mk_args(&[collection.into(), item.into(), given.into()]),\n",
    replace: "            self.tcx.mk_args(&[collection.into(), given.into(), given.into()]),\n",
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
    find: "        if generic\n            || boxed_dyn\n",
    replace: "        if false\n            || boxed_dyn\n",
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
    name: "list-iter-not-items",
    breaks:
      "a `LinkedList`'s `iter()` is a JS helper of it, which can't be cloned where it is",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::option::Iter" | "std::option::IntoIter" | "std::result::Iter" | "std::collections::linked_list::Iter"\n',
    replace:
      '            "std::option::Iter" | "std::option::IntoIter" | "std::result::Iter"\n',
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
  },
  {
    name: "option-iter-not-items",
    breaks: "an `Option`'s `iter()` can't be cloned where it is",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::option::Iter" | "std::option::IntoIter" | "std::result::Iter" | "std::collections::linked_list::Iter"\n',
    replace:
      '            "std::result::Iter" | "std::collections::linked_list::Iter"\n',
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
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
    tests: ["test/iterator-forms.test.ts", "-t", "^last_takes_its_source with"],
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
    name: "deque-into-iter-unowned",
    breaks:
      "a `VecDeque`'s `into_iter()` doesn't own its items: behind `fuse()` its drop reads std's `inner`, which throws, and what `take_while` leaves is never dropped (#203)",
    file: "src/lower/recognition/iterators.rs",
    find: '                    | "std::collections::vec_deque::IntoIter"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^deque_into_iter_drops.rs$"],
  },
  {
    name: "inspect-holds-no-function",
    breaks:
      "an `Inspect` whose closure took what has a destructor is refused, not dropped after its source as a `Map`'s is",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::iter::Filter" | "std::iter::FilterMap" | "std::iter::Inspect" => HeldFolded::Before,\n',
    replace: '            "std::iter::Filter" | "std::iter::FilterMap" => HeldFolded::Before,\n',
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
  {
    name: "map-folded-before",
    breaks: "a `map`'s function, folded inside another's, drops before what drops with the fold's `f` (#710)",
    file: "src/lower/recognition/iterators.rs",
    find: "            \"std::iter::Map\" => HeldFolded::After,\n",
    replace: "            \"std::iter::Map\" => HeldFolded::Before,\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
  },
  {
    name: "filter-folded-after",
    breaks: "a `filter`'s function, folded, drops after the function of the `map` over it (#710)",
    file: "src/lower/recognition/iterators.rs",
    find: "            \"std::iter::Filter\" | \"std::iter::FilterMap\" | \"std::iter::Inspect\" => HeldFolded::Before,\n",
    replace: "            \"std::iter::Filter\" | \"std::iter::FilterMap\" | \"std::iter::Inspect\" => HeldFolded::After,\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
  },
  {
    name: "flat-map-folded-after",
    breaks: "a `flat_map`'s function is folded as a `map`'s, with what drops with its `f`, after its source (#710)",
    file: "src/lower/recognition/iterators.rs",
    find: "            \"std::iter::FlatMap\" => return Some((args.type_at(2), HeldFolded::Alone)),\n",
    replace: "            \"std::iter::FlatMap\" => return Some((args.type_at(2), HeldFolded::After)),\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
  },
  {
    name: "take-while-folded",
    breaks: "a `take_while`'s predicate is moved by its fold, so it drops before what's left, where std's `try_fold` keeps it (#710)",
    file: "src/lower/recognition/iterators.rs",
    find: "            \"std::iter::TakeWhile\" | \"std::iter::SkipWhile\" | \"std::iter::MapWhile\" => HeldFolded::Kept,\n",
    replace: "            \"std::iter::TakeWhile\" | \"std::iter::SkipWhile\" | \"std::iter::MapWhile\" => HeldFolded::After,\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
  },

  {
    name: "scan-not-closed-on-drop",
    breaks: "a `scan()` isn't closed to drop what it holds, so a compiled one a consumer's fold took is dropped again by its fields, its source twice (#741, #748)",
    file: "src/lower/recognition/iterators.rs",
    find: "            \"std::iter::Scan\" => {\n",
    replace: "            \"std::iter::Scan\" if false => {\n",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^scan_dropped_once.rs$|^scan_dropped_once with"],
  },
  {
    name: "scan-state-not-closed",
    breaks: "closing a `scan()` drops what its source holds and its function, not its state, so a compiled one of a state with a destructor over a range drops it twice (#741)",
    file: "src/lower/recognition/iterators.rs",
    find: "                self.closing_both(source, self.dropping_parts(&[function, state]))\n",
    replace: "                self.closing_both(source, self.dropping_parts(&[function]))\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^a scan's state and function drop once"],
  },
  {
    name: "max-by-not-from-mir",
    breaks: "`max_by` of the crate's iterator with a destructor isn't lowered from MIR, and is refused (#734)",
    file: "src/lower/recognition/iterators.rs",
    find: "            \"max_by\" => IterMethod::MaxBy(true),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^max_by_drops_its_function_first.rs$"],
  },
  {
    name: "btree-extract-if-not-closed-on-drop",
    breaks: "a B-tree's `extract_if` of owned bounds with a destructor isn't closed where a variable that holds it is dropped, so its bounds are never dropped (#945)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::collections::btree_map::ExtractIf" => {\n',
    replace: '            "std::collections::btree_map::ExtractIf" if false => {\n',
    tests: ["test/corpus.test.ts", "-t", "^extract_if_drops_what_it_holds.rs$"],
  },
  {
    name: "hash-extract-if-not-closed-on-drop",
    breaks: "a hashed map's `extract_if` whose test holds what has a destructor isn't closed where it's dropped, so its test is never dropped (#945)",
    file: "src/lower/recognition/iterators.rs",
    find: '            "std::collections::hash_map::ExtractIf" => self.dropping_parts(&[sources.nth(2)?]),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extract_if_drops_what_it_holds.rs$"],
  },
  {
    name: "from-iter-iterator-its-items",
    breaks: "`Vec::from_iter(iter)` of a type parameter's iterator asks its caller nothing, so it's refused, `the buffer std collects an iterator in place in` (#737)",
    file: "src/lower/recognition/iterators.rs",
    find: "        .then(|| args.types().nth(2))\n",
    replace: "        .then(|| args.types().nth(1))\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_from_iter_in_place.rs$"],
  },
  {
    name: "flattened-option-owns-nothing",
    breaks: "a `flatten()` of `Option`s given to generic code with its drop is given std's `Flatten` struct's glue, `flatten.inner.iter.return()`, of a JS filter that has no `inner`: `Cannot read properties of undefined (reading 'iter')` (#1240)",
    file: "src/lower/recognition/iterators.rs",
    find: "        match self.owned_into_items(inner).or_else(|| self.option_of(inner)) {",
    replace: "        match self.owned_into_items(inner) {",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_given_to_generic_code\\.rs$"],
  },
  {
    name: "flat-map-options-unmapped",
    breaks: "a `flat_map` to `Option`s isn't said to make them by its function, so it's made `src.filter(present)`, its function never called (#1326)",
    file: "src/lower/recognition/iterators.rs",
    find: "            Some(returned) => Some((source, self.option_of(returned)?, true)),\n",
    replace: "            Some(returned) => Some((source, self.option_of(returned)?, false)),\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_from_its_back.rs$"],
  },
  {
    name: "flatten-of-option-refs-flat",
    breaks:
      '`parts.iter().flatten()` of `&[Option<&str>]` is `parts.flat()`, which keeps the `None`s: `"a  b "`; and one from its back JS\'s own `flatMap` of each, which isn\'t iterable (#1326)',
    file: "src/lower/recognition/iterators.rs",
    find: "            None => Some((source, self.option_of(self.iterator_item(source)?.peel_refs())?, false)),\n",
    replace: "            None => Some((source, self.option_of(self.iterator_item(source)?)?, false)),\n",
    tests: ["test/corpus.test.ts", "-t", "iter_flatten|^flatten_of_options_from_its_back.rs$"],
  },
  {
    name: "once-no-array-of-items",
    breaks: "`iter::once(x)`, `[x]`, isn't taken for an array of items, so its `size_hint()` is refused, \"from its MIR\", where std's is `(1, Some(1))` (#1375)",
    file: "src/lower/recognition/iterators.rs",
    find: "            | \"std::iter::Once\"",
    replace: "            | \"std::iter::OnceNever\"",
    tests: ["test/corpus.test.ts", "-t", "^once_and_once_with_hint.rs$"],
  },
  {
    name: "boxed-dyn-iterator-source-unclosed",
    breaks: "a `filter_map` of a `Box<dyn Iterator>` that carries a drop, quickcheck's `char` shrinker, is no source it can close, and boxing it is refused (#1420)",
    file: "src/lower/recognition/iterators.rs",
    find: "            || boxed_dyn\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "library_filter_map_of_a_boxed_iterator"],
  },
  {
    name: "slice-search-runs-no-code",
    breaks: "a slice's search by a predicate that runs code is taken to run none, so it's the array of its pieces, the predicate asked of every item first (#1501)",
    file: "src/lower/recognition/iterators.rs",
    find: "            return args\n                .types()\n                .nth(1)\n                .is_some_and(|pattern| self.pattern_runs_code(pattern));\n",
    replace: "            return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_searches_ask_a_predicate_as_they_step.rs$"],
  },
  {
    name: "box-passes-on-every-own-step",
    breaks: "a `Box` of the crate's iterator reaches its own `fold` and `count`, which std's `Box<I>` never calls (#1601)",
    file: "src/lower/recognition/iterators.rs",
    find: "        if !self.is_user_iterator(ty) || (behind_a_box(ty) && !BOX_PASSES_ON.contains(&name)) {\n",
    replace: "        if !self.is_user_iterator(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
  {
    name: "box-iterator-not-held",
    breaks: "a `Box` of the crate's iterator isn't its iterator, so its value is stepped as plain data, `count()` of `Box::new(Up(0, 3))` its fields' (#1601)",
    file: "src/lower/recognition/iterators.rs",
    find: "    while let Some(inner) = ty.boxed_ty() {\n",
    replace: "    while let Some(inner) = ty.boxed_ty().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
];
