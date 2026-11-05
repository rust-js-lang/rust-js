// Mutations of src/lower/mir/iter/collect.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "counted-collect-made-after-room",
    breaks:
      "`repeat(made()).take(usize::MAX).collect()` asks room before `made()` runs, so it never prints, where Rust makes the iterator first (#246)",
    file: "src/lower/mir/iter/collect.rs",
    find: '                    true => self.spill("it", it, made),\n',
    replace: "                    true => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "counted-collect-room-unasked",
    breaks:
      "`repeat_n(1u8, usize::MAX).collect::<Vec<_>>()` asks no room, so it steps until memory runs out, where std panics `capacity overflow` at once (#246)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        let counted = item.is_some()\n",
    replace: "        let counted = false && item.is_some()\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "trusted-collect-by-next",
    breaks:
      "`collect()` of std's struct that's `TrustedLen`, a zip's, steps it by `next()`, not its `fold` as std's `extend_trusted` does: its closure runs once more (#380)",
    file: "src/lower/mir/iter/collect.rs",
    find: '                None if folded && self.collected_items(&it, iter_ty).is_none() => {\n                    let args = self.folded_unwinding(it, output, item, iter_ty, span)?;\n                    Expr::call_helper("$foldToArray", args)\n                }\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_fold_steps_the_shorter.rs$"],
  },
  {
    name: "endless-collect-unchecked",
    breaks:
      "`(0..).map(f).collect::<Vec<_>>()`, a `TrustedLen` with no upper bound, maps without end where Rust panics `capacity overflow` before any item's made (ADR 0940)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            let endless = trusted && self.recognition().endless_by_type(iter_ty);\n",
    replace:
      "            let endless = trusted && self.recognition().endless_by_type(iter_ty) && false;\n",
    tests: ["test/corpus.test.ts", "-t", "^asked_length_overflow.rs$"],
  },
  {
    name: "endless-once-collect-unchecked",
    breaks:
      "`collect()` into a `Vec` of a `cycle()` of an array's items, not `TrustedLen`, maps without end where Rust steps it once and panics `capacity overflow` (#433, ADR 0940)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            let endless_once = match !trusted && asks_room && item.is_some_and(sized) {\n",
    replace:
      "            let endless_once = match false && !trusted && asks_room && item.is_some_and(sized) {\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_endless_once_stepped.rs$"],
  },
  {
    name: "endless-once-of-items-of-no-size",
    breaks:
      "`collect()` of a `cycle()` into a `Vec<()>` panics `capacity overflow`, where Rust has room for any number of items of no size and goes on (#433, ADR 0940)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            let endless_once = match !trusted && asks_room && item.is_some_and(sized) {\n",
    replace:
      "            let endless_once = match !trusted && asks_room && item.is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_endless_once_stepped.rs$"],
  },
  {
    name: "endless-once-item-drop-ungiven",
    breaks:
      "the item a `collect()` of what has no end once stepped makes, `(0..).skip(1).map(|_| Noisy(..))`'s, isn't dropped as it panics `capacity overflow` (#433, ADR 0940)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                    if endless_once\n                        && self.drops(item) != crate::lower::drops::Drops::Nothing\n",
    replace:
      "                    if false\n                        && self.drops(item) != crate::lower::drops::Drops::Nothing\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_endless_once_stepped.rs$"],
  },
  {
    name: "collect-hint-room-unasked",
    breaks:
      "`collect()` into a `Vec` of std's struct asks no room of its hint first, so `(0..usize::MAX).map(f).collect()` maps without end where Rust's `SpecFromIter` panics `capacity overflow` (ADR 0940)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                            || self.compiled_struct_at(iter_ty, span)\n",
    replace:
      "                            || false && self.compiled_struct_at(iter_ty, span)\n",
    tests: ["test/corpus.test.ts", "-t", "^std_rev.rs$"],
    snapshots: true,
  },
  {
    name: "collect-into-unit-refused",
    breaks: "`collect()` into `()` is refused where it steps each item (#322)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        } else if output.is_unit() {\n",
    replace: "        } else if output.is_unit() && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_into_unit.rs$"],
  },
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
    tests: ["test/corpus.test.ts", "-t", "value_keys|user_hash|range_into_set"],
  },
  {
    name: "mir-mapped-range-lazy",
    breaks:
      "a range mapped and collected stays `Array.from($iterator({ start, end }, $rangeNext).map(f))`, not `$range(a, b).map(f)`",
    file: "src/lower/mir/iter/collect.rs",
    find: "match self.mapped_range(&it, iter_ty, item) {",
    replace: "match None::<Expr> {",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "a range mapped and collected is its array mapped",
    ],
  },
  {
    name: "mir-float-sum-from-zero",
    breaks:
      "a float's sum from MIR starts at 0, an empty sum `0`, not Rust's -0.0",
    file: "src/lower/mir/iter/collect.rs",
    find: "(true, true) => Expr::num(-0.0),",
    replace: "(true, true) => Expr::num(0.0),",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a float's sum from MIR keeps the sign",
    ],
  },
  {
    name: "mir-collect-heap-unheaped",
    breaks:
      "`collect()` into a `BinaryHeap` from MIR is its items as they came, not in heap order",
    file: "src/lower/mir/iter/collect.rs",
    find: "        Ok(if self.is_std_type(output, StdItem::BinaryHeap) {\n",
    replace:
      "        Ok(if false && self.is_std_type(output, StdItem::BinaryHeap) {\n",
    tests: ["test/corpus.test.ts", "-t", "heap_peek_mut"],
  },
  {
    name: "mir-fold-own-of-init",
    breaks:
      "MIR's `fold` asks whether what it starts from is the crate's function, not its callback",
    file: "src/lower/mir/iter/collect.rs",
    find: "        let own = arg_tys.last().is_some_and(|ty| self.is_called_as_written(*ty));\n",
    replace:
      "        let own = arg_tys.get(1).is_some_and(|ty| self.is_called_as_written(*ty));\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "a fold given a function of the crate's",
    ],
  },
  {
    name: "mir-iterable-collected-lazy",
    breaks:
      "a binding's list collected stays `Array.from(Iterator.from(nodes).map(f))`, not `Array.from(nodes).map(f)`",
    file: "src/lower/mir/iter/collect.rs",
    find: "None => match self.collected_items(&it, iter_ty) {",
    replace: "None => match None::<Expr> {",
    tests: [
      "test/compiler.test.ts",
      "-t",
      "methods are their type's object|webapi crate's bindings",
    ],
  },
  {
    name: "fold-start-left-out",
    breaks:
      "`fold(None, f)`'s `reduce(f, undefined)` loses its start, so an empty slice's throws and another's starts from its first item",
    file: "src/lower/mir/iter/collect.rs",
    find: "        crate::lower::calls::given_to_js(&mut args);\n        // `$foldDropping(it, init, f, drop)`",
    replace: "        // `$foldDropping(it, init, f, drop)`",
    tests: ["test/corpus.test.ts", "-t", "fold_from_none"],
  },
  {
    name: "rposition-of-lent-wrapped",
    breaks:
      "`rposition` of a kept iterator is given its `$lent` wrapper, `$rposition($lent(it), f)`, where `it` is what it steps: the wrapper passes on its back and its length, so it finds the same, but the JS reads a helper no one wrote",
    file: "src/lower/mir/iter/collect.rs",
    find: "        let items = crate::lower::iterators::unlent(&it).clone();\n",
    replace: "        let items = it;\n",
    tests: ["test/corpus.test.ts", "-t", "^rposition.rs$"],
    snapshots: true,
  },
  {
    name: "rposition-of-lazy-unrefused",
    breaks:
      "`rposition` of a lazy chain isn't refused, and is given what it can't step from the back",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if self.array_source(iter_ty).is_none() {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^rposition_of_lazy.rs$"],
    snapshots: true,
  },
  {
    name: "own-from-iter-unused",
    breaks:
      "`collect()` into a `Result` of a collection of the crate's own is refused, though its `from_iter` is the crate's",
    file: "src/lower/mir/iter/collect.rs",
    find: "        } else if let Some(made) = self.made_by_own_from_iter(it.clone(), output, item, span)? {\n",
    replace: "        } else if let Some(made) = None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "collect_fallible_into"],
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
    breaks:
      "`String::from_iter` of an array steps it as an iterator, which an array isn't, and JS throws",
    file: "src/lower/mir/iter/collect.rs",
    find: "            return Ok(Some(self.collected_into(items, output, item, None, span)?));\n",
    replace:
      "            return Ok(Some(self.collected_into(items, output, item, Some(iter_ty), span)?));\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(from_iter_called|text_predicates)\\.rs$",
    ],
  },
  {
    name: "from-iter-of-anything",
    breaks:
      "`Vec::from_iter` of an `Option` is collected as if it were an iterator",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if !matches!(given.kind(), ty::Param(_)) && !self.implements_iterator(given) {\n            return Ok(None);\n        }\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "from_iter of an Option"],
  },
  {
    name: "try-fold-result-as-option",
    breaks:
      "`try_fold` into a `Result` stops at a `None`, which an `Ok` never is, and goes on past an `Err`",
    file: "src/lower/mir/iter/collect.rs",
    find: '            (None, None) => "$tryFoldOk",\n',
    replace: '            (None, None) => "$tryFoldSome",\n',
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "collect-into-held-param-unbound",
    breaks:
      "`collect()` into a `Result<U, E>` a bound says it's collected into is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if output.has_param()\n            && let Some(item) = item\n            && let Some(dictionary) = self.collected_evidence(output, item)\n",
    replace:
      "        if false\n            && let Some(item) = item\n            && let Some(dictionary) = self.collected_evidence(output, item)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_accumulations.rs$"],
  },
  {
    name: "reduce-unboxed",
    breaks: "`reduce`'s `Some` of what may look like `None` isn't boxed",
    file: "src/lower/mir/iter/collect.rs",
    find: "            let boxed = item.is_some_and(|item| self.boxed_payload(item));\n",
    replace: "            let boxed = false;\n",
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
    breaks:
      "`try_for_each` into a `fmt::Result` reads its function's nothing as a `Result`, and throws",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if self.recognition().is_fmt_result(output) {\n",
    replace: "        if false {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^closure_writes_captured_formatter.rs$",
    ],
  },
  {
    name: "try-fold-boxed-option-refused",
    breaks:
      "`try_fold` into an `Option` of what could look like `None` is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: '            (Some(_), _) => "$tryFoldSome",\n',
    replace:
      '            (Some(made), _) if !self.boxed_payload(made) => "$tryFoldSome",\n            (Some(_), _) => return Err(self.unsupported(span, "a boxed `try_fold`")),\n',
    tests: ["test/corpus.test.ts", "-t", "^try_for_each.rs$"],
  },
  {
    name: "collect-fallible-source-unclosed",
    breaks:
      "a `collect` into a `Result` that stops at an `Err` leaves what's left of its source undropped",
    file: "src/lower/mir/iter/collect.rs",
    find: "                Some(iter_ty) => self.closing(it, iter_ty, span)?,\n",
    replace: "                Some(_) => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^drops_of_consumed_iterators.rs$"],
  },
  {
    name: "fallible-sum-of-results-as-options",
    breaks:
      "`sum()` into a `Result` collects as into an `Option`, so an `Err` is a value",
    file: "src/lower/mir/iter/collect.rs",
    find: '                None => Expr::call_helper("$collectResults", vec![it, into]),\n',
    replace:
      '                None => Expr::call_helper("$collectOptions", vec![it, Expr::bool(false), into]),\n',
    tests: ["test/corpus.test.ts", "-t", "^sum_fallible.rs$"],
  },
  {
    name: "fallible-sum-refused",
    breaks:
      "`sum()` into an `Option` of numbers is refused, as a dictionary of no impl",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if let Some(num) = option.or(result).and_then(Num::of) {\n",
    replace:
      "        if let Some(num) = option.or(result).and_then(Num::of).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^sum_fallible.rs$"],
  },
  {
    name: "from-iter-of-a-map-refused",
    breaks: "`Vec::from_iter(set)`, of a map's or a set's items, is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if self.is_map(iter_ty)\n            && let ty::Adt(_, args) = iter_ty.kind()\n",
    replace:
      "        if false\n            && let ty::Adt(_, args) = iter_ty.kind()\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_of_maps.rs$"],
  },
  {
    name: "collects-by-fold-not-strings",
    breaks:
      "`collect` into a `String` of the crate's iterator steps it by `next`, not by its own `fold`",
    file: "src/lower/mir/iter/collect.rs",
    find: "        self.is_lang_adt(output, LangItem::String)\n            ||",
    replace: "        false\n            ||",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "collects-by-fold-vecs",
    breaks:
      "`collect` into a `Vec` of the crate's iterator folds it, where std's steps it by `next`",
    file: "src/lower/mir/iter/collect.rs",
    find: "            || output.is_unit()\n",
    replace:
      "            || output.is_unit()\n            || self.is_vec_like(output)\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "by-fold-ignores-own-nth",
    breaks: "what takes by fold asks a skip's nth(n - 1)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            || recognition.folds_by_its_own_nth(iter_ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "rposition-of-lazy-refused",
    breaks:
      "`rposition` of a lazy exact-sized, double-ended iterator is made so by `as_exact_size`",
    file: "src/lower/mir/iter/collect.rs",
    find: "            // What a `&mut` lends, which it's stepped from its back as it is.\n",
    replace:
      '            if self.knows_its_back(iter_ty) == self.knows_its_back(iter_ty) { return Err(self.unsupported(span, "`rposition`")); }\n',
    tests: ["test/corpus.test.ts", "-t", "^rposition_of_lazy.rs$"],
  },
  {
    name: "counted-range-into-a-heap",
    breaks:
      "`(0..5).map(f).collect()` into a `BinaryHeap` is the array of them, `Array.from(..)`, not the heap made of it (#361)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if self.is_std_type(output, crate::lower::recognition::StdItem::Vec)\n",
    replace: "        if self.is_vec_like(output)\n",
    tests: ["test/corpus.test.ts", "-t", "^range_map_collected.rs$"],
  },
  {
    name: "counted-range-never",
    breaks:
      "`(0..n).map(f).collect()` is `$range(0, n, size).map(f)`, an array of its indices made to be mapped away (#361)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            && let Some(items) = self.counted_range(&it, iter_ty, item, span, made)\n",
    replace: "            && let Some(items) = None::<Expr>\n",
    tests: ["test/corpus.test.ts", "-t", "^range_map_collected.rs$"],
    snapshots: true,
  },
  {
    name: "collected-lent-range-peeled",
    breaks:
      "a `collect()` of a lent inclusive range, `r.by_ref().collect()`, is taken as the range it lends, stepped by `next` past its end (#414)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            true => receiver,\n            false => iter_ty,\n",
    replace: "            true => iter_ty,\n            false => iter_ty,\n",
    tests: ["test/corpus.test.ts", "-t", "^range_inclusive_exhausted.rs$"],
  },
  {
    name: "collect-unwinding-unseen",
    breaks:
      "a `collect` into a `Vec` a closure's panic may stop is `Array.from`, which drops neither what it collected nor what's left (#422)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        if !iter_ty.is_some_and(|ty| self.steps_may_panic(ty)) {\n            return Ok(None);\n        }\n",
    replace: "        return Ok(None);\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_iteration_drops.rs$"],
  },
  {
    name: "rest-first-never",
    breaks:
      "a collect std does otherwise than in place, `chain`'s, a panic stopping it, drops what it collected before what's left (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            (Some(_), Some(item)) => self.rest_dropped_first(output, iter_ty, item, span)?,",
    replace: "            (Some(_), Some(_)) => None,",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "map-collect-unwinding-unseen",
    breaks:
      "a `collect` into a hash map or set a panic may stop is `$extendMap`'s, which drops neither what's left nor what it held (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                unwinding => (it, unwinding.is_some()),",
    replace: "                _ => (it, false),",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "sorted-collect-unwinding-unseen",
    breaks:
      "a `collect` into a B-tree a panic may stop drops nothing it took (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                Some(unwinding) if self.is_sorted(output) => (unwinding.collected(it), false),",
    replace:
      "                Some(_) if self.is_sorted(output) => (it, false),",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "string-collect-unwinding-unseen",
    breaks:
      "a `collect` into a `String` a panic may stop leaves what's left of its owning source undropped (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        } else if self.is_lang_adt(output, LangItem::String) {\n            let items = match self.unwinding_collect(output, item, iter_ty, span)? {",
    replace:
      "        } else if self.is_lang_adt(output, LangItem::String) {\n            let items = match None::<UnwindingCollect> {",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "heap-collect-unwinding-unseen",
    breaks:
      "a `collect` into a `BinaryHeap` a panic may stop drops nothing it took (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        Ok(if self.is_std_type(output, StdItem::BinaryHeap) {\n            let items = match self.unwinding_collect(output, item, iter_ty, span)? {",
    replace:
      "        Ok(if self.is_std_type(output, StdItem::BinaryHeap) {\n            let items = match None::<UnwindingCollect> {",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "folded-collect-given-no-drop",
    breaks:
      "a compiled std struct's collect of items with a destructor, a `zip`'s, a panic stopping it, leaves what it collected undropped (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some(item) if self.steps_may_panic(iter_ty) && self.drops(item) != crate::lower::drops::Drops::Nothing => {",
    replace:
      "            Some(item) if false && self.drops(item) != crate::lower::drops::Drops::Nothing => {",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "fold-acc-unwind-unseen",
    breaks:
      "a `fold` whose accumulator has a destructor is given no drop for it, so a stage's panic leaves it undropped (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        match may_throw && self.drops(acc) != crate::lower::drops::Drops::Nothing {",
    replace:
      "        match false && may_throw && self.drops(acc) != crate::lower::drops::Drops::Nothing {",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "fold-stage-by-type-unseen",
    breaks:
      "a `fold` whose stages its JS still holds as a variable is judged one no stage of may panic (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        self.panicking_steps(iter_ty) > 0 || self.unseen_steps_may_panic(iter_ty)",
    replace: "        self.unseen_steps_may_panic(iter_ty)",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "fold-of-a-param-unwinding-unseen",
    breaks:
      "a fold of a type parameter's iterator, std's `Fuse::fold`'s of what it fuses, is given no drop of its accumulator (#424)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            ty::Dynamic(..) | ty::Param(_) => true,\n",
    replace: "            ty::Dynamic(..) => true,\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std",
    ],
  },
  {
    name: "folded-collect-after-unwinding",
    breaks:
      "a compiled std struct's collect a panic may stop, a `zip`'s, steps it by `next`, not its `SpecFold`, as std's `extend_trusted` does (#380, #423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                None if folded && self.collected_items(&it, iter_ty).is_none() => {\n",
    replace:
      "                None if false && folded && self.collected_items(&it, iter_ty).is_none() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
    snapshots: true,
  },
  {
    name: "try-fold-acc-unwind-unseen",
    breaks:
      "a `try_fold` into a `ControlFlow` of an accumulator with a destructor, a stage panicking part-way, leaves the accumulator undropped (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                self.stranded_drop(*receiver, *acc, span)?\n",
    replace: "                let _ = (receiver, acc);\n                None\n",
    tests: [
      "test/corpus.test.ts",
      "test/iterator-forms.test.ts",
      "-t",
      "^generic_folds_unwind.rs$|^panic_mid_fold_drops with",
    ],
  },
  {
    name: "by-fold-ignores-dropping-nth",
    breaks: "a collect into a `LinkedList` of a `rev().take(3).skip(2).map(..)` of items with a destructor steps it by `next`, whose `nth(2)` drops a slice front to back, where std's fold drops each from the back (#494)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            || recognition.nth_folded_sources(iter_ty).into_iter().any(drops)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^skip_folded_by_consumers.rs$"],
  },
  {
    name: "string-of-strings-folded-whole",
    breaks: "a `String` collected of `String`s folds its first item too, so a `skip`'s first step asks the crate's `nth(n - 1)`, not `nth(n)` (#494)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            true if first_by_next => Expr::call_helper(\"$folded\", vec![it, Expr::bool(true)]),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^skip_folded_by_consumers.rs$"],
  },
  {
    name: "collected-cows-as-objects",
    breaks: "`collect::<String>()` of `Cow<str>`s joins the `Cow`s themselves, `[object Object]` (#495)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                Some(item) => self.cow_texts(items, item),\n",
    replace: "                Some(_) => items,\n",
    tests: ["test/corpus.test.ts", "-t", "^cow_texts_joined.rs$"],
  },
  {
    name: "own-iterator-steps-unseen",
    breaks: "what may panic as an iterator is stepped doesn't see the crate's own iterator behind std's adapters, so `own.fuse().fold(Vec::new(), push)` is `.reduce(..)`, which never drops the accumulator its panicking `next` strands (rd763, rd768)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            ty::Adt(..) => recognition.is_user_iterator(part) && may_unwind(part),\n",
    replace: "            ty::Adt(..) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_over_own_next_that_panics.rs$"],
  },
  {
    name: "own-iterator-never-unwinding-ignored",
    breaks: "each of the crate's iterators behind std's adapters is taken to panic, its methods that never unwind too, so a fold of one that can't, `Calm`'s, is `$foldDropping`, not `reduce` (rd763)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                && !self.krate.never_unwinding.contains(&method)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_steps_behind_adapters_unwind.rs$"],
    snapshots: true,
  },
  {
    name: "dyn-iterator-steps-unseen",
    breaks: "a `dyn` iterator behind std's adapters is taken to step without panicking, so `boxed.fuse().fold(..)` never drops the accumulator its panicking `next` strands (rd768)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            ty::Dynamic(..) | ty::Param(_) => true,\n",
    replace: "            ty::Param(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^own_steps_behind_adapters_unwind.rs$"],
  },
  {
    name: "collect-own-steps-unseen",
    breaks: "a collect of std's adapter over the crate's iterator whose `next` panics is `Array.from(..)`, which drops neither what it collected nor what's left (rd765)",
    file: "src/lower/mir/iter/collect.rs",
    find: " || self.unseen_steps_may_panic(iter_ty) ||",
    replace: " ||",
    tests: ["test/corpus.test.ts", "-t", "^adapters_over_own_next_that_panics.rs$"],
  },
  {
    name: "try-fold-option-unwind-ungiven",
    breaks: "a `try_fold` into an `Option` or a `Result` is given no drop of its accumulator, so one a panicking step strands is never dropped (rd767)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            ([receiver, acc, _], IterMethod::TryFold | IterMethod::TryRFold) => {\n",
    replace: "            ([receiver, acc, _], IterMethod::TryRFold) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_steps_behind_adapters_unwind.rs$"],
  },
  {
    name: "opaque-stage-unseen",
    breaks: "a stage an `impl FnMut` is, a closure a function made, isn't counted among the steps that may panic, so a `try_fold` of a held chain of one never drops the accumulator it strands (rd767)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        (self.reveal(iter_ty).walk().filter_map(|part| part.as_type()))\n            .filter(|part| match *part.kind() {\n",
    replace: "        (iter_ty.walk().filter_map(|part| part.as_type()))\n            .filter(|part| match *part.kind() {\n",
    tests: ["test/corpus.test.ts", "-t", "^try_folds_of_stages_unwind.rs$"],
  },
  {
    name: "collect-holding-unclosed",
    breaks: "a collect of std's compiled `fuse()` of a `Vec`'s items, mapped by a closure that panics, is `Array.from(..)`, which never closes what the source has left (#653)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        let holds = iter_ty.is_some_and(|ty| self.drops(ty) != crate::lower::drops::Drops::Nothing);\n",
    replace: "        let holds = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^compiled_adapters_closed_as_a_collect_unwinds.rs$"],
  },
  {
    name: "passed-drops-panic-unseen",
    breaks: "a `skip()` and a `step_by()` of a `Vec`'s items whose drop panics, collected, are `Array.from(..)`: the panic of a drop they pass over leaves what's left undropped",
    file: "src/lower/mir/iter/collect.rs",
    find: " || self.std_work_may_panic(iter_ty)\n",
    replace: "\n",
    tests: ["test/corpus.test.ts", "-t", "^compiled_adapters_closed_as_a_collect_unwinds.rs$"],
  },
  {
    name: "folded-unwinding-by-closures",
    breaks: "a collect of std's compiled `zip()` by its fold is given its items' drop only where its type holds a closure, so the panic of the item the zip drops as its other side ends leaves what it collected undropped (#707)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some(item) if self.steps_may_panic(iter_ty) && self.drops(item) != crate::lower::drops::Drops::Nothing => {",
    replace:
      "            Some(item) if iter_ty.walk().any(|part| part.as_type().is_some_and(|t| t.is_closure() || t.is_fn())) && self.drops(item) != crate::lower::drops::Drops::Nothing => {",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "exhausted-drop-panic-unseen",
    breaks: "a `chain()` or a `flatten()` of the crate's iterators whose drop panics is taken to step without a panic, so the drop of a part it has exhausted leaves what's left undropped (#709)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                SteppingWork::DropsExhausted(part) => self.drop_may_panic(part),\n",
    replace: "                SteppingWork::DropsExhausted(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "own-clone-never-panics",
    breaks: "a clone of the crate's type is taken to never panic, so a `cloned()` whose clone panics leaves what it collected undropped (#708)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                    .is_some_and(|imp| self.impl_may_unwind(imp))\n",
    replace: "                    .is_some_and(|imp| false && self.impl_may_unwind(imp))\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "string-collected-by-next",
    breaks: "a collect into a `String` a panic may stop steps its chain by `next`, not its fold as `String::extend` does, so a nested flatten's source is dropped before the inner iterator it's in (#644); the matrix's `vec | flatten.flatten | string | consumer` shows it, which `--std-iterators` gets wrong still (#644's std cases), so the corpus holds its JS",
    file: "src/lower/mir/iter/collect.rs",
    find: "                Some(unwinding) => unwinding.folded(it),\n",
    replace: "                Some(unwinding) => unwinding.collected(it),\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^flatten_closed_part_way_drops_source_first.rs$"],
  },
  {
    name: "in-place-collect-by-next",
    breaks: "a collect std makes in place, of a `filter` whose discarded item's drop panics, steps it by `next`, so it drops what it collected, which std leaks (#681)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            InPlace::Yes => Expr::call_helper(\"$foldToArray\", args),\n",
    replace: "            InPlace::Yes => Expr::call_helper(\"$collectDropping\", args),\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_leaks_what_a_discard_strands.rs$"],
  },
  {
    name: "by-its-fold-past-nth-back",
    breaks: "a `take(n).rev()` of a `Vec` of destructors collected into a `String` of `char`s steps by `next`, its source's `nth_back(len - n)`, where std's extend folds it, `nth_back(len - n - 1)`, dropping 4 before 5 (#670)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            || recognition.nth_back_folded_sources(iter_ty).into_iter().any(drops)\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^a take's rev, consumed by fold or by next"],
  },
  {
    name: "generic-vec-collect-by-next",
    breaks: "a generic `collect()` into a `Vec` of a `take(n).rev()` of a `Vec`'s steps by `next` though the caller's iterator is `TrustedLen`, where std's `extend_trusted` folds it, dropping 3, 4, 5 for 4, 5, 3 (#670)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                return Ok(Expr::cond(trusted, by_fold, by_next));\n",
    replace: "                return Ok(Expr::cond(Expr::bool(false), by_fold, by_next));\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^a take's rev, consumed by fold or by next"],
  },
  {
    name: "in-place-of-a-parameter-unasked",
    breaks:
      "generic code reads no `IInPlace` of its caller, `undefined`, so its `src.map(f).collect()` of a `Vec`'s `IntoIter`, which std collects in place, a panic stopping it, drops what's left before what it collected (#672)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                self.type_fact_value(iter_ty, TypeFact::InPlace, span)?\n",
    replace: "                Expr::undefined()\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "never-in-place-not-first",
    breaks:
      "generic code's `src.map(f).fuse().collect()`, which std never collects in place, a panic stopping it, drops what it collected before what's left (#672)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some(false) => return Ok(first),\n",
    replace: "            Some(false) => return Ok(None),\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "in-place-default-in-trait-terms",
    breaks:
      "a trait's default copied into its blanket impl, compiled as a library, asks what std collects `self` in of `Self`, not the impl's `I` its dictionary is given, so it's refused (#672)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        (self.impl_type(iter_ty), self.impl_type(item), typing_env)\n",
    replace: "        (iter_ty, item, typing_env)\n",
    tests: ["test/corpus.test.ts", "-t", "^extension_trait_params.rs$"],
  },
  {
    name: "in-place-by-caller-by-next",
    breaks:
      "generic code's `src.filter(p).collect()` of a `Vec`'s `IntoIter`, which std collects in place by its fold, whose discarded item's drop panics, steps it by `next`, so it drops what it collected, which std leaks (#672, #681)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                    None => InPlace::IfCaller(self.caller_in_place(iter_ty, item, span)?),\n",
    replace: "                    None => InPlace::No,\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "in-place-by-caller-fold-unasked",
    breaks:
      "generic code's collect of a `Vec`'s `IntoIter` that std collects in place, its caller says, steps it by `next`, so a discarded item's drop that panics drops what it collected, which std leaks (#672, #681)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                let by_fold = Expr::call_helper(\"$foldToArray\", args.clone());\n",
    replace: "                let by_fold = Expr::call_helper(\"$collectDropping\", args.clone());\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "collect-trusted-by-next",
    breaks: "a `TrustedLen` collect into a `Vec` of one that folds a held function steps it by `next`, so the function drops after what it left (#710)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                || self.folds_held_function(iter_ty) && self.collects_in_place(Some(iter_ty), item) == Some(false))\n",
    replace: "                || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
  },
  {
    name: "fold-given-no-ends",
    breaks: "a compiled `Map::fold` gives the fold it asks no drop of the closure it moved into it, so its function drops after what the `take` left (#710)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some(&ty) if self.carries_its_drop(ty) || compiled && self.given_drop(ty).is_some() => {\n",
    replace: "            Some(&ty) if false && self.carries_its_drop(ty) => {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "fold-given-no-ends-of-a-param",
    breaks: "a compiled `Skip::fold` gives the fold it asks no drop of the `F` it was given, so a `map`'s function over it drops after what the `take` left (#710)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some(&ty) if self.carries_its_drop(ty) || compiled && self.given_drop(ty).is_some() => {\n",
    replace: "            Some(&ty) if self.carries_its_drop(ty) => {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "in-place-of-numbers-unsaid",
    breaks: "a collect in place of items with nothing to drop isn't said to be, so its fold moves a `map`'s function, which drops before what's left (#710)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                args.extend([Expr::undefined(), Expr::bool(false), Expr::bool(true)]);\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^fold_drops_moved_function_first with std"],
  },
  {
    name: "collect-dropping-closing-unsaid",
    breaks: "a `$collectDropping` collect, which closes what it steps, doesn't say so, so its consumer closes it again after in a `finally`, `try { result = $collectDropping(iter); } finally { iter.return(); }`, which does nothing (#660)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                Expr::call_helper(\"$collectDropping\", args).closes_what_it_steps()\n",
    replace: "                Expr::call_helper(\"$collectDropping\", args)\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_of_a_value_given_as_it_is.rs$"],
    snapshots: true,
  },
];
