// Mutations of src/lower/iterators/stepping.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
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
    find: "        if self.range_kind(ty.peel_refs()).is_none() && !self.iterator_given_as_it_is(ty.peel_refs()) {\n            return false;\n        }",
    replace:
      "        if self.range_kind(ty.peel_refs()).is_some() {\n            return true;\n        }\n        if !self.iterator_given_as_it_is(ty.peel_refs()) {\n            return false;\n        }",
    tests: ["test/corpus.test.ts", "-t", "range_values"],
  },
  {
    name: "lent-iterator-closable",
    breaks:
      "a lazy chain lent to `it.take(2)` is closed by it, and its lender's `next()` finds it done",
    file: "src/lower/iterators/stepping.rs",
    find: '        Expr::call(Expr::helper("$lent"), vec![it])',
    replace:
      '        Expr::call(Expr::member(Expr::var("Iterator"), "from"), vec![it])',
    tests: ["test/corpus.test.ts", "-t", "lent_iterators"],
  },
  {
    name: "collect-fallible-any-target",
    breaks:
      "`collect()` into a `Result` of a map, a set or text is `Ok` of an array, not of it",
    file: "src/lower/iterators/stepping.rs",
    find: "        let into = match self.is_vec_like(collection) || collection.boxed_ty().is_some_and(|t| t.is_slice()) {\n",
    replace: "        let into = match true {\n",
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
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^std_projection_drops.rs$"],
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
    find: "                .any(|&cloned| adapter(cloned) && self.instance_of(generic_closures(cloned), generic_closures(ty)))\n",
    replace:
      "                .any(|&cloned| self.instance_of(generic_closures(cloned), generic_closures(ty)))\n",
    tests: ["test/corpus.test.ts", "-t", "^cycled_iterators.rs$"],
  },
  {
    name: "cloned-closures-not-generic",
    breaks:
      "a `map` generic code makes isn't seen as the one its caller clones, whose closure is the caller's instance",
    file: "src/lower/iterators/stepping.rs",
    find: "                        ty::Closure(id, _) => tcx.type_of(id).instantiate_identity().skip_normalization(),\n",
    replace: "                        ty::Closure(..) => t,\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "cloned-apart-unrevealed",
    breaks:
      "a `map` a function gives as an `impl Iterator` isn't made cloned apart",
    file: "src/lower/iterators/stepping.rs",
    find: "        // What an `impl Iterator` hides, a function's result.\n        let ty = self.reveal(ty);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
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
    tests: ["test/corpus.test.ts", "-t", "^blanket_iterator_trait.rs$"],
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
    name: "mut-param-made-items",
    breaks:
      "a &mut to a type parameter's iterator is itself, stepped by its dictionary",
    file: "src/lower/iterators/stepping.rs",
    find: "            && !matches!(inner.kind(), ty::Param(_))\n            && self.option_of(*inner).is_none()\n",
    replace: "            && self.option_of(*inner).is_none()\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(copied_default_own_params|self_given_on_as_into_iter).rs$",
    ],
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
    find: "        let rest_first = rest_first && drop.is_some();",
    replace: "        let rest_first = false;",
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
];
