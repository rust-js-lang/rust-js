// Mutations of src/lower/mir/iter/backs.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "compiled-map-reversed-unknowing",
    breaks:
      "a compiled `Map` of items with a destructor given back by a call is reversed as it is, `$rev(it)`, which finds no back (#999)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                true => self.as_double_ended(receiver, iter_ty, span)?,\n",
    replace: "                true => receiver,\n",
    tests: ["test/corpus.test.ts", "-t", "^compiled_map_given_back_reversed.rs$"],
  },
  {
    name: "given-as-it-is-rev-unclosed",
    breaks:
      "`rev()` of a type parameter given as it is, stepped by its dictionary's `next_back`, never drops it: `drop a at 0` is lost (#971)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            None if (self.reversed_made_knowing(output) || self.closes_to_drop(receiver_ty))\n",
    replace: "            None if self.reversed_made_knowing(output)\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_named_by_another_bound.rs$"],
  },
  {
    name: "given-enumeration-unclosed",
    breaks:
      "an `enumerate()` of a type parameter given as it is, made knowing its back, never drops it: `drop 0` and `drop 1` are lost (#971)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    Some(it) => self.closing(it, held, span)?,\n",
    replace: "                    Some(it) => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_named_by_another_bound.rs$"],
  },
  {
    name: "range-map-made-lengthless",
    breaks:
      "a range's map made knowing its length, a zip's `b` read by index, is made knowing only its back: `$trustedZip` throws `b.len is not a function` (#775)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    None if len && self.range_kind(iter_ty).is_some() => {\n",
    replace: "                    None if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_by_index_of_holding_maps.rs$"],
  },
  {
    name: "enumerate-of-a-slice-nth-mapped",
    breaks:
      "an `enumerate()` of a reversed `Vec` is a JS `map`, whose `nth` steps each item back to front, not the source's one slice (#277)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    || self.recognition().nths_a_slice(output)\n",
    replace: "                    || false\n",
    tests: ["test/corpus.test.ts", "-t", "^step_by_compiled_source_nth.rs$"],
  },
  {
    name: "mir-rev-items-forward",
    breaks: "`rev()` of items made already gives them in order",
    file: "src/lower/mir/iter/backs.rs",
    find: "        let items = crate::lower::iterators::reversed(items);\n",
    replace: "        let items = { items };\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(boxed_array_iter|fused_iterator|str_bytes|sales_report|matrix_strings).rs$",
    ],
  },
  {
    name: "mir-rev-of-fresh-copied",
    breaks:
      "`xs.iter().rev()` from MIR copies the array first, `$rest($iter(xs)).toReversed()`",
    file: "src/lower/mir/iter/backs.rs",
    find: "                it if crate::lower::iterators::is_stepped_items(&it) => crate::lower::iterators::iterated(it),\n",
    replace:
      "                it if false => crate::lower::iterators::iterated(it),\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "iterator chains from MIR are array methods",
    ],
  },
  {
    name: "mir-rev-of-counted-refused",
    breaks: "MIR's `rev` of an array's items skipped and taken from is refused",
    file: "src/lower/mir/iter/backs.rs",
    find: "            None if counted_chain(&receiver) => sliced(&receiver),\n",
    replace: "",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "rev of skipped and taken",
    ],
  },
  {
    name: "mir-rev-of-kept-iterator-wrapped",
    breaks:
      "`rev()` of an iterator a variable holds wraps its `$iter` again, `it.toReversed()`, which throws",
    file: "src/lower/mir/iter/backs.rs",
    find: "                if matches!(\n                    receiver.kind,\n                    js::ExprKind::Var(_) | js::ExprKind::Member(..) | js::ExprKind::Index(..)\n                ) || given_by_a_call(&receiver) =>\n",
    replace: "                if false || given_by_a_call(&receiver) =>\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "char ranges step by code point",
    ],
  },
  {
    name: "double-ended-next-back-refused",
    breaks: "`next_back()` of a generic `T: DoubleEndedIterator` is refused",
    file: "src/lower/mir/iter/backs.rs",
    find: "        (self.steps_an_array(iter_ty) || structural() && !crate::lower::iterators::form_lacks_a_back(value))\n",
    replace: "        (self.steps_an_array(iter_ty) || false && !crate::lower::iterators::form_lacks_a_back(value))\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "rev-chain-refused",
    breaks:
      "`rev()` of a lazy chain over a slice, `v.iter().map(f).rev()`, is refused, though its closures run in the same order over the items reversed",
    file: "src/lower/mir/iter/backs.rs",
    find: "            None if let Some(reversed) = reversed_inward(&receiver)\n",
    replace: "            None if let Some(reversed) = None::<Expr>\n",
    tests: ["test/corpus.test.ts", "-t", "rev_of_lazy_chains"],
    snapshots: true,
  },
  {
    name: "rev-chain-index-from-back",
    breaks:
      "an `enumerate().rev()`'s index is counted over the items reversed, `(2, 5)` where it's `(0, 5)`",
    file: "src/lower/mir/iter/backs.rs",
    find: "        js::ExprKind::Arrow(params, _) => params.len() == 1,\n",
    replace: "        js::ExprKind::Arrow(..) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "rev_of_lazy_chains"],
  },
  {
    name: "back-step-receiver-wrapped",
    breaks:
      "`nth_back` and `rfold` of an iterator that knows its back wrap it again, `$nthBack($iter(it), ..)`, which `$iter` gives back as it is (ADR 0456)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        let it = match self.range_kind(iter_ty).is_none() && self.has_a_back(iter_ty, &it) {\n",
    replace: "        let it = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
    snapshots: true,
  },
  {
    name: "back-step-unboxed",
    breaks:
      "`nth_back` of items that could look like `None` gives `None` for a `Some(None)`",
    file: "src/lower/mir/iter/backs.rs",
    find: '                true => Expr::helper("$some"),\n',
    replace: "                true => Expr::undefined(),\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "back-step-drops-undone",
    breaks:
      "what `nth_back` and `rfind` pass over of items with a destructor isn't dropped",
    file: "src/lower/mir/iter/backs.rs",
    find: "        given.extend(drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^back_steps_drop.rs$"],
  },
  {
    name: "by-ref-back-lost",
    breaks:
      "`by_ref()` of an iterator that knows its back is lent as `$lent(Iterator.from(it))`, the same iterator, as `Iterator.from` gives it back, read less plainly than `it`",
    file: "src/lower/mir/iter/backs.rs",
    find: "            Some(IterMethod::ByRef) if self.knows_its_back(iter_ty) => {\n",
    replace: "            Some(IterMethod::ByRef) if false => {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "rev-range-chain-refused",
    breaks: "`rev()` of a lazy chain over a range is refused",
    file: "src/lower/mir/iter/backs.rs",
    find: "    if let Some(range) = crate::lower::iterators::stepped_range(e)\n        && !range.back\n    {\n",
    replace:
      "    if let Some(range) = crate::lower::iterators::stepped_range(e)\n        && false\n    {\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_range.rs$"],
    snapshots: true,
  },
  {
    name: "rev-range-chain-inclusive-as-exclusive",
    breaks:
      "`rev()` of a chain over `a..=b` steps it from `b - 1`, leaving `b` out",
    file: "src/lower/mir/iter/backs.rs",
    find: "            true => crate::lower::std_types::range::RangeKind::Inclusive,\n",
    replace:
      "            true => crate::lower::std_types::range::RangeKind::Exclusive,\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_range.rs$"],
  },
  {
    name: "given-rev-refused",
    breaks: "`rev()` of an `I` given as it is is refused",
    file: "src/lower/mir/iter/backs.rs",
    find: "        let back = Expr::member(self.iterator_evidence(iter_ty, double_ended)?, next_back);\n",
    replace: "        let back: Expr = return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^given_iterator_extensions.rs$"],
    snapshots: true,
  },
  {
    name: "given-map-source-raw",
    breaks:
      "a `map` of an `I` given as it is that knows its back is `$mapEnds` of its caller's value, a range, not a JS iterator",
    file: "src/lower/mir/iter/backs.rs",
    find: "                // another adapter, made knowing it, or refused (ADR 0545).\n                let it = match self.given_stepped(receiver.clone(), iter_ty, back, len) {\n",
    replace:
      "                // another adapter, made knowing it, or refused (ADR 0545).\n                let it = match self.given_stepped(receiver.clone(), iter_ty, back, len).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^map_stepped_from_its_back.rs$"],
  },
  {
    name: "rev-of-destructors-reversed-copy",
    breaks:
      "`rev()` of an array's items with a destructor is a reversed copy, which drops what's left back to front",
    file: "src/lower/mir/iter/backs.rs",
    find: "        if item_drop.is_none() {\n            return Ok(None);\n        }\n",
    replace:
      "        if item_drop.is_none() || true {\n            return Ok(None);\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_drops.rs$"],
  },
  {
    name: "cloned-apart-source-shared",
    breaks:
      "a clone of a `map` cloned apart shares its source, stepped by both",
    file: "src/lower/mir/iter/backs.rs",
    find: "            self.clone_in_place(Expr::var(&a), part_tys[0], span, &mut cloned)?,\n",
    replace: "            Expr::var(&a),\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "cloned-apart-function-shared",
    breaks:
      "a clone of a `map` cloned apart shares its closure, which changes what it took for both",
    file: "src/lower/mir/iter/backs.rs",
    find: "            self.clone_in_place(Expr::var(&b), part_tys[1], span, &mut cloned)?,\n",
    replace: "            Expr::var(&b),\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "cloned-apart-unbound-shared",
    breaks: "generic code clones a `filter`'s source it has no `Clone` of",
    file: "src/lower/mir/iter/backs.rs",
    find: "        if let Some(ty) = unbound {\n",
    replace: "        if let Some(ty) = unbound.filter(|_| false) {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a filter cloned where its source isn't bound by Clone",
    ],
  },
  {
    name: "filter-ends-not-made",
    breaks:
      "generic code's filter of what knows its back is a JS helper, which doesn't",
    file: "src/lower/mir/iter/backs.rs",
    find: "            Some(IterMethod::Filter) if self.knows_its_back(output) => {\n",
    replace:
      "            Some(IterMethod::Filter) if false && self.knows_its_back(output) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^filter_ends.rs$"],
  },
  {
    name: "enumerate-ends-native",
    breaks:
      "an `enumerate` of what knows its back or its length is a JS helper, which knows neither (a type parameter's is `$enumerate` as its `nth` may be its own, #496)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            Some(IterMethod::Enumerate)\n                if self.knows_its_back(output)\n                    || self.knows_its_length(output)\n",
    replace: "            Some(IterMethod::Enumerate)\n                if false\n                    || false\n",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_of_known_length.rs$"],
  },
  {
    name: "rev-stored-items-lazy",
    breaks:
      "a `rev()` of a string's `chars()` a variable holds is a lazy iterator's, `$rev` of it, not what's left of its array reversed",
    file: "src/lower/mir/iter/backs.rs",
    find: "        let items = match self.steps_an_array(iter_ty).then_some(()) {\n",
    replace:
      "        let items = match self.array_source(iter_ty).map(|_| ()) {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
    snapshots: true,
  },
  {
    name: "try-rfold-forward",
    breaks: "`try_rfold` folds from the front",
    file: "src/lower/mir/iter/backs.rs",
    find: "            let reversed = crate::lower::iterators::reversed_by(it);\n",
    replace: "            let reversed = it;\n",
    tests: ["test/corpus.test.ts", "-t", "^try_rfold.rs$"],
  },
  {
    name: "mir-rev-of-a-lazy-map-refused",
    breaks:
      "`rev()` of a lazy map of a range is refused, not stepped from its back",
    file: "src/lower/mir/iter/backs.rs",
    find: "            None => {\n                let in_place = crate::lower::iterators::made_in_place(&receiver);\n                let it = self.as_double_ended(receiver, iter_ty, span)?;\n                let it = self.closing(it, iter_ty, span)?;\n                let it = match in_place {\n                    true => it.in_place(),\n                    false => it,\n                };\n                return Ok(crate::lower::iterators::reversed_by(it));\n            }",
    replace: "            None => {\n                return Err(self.unsupported(span, \"`rev` of a lazy iterator, from its MIR\"));\n            }",
    tests: ["test/corpus.test.ts", "-t", "^range_mapped_by_a_function.rs$"],
  },

  {
    name: "back-stepped-source-raw",
    breaks:
      "a range a map the crate steps from its back maps is given as it is, not knowing its back",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    None if back && !self.has_a_back(iter_ty, receiver) => {\n",
    replace:
      "                    None if back && !self.has_a_back(iter_ty, receiver) && self.range_kind(iter_ty).is_none() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(map_stepped_from_its_back|range_map_rfold_holding).rs$"],
  },
  {
    name: "map-ends-source-unclosed",
    breaks:
      "a map made knowing its back, of a `Vec`'s items with a destructor, doesn't close its source, so what it never gave isn't dropped",
    file: "src/lower/mir/iter/backs.rs",
    find: "                // through a `&mut`, `by_ref()`'s, its owner's (#749).\n                let it = self.closing(it, held, span)?;\n",
    replace: "                // through a `&mut`, `by_ref()`'s, its owner's (#749).\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_drops.rs$"],
  },
  {
    name: "range-by-ref-a-js-iterator",
    breaks:
      "`by_ref()` of a range is a JS iterator of it, which what steps it after steps as a range again: `take` gives nothing, and the range never moves",
    file: "src/lower/mir/iter/backs.rs",
    find: "            Some(IterMethod::ByRef) if self.range_kind(iter_ty).is_some() => receiver.clone(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^by_ref_steps.rs$"],
  },
  {
    name: "enumerate-of-user-lengthless",
    breaks:
      "an enumeration of the crate's exact-sized iterator is made without its length, so its index from the back throws",
    file: "src/lower/mir/iter/backs.rs",
    find: "                let len = of_param || self.knows_its_length(iter_ty) || self.user_exact_size(iter_ty);\n",
    replace: "                let len = of_param || self.knows_its_length(iter_ty);\n",
    tests: ["test/corpus.test.ts", "-t", "^user_iterator_ends.rs$"],
  },
  {
    name: "rev-of-returned-iter-unwrapped",
    breaks: "what a function gives is reversed by what's left of it",
    file: "src/lower/mir/iter/backs.rs",
    find: "    matches!(&e.kind, js::ExprKind::Call(callee, _) if callee.helper_name().is_none())\n",
    replace: "    false\n",
    tests: ["test/corpus.test.ts", "-t", "^returned_slice_iters.rs$"],
  },
  {
    name: "enumerate-of-own-nth-mapped",
    breaks: "an enumerate of what reaches the crate's own nth is $enumerate",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    || self.recognition().reaches_its_own(output, OwnStep::Nth, Reach::Sure)\n",
    replace: "                    || false\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "rev-of-held-rev-taken-out",
    breaks:
      "`rev().rev()` of an array's items with a destructor takes them out, `$rest(..).toReversed()`, nothing closing what's left (#205)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            if !self.reversed_in_order(iter_ty) {\n",
    replace:
      "            if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_of_closed_items.rs$"],
  },
  {
    name: "rev-of-rev-kept",
    breaks:
      "`rev().rev()` of an array's items or a range stays `$rev($rev(it))`, not `it` (#205)",
    file: "src/lower/mir/iter/backs.rs",
    find: "    // `Rev`'s `last` folds, which a drop that panics shows (#722).\n    if let Some(inner) = crate::lower::iterators::reversed_of(e)\n",
    replace: "    // `Rev`'s `last` folds, which a drop that panics shows (#722).\n    if false && let Some(inner) = crate::lower::iterators::reversed_of(e)\n",
    tests: ["test/corpus.test.ts", "-t", "^(vec_extend_indexed|rev_given_to_generic_code).rs$"],
    snapshots: true,
  },
  {
    name: "back-kept-map-source-unknowing",
    breaks:
      "a map stepped from its back of a `skip` or a `step_by` made here steps a JS iterator with no back, and throws (#218)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    None if back && !self.has_a_back(iter_ty, receiver) => {\n",
    replace: "                    None if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^back_stepped_adapter_sources.rs$"],
  },
  {
    name: "back-kept-filter-source-unknowing",
    breaks:
      "a filter stepped from its back of a `take` made here steps a JS iterator with no back, and throws (#218)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    None if !self.has_a_back(iter_ty, receiver) => {\n                        self.as_double_ended(receiver.clone(), iter_ty, span)?\n                    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^back_stepped_adapter_sources.rs$"],
  },

  {
    name: "enumerate-back-own-unclosed",
    breaks:
      "an `enumerate()` of the crate's iterator with a destructor stepped from its back isn't closed to drop it (#223)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                        let knowing = self.user_iterator_knowing(receiver.clone(), held, (back, len), span)?;\n                        self.closing(knowing, held, span)?\n",
    replace:
      "                        self.user_iterator_knowing(receiver.clone(), held, (back, len), span)?\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_dropped_from_back.rs$"],
  },
  {
    name: "enumerate-back-source-unclosed",
    breaks:
      "what an `enumerate()` stepped from its back steps, a compiled `fuse()`, isn't closed to drop it (#225)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    None => self.closing(receiver.clone(), held, span)?,\n",
    replace: "                    None => receiver.clone(),\n",
    tests: ["test/corpus.test.ts", "-t", "^owned_items_through_fuse.rs$"],
  },
  {
    name: "back-kept-closes-borrowed",
    breaks:
      "an adapter stepped from its back of a `&mut` of the crate's iterator drops what it borrows (#225)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        let held = arg_tys.first().copied().unwrap_or(iter_ty);\n",
    replace: "        let held = iter_ty;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^own_iterator_dropped_through_adapters.rs$",
    ],
  },
  {
    name: "filter-back-own-unclosed",
    breaks:
      "a `filter` stepped from its back of the crate's iterator with a destructor, or a `fuse()` of it, isn't closed to drop it (#225)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                // iterator or a compiled `fuse()` of it (#225).\n                let it = self.closing(it, held, span)?;\n",
    replace: "                // iterator or a compiled `fuse()` of it (#225).\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^own_iterator_dropped_through_adapters.rs$",
    ],
  },
  {
    name: "mir-rev-own-next-back-unused",
    breaks:
      "`rev()` of an iterator of the crate's with its own `next_back` is refused",
    file: "src/lower/mir/iter/backs.rs",
    find: "        let Some(back) = self.own_next_back(iter_ty)? else {\n",
    replace:
      "        let Some(back) = self.own_next_back(iter_ty)?.filter(|_| false) else {\n",
    tests: ["test/corpus.test.ts", "-t", "iterator_back_ends"],
  },
  {
    name: "rev-own-methods-peeled",
    breaks:
      "`rev()` of a `&mut` of the crate's iterator is given its own `rfold`, which std's `&mut I` doesn't reach (#217)",
    file: "src/lower/mir/iter/backs.rs",
    find: '        let reversed = self.user_iterator(receiver, receiver_ty, back, "next_back", span)?;\n',
    replace:
      '        let reversed = self.user_iterator(receiver, iter_ty, back, "next_back", span)?;\n',
    tests: ["test/corpus.test.ts", "-t", "^rev_own_back_methods.rs$"],
  },
  {
    name: "rev-of-own-unclosed",
    breaks:
      "`rev()` of the crate's iterator with a destructor isn't closed to drop it, so its temporary is never dropped (#223)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        self.closing(reversed, receiver_ty, span)\n    }\n",
    replace: "        Ok(reversed)\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_dropped_from_back.rs$"],
  },
  {
    name: "rev-of-own-closed-through-mut-ref",
    breaks:
      "`rev()` of a `&mut` of the crate's iterator drops what it borrows (#223)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        self.closing(reversed, receiver_ty, span)\n    }\n",
    replace: "        self.closing(reversed, iter_ty, span)\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_dropped_from_back.rs$"],
  },
  {
    name: "rev-of-items-not-reversed-inward",
    breaks:
      "`$rev($iter(v))` isn't the array's `$iter(v.toReversed())`, the one rule of it since #228",
    file: "src/lower/mir/iter/backs.rs",
    find: "    if crate::lower::iterators::is_stepped_items(e) {\n        let items = crate::lower::iterators::iterated(e.clone());\n",
    replace:
      "    if false && crate::lower::iterators::is_stepped_items(e) {\n        let items = crate::lower::iterators::iterated(e.clone());\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(fused_iterator|rev_of_closed_items|rev_drops).rs$",
    ],
    snapshots: true,
  },
  {
    name: "back-filter-keeps-dropped",
    breaks:
      "a filter stepped from its back, of items with a destructor, drops none it doesn't keep",
    file: "src/lower/mir/iter/backs.rs",
    find: "                let mut given = vec![it, p];\n                if let Some(item) = self.iterator_item(output) {\n",
    replace:
      "                let mut given = vec![it, p];\n                if let Some(item) = self.iterator_item(output).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_of_closed_items.rs$"],
  },
  {
    name: "advance-back-by-unlowered",
    breaks:
      "`advance_back_by` in std's bodies is std's `nth_back`, so `Chain::nth_back` gives the item it should pass (ADR 0920)",
    file: "src/lower/mir/iter/backs.rs",
    find: '            IterMethod::AdvanceBackBy => "$advanceBackBy",\n',
    replace: '            IterMethod::AdvanceBackBy => "$nthBack",\n',
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "rfold-acc-unwind-unseen",
    breaks:
      "an `rfold` of an accumulator with a destructor, a stage panicking part-way, leaves the accumulator undropped (#423)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                Some(&acc) => self.stranded_drop(iter_ty, acc, span)?,\n",
    replace: "                Some(_) => None,\n",
    tests: [
      "test/corpus.test.ts",
      "test/iterator-forms.test.ts",
      "-t",
      "^generic_folds_unwind.rs$|^panic_in_own_rfold_drops with",
    ],
  },
  {
    name: "try-rfold-acc-unseen",
    breaks:
      "a `try_rfold`, std's `Rev::try_fold`'s, of an accumulator with a destructor, a stage panicking part-way, leaves it undropped (#423)",
    file: "src/lower/mir/iter/backs.rs",
    find: "(false, output), arg_tys, span)?;",
    replace: "(false, output), &[], span)?;",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std",
    ],
  },
  {
    name: "enumerate-of-own-count-mapped",
    breaks: "an `enumerate()` of the crate's iterator with its own `count` is a map of it, whose `count` steps it (#505)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    || self.recognition().reaches_its_own(output, OwnStep::Count, Reach::Sure)\n                    || self.counts_by_compiled_count(iter_ty) =>",
    replace: "                    || self.counts_by_compiled_count(iter_ty) =>",
    tests: ["test/corpus.test.ts", "-t", "^own_count_through_adapters.rs$"],
  },
  {
    name: "enumerate-compiled-count-unseen",
    breaks:
      "an `enumerate()` of a held, peeked `peekable()` is a map of it, whose `count` gives what it peeked at first, where std's `Enumerate::count` is `Peekable::count`, which drops it after its source's `count` ends or unwinds (#655)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        self.compiled_step(iter_ty, std_item(self.tcx, StdItem::Iterator), \"count\")\n",
    replace: "        false\n",
    tests: ["test/corpus.test.ts", "-t", "^(enumerate_count_is_its_sources|enumerate_of_a_reversible_source).rs$"],
  },
  {
    name: "compiled-struct-has-its-back",
    breaks:
      "std's compiled `Map`, given back by a function behind an `impl DoubleEndedIterator`, is taken to have a back as its type says, so its `enumerate().rev()` steps a back it hasn't: `TypeError` where Rust gives each item from the back (#1142)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            && !self.compiled_struct(iter_ty.peel_refs(), value)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(impl_trait_hides_std_adapters|returned_impl_ends).rs$"],
  },
  {
    name: "rfold-receiver-unclosed",
    breaks: "what `rfold` takes isn't closed: a panic in its closure leaves a `Vec`'s rest and the crate's double-ended iterator undropped (#650)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            let it = match self.consumer_closes(Some(method), iter_ty, lent, dropping) {\n",
    replace: "            let it = match false && self.consumer_closes(Some(method), iter_ty, lent, dropping) {\n",
    tests: ["test/corpus.test.ts", "-t", "^rfold_and_rev_close_what_they_step.rs$"],
  },
  {
    name: "generic-rev-unclosed",
    breaks: "a generic `rev()` of a type parameter isn't closed: the crate's iterator it reverses is never dropped (#650)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                let it = self.closing(it, iter_ty, span)?;\n                let it = match in_place {\n",
    replace: "                let it = match in_place {\n",
    tests: ["test/corpus.test.ts", "-t", "^rfold_and_rev_close_what_they_step.rs$"],
  },
  {
    name: "copy-of-skip-backless",
    breaks: "a `copied()` of a slice's `skip(n)` or `take(n)`, stepped from its back, is JS's own `drop` or `take`, which has no back: `it.back is not a function` (#646)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            Some(IterMethod::Copied | IterMethod::Cloned)\n                if self.is_its_source(output)\n",
    replace: "            Some(IterMethod::Copied | IterMethod::Cloned)\n                if false && self.is_its_source(output)\n",
    tests: ["test/corpus.test.ts", "-t", "^(back_folds_of_sources_made_knowing_their_back|copies_of_forward_forms_stepped_from_their_back).rs$"],
  },
  {
    name: "rev-of-rev-of-own-steps-dropped",
    breaks: "`rev().rev()` of a type parameter's iterator is the iterator itself, so its `last()` asks the crate's own `last`, which std's `Rev` never passes on (#621)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        && (crate::lower::iterators::is_stepped_items(it) || crate::lower::iterators::is_range_stepped(it))\n",
    replace: "        && (true || crate::lower::iterators::is_range_stepped(it))\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_of_rev_folds_its_source.rs$"],
  },
  {
    name: "enumerate-of-take-from-back-unknowing",
    breaks: "an enumeration the crate steps from its back of a source whose JS knows no back is given it as it's made: of a `take()` of items with a destructor, `.take(n)`, `enumerated.iter.len is not a function` (#651); of a map, `ds().into_iter().map(f).enumerate().rfold(..)`, and of a slice's `cloned()`'s and the crate's iterator's `skip()`'s and `rev()`'s, `enumerated.iter.back is not a function` (#778)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    None if self.stepped_from_its_back(output) && !self.has_a_back(iter_ty, receiver) => {\n",
    replace: "                    None if false && self.stepped_from_its_back(output) && !self.has_a_back(iter_ty, receiver) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(enumerate_of_take_from_back|back_folds_of_sources_made_knowing_their_back|enumerate_of_droppers_from_back).rs$"],
  },
  {
    name: "nth-back-of-a-vec-unsliced",
    breaks: "a `Vec`'s own `nth_back(k)` drops what it passes from its back, 7, 6, 5, where std drops it as one slice, 5, 6, 7 (#234)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            IterMethod::NthBack | IterMethod::AdvanceBackBy => self.slicing(it, iter_ty, true, span)?,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(nth_passes_a_slice_or_each|vec_skipped_from_back_drops).rs$"],
  },
  {
    name: "rev-of-rev-of-closed-items-collapsed",
    breaks:
      "`rev().rev()` of a `Vec`'s items with a destructor is the `Vec`'s own `IntoIter`, whose `last` takes its back, where std's `Rev` folds: a drop that panics drops the item before the last and leaks the last (#722)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        && (crate::lower::iterators::is_stepped_items(it) || crate::lower::iterators::is_range_stepped(it))\n",
    replace: "        && (crate::lower::iterators::is_stepped_items(it)\n            || crate::lower::iterators::is_range_stepped(it)\n            || crate::lower::iterators::closed_parts(it).is_some())\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_of_rev_last_folds.rs$"],
  },
  {
    name: "rev-reversed-inward-unread",
    breaks:
      "a range's map reversed inward, `$iterator(range, $rangeNextBack).map(f)`, is kept for a `rev()` made knowing its back, whose form `reversal` can't read: `(0..5).map(f).rev().rfold(mk(), g)` is refused (#718)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                && !(knowing && self.reversal_of(&reversed, iter_ty).is_none()) =>\n",
    replace: "                && true =>\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_made_knowing_its_back.rs$"],
  },
  {
    name: "by-ref-knowing-unlent",
    breaks:
      "`by_ref()` of what knows its back, a type parameter's, is the value itself, not lent, so `Total::sum` of it reaches the crate's own `fold`, which drops what the caller still owns (#749)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                self.lent_iterator(back)\n",
    replace: "                back\n",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^by_ref_lends.rs$|^by_ref_lends with"],
  },
  {
    name: "rfold-of-mut-unlent",
    breaks:
      "`rfold` of a `&mut` to the crate's iterator, `by_ref().rfold(..)`, is its own `rfold`, which takes it, so it's dropped there and again by its owner (#747)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                false if lent => self.lent_iterator(it),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^by_ref_lends.rs$|^by_ref_lends with"],
  },
  {
    name: "map-of-mut-closed",
    breaks:
      "a `map()` of a `by_ref()` of a type parameter's iterator that knows its back is closed to drop it, so a collect of a `take(2)` of it drops what its owner still holds, and its owner drops it again (#749)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                // through a `&mut`, `by_ref()`'s, its owner's (#749).\n                let it = self.closing(it, held, span)?;\n",
    replace: "                // through a `&mut`, `by_ref()`'s, its owner's (#749).\n                let it = self.closing(it, iter_ty, span)?;\n",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^by_ref_lends.rs$|^by_ref_lends with"],
  },
  {
    name: "rev-made-unknowing",
    breaks:
      "a `rev()` the crate steps from its back isn't made knowing it where it's made: a slice's map's `rev().nth_back(1)` lent to `nth_back` is `$iter(v.toReversed()).map(f)`, which has no back, and throws (#702)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        let reversed = self.reversed_knowing(reversed, output, span)?;\n",
    replace: "        let reversed = reversed;\n",
    tests: ["test/corpus.test.ts", "-t", "^(own_iterator_flattened_from_its_back|std_rev|rev_range_counted_beside_its_rfold).rs$"],
    snapshots: true,
  },
  {
    name: "rev-of-mut-closed",
    breaks:
      "a `rev()` of a `&mut`, `by_ref().rev()` of a `Vec`'s `IntoIter`, is closed to drop what it steps, so a `take(1)` of it drops what its owner still holds (#755)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            None if receiver_ty.is_mutable_ptr() => {\n",
    replace: "            None if false => {\n",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^by_ref_lends.rs$|^by_ref_lends with"],
  },
  {
    name: "rev-of-mut-unsliced",
    breaks:
      "a `rev()` of a `by_ref()` of a `Vec`'s `IntoIter` isn't given its slice's steps, so its `nth` drops what it passes from the back one by one, not as one slice (#755)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                let lent = self.slicing(lent, iter_ty, true, span)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^by_ref_lends.rs$|^by_ref_lends with"],
  },
  {
    name: "copy-of-compiled-struct-backless",
    breaks: "a `copied()` that's its source, of std's compiled `fuse()` or `peekable()`, stepped from its back, is the struct given `next` alone: `nums.iter().fuse().copied().rfold(..)` throws `it.back is not a function` (#778)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                        || crate::lower::recognition::is_compiled_std_iterator(self.tcx, iter_ty)) =>\n",
    replace: "                        || false && crate::lower::recognition::is_compiled_std_iterator(self.tcx, iter_ty)) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^back_folds_of_sources_made_knowing_their_back.rs$"],
  },
  {
    name: "counted-back-stepped-made-unknowing",
    breaks:
      "a `skip(n)` or a `take(n)` the crate steps from its back isn't made knowing it where it's made, so one a variable holds, or a chain's side, has no form left to step from its back: refused (#718)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            Some(IterMethod::Skip | IterMethod::Take) if self.made_stepped_from_its_back() && !held.is_ref() => {\n",
    replace: "            Some(IterMethod::Skip | IterMethod::Take) if self.made_stepped_from_its_back() && false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(skip_take_of_droppers_from_back|held_beneath_adapters_from_its_back).rs$"],
  },
  {
    name: "back-stepped-temporary-unmarked",
    breaks: "a `take(n)` made knowing its back where a temporary holds it isn't marked so, and `next_back` of its variable reads its type, which doesn't, and is refused (#718)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        Ok(crate::lower::iterators::made_knowing_its_back(knowing).in_place())\n",
    replace: "        Ok(knowing.in_place())\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_and_take_stepped_from_their_back_in_place.rs$"],
  },
  {
    name: "back-stepped-temporary-held-unread",
    breaks: "a `step_by(n)` its owner holds, `$held(it)`, which a temporary a back step takes holds, isn't made knowing its back where it's made, and its `nth_back` is refused (#718)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        if let Some(held) = crate::lower::iterators::held_of(&value) {\n",
    replace: "        if let Some(held) = None::<&Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "^step_by_stepped_from_its_back.rs$"],
  },
  {
    name: "rev-of-given-param-unknowing",
    breaks: "`rev()` of a type parameter a struct holds, given as it is, stepped from its back, is made by its dictionary's `next_back` alone, which knows no back, and its `nth_back` is refused (#718)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            None if (self.reversed_made_knowing(output) || self.closes_to_drop(receiver_ty))\n",
    replace: "            None if false && self.closes_to_drop(receiver_ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_of_a_type_parameter_held_as_it_is.rs$"],
  },
  {
    name: "rev-source-remade-unmarked",
    breaks:
      "what a `rev()` made in place reverses, made again to step from its back, loses its maker's mark, so where it's asked as an `ExactSizeIterator` it's taken as std's struct, `$knowing` of its compiled back (#730)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                let it = match in_place {\n                    true => it.in_place(),\n                    false => it,\n                };\n                return Ok(crate::lower::iterators::reversed_by(it));\n",
    replace: "                let _ = in_place;\n                return Ok(crate::lower::iterators::reversed_by(it));\n",
    tests: ["test/corpus.test.ts", "-t", "^lengths_and_backs_given_on.rs$"],
    snapshots: true,
  },
  {
    name: "rev-made-in-place-unmarked",
    breaks:
      "a `rev()` made in place isn't marked so before it's made knowing its back, so the map it reverses, `v.iter().map(f).rev().nth_back(1)`'s, is taken as std's struct, `$knowing` of its compiled back (#730)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        let reversed = match in_place {\n            true => reversed.in_place(),\n            false => reversed,\n        };\n",
    replace: "        let _ = in_place;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_flattened_from_its_back.rs$"],
    snapshots: true,
  },
  {
    name: "back-stepped-temporary-remade-unmarked",
    breaks:
      "a `skip(n)` or a `take(n)` made in place that a temporary holds for a back step, made knowing its back there, loses its maker's mark, so its back step calls std's compiled `Skip`'s `next_back`, of a struct it isn't (#730)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        Ok(crate::lower::iterators::made_knowing_its_back(knowing).in_place())\n",
    replace: "        Ok(crate::lower::iterators::made_knowing_its_back(knowing))\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_and_take_stepped_from_their_back_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "counted-made-knowing-unmarked",
    breaks: "a `skip(n)` or a `take(n)` made knowing its back by its maker isn't marked so, and a variable holding it, given to a `rev()` whose `len()` is asked, is refused (#814)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                crate::lower::iterators::made_knowing_its_back(Expr::call_helper(\n                    if skip { \"$skipping\" } else { \"$taking\" },\n                    given,\n                ))\n",
    replace: "                Expr::call_helper(if skip { \"$skipping\" } else { \"$taking\" }, given)\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
  },
  {
    name: "rev-of-counted-knowing-unmarked",
    breaks: "a `rev()` of a `skip(n)` or a `take(n)` that knows its length isn't marked so, and a variable holding it, held to drop its items, is asked its `len()` past its maker and refused (#814)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            Some(source) if crate::lower::recognition::counted_source(self.tcx, iter_ty).is_some() && knows(source) => {\n",
    replace: "            Some(source) if false && knows(source) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_adapter_given_back_from_its_back.rs$"],
  },
  {
    name: "rev-of-given-counted-unmarked",
    breaks: "a `rev()` of a `skip(n)` or a `take(n)` a function gives behind an `impl`, std's struct given its compiled `len`, isn't marked knowing its back, and its `len()` is refused (#814)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                || crate::lower::iterators::knows_its_given_length(source)\n",
    replace: "                || false\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_adapter_given_back_from_its_back.rs$"],
  },

  {
    name: "back-stepped-temporary-made-unknowing",
    breaks: "a temporary a back step takes, lent or whole, written to a variable is made as it is, `$iter(v).take(2)`, not knowing its back, and its `nth_back` is refused (#718, #811)",
    file: "src/lower/mir/iter/backs.rs",
    find: "            Some(crate::lower::analysis::SteppedLocal::Lent) => self.made_knowing_where_held(value, ty, span),\n",
    replace: "            Some(crate::lower::analysis::SteppedLocal::Lent) if false => self.made_knowing_where_held(value, ty, span),\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_and_take_stepped_from_their_back_in_place.rs$"],
  },
  {
    name: "tuple-field-iterator-taken-for-its-array",
    breaks: "an array's iterator a tuple struct's field holds, `held[0]`, is taken for its array, `held[0].toReversed()`, which throws (#1013)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                    js::ExprKind::Var(_) | js::ExprKind::Member(..) | js::ExprKind::Index(..)\n",
    replace: "                    js::ExprKind::Var(_) | js::ExprKind::Member(..)\n",
    tests: ["test/corpus.test.ts", "-t", "^iterators_held_in_a_generic_structs_field.rs$"],
  },
  {
    name: "map-made-knowing-its-length-unmarked",
    breaks: "a map made knowing its length isn't marked so, so what's asked of a variable that holds it, `len()`, reads its type alone and is refused (#851)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                match len {\n                    true => crate::lower::iterators::made_knowing_its_length(mapped),\n",
    replace: "                match false {\n                    true => crate::lower::iterators::made_knowing_its_length(mapped),\n",
    tests: ["test/corpus.test.ts", "-t", "^(owned_maps_asked_their_length|zip_by_index_of_holding_maps).rs$"],
  },
  {
    name: "type-parameter-source-unstepped",
    breaks:
      "a map of an enumeration of a type parameter's iterator generic code makes, whose bounds give it no back, is refused or given as it is, not a JS iterator: `enumerated.iter.next is not a function`",
    file: "src/lower/mir/iter/backs.rs",
    find: "        let parameter = matches!(iter_ty.peel_refs().kind(), ty::Param(_));\n",
    replace: "        let parameter = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_of_a_made_type_parameter_collected.rs$"],
  },
  {
    name: "reversed-range-counted",
    breaks: "a `rev()` of a B-tree's `range(..)` is `$iter` of its items reversed with no mark, whose hint counts them, so a crate that reads a capacity grows a collect of it otherwise (#1215)",
    file: "src/lower/mir/iter/backs.rs",
    find: "        Ok(self.stepped_items_of(items, iter_ty))\n",
    replace: "        Ok(self.stepped_items(items))\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_what_std_hints_less.rs$"],
  },
  {
    name: "hinted-skip-source-unclosed",
    breaks: "a `skip(n)` generic code asks the hint of is made `$skipping` of its source unclosed, so where a drop of what's left panics the rest aren't dropped, where `IntoIter`'s drop goes on (#1244)",
    file: "src/lower/mir/iter/backs.rs",
    find: "                let it = self.closing(receiver.clone(), held, span)?;\n                let n = values.next().expect(\"how many\");\n                let mut given = vec![it, n];\n                if let Some(item) = self.iterator_item(output) {\n                    given.extend(self.std_drop_function(item, span)?);\n                }\n                let skip = method == Some(IterMethod::Skip);\n                Expr::call_helper(",
    replace: "                let n = values.next().expect(\"how many\");\n                let mut given = vec![receiver.clone(), n];\n                if let Some(item) = self.iterator_item(output) {\n                    given.extend(self.std_drop_function(item, span)?);\n                }\n                let skip = method == Some(IterMethod::Skip);\n                Expr::call_helper(",
    tests: ["test/corpus.test.ts", "-t", "^skip_hinted_closes_its_source.rs$"],
  },
];
