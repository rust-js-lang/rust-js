// Mutations of src/lower/iterators/stepping.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unbounded-param-given-crate-iterator-wrapped",
    breaks:
      "the crate's iterator given for a `T: Debug` is made a JS iterator, which its dictionary reads as the struct: `Gen(undefined)` (#991)",
    file: "src/lower/iterators/stepping.rs",
    find: "        if !self.is_user_iterator(ty)\n            && self.range_kind(ty.peel_refs()).is_none()\n",
    replace: "        if self.is_user_iterator(ty) {\n            return true;\n        }\n        if self.range_kind(ty.peel_refs()).is_none()\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_given_where_no_iterator_goes.rs$"],
  },
  {
    name: "result-items-unstepped",
    breaks:
      "a `Result` given to `chain` or `extend` is spread as the object it is: `TypeError: ... is not iterable` where Rust steps its `Ok` value (ADR 0923)",
    file: "src/lower/iterators/stepping.rs",
    find: "        if self.recognition().is_std_type(ty, StdItem::Result) {\n",
    replace:
      "        if false && self.recognition().is_std_type(ty, StdItem::Result) {\n",
    tests: ["test/corpus.test.ts", "-t", "^result_given_as_items.rs$"],
  },
  {
    name: "range-any-param-items",
    breaks:
      "a range given where any `T: Clone` goes is its items, not the range",
    file: "src/lower/iterators/stepping.rs",
    find: "            && self.range_kind(ty.peel_refs()).is_none()\n            && !self.iterator_given_as_it_is(ty.peel_refs())\n        {\n            return false;\n        }",
    replace:
      "            && self.range_kind(ty.peel_refs()).is_none()\n            && !self.iterator_given_as_it_is(ty.peel_refs())\n        {\n            return false;\n        }\n        if self.range_kind(ty.peel_refs()).is_some() {\n            return true;\n        }",
    tests: ["test/corpus.test.ts", "-t", "range_values"],
  },
  {
    name: "collect-fallible-any-target",
    breaks:
      "`collect()` into a `Result` of a map, a set or text is `Ok` of an array, not of it",
    file: "src/lower/iterators/stepping.rs",
    find: "            && (self.is_vec_like(collection) || collection.boxed_ty().is_some_and(|t| t.is_slice()))\n",
    replace: "            && true\n",
    tests: ["test/corpus.test.ts", "-t", "collect_fallible_into"],
  },
  {
    name: "stepped-clone-from-start",
    breaks:
      "a clone of an iterator stepped through starts again from its first item",
    file: "src/lower/iterators/stepping.rs",
    find: 'vec![Expr::member(it.clone(), "at"), Expr::member(it, "end")],',
    replace: "vec![],",
    tests: ["test/corpus.test.ts", "-t", "iterator_clones"],
  },
  {
    name: "clone-past-end",
    breaks: "a clone of a stepped iterator has what `next_back()` took",
    file: "src/lower/iterators/stepping.rs",
    find: '            vec![Expr::member(it.clone(), "at"), Expr::member(it, "end")],',
    replace: '            vec![Expr::member(it, "at")],',
    tests: ["test/corpus.test.ts", "-t", "iter_next_back"],
  },
  {
    name: "user-iterator-arg-unwrapped",
    breaks:
      "an iterator of the crate's given where a generic iterator goes is its struct, which JS can't step",
    file: "src/lower/iterators/stepping.rs",
    find: "            } else if matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(callee, input, given) {\n",
    replace:
      "            } else if false && matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(callee, input, given) {\n",
    tests: ["test/corpus.test.ts", "-t", "generic_iterator_rest_lazy"],
  },
  {
    name: "iterated-slot-unwrapped",
    breaks:
      "a crate's iterator given as a JS iterator is dropped as the JS iterator, not what it's of",
    file: "src/lower/iterators/stepping.rs",
    find: "            && self.given_as_iterator(callee, ty::Ty::new_param(self.tcx, param.index, param.name), given)\n",
    replace: "            && true\n",
    tests: ["test/corpus.test.ts", "-t", "^(std_projection_drops|trait_param_drops|adapter_drops).rs$"],
  },
  {
    name: "double-ended-given-forgetting-its-back",
    breaks:
      "a range given for a `T: DoubleEndedIterator` is a JS iterator with no back",
    file: "src/lower/iterators/stepping.rs",
    find: "            } else if self.double_ended_param(callee, input) {\n",
    replace: "            } else if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "mir-as-slice-whole",
    breaks: "`it.as_slice()` of a stepped `$iter` is all its items",
    file: "src/lower/iterators/stepping.rs",
    find: 'vec![Expr::member(it.clone(), "at"), Expr::member(it, "end")],',
    replace: 'vec![Expr::int(0), Expr::member(it, "end")],',
    tests: ["test/corpus.test.ts", "-t", "iterator_back_ends"],
  },
  {
    name: "exact-size-ungiven",
    breaks:
      "a range given where generic code asks its length is the range, which has no `len()`, and JS throws",
    file: "src/lower/iterators/stepping.rs",
    find: "            if self.exact_size_param(callee, input) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^exact_size_len.rs$"],
  },
  {
    name: "shared-ref-iterator-unconverted",
    breaks:
      "a range given by a `&`, `remaining(&(0..5))`, isn't a JS iterator that knows its length",
    file: "src/lower/iterators/stepping.rs",
    find: "                ty::Ref(_, inner, _) => (inner, given.peel_refs()),\n",
    replace:
      "                ty::Ref(_, inner, ty::Mutability::Mut) => (inner, given.peel_refs()),\n",
    tests: ["test/corpus.test.ts", "-t", "^exact_size_len.rs$"],
  },
  {
    name: "lent-lent-again",
    breaks:
      "an iterator already lent, `by_ref()`'s, is lent again, `$lent($lent(it))`",
    file: "src/lower/iterators/stepping.rs",
    find: "        if is_lent(&it) {\n            return it;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^by_ref_chains.rs$"],
    snapshots: true,
  },
  {
    name: "lent-made-iterator",
    breaks:
      "a lent iterator, already a JS one, is made one again, `Iterator.from($lent(it))`",
    file: "src/lower/iterators/stepping.rs",
    find: "        if is_lent(&value) || is_js_iterator(&value) {\n            return value;\n        }\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(by_ref_chains|take_destructors).rs$",
    ],
    snapshots: true,
  },
  {
    name: "cloned-param-matches-every-adapter",
    breaks:
      "a type parameter generic code clones stands for every `map` and `filter`, each made cloned apart, which refuses what it can't clone",
    file: "src/lower/iterators/stepping.rs",
    find: "            adapter(cloned_ty)\n                && crate::lower::copies::instance_within_bounds(",
    replace: "            true && crate::lower::copies::instance_within_bounds(",
    tests: ["test/corpus.test.ts", "-t", "^generic_clones_of_unseen_callers.rs$"],
  },
  {
    name: "cloned-closures-not-generic",
    breaks:
      "a `map` generic code makes isn't seen as the one its caller clones, whose closure is the caller's instance",
    file: "src/lower/iterators/stepping.rs",
    find: "                    ty::Closure(id, _) => tcx.type_of(id).instantiate_identity().skip_normalization(),\n",
    replace: "                    ty::Closure(..) => t,\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_clones_of_unseen_callers.rs$"],
  },
  {
    name: "into-iter-option-unstepped",
    breaks:
      "an `Option` given to `extend` or `chain` is taken as an iterable, not its none or one",
    file: "src/lower/iterators/stepping.rs",
    find: "        if let Some(item) = self.option_of(ty) {\n            return Ok(self.option_items(value, item, out));\n",
    replace:
      "        if let Some(item) = self.option_of(ty).filter(|_| false) {\n            return Ok(self.option_items(value, item, out));\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_arguments.rs$"],
  },
  {
    name: "into-iter-range-unstepped",
    breaks: "a range given to `chain` or `zip` is its object, not its numbers",
    file: "src/lower/iterators/stepping.rs",
    find: "        if self.range_kind(ty).is_some() || self.is_user_iterator(ty) || self.iterator_given_as_it_is(ty) {\n",
    replace:
      "        if self.is_user_iterator(ty) || self.iterator_given_as_it_is(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_arguments.rs$"],
  },
  {
    name: "trait-self-made-js-iterator",
    breaks:
      "a trait's `Self`, given to its method through a dictionary, is made a JS iterator",
    file: "src/lower/iterators/stepping.rs",
    find: "(given_as_it_is(self.tcx, callee, *param) || given_by_its_trait(self.tcx, callee, param.index))",
    replace: "given_as_it_is(self.tcx, callee, *param)",
    tests: ["test/corpus.test.ts", "-t", "^(blanket_iterator_trait|fused_iterators|std_fuse).rs$"],
  },
  {
    name: "btree-given-unordered",
    breaks:
      "a `BTreeSet` given to `extend` or `chain` is stepped in its JS set's order, where a JS caller's `Set` may not be in its keys' (ADR 0404, amended)",
    file: "src/lower/iterators/stepping.rs",
    find: "        if self.is_sorted(ty) {\n            return self.in_order_of(value, ty, span);\n        }\n",
    replace: "",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a B-tree its JS caller gives to extend or chain",
    ],
  },
  {
    name: "js-iterator-rewrapped",
    breaks: "a JS iterator isn't made one again",
    file: "src/lower/iterators/stepping.rs",
    find: "        if is_lent(&value) || is_js_iterator(&value) {",
    replace: "        if is_lent(&value) {",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
    snapshots: true,
  },
  {
    name: "into-iter-arg-raw",
    breaks: "what an IntoIterator bound takes is its items as JS steps them",
    file: "src/lower/iterators/stepping.rs",
    find: "            } else if matches!(input.kind(), ty::Param(_)) && self.given_iterable(callee, input) {",
    replace:
      "            } else if false && self.given_iterable(callee, input) {",
    tests: ["test/corpus.test.ts", "-t", "^iterables_given_to_generics.rs$"],
  },
  {
    name: "mut-collection-items-raw",
    breaks: "a &mut collection's items are handles",
    file: "src/lower/iterators/stepping.rs",
    find: "            && let Some(items) = self.ref_into_iter(value.clone(), ty, span, out)?\n",
    replace: "            && let Some(items) = None::<Expr>\n",
    tests: ["test/corpus.test.ts", "-t", "^iterables_given_to_generics.rs$"],
  },
  {
    name: "mut-range-extend-refused",
    breaks: "a &mut range is stepped where it is",
    file: "src/lower/iterators/stepping.rs",
    find: "            return self.as_js_iterator(value, ty, span);\n        }\n        // A `&mut` collection's",
    replace:
      '            return Err(self.unsupported(span, "a `&mut` range"));\n        }\n        // A `&mut` collection\'s',
    tests: ["test/corpus.test.ts", "-t", "^iterables_given_to_generics.rs$"],
  },
  {
    name: "into-iter-arg-of-dictionary",
    breaks: "a parameter given an IntoIterator dictionary isn't made its items",
    file: "src/lower/iterators/stepping.rs",
    find: "(self.declared_bounds(input, env.param_env).into_iter()).any(|id| id == into_iter) && !given_one",
    replace:
      "(self.declared_bounds(input, env.param_env).into_iter()).any(|id| id == into_iter)",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_into_iter_values|into_iter_of_a_bound|fused_iterators|drop_functions|into_iter_held_as_it_is|self_given_on_as_into_iter).rs$",
    ],
  },
  {
    name: "into-iter-items-unhandled",
    breaks:
      "std's `IntoIterator` of a `&mut Vec` gives its items, not a handle on each, and what's written through one is lost",
    file: "src/lower/iterators/stepping.rs",
    find: "            ty::Mutability::Mut => self.item_handles_of(known, vec![value.clone()], pointee, span, out)?,\n",
    replace: "            ty::Mutability::Mut => None,\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(into_iter_of_ref_param|iterables_given_to_generics).rs$",
    ],
  },
  {
    name: "own-methods-mut-ref-folds",
    breaks:
      "a `&mut` of the crate's iterator is given its own `fold` and `rfold`, which std's `&mut I` doesn't reach (#216, #217)",
    file: "src/lower/iterators/stepping.rs",
    find: "        let folds = !ty.is_ref();\n",
    replace: "        let folds = true;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(adapters_own_nth_fold|rev_own_back_methods).rs$",
    ],
  },
  {
    name: "own-methods-back-forward",
    breaks:
      "`rev()` of the crate's iterator is given its forward `nth`, so `rev().nth(1)` counts from the front (#217)",
    file: "src/lower/iterators/stepping.rs",
    find: '                fold: self.own_fold(ty, ("rfold", folds), span)?,\n                nth: self.own_method(ty, ("nth_back", true), span)?,\n',
    replace:
      '                fold: self.own_fold(ty, ("fold", folds), span)?,\n                nth: self.own_method(ty, ("nth", false), span)?,\n',
    tests: ["test/corpus.test.ts", "-t", "^rev_own_back_methods.rs$"],
  },
  {
    name: "fallible-rest-first-dropped",
    breaks:
      "a `collect` into a `Result` of a `Vec` std does otherwise than in place, a panic stopping it, drops what it collected before what's left (#423)",
    file: "src/lower/iterators/stepping.rs",
    find: "            (Some(_), Some(value)) => self.rest_dropped_first(collection, iter_ty, value, span)?,",
    replace: "            (Some(_), Some(_)) => None,",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "own-fold-unwind-as-impl-drop",
    breaks:
      "the crate's own `fold` of an `impl<T> Iterator for Wrap<T>` is given the runtime's `unwind`, its accumulator's drop, as its `T`'s drop, which comes first: `drop undefined` as a stage panics, and none of its `T`s as std's struct folds it (#423)",
    file: "src/lower/iterators/stepping.rs",
    find: "            (true, Some(found)) => self.fold_entry(found, span).map(Some),\n",
    replace: "            (true, Some(_)) => Ok(Some(self.fn_ref(written))),\n",
    tests: [
      "test/corpus.test.ts",
      "test/iterator-forms.test.ts",
      "-t",
      "^own_folds_unwind.rs$|^own_folds_unwind with",
    ],
  },
  {
    name: "own-fold-impl-dictionaries-ungiven",
    breaks:
      "an iterator's own `fold` or `rfold` that takes its impl's dictionaries, either's `Either<L, R>`'s, is called by the runtime given none of them, and throws on `undefined.next_back` (#475)",
    file: "src/lower/iterators/stepping.rs",
    find: "        match (before || given, fold) {\n",
    replace: "        match (before, fold) {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_methods_given_dictionaries.rs$"],
  },
  {
    name: "own-nth-impl-dictionaries-ungiven",
    breaks:
      "an iterator's own `nth` or `nth_back` that takes its impl's dictionaries is called by the runtime given none of them (#475)",
    file: "src/lower/iterators/stepping.rs",
    find: "        if !self.takes_more_than_drops(written) {\n            return Ok(Some(self.fn_ref(written)));\n",
    replace: "        if true {\n            return Ok(Some(self.fn_ref(written)));\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_methods_given_dictionaries.rs$"],
  },
  {
    name: "own-method-of-its-own-ungiven",
    breaks:
      "an iterator's own `fold` asking a fact of its own `B`, which the runtime's fold gives no type of, is given one of `()` (#475)",
    file: "src/lower/iterators/stepping.rs",
    find: "        if self.takes_of_its_own(written) {\n",
    replace: "        if false {\n",
    tests: ["test/diagnostics.test.ts", "-t", "own fold asking of its own type parameter"],
  },
  {
    name: "own-count-unlisted",
    breaks: "the crate's iterator made a JS one has no `count` of its own, so a `skip`'s and generic code's `count` never ask it (#505)",
    file: "src/lower/iterators/stepping.rs",
    find: "                    true => self.own_method(ty, (\"count\", false), span)?,\n",
    replace: "                    true => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^own_count_through_adapters.rs$"],
  },
  {
    name: "generic-code-given-no-slicing",
    breaks: "a `Vec`'s `IntoIter` given generic code, or std's compiled `StepBy`, isn't given its slice's steps, so its `nth` there drops each, stopping at a panic, and its `nth_back` drops each from the back (#668, #703)",
    file: "src/lower/iterators/stepping.rs",
    find: "                *value = self.slicing(held, given, back, span)?;\n",
    replace: "                *value = held;\n",
    tests: ["test/corpus.test.ts", "-t", "^(nth_passes_a_slice_or_each|nth_passed_drops_unwind).rs$"],
  },
  {
    name: "given-param-drop-unwrapped",
    breaks: "a type parameter's value generic code gives on as `$given` made it is dropped by the drop of the value, given the JS iterator, a `TypeError` (#766)",
    file: "src/lower/iterators/stepping.rs",
    find: "        (self.is_user_iterator(given) || matches!(given.kind(), ty::Param(_)))\n",
    replace: "        self.is_user_iterator(given)\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "step-by-made-unstepped",
    breaks: "`rev()` of a `step_by` made in place is refused",
    file: "src/lower/iterators/stepping.rs",
    find: "            crate::lower::std_types::range::every_nth(f).map(|step| ((**source_value).clone(), step))\n",
    replace: "            crate::lower::std_types::range::every_nth(f).filter(|_| false).map(|step| ((**source_value).clone(), step))\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_step_by.rs$"],
  },
  {
    name: "step-by-made-by-nth-backless",
    breaks:
      "a range's `step_by(n).rev()`, `$stepBy(range, n)`, isn't made knowing its back, and is refused (#231)",
    file: "src/lower/iterators/stepping.rs",
    find: "        (_, Some([source_value, step, ..])) => Some((source_value.clone(), step.clone())),\n",
    replace: "        (_, Some([source_value, step, ..])) if false => Some((source_value.clone(), step.clone())),\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_step_by.rs$"],
  },
  {
    name: "step-by-ends-unread",
    breaks: "a `step_by(n)` made knowing its back, `$stepByEnds(src, n)`, a `rev()`'s source, isn't read as one, and `len()` of its `rev()` is refused (#811)",
    file: "src/lower/iterators/stepping.rs",
    find: "    if let Some((source_value, step)) = crate::lower::iterators::stepped_by_ends_parts(value) {\n",
    replace: "    if let Some((source_value, step)) = None::<(&Expr, &Expr)> {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_and_counted_steps_from_their_back.rs$"],
  },
  {
    name: "step-by-ends-undropped",
    breaks: "a `step_by(n)` made knowing its back isn't given its items' drop, so what it skips from the back of the crate's iterator is never dropped (#823)",
    file: "src/lower/iterators/stepping.rs",
    find: "        Ok(Some(crate::lower::iterators::stepped_by_ends(source_value, step, drop)))\n",
    replace: "        Ok(Some(crate::lower::iterators::stepped_by_ends(source_value, step, None.or(drop.filter(|_| false)))))\n",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_of_own_iterator_folded_from_its_back.rs$"],
  },
  {
    name: "collected-some-unboxed",
    breaks: "a `collect()` into an `Option<()>`, or any `Option` whose `Some` could look like `None`, gives the collection itself as its `Some`, which reads as `None` (#984)",
    file: "src/lower/iterators/stepping.rs",
    find: "let boxed_some = matches!(self.option_of(target), Some(payload) if self.boxed_payload(payload));",
    replace: "let boxed_some = false;",
    tests: ["test/corpus.test.ts", "-t", "^collect_into_option_of_unit.rs$"],
  },
  {
    name: "collected-some-always-boxed",
    breaks: "a `collect()` into an `Option<Vec<_>>`, whose `Some` can't look like `None`, boxes it anyway: `$some` where the vector itself is its `Some` (#984)",
    file: "src/lower/iterators/stepping.rs",
    find: "let boxed_some = matches!(self.option_of(target), Some(payload) if self.boxed_payload(payload));",
    replace: "let boxed_some = self.option_of(target).is_some();",
    tests: ["test/corpus.test.ts", "-t", "^collect_fallible.rs$"],
    snapshots: true,
  },
  {
    name: "lowerless-iter-unmarked",
    breaks: "what std hints no lower bound of, a B-tree's `range(..)`, is stepped as `$iter(items)`, whose hint counts what's left, so a crate that reads a capacity grows it otherwise (#1215)",
    file: "src/lower/iterators/stepping.rs",
    find: "            args.push(Expr::bool(true));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_what_std_hints_less.rs$"],
  },
  {
    name: "cloned-apart-uninstanced",
    breaks:
      "generic code's adapter of its own type parameters is asked as any type they may be, not as each instance its callers give it: `last`'s `Chain<I, I>` stepped from its back is taken for `cycled`'s, which rustc names alike, and refused as cloned where `I` isn't `Clone` (#1119, #1243)",
    file: "src/lower/iterators/stepping.rs",
    find: "    adapter(ty) && cloned.as_any_instance(tcx, ty, owner, cloned_as)\n",
    replace: "    adapter(ty) && cloned_as(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_chains_cloned_by_their_instances.rs$"],
    snapshots: true,
  },
  {
    name: "iterated-slot-told-apart-by-a-region",
    breaks: "the crate's own iterator holding a closure's adapter, given to generic code as `$iterator(value, next)`, is given the value's own drop, `own[0].return()`, which reads the JS iterator as the struct: `Cannot read properties of undefined (reading 'return')` (#1239)",
    file: "src/lower/iterators/stepping.rs",
    find: "            && self.tcx.erase_and_anonymize_regions(ty) == self.tcx.erase_and_anonymize_regions(given)",
    replace: "            && ty == given",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_holding_a_closure_dropped_by_generic_code\\.rs$"],
  },
  {
    name: "ref-param-into-iter-undictionaried",
    breaks:
      "generic code's `zip` of a `&'a I` bounded `&'a I: IntoIterator` zips the value itself, not its dictionary's `into_iter`: `[(1, 'abc')]` where Rust gives `[(1, 'a'), (2, 'b'), (3, 'c')]` (strsim's `jaro`, #1264)",
    file: "src/lower/iterators/stepping.rs",
    find: "            ty::Ref(..) => true,\n",
    replace: "            ty::Ref(..) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_of_a_reference_given_to_generic_code.rs$"],
  },
  {
    name: "collected-write-results-untagged",
    breaks: "a collect of writes' results into a `Result` reads each as `{ TAG }`, `undefined.TAG` (#1410)",
    file: "src/lower/iterators/stepping.rs",
    find: "        let Some(crossing) = item.and_then(|item| self.std_reads_tagged(item)) else {\n",
    replace: "        let Some(crossing) = item.and_then(|item| self.std_reads_tagged(item)).filter(|_| false) else {\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_items.rs$"],
  },
  {
    name: "cloned-owners-bounds-unasked",
    breaks: "a type taken for a `pub` generic function's cloned `Map<I, F>`, any type its callers may give, isn't asked of that function's bounds, so another generic function's `map(Some)` of items with a destructor is cloned apart and refused (#1318)",
    file: "src/lower/iterators/stepping.rs",
    find: "                && within_its_owners(cloned_ty, ty)\n",
    replace: "                && (true || within_its_owners(cloned_ty, ty))\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_map_beside_a_cloned_one_of_other_bounds.rs$"],
  },
  {
    name: "copied-source-cloned-apart",
    breaks: "a `copied()` whose copy is its source, of numbers a generic or the crate's own source gives, is taken as cloned apart, made as its source, and its clone `$clone` of what carries none throws (#1181)",
    file: "src/lower/iterators/stepping.rs",
    find: "        ) && !self.is_its_source(ty)\n",
    replace: "        )\n",
    tests: ["test/corpus.test.ts", "-t", "^(copied_clone_of_its_own|lazy_iterator_clone|generic_copied_cloned).rs$"],
  },
  {
    name: "mut-sequence-adapted-by-its-recogniser",
    breaks: "a `&mut` to a sequence given to an `IntoIterator` bound is given as it is where no recogniser of `into_iter` answers, not a handle on each item (#1477)",
    file: "src/lower/iterators/stepping.rs",
    find: "            None if mutability == ty::Mutability::Mut && sequence(pointee) => Std::Same,\n",
    replace: "",
    tests: ["test/tidy-stages.test.ts","-t","adapted by its type, without into_iter's recogniser"],
  },
  {
    name: "mut-result-given-its-shared-items",
    breaks: "a `&mut Result` given to an `IntoIterator` bound gives its value, not a handle on it, where no recogniser answers (#1477)",
    file: "src/lower/iterators/stepping.rs",
    find: "                true => crate::lower::combinators::Comb::ResultIterMut,\n",
    replace: "                true => crate::lower::combinators::Comb::ResultIter,\n",
    tests: ["test/tidy-stages.test.ts","-t","adapted by its type, without into_iter's recogniser"],
  },

  {
    name: "box-param-not-given-a-js-iterator",
    breaks: "generic code's `Box<I>` is given the crate's iterator's value as it is, which it steps as plain data, `count()` of `Up(0, 3)` its two fields (#1601)",
    file: "src/lower/iterators/stepping.rs",
    find: "                _ if let Some(inner) = input.boxed_ty() => (inner, given),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
  {
    name: "into-iter-bound-made-unknowing",
    breaks: "the crate's iterator given for a `T: IntoIterator` whose `T::IntoIter` a bound says is double-ended or exact-sized is made a JS iterator that knows neither, so generic code's `t.into_iter().rev()` throws `it.back is not a function` (#1604)",
    file: "src/lower/iterators/stepping.rs",
    find: "        (back || len).then_some((back, len))\n",
    replace: "        (false && (back || len)).then_some((back, len))\n",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^into_iter_bound_stepped_from_its_back.rs$|^into_iter_bound_stepped_from_its_back with"],
  },
  {
    name: "into-iter-bound-length-unknown",
    breaks: "the crate's iterator given for a `T` whose `T::IntoIter` is bound `ExactSizeIterator` is made knowing its back alone, so generic code's `t.into_iter().len()` throws `it.len is not a function` (#1604)",
    file: "src/lower/iterators/stepping.rs",
    find: "            true => self.as_exact_size(held, given, back, span),\n",
    replace: "            true => self.as_double_ended(held, given, span),\n",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^into_iter_bound_stepped_from_its_back.rs$|^into_iter_bound_stepped_from_its_back with"],
  },
  {
    name: "made-into-iter-of-what-it-doesnt-take",
    breaks: "the crate's iterator a `U::IntoIter` bound is of, where `U` is no input of the callee, std's compiled `flatten`'s, is taken for one the call made a JS iterator, and given its dictionary, so the flatten's back steps throw `it.back is not a function` (#1604)",
    file: "src/lower/iterators/stepping.rs",
    find: "        if !inputs.iter().any(taken) {\n",
    replace: "        if false && !inputs.iter().any(taken) {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_count_takes_each_inner_count.rs$"],
  },
];
