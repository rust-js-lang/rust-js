// Mutations of src/lower/mir/iter/collect.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "quiet-call-may-unwind",
    breaks: "a closure of the crate's that never unwinds is taken to, so `inspect` of one guards its value for no reason (#903)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                !self.krate.never_unwinding.contains(&def_id)\n                    && !matches!(self.tcx.def_kind(def_id), rustc_hir::def::DefKind::Ctor(..))\n",
    replace: "                !matches!(self.tcx.def_kind(def_id), rustc_hir::def::DefKind::Ctor(..))\n",
    tests: ["test/corpus.test.ts", "-t", "^inspect_unwinds_its_value.rs$"],
    snapshots: true,
  },
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
    name: "counted-collect-of-a-called-count-unasked",
    breaks:
      "`repeat_n(1u8, *n.as_ref().unwrap()).collect::<Vec<_>>()` of `usize::MAX` asks no room, as its count is read through a call, so JS throws its own `RangeError` where std panics `capacity overflow` (#1044)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some(Err(Uncounted::RunsCode | Uncounted::Unknown)) => true,\n",
    replace: "            Some(Err(Uncounted::RunsCode)) => false,\n            Some(Err(Uncounted::Unknown)) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "counted-collect-asked-by-hint-unmarked",
    breaks:
      "`repeat(1u8).take(*n.as_ref().unwrap()).collect()` asked its room by its hint loses its marks, so it's taken for std's struct, asked again and collected by `$foldToArray` (#1044)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                        asked.facts = marks;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
    snapshots: true,
  },
  {
    name: "counted-collect-of-what-drops-asked-in-place",
    breaks:
      "`repeat_n(Dropped(1), usize::MAX).collect()` asks `$roomFor(n, size)` in place, whose panic doesn't close the iterator, so its value is never dropped where Rust drops it as the collect unwinds (#1044)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some((len, size)) if held.is_empty() && !drops => {\n",
    replace: "            Some((len, size)) if held.is_empty() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "counted-collect-asked-by-hint-twice",
    breaks:
      "`repeat(1).chain(own).take(counted(n)).collect()`, whose hint runs the crate's own `size_hint`, is asked its room by its hint as it's counted and again as a `Vec` grows it (#1044)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                && item.is_some()\n                && (self.grown_ask(\n                    output,\n                    item,\n                    (iter_ty, crate::lower::iterators::binding_made(&it)),\n                    self.compiled_struct(iter_ty, &it),\n                    span,\n                )?)\n                .is_none() =>\n",
    replace: "                && item.is_some() =>\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
    snapshots: true,
  },
  {
    name: "counted-collect-held-unasked",
    breaks:
      "`let it = repeat_n(1u8, n); it.collect::<Vec<_>>()` of `usize::MAX`, held in a variable, asks no room, as its count isn't read where it's made, so JS throws its own `RangeError` where std panics `capacity overflow` (#1046)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some(Err(Uncounted::RunsCode | Uncounted::Unknown)) => true,\n",
    replace: "            Some(Err(Uncounted::RunsCode)) => true,\n            Some(Err(Uncounted::Unknown)) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_iterators_held_ask_room.rs$"],
  },
  {
    name: "trusted-collect-by-next",
    breaks:
      "`collect()` of std's struct that's `TrustedLen`, a zip's, steps it by `next()`, not its `fold` as std's `extend_trusted` does: its closure runs once more (#380)",
    file: "src/lower/mir/iter/collect.rs",
    find: '            None if folded && self.collected_items(&it).is_none() => {\n                let args = self.folded_unwinding(it, output, item, iter_ty, span)?;\n                Expr::call_helper("$foldToArray", args)\n            }\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_fold_steps_the_shorter.rs$"],
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
    name: "mir-collect-heap-unheaped",
    breaks:
      "`collect()` into a `BinaryHeap` from MIR is its items as they came, not in heap order",
    file: "src/lower/mir/iter/collect.rs",
    find: "                true => self.heap_values(HeapOp::From, vec![items], (output, false), span)?,\n",
    replace: "                true => items,\n",
    tests: ["test/corpus.test.ts", "-t", "heap_peek_mut"],
  },
  {
    name: "mir-iterable-collected-lazy",
    breaks:
      "a binding's list collected stays `Array.from(Iterator.from(nodes).map(f))`, not `Array.from(nodes).map(f)`",
    file: "src/lower/mir/iter/collect.rs",
    find: "None => match self.collected_items(&it) {",
    replace: "None => match None::<Expr> {",
    tests: [
      "test/compiler.test.ts",
      "-t",
      "methods are their type's object|webapi crate's bindings",
    ],
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
    name: "boxed-slice-collect-refused",
    breaks: "`collect` into a `Box<[T]>` is refused",
    file: "src/lower/mir/iter/collect.rs",
    find: "        } else if self.is_vec_like(output) || output.boxed_ty().is_some_and(|inner| inner.is_slice()) {\n",
    replace: "        } else if self.is_vec_like(output) {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_slice_values.rs$"],
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
      "a `collect` into a B-tree isn't the `Vec`'s it's made of, so a panic that may stop it drops nothing it took (#423, #797)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                Some(iter_ty) => self.collected_as_vec((it, &given), grown, item, iter_ty, !heap, span)?,\n",
    replace: "                Some(_) if !heap => it,\n                Some(iter_ty) => self.collected_as_vec((it, &given), grown, item, iter_ty, !heap, span)?,\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "string-collect-unwinding-unseen",
    breaks:
      "a `collect` into a `String` a panic may stop leaves what's left of its owning source undropped (#423)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            };\n            let items = match self.unwinding_collect(output, item, iter_ty, span)? {",
    replace: "            };\n            let items = match None::<UnwindingCollect> {",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "heap-collect-unwinding-unseen",
    breaks:
      "a `collect` into a `BinaryHeap` isn't the `Vec`'s it's made of, so a panic that may stop it drops nothing it took (#423, #797)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                Some(iter_ty) => self.collected_as_vec((it, &given), grown, item, iter_ty, !heap, span)?,\n",
    replace: "                Some(_) if heap => Expr::collected(it),\n                Some(iter_ty) => self.collected_as_vec((it, &given), grown, item, iter_ty, !heap, span)?,\n",
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
    find: "            None if folded && self.collected_items(&it).is_none() => {\n",
    replace:
      "            None if false && folded && self.collected_items(&it).is_none() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
    snapshots: true,
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
    name: "string-collected-by-next",
    breaks: "a collect into a `String` a panic may stop steps its chain by `next`, not its fold as `String::extend` does, so a nested flatten's source is dropped before the inner iterator it's in (#644); the matrix's `vec | flatten.flatten | string | consumer` shows it, which `--std-iterators` gets wrong still (#644's std cases), so the corpus holds its JS",
    file: "src/lower/mir/iter/collect.rs",
    find: "                Some(unwinding) => unwinding.folded(it),\n",
    replace: "                Some(unwinding) => unwinding.collected(it, None),\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^flatten_closed_part_way_drops_source_first.rs$"],
  },
  {
    name: "in-place-collect-by-next",
    breaks: "a collect std makes in place, of a `filter` whose discarded item's drop panics, steps it by `next`, so it drops what it collected, which std leaks (#681)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            InPlace::Yes => in_place(args),\n",
    replace: "            InPlace::Yes => Expr::call_helper(\"$collectDropping\", args),\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_leaks_what_a_discard_strands.rs$"],
  },
  {
    name: "generic-vec-collect-by-next",
    breaks: "a generic `collect()` into a `Vec` of a `take(n).rev()` of a `Vec`'s steps by `next` though the caller's iterator is `TrustedLen`, where std's `extend_trusted` folds it, dropping 3, 4, 5 for 4, 5, 3 (#670)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            return Ok(Expr::cond(trusted, by_fold, by_next));\n",
    replace: "            return Ok(Expr::cond(Expr::bool(false), by_fold, by_next));\n",
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
    find: "                let by_fold = in_place(args.clone());\n",
    replace: "                let by_fold = Expr::call_helper(\"$collectDropping\", args.clone());\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "collect-trusted-by-next",
    breaks: "a `TrustedLen` collect into a `Vec` of one that folds a held function steps it by `next`, so the function drops after what it left (#710)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            || self.folds_held_function(iter_ty) && self.collects_in_place(Some(iter_ty), item) == Some(false))\n",
    replace: "            || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
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
  {
    name: "in-place-collect-asks-room",
    breaks:
      "std's compiled collect in place of a `filter()` is given `$roomForHint`, which std's in-place collect never asks, so where a dropped item's drop panics it drops what it gathered, which rustc leaks (#770)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                args[0] = unroomed;\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^generic_collect_drops_rest_first with"],
  },
  {
    name: "string-collect-asks-no-hint",
    breaks: "a collect into a `String` of `char`s asks no hint before it steps, as std's `Extend<char>` does (#733)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                true => self.reserved_by_hint(it, iter_ty, crate::lower::iterators::Room::Bytes, None),\n",
    replace: "                true => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_of_adapters.rs$"],
  },
  {
    name: "table-collect-asks-no-hint",
    breaks: "a collect into a `HashSet` asks no hint before it steps, as hashbrown's `extend` does (#733)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                false => self.reserved_by_hint(\n                    it,\n                    iter_ty,\n",
    replace: "                false => self.reserved_by_hint(\n                    it,\n                    None,\n",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_of_adapters.rs$"],
  },
  {
    name: "dictionary-collect-without-own-evidence",
    breaks:
      "a generic collect into its caller's `T: FromIterator<D>` calls the dictionary's `from_iter` with the items alone, so a `Vec`'s iterator's in-place buffer and its drops aren't given (#794)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                args.extend(self.own_evidence(method, generic_args, None, closed, span)?);\n",
    replace: "                let _ = (method, generic_args, closed);\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_impl_of_dropping.rs$"],
    snapshots: true,
  },
  {
    name: "mapped-range-unwinding-unasked",
    breaks:
      "a range's `map` collected into a `Vec` is an array's `map` whatever its items, so where a panic stops it, `(1..=3).map(made).collect()`, what it made isn't dropped: Rust prints `drop 1`, `drop 2`, the JS nothing, a wrong answer (#800)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            Some(items) if self.unwinding_collect(output, item, Some(iter_ty), span)?.is_none() => Some(items),\n",
    replace: "            Some(items) => Some(items),\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_panic_drops_collected.rs$"],
  },
  {
    name: "heap-and-btree-not-grown-as-vec",
    breaks:
      "a collect into a `BinaryHeap`, a `BTreeSet` or a `BTreeMap` isn't the `Vec` std's `from_iter` collects first, so the crate's iterator is never asked its `size_hint`, where native asks at 1, 5 and 9 (#797)",
    file: "src/lower/mir/iter/collect.rs",
    find: "        let made_of_vec = (heap || self.is_sorted(output)) && self.recognition().grows_as_vec(output);\n",
    replace: "        let made_of_vec = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_as_vec_asks_hint.rs$"],
  },
  {
    name: "heap-grown-as-itself",
    breaks:
      "the `Vec` a heap or a B-tree is made of is collected as the heap or the B-tree, so std's in-place collect of it isn't seen: a panic stopping it drops what's left and what it collected in another order than native's (#797)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            let grown = item.map_or(output, |item| self.vec_grown_for(output, item));\n",
    replace: "            let grown = output;\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
    snapshots: true,
  },
  {
    name: "btree-collected-as-array",
    breaks:
      "a B-tree, which takes any iterable, is made of `Array.from(it)` where nothing else is asked of what it collects, an array no one needs (#797)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                None if iterable => it,\n",
    replace: "                None if iterable && false => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_as_vec_asks_hint.rs$"],
    snapshots: true,
  },
  {
    name: "step-by-of-items-asks-room",
    breaks: "a collect of a `step_by(n)` of an array's items asks room of its count, `$roomFor(Math.ceil(items.length / n), ..)`, which no array's length overflows, and its eager filter is made lazy for it (#811)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                || r::stepped_by_source(self.tcx, ty).is_some_and(|source| !of_items(source))\n",
    replace: "                || r::stepped_by_source(self.tcx, ty).is_some()\n",
    tests: ["test/corpus.test.ts", "-t", "^(step_by_zero|sales_report|returned_slice_iters).rs$"],
    snapshots: true,
  },
  {
    name: "take-rev-collect-stepped-not-folded",
    breaks:
      "a `take(n).rev()` of droppers made in place, collected into a `Vec`, is stepped by `next`, whose `nth_back` drops what it passes front to back, where std's `extend_trusted` folds it and `Take::rfold` drops it from the back (#826)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            || folds_by_dropping_nth_back\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^take_from_its_back_drops_as_std.rs$"],
  },
  {
    name: "generic-take-rev-collect-trust-unasked",
    breaks:
      "generic code's `take(n).rev()` of droppers made in place, collected into a `Vec`, never asks its caller's `ITrustedLen`, so it's stepped by `next` where std folds it (#826)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            && (self.compiled_struct(iter_ty, given) || folds_by_dropping_nth_back))\n",
    replace: "            && self.compiled_struct(iter_ty, given))\n",
    tests: ["test/corpus.test.ts", "-t", "^take_from_its_back_drops_as_std.rs$"],
  },
  {
    name: "in-place-collect-capacity-known",
    breaks: "a collect std may make in place, in a crate that reads a capacity, gives its length as its capacity, where std's is its source's (#1180)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            true => Expr::call_helper(\"$untracked\", vec![collected]),\n",
    replace: "            true => collected,\n",
    tests: ["test/lowering-collections.test.ts", "-t", "capacity\\(\\) of a Vec whose growth wasn't kept"],
  },
  {
    name: "from-iter-given-closed-drops-as-its-type",
    breaks: "generic code's `collect()` into a `B: FromIterator` gives `from_iter` the `$closing` it made of the crate's own iterator, and the iterator's own drop for it, which reads fields the closing hasn't: `Cannot read properties of undefined (reading '_loud')` (#1286)",
    file: "src/lower/mir/iter/collect.rs",
    find: "                let closed = crate::lower::iterators::closed_parts(&args[0]).map(|_| iter_ty);\n",
    replace: "                let closed = crate::lower::iterators::closed_parts(&args[0]).and(None);\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_collected_by_generic_code_into_own_from_iterator.rs$"],
  },
  {
    name: "vec-collect-drops-nothing",
    breaks: "`$collectVec` of items that drop is given no drop, so a panic leaves what it collected undropped (#1239)",
    file: "src/lower/mir/iter/collect.rs",
    find: "            let mut args = vec![it, size, how];\n            if self.drops(item) != crate::lower::drops::Drops::Nothing {\n",
    replace: "            let mut args = vec![it, size, how];\n            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collect_as_its_caller_knows.rs$"],
  },
];
