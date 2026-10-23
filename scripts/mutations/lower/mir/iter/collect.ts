// Mutations of src/lower/mir/iter/collect.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-sum-from-one",
    breaks: "`sum` of an iterator starts from one",
    file: "src/lower/mir/iter/collect.rs",
    find: "            true => (Op::Add, 0),\n",
    replace: "            true => (Op::Add, 1),\n",
    tests: ["test/corpus.test.ts", "-t", "assert_failure"],
  },
  {
    name: "mir-collect-map-refused",
    breaks: "`collect` into a map or a set is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "        } else if self.is_map(output) {\n",
    replace: "        } else if false && self.is_map(output) {\n",
    tests: ["test/corpus.test.ts","-t","value_keys|user_hash|range_into_set"],
  },
  {
    "name": "mir-mapped-range-lazy",
    "breaks": "a range mapped and collected stays `$iterator({ start, end }, $rangeNext).map(f).toArray()`, not `$range(a, b).map(f)`",
    "file": "src/lower/mir/iter/collect.rs",
    "find": "match self.mapped_range(&it, iter_ty) {",
    "replace": "match None::<Expr> {",
    "tests": ["test/lowering-iterators.test.ts", "-t", "a range mapped and collected is its array mapped"]
  },
  {
    "name": "mir-float-sum-from-zero",
    "breaks": "a float's sum from MIR starts at 0, an empty sum `0`, not Rust's -0.0",
    "file": "src/lower/mir/iter/collect.rs",
    "find": "(true, true) => Expr::num(-0.0),",
    "replace": "(true, true) => Expr::num(0.0),",
    "tests": ["test/lowering-numbers.test.ts", "-t", "a float's sum from MIR keeps the sign"]
  },
  {
    "name": "mir-collect-heap-unheaped",
    "breaks": "`collect()` into a `BinaryHeap` from MIR is its items as they came, not in heap order",
    "file": "src/lower/mir/iter/collect.rs",
    "find": "        Ok(if self.is_std_type(output, StdItem::BinaryHeap) {\n",
    "replace": "        Ok(if false && self.is_std_type(output, StdItem::BinaryHeap) {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "heap_peek_mut"
    ]
  },
  {
    name: "mir-fold-own-of-init",
    breaks: "MIR's `fold` asks whether what it starts from is the crate's function, not its callback",
    file: "src/lower/mir/iter/collect.rs",
    find: "        let own = arg_tys.last().is_some_and(|ty| self.is_crate_fn_item(*ty));\n        let callback = self.binary_callback(f, own);\n        let Some(init) = init else {\n",
    replace: "        let own = arg_tys.get(1).is_some_and(|ty| self.is_crate_fn_item(*ty));\n        let callback = self.binary_callback(f, own);\n        let Some(init) = init else {\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "a fold given a function of the crate's"],
  },
  {
    "name": "mir-iterable-collected-lazy",
    "breaks": "a binding's list collected stays `Iterator.from(nodes).map(f).toArray()`, not `Array.from(nodes).map(f)`",
    "file": "src/lower/mir/iter/collect.rs",
    "find": "None => match self.collected_items(&it, iter_ty) {",
    "replace": "None => match None::<Expr> {",
    "tests": [
      "test/compiler.test.ts",
      "-t",
      "methods are their type's object|webapi crate's bindings"
    ]
  },
  {
    "name": "fold-start-left-out",
    "breaks": "`fold(None, f)`'s `reduce(f, undefined)` loses its start, so an empty slice's throws and another's starts from its first item",
    "file": "src/lower/mir/iter/collect.rs",
    "find": "        crate::lower::calls::given_to_js(&mut args);\n        Expr::call(Expr::member(it, \"reduce\"), args)\n",
    "replace": "        Expr::call(Expr::member(it, \"reduce\"), args)\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "fold_from_none"
    ]
  },
  {
    "name": "rposition-of-lent-wrapped",
    "breaks": "`rposition` of a kept iterator is given its `$lent` wrapper, whose items it can't see, so it finds none",
    "file": "src/lower/mir/iter/collect.rs",
    "find": "        let items = crate::lower::iterators::unlent(&it).clone();\n",
    "replace": "        let items = it;\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "rposition"
    ]
  },
  {
    "name": "rposition-of-lazy-unrefused",
    "breaks": "`rposition` of a lazy chain isn't refused, and is given what it can't step from the back",
    "file": "src/lower/mir/iter/collect.rs",
    "find": "        if self.array_source(iter_ty).is_none() {\n",
    "replace": "        if false {\n",
    "tests": [
      "test/traits.test.ts",
      "-t",
      "rposition of a lazy chain"
    ]
  },
  {
    "name": "own-from-iter-unused",
    "breaks": "`collect()` into a `Result` of a collection of the crate's own is refused, though its `from_iter` is the crate's",
    "file": "src/lower/mir/iter/collect.rs",
    "find": "        } else if let Some(made) = self.made_by_own_from_iter(it.clone(), output, item, span)? {\n",
    "replace": "        } else if let Some(made) = None::<Expr> {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "collect_fallible_into"
    ]
  },
  {
    name: "collect-into-param-refused",
    breaks: "`collect` into a type parameter, a `B: FromIterator`, is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "            && let Some(dictionary) = self.collected_evidence(output, item)\n",
    replace: "            && let Some(dictionary) = None::<Expr>\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect.rs$"],
  },
  {
    name: "from-iter-array-stepped",
    breaks: "`String::from_iter` of an array steps it as an iterator, which an array isn't, and JS throws",
    file: "src/lower/mir/iter/collect.rs",
    find: "            return Ok(Some(self.collected_into(items, output, item, None, span)?));\n",
    replace: "            return Ok(Some(self.collected_into(items, output, item, Some(iter_ty), span)?));\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_called.rs$"],
  },
  {
    name: "from-iter-of-anything",
    breaks: "`Vec::from_iter` of an `Option` is collected as if it were an iterator",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if !matches!(given.kind(), ty::Param(_)) && !self.implements_iterator(given) {\n            return Ok(None);\n        }\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "from_iter of an Option"],
  },
  {
    name: "try-fold-result-as-option",
    breaks: "`try_fold` into a `Result` stops at a `None`, which an `Ok` never is, and goes on past an `Err`",
    file: "src/lower/mir/iter/collect.rs",
    find: "            None if self.recognition().is_result(output) => \"$tryFoldOk\",\n",
    replace: "            None if self.recognition().is_result(output) => \"$tryFoldSome\",\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "collect-into-held-param-unbound",
    breaks: "`collect()` into a `Result<U, E>` a bound says it's collected into is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if output.has_param()\n            && let Some(item) = item\n            && let Some(dictionary) = self.collected_evidence(output, item)\n",
    replace: "        if false\n            && let Some(item) = item\n            && let Some(dictionary) = self.collected_evidence(output, item)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_accumulations.rs$"],
  },
  {
    name: "reduce-unboxed",
    breaks: "`reduce`'s `Some` of what may look like `None` isn't boxed",
    file: "src/lower/mir/iter/collect.rs",
    find: "            if item.is_some_and(|item| self.boxed_payload(item)) {\n                args.push(Expr::bool(true));\n            }\n            return Expr::call_helper(\"$reduce\", args);\n",
    replace: "            return Expr::call_helper(\"$reduce\", args);\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_reduce.rs$"],
  },
  {
    name: "boxed-slice-collect-refused",
    breaks: "`collect` into a `Box<[T]>` is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "        } else if self.is_vec_like(output) || output.boxed_ty().is_some_and(|inner| inner.is_slice()) {\n",
    replace: "        } else if self.is_vec_like(output) {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_slice_values.rs$"],
  },
  {
    name: "try-for-each-fmt-result-tagged",
    breaks: "`try_for_each` into a `fmt::Result` reads its function's nothing as a `Result`, and throws",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if self.recognition().is_fmt_result(output) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_writes_captured_formatter.rs$"],
  },
  {
    name: "try-fold-boxed-option-refused",
    breaks: "`try_fold` into an `Option` of what could look like `None` is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some(_) => \"$tryFoldSome\",\n",
    replace: "            Some(made) if !self.boxed_payload(made) => \"$tryFoldSome\",\n",
    tests: ["test/corpus.test.ts", "-t", "^try_for_each.rs$"],
  },
  {
    name: "collect-fallible-source-unclosed",
    breaks: "a `collect` into a `Result` that stops at an `Err` leaves what's left of its source undropped",
    file: "src/lower/mir/iter/collect.rs",
    find: "                Some(iter_ty) => self.closing(it, iter_ty, span)?,\n",
    replace: "                Some(_) => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^drops_of_consumed_iterators.rs$"],
  },
  {
    name: "fallible-sum-of-results-as-options",
    breaks: "`sum()` into a `Result` collects as into an `Option`, so an `Err` is a value",
    file: "src/lower/mir/iter/collect.rs",
    find: "                None => Expr::call_helper(\"$collectResults\", vec![it, into]),\n",
    replace: "                None => Expr::call_helper(\"$collectOptions\", vec![it, Expr::bool(false), into]),\n",
    tests: ["test/corpus.test.ts", "-t", "^sum_fallible.rs$"],
  },
  {
    name: "fallible-sum-refused",
    breaks: "`sum()` into an `Option` of numbers is refused, as a dictionary of no impl",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if let Some(num) = option.or(result).and_then(Num::of) {\n",
    replace: "        if let Some(num) = option.or(result).and_then(Num::of).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^sum_fallible.rs$"],
  },
  {
    name: "from-iter-of-a-map-refused",
    breaks: "`Vec::from_iter(set)`, of a map's or a set's items, is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if self.is_map(iter_ty)\n            && let ty::Adt(_, args) = iter_ty.kind()\n",
    replace: "        if false\n            && let ty::Adt(_, args) = iter_ty.kind()\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_of_maps.rs$"],
  },
  {
    name: "rposition-of-user-refused",
    breaks: "`rposition` of the crate's double-ended, exact-sized iterator is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "            let back = made || self.knows_its_back(iter_ty) || self.user_double_ended(iter_ty);\n",
    replace: "            let back = made || self.knows_its_back(iter_ty);\n",
    tests: ["test/corpus.test.ts", "-t", "^user_iterator_ends.rs$"],
  },
  {
    name: "collects-by-fold-not-strings",
    breaks: "`collect` into a `String` of the crate's iterator steps it by `next`, not by its own `fold`",
    file: "src/lower/mir/iter/collect.rs",
    find: "        self.is_lang_adt(output, LangItem::String)\n            ||",
    replace: "        false\n            ||",
    tests: ["test/corpus.test.ts",  "-t",  "^own_fold_and_nth.rs$"],
  },
  {
    name: "collects-by-fold-vecs",
    breaks: "`collect` into a `Vec` of the crate's iterator folds it, where std's steps it by `next`",
    file: "src/lower/mir/iter/collect.rs",
    find: "            || output.is_unit()\n",
    replace: "            || output.is_unit()\n            || self.is_vec_like(output)\n",
    tests: ["test/corpus.test.ts",  "-t",  "^own_fold_and_nth.rs$"],
  },
];
