// Mutations of src/lower/mir/iter.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "size-hint-in-place-uncounted",
    breaks:
      "`size_hint()` of a slice's `skip(1)` made in place is a JS iterator helper's `(0, None)`, where Rust gives `(4, Some(4))` (#156)",
    file: "src/lower/mir/iter.rs",
    find: "            && let Some(len) = self.len_in_place(&receiver, iter_ty, made)\n",
    replace: "            && let Some(len) = None::<Expr>\n",
    tests: ["test/corpus.test.ts", "-t", "^size_hints_made_in_place.rs$"],
  },
  {
    name: "next-back-enumeration-refused",
    breaks:
      "`next_back()` of an enumeration of an array's items made in place is refused, not stepped from its back by `$enumerate` (#395)",
    file: "src/lower/mir/iter.rs",
    find: "        if crate::lower::iterators::enumerated_of(&it).is_some() {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_next_back_of_items.rs$"],
  },
  {
    name: "copied-items-mapped-lengthless",
    breaks:
      "a `copied()` that knows its length, of items copied as it steps, is JS's `map` of them, whose `len()` throws (#387)",
    file: "src/lower/mir/iter.rs",
    find: "        let ends = back || self.knows_its_length(output);\n",
    replace: "        let ends = back || false;\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_given_len.rs$"],
  },
  {
    name: "cloned-apart-filter-unknowing",
    breaks:
      "a held `filter()` cloned apart and stepped from its back is JS's `it.filter(f)`, which has no back: `TypeError: it.back is not a function` (ADR 0984)",
    file: "src/lower/mir/iter.rs",
    find: "                if matches!(parted, Parted::Map | Parted::Filter) && back {\n",
    replace: "                if false && matches!(parted, Parted::Map | Parted::Filter) && back {\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_adapters_given.rs$"],
  },
  {
    name: "mir-mut-loop-items-unhandled",
    breaks:
      "`for x in &mut v` of numbers gives numbers, whose `value` writes nothing",
    file: "src/lower/mir/iter.rs",
    find: "        (!handles && items).then(",
    replace: "        (items).then(",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_loop"],
  },
  {
    name: "mir-generic-into-iter-unwrapped",
    breaks:
      "`for x in items` of a generic `IntoIterator` given an array steps the array",
    file: "src/lower/mir/iter.rs",
    find: "            return Some(crate::lower::iterators::given_into_iter(value));\n",
    replace: "            return Some(value);\n",
    tests: ["test/lowering-loops.test.ts", "-t", "stepped by hand"],
  },
  {
    name: "mir-next-back-front",
    breaks: "`it.next_back()` of a `$iter` gives its first item",
    file: "src/lower/mir/iter.rs",
    find: '                (Some(IterMethod::NextBack), None) if back => Expr::call_helper("$nextBack", vec![it]),\n',
    replace:
      '                (Some(IterMethod::NextBack), None) if back => Expr::call_helper("$next", vec![it]),\n',
    tests: ["test/corpus.test.ts", "-t", "iterator_back_ends"],
  },
  {
    name: "mir-size-hint-map-unseen",
    breaks:
      "`v.iter().map(f).size_hint()` is refused, though a `map`'s is its source's",
    file: "src/lower/mir/iter.rs",
    find: '            && method == "map"\n            && args.iter().all(|f| !f.has_effects())\n',
    replace:
      '            && method == "map" && false\n            && args.iter().all(|f| !f.has_effects())\n',
    tests: ["test/corpus.test.ts", "-t", "size_hint"],
  },
  {
    name: "mir-iter-eq-refused",
    breaks: "`a.eq(b)` of two iterators from MIR is refused",
    file: "src/lower/mir/iter.rs",
    find: "            Some(method @ (IterMethod::Eq | IterMethod::Ne)) => {",
    replace:
      "            Some(method @ (IterMethod::Eq | IterMethod::Ne)) if false => {",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "iterators compared equal from MIR",
    ],
  },
  {
    name: "mir-user-len-refused",
    breaks:
      "an iterator of the crate's `len()` from MIR is refused, not its `size_hint()`'s",
    file: "src/lower/mir/iter.rs",
    find: "            if (self.is_user_iterator(iter_ty) || self.compiled_struct(iter_ty, &it))\n                && std_method == Some(IterMethod::Len)\n",
    replace:
      "            if false\n                && std_method == Some(IterMethod::Len)\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "an iterator's len from MIR",
    ],
  },
  {
    name: "mir-next-through-mut-refused",
    breaks:
      "`next` of a `&mut` to the crate's iterator from MIR is refused, not its own",
    file: "src/lower/mir/iter.rs",
    find: "        let through_mut = matches!(receiver_ty.kind(), ty::Ref(_, through, _) if through.is_ref());\n",
    replace: "        let through_mut = false;\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "stepped through by_ref from MIR",
    ],
  },
  {
    name: "mir-find-boxed-unboxed",
    breaks:
      "MIR's `find` of what looks like `None` gives it unboxed, `None` where it found `Some(None)`",
    file: "src/lower/mir/iter.rs",
    find: '                    (None, true) => Expr::call_helper("$nextSome", vec![method("filter", it, vec![test])]),\n',
    replace:
      '                    (None, true) => method("find", it, vec![test]),\n',
    tests: ["test/corpus.test.ts", "-t", "lazy_chains"],
  },
  {
    name: "mir-next-back-map-all",
    breaks: "MIR's `v.iter().map(f).next_back()` is refused",
    file: "src/lower/mir/iter.rs",
    find: "            && crate::lower::iterators::is_items_map(&it)\n",
    replace: "            && false\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "next_back of a map"],
  },
  {
    name: "mir-len-closure-type-shown",
    breaks:
      "MIR's refused `len()` names its closures' types, where they're written",
    file: "src/lower/mir/iter.rs",
    find: "                        ty::Adt(adt, _) => self.tcx.def_path_str(adt.did()),\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "len of a reversed step_by of a map"],
  },
  {
    name: "identity-map-step",
    breaks:
      "a map by an untagged variant constructor stays a step, `.map((value) => value)`",
    file: "src/lower/mir/iter.rs",
    find: "                f if f.is_identity() && dropping.is_none() && !self.recognition().reaches_own_steps(iter_ty) => it,\n",
    replace: "",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "gives the value itself is none",
    ],
  },
  {
    name: "consumed-iterator-undropped",
    breaks:
      "an iterator of the crate's with a destructor, consumed by `collect()`, is never dropped, so what its drop does doesn't happen",
    file: "src/lower/mir/iter.rs",
    find: "            return self.drop_value(held, receiver, span, made);\n",
    replace: "            return Ok(());\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_drop_consumed.rs$"],
  },
  {
    name: "find-dropping-undropped",
    breaks: "`find` of items with a destructor drops none it passes over",
    file: "src/lower/mir/iter.rs",
    find: '                    (Some(item), _) => self.dropping_pick("$findDropping", it, test, item, span)?,\n',
    replace:
      '                    (Some(_), _) => method("find", it, vec![test]),\n',
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
  {
    name: "known-length-as-array",
    breaks:
      "`len()` of a generic iterator that knows its back and its length reads it as an array's, `it.end - it.at`, which a range's has not",
    file: "src/lower/mir/iter.rs",
    find: "                (Some(IterMethod::Len), None) if self.knows_its_length(iter_ty) => {\n",
    replace: "                (Some(IterMethod::Len), None) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^exact_size_len.rs$"],
  },
  {
    name: "from-iter-unlowered",
    breaks: "`Vec::from_iter(items)`, which `items.collect()` is, is refused",
    file: "src/lower/mir/iter.rs",
    find: "            return self.collected_from_iter(given, values, output, span, made);\n",
    replace: "            return Ok(None);\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_called.rs$"],
  },
  {
    name: "try-fold-unlowered",
    breaks: "`try_fold` is refused",
    file: "src/lower/mir/iter.rs",
    find: "            Some(m) if m.tries() => self.try_folded(",
    replace:
      "            Some(IterMethod::TryFold) => return Ok(None),\n            Some(m) if m.tries() => self.try_folded(",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-lent",
    breaks:
      "`try_fold` of a `&mut` is given `$lent(it)`, which its helper, closing nothing, needn't be",
    file: "src/lower/mir/iter.rs",
    find: " && !std_method.is_some_and(IterMethod::tries);\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
    snapshots: true,
  },
  {
    name: "consumed-generic-refused",
    breaks:
      "generic code that consumes an iterator its caller gives a drop of, `items.into_iter().last()`, takes none, and a caller's destructor is refused",
    file: "src/lower/mir/iter.rs",
    find: "                self.has_generic_drop(receiver)\n                    || (compiled && self.holds_given_drops(receiver))\n",
    replace: "                (compiled && self.holds_given_drops(receiver))\n",
    tests: ["test/corpus.test.ts", "-t", "^extreme_drops.rs$"],
  },
  {
    name: "held-function-unclosed",
    breaks:
      "a `map(f)` of a function with a destructor isn't made closing, so its function is never dropped",
    file: "src/lower/mir/iter.rs",
    find: '        let holding = Expr::call_helper("$holding", args);\n',
    replace: "        let holding = args.swap_remove(0);\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(held_function_drops|adapter_functions_dropped).rs$",
    ],
  },
  {
    name: "consumed-held-function-kept",
    breaks:
      "a `map(f)` consumed by `collect` isn't dropped after, nor its function",
    file: "src/lower/mir/iter.rs",
    find: "            .is_some_and(|function| self.drops(function) != crate::lower::drops::Drops::Nothing);\n        // Or an adapter closed",
    replace:
      "            .is_some_and(|function| self.drops(function) != crate::lower::drops::Drops::Nothing && false);\n        // Or an adapter closed",
    tests: ["test/corpus.test.ts", "-t", "^held_function_drops.rs$"],
  },
  {
    name: "consumed-closed-adapter-refused",
    breaks:
      "`iter.take(2).collect()` of a type parameter given its drop is refused, its consumed `Take` not closed",
    file: "src/lower/mir/iter.rs",
    find: "                    || holds_function\n                    || closed\n            }\n",
    replace: "                    || holds_function\n            }\n",
    tests: ["test/corpus.test.ts", "-t", "^cycled_iterators.rs$"],
  },
  {
    name: "cloned-strings-mapped",
    breaks: "`cloned()` of strings maps each to itself",
    file: "src/lower/mir/iter.rs",
    find: "                .is_some_and(|item| !self.needs_clone(item.peel_refs()))\n",
    replace:
      "                .is_some_and(|item| self.is_copy(item.peel_refs()))\n",
    tests: ["test/corpus.test.ts", "-t", "^(copied_structs_apart).rs$"],
  },
  {
    name: "mir-iterator-method-ungated",
    breaks:
      "a method of another trait than std's iterators', `AtomicU32::fetch_add`'s, is lowered as an iterator method's tail",
    file: "src/lower/mir/iter.rs",
    find: "        if std_method.is_none() {\n            return Ok(None);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(atomics|std_odds).rs$"],
  },
  {
    name: "cloned-apart-adapter-native",
    breaks:
      "a `map` the crate clones is made a JS iterator helper alone, which carries no clone",
    file: "src/lower/mir/iter.rs",
    find: "        if values.len() != 2 || !self.iterator_cloned_apart(output) {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "cloned-mapped-anyway",
    breaks: "a `cloned()` of what needs no clone is mapped as one that does",
    file: "src/lower/mir/iter.rs",
    find: "            || crate::lower::recognition::cloned_source(self.tcx, ty).is_some())\n",
    replace: "            )\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(consumed_adapter_drops|cloned_items_from_both_ends).rs$",
    ],
    snapshots: true,
  },
  {
    name: "into-iter-held-as-given-iterable",
    breaks:
      "`into_iter()` of one whose `IntoIter` is held as it is is `$intoIter`, a JS iterator",
    file: "src/lower/mir/iter.rs",
    find: "        if std_method == Some(IterMethod::IntoIter) && !self.holds_into_iter(receiver_ty) {\n",
    replace: "        if std_method == Some(IterMethod::IntoIter) {\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_held_as_it_is.rs$"],
  },
  {
    name: "compiled-iterator-not-dropped-after",
    breaks:
      "a generic `Peekable` consumed, `p.sum()`'s, isn't dropped after by the drops it's given, so what it holds with a destructor is never dropped",
    file: "src/lower/mir/iter.rs",
    find: "                    || (compiled && self.holds_given_drops(receiver))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^peekable_given_back_drops.rs$"],
  },
  {
    name: "compiled-iterator-of-destructors-not-dropped-after",
    breaks:
      "a `Peekable` of a type with a destructor, consumed, isn't dropped after its method",
    file: "src/lower/mir/iter.rs",
    find: "            false => self.is_user_iterator(receiver) || compiled || holds_function,\n",
    replace:
      "            false => self.is_user_iterator(receiver) || holds_function,\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_given_back_drops.rs$"],
    snapshots: true,
  },
  {
    name: "size-hint-undictionaried",
    breaks:
      "`I::size_hint` in a compiled std body isn't given `I`'s dictionary, so a `Fuse` of the crate's iterator says `(0, None)`",
    file: "src/lower/mir/iter.rs",
    find: "            args.extend(self.iterator_evidence(iter_ty.peel_refs(), iterator));\n",
    replace: "            let _ = iterator;\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std"],
  },
  {
    name: "copied-items-shared",
    breaks:
      "`copied()` is the iterator it copies, its `Copy` structs the same objects, so changing a copy changes the slice's",
    file: "src/lower/mir/iter.rs",
    find: "            || crate::lower::recognition::cloned_source(self.tcx, ty).is_some())\n            && item()",
    replace:
      "            || crate::lower::recognition::cloned_source(self.tcx, ty).is_some())\n            && (crate::lower::recognition::copied_source(self.tcx, ty).is_some() || item())",
    tests: ["test/corpus.test.ts", "-t", "^copied_structs_apart.rs$"],
  },
  {
    name: "utf16-count-reduced",
    breaks:
      "`s.encode_utf16().count()` counts each unit, `Iterator.from($encodeUtf16(s)).reduce((n) => n + 1, 0)`, not `s.length`",
    file: "src/lower/mir/iter.rs",
    find: "            && let Some(length) = crate::lower::std_types::text::utf16_length(&it)\n",
    replace:
      "            && let Some(length) = crate::lower::std_types::text::utf16_length(&it).filter(|_| false)\n",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a string's count of UTF-16 units is its length",
    ],
  },
  {
    name: "own-fold-consumer-dropped-again",
    breaks:
      "an iterator the crate's own fold took and dropped is dropped again after its consumer",
    file: "src/lower/mir/iter.rs",
    find: "            return Ok(false);\n        }\n        let params_only",
    replace: "        }\n        let params_only",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "given-own-fold-dropped-again",
    breaks:
      "a value given as it is, which its dictionary's own fold took and dropped, is dropped again",
    file: "src/lower/mir/iter.rs",
    find: "            made.push(StmtKind::If(unfolded, dropped, None).at(self.js_span(span)));\n",
    replace:
      "            let _ = unfolded;\n            made.extend(dropped);\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "identity-map-dropped-over-own-steps",
    breaks:
      "an identity `map` over the crate's iterator is dropped, so a `skip` and `last()` past it reach its own `nth` and `last`, which std's `Map` hides (#235)",
    file: "src/lower/mir/iter.rs",
    find: "                f if f.is_identity() && dropping.is_none() && !self.recognition().reaches_own_steps(iter_ty) => it,\n",
    replace: "                f if f.is_identity() && dropping.is_none() => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_map_hides_own_nth.rs$"],
  },
  {
    name: "copied-pointee-peeled",
    breaks:
      "`copied()` of an iterator of references to references copies what the inner one points at, `{ ...item }`, a set of `&Fiber` keys of copies",
    file: "src/lower/mir/iter.rs",
    find: "        let pointee = match *item.kind() {\n            ty::Ref(_, pointee, _) => pointee,\n            _ => item,\n        };\n",
    replace: "        let pointee = item.peel_refs();\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_keys.rs$"],
  },
  {
    name: "in-place-rev-a-struct",
    breaks:
      "a `Rev` stepped where it's made is taken as std's struct, closed after it's consumed",
    file: "src/lower/mir/iter.rs",
    find: "                && crate::lower::iterators::made_in_place(value))\n",
    replace: "                && false)\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^own_iterator_dropped_through_adapters.rs$",
    ],
  },
  {
    name: "in-place-map-len-struct",
    breaks:
      "`len()` of a `map()` made in place, the recognizers' JS iterator, is std's struct's hint, `$exactLen($sizeHint(..))`, not its own `len()`",
    file: "src/lower/mir/iter.rs",
    find: "            if (self.is_user_iterator(iter_ty) || self.compiled_struct(iter_ty, &it))\n",
    replace:
      "            if (self.is_user_iterator(iter_ty) || crate::lower::recognition::is_compiled_std_iterator(self.tcx, iter_ty))\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(map_stepped_from_its_back|exact_size_len).rs$",
    ],
  },

  {
    name: "param-count-reduced",
    breaks:
      "std's `Skip::count` counts its `Take` by folding it, not its own `count`, so a reversed `Vec`'s items are dropped back to front, not as one slice (ADR 0913)",
    file: "src/lower/mir/iter.rs",
    find: "        let own_count = matches!(iter_ty.kind(), ty::Param(_))\n",
    replace:
      "        let own_count = false && matches!(iter_ty.kind(), ty::Param(_))\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "consumed-unclosed-on-throw",
    breaks:
      "an iterator a std method took by value, dropped after it, is left undropped where the method throws, a panic, `fuse().count()`'s inner (#424)",
    file: "src/lower/mir/iter.rs",
    find: "            true => self.unwinding(call, dropped.clone(), self.js_span(span), made),\n",
    replace: "            true => made.extend(call),\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std",
    ],
  },
  {
    name: "consumed-unwinding-everywhere",
    breaks:
      "an iterator a std method took by value is dropped in a `finally` though nothing it steps may panic, `Array.from` of a `filter` that can't (#424)",
    file: "src/lower/mir/iter.rs",
    find: "        match receiver_ty.has_param() || self.steps_may_panic(receiver_ty) || given_may_panic {\n",
    replace: "        match true {\n",
    tests: ["test/corpus.test.ts", "-t", "^drops_of_consumed_iterators.rs$"],
    snapshots: true,
  },
  {
    name: "rfold-consumed-undropped",
    breaks:
      "what `rfold` takes by value, std's `Map::rfold`'s `IntoIter`, isn't dropped after it, nor as a stage's panic unwinds: what it hadn't stepped is left undropped (#423)",
    file: "src/lower/mir/iter.rs",
    find: "            && (is_std_def(self.tcx, trait_id, StdItem::Iterator) || self.recognition().rfolds_what_it_consumes(def_id))\n",
    replace:
      "            && is_std_def(self.tcx, trait_id, StdItem::Iterator)\n",
    tests: [
      "test/std-package.test.ts",
      "test/iterator-forms.test.ts",
      "-t",
      "the committed @rust-js/std is std|^generic_folds_unwind with",
    ],
  },
  {
    name: "rfold-consumed-by-fold-asked",
    breaks:
      "what `rfold` takes by value is dropped after it unless its `Iterator` dictionary's `fold` is there, where its `DoubleEndedIterator`'s `rfold` is what drops it",
    file: "src/lower/mir/iter.rs",
    find: '            true => (self.recognition().double_ended_iterator(), "rfold"),\n',
    replace:
      '            true => (Some(std_item(self.tcx, StdItem::Iterator)), "fold"),\n',
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std",
    ],
  },
  {
    name: "identity-map-of-destructors-dropped",
    breaks: "an identity `map` of items with a destructor is dropped, so `rev().fuse().take(3).map(|d| d).nth(2)` reaches the reversed `Vec`'s `nth_back`, one slice front to back, where std's `Map` steps each from the back (#493)",
    file: "src/lower/mir/iter.rs",
    find: "                f if f.is_identity() && dropping.is_none() && !self.recognition().reaches_own_steps(iter_ty) => it,\n",
    replace: "                f if f.is_identity() && !self.recognition().reaches_own_steps(iter_ty) => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_order_through_take_adapters.rs$"],
  },
  {
    name: "own-count-dropped-again",
    breaks: "std's compiled `Fuse::count` of the crate's iterator with its own `count` drops what it holds after the crate's `count` did, so @rust-js/std isn't what the compiler compiles (#505)",
    file: "src/lower/mir/iter.rs",
    find: "            Some(IterMethod::Count) => Some(\"count\"),\n",
    replace: "",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "own-last-dropped-again",
    breaks: "std's compiled `Skip::last` of the crate's iterator with its own `last` drops its source after the crate's `last` did, twice (#522)",
    file: "src/lower/mir/iter.rs",
    find: "            Some(IterMethod::Last) => Some(\"last\"),\n",
    replace: "",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "last-output-holds-its-iterator",
    breaks: "`last()`'s `Option<I::Item>` is taken to hold the `I` it consumed, so std's compiled `Skip::last` never drops its source and its caller's close drops it after the crate's own `last` did (#522)",
    file: "src/lower/mir/iter.rs",
    find: "        let holds = crate::lower::mut_refs::holds_outside_projections(output, receiver);\n",
    replace: "        let holds = output.walk().any(|part| part.as_type() == Some(receiver));\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "generic-own-steps-unasked",
    breaks: "generic code's `last()`, `sum()` and `for_each` of a value given no dictionary drop it after, though the crate's own `fold` or `last` took it and dropped it (#522)",
    file: "src/lower/mir/iter.rs",
    find: "                None if matches!(receiver.kind(), ty::Param(_)) => {\n",
    replace: "                None if false && matches!(receiver.kind(), ty::Param(_)) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^last_takes_its_source.rs$"],
  },
  {
    name: "consumed-closed-dropped-again",
    breaks: "an iterator its consumer closes, the crate's with a `Drop` over items with one, is dropped after its `fold`, `for_each`, `count` or `last` too, so twice (#637)",
    file: "src/lower/mir/iter.rs",
    find: "            && (self.consumer_closes(method, receiver, false, dropping) || self.keyed_consumer_closes(method, arg_tys))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^consumed_own_iterator_dropped_once.rs$"],
  },
  {
    name: "dictionary-step-took-unasked",
    breaks:
      "whether a dictionary's `fold` took the iterator is whether it has one, not the value's answer: generic code's `sum()` of a held `map()` of what it was given, whose dictionary's `fold` is `$foldOf`, the compiled `Map`'s that took it, drops it again after: the crate's iterator with a `Drop` is dropped twice (#621, #649)",
    file: "src/lower/mir/iter.rs",
    find: '                        Expr::unary(js::UnaryOp::Not, Expr::call_helper("$took", vec![step, held.clone()]))\n',
    replace: "                        Expr::bin(Op::Eq, step, Expr::undefined())\n",
    tests: ["test/corpus.test.ts", "-t", "^own_steps_a_type_parameter_may_reach.rs$"],
  },
  {
    name: "holding-unmoved",
    breaks: "an adapter holding a function is made with no `moved`, so a fold drops its function after what it steps (#710)",
    file: "src/lower/mir/iter.rs",
    find: "        args.extend(moved.map(Expr::str));\n",
    replace: "        let _ = moved;\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
  },
  {
    name: "consumed-collect-closing-any",
    breaks: "a collect that closes what it steps drops nothing after of an iterator with a destructor of its own, the crate's, which closing it doesn't drop, so its `Drop` never runs (#660)",
    file: "src/lower/mir/iter.rs",
    find: "        if result.facts.has(js::Known::ClosesWhatItSteps) && self.dropped_by_closing(receiver_ty) {\n",
    replace: "        if result.facts.has(js::Known::ClosesWhatItSteps) {\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_drop_consumed.rs$"],
  },
  {
    name: "rev-in-place-compiled",
    breaks:
      "a `rev()` stepped where it's made is std's compiled `rev()`, not the recognizers'",
    file: "src/lower/mir/iter.rs",
    find: "        crate::lower::recognition::makes_held_compiled(self.tcx, def_id)\n            && self.frame.facts.in_place_adapters.contains(&span)",
    replace:
      "        crate::lower::recognition::makes_held_compiled(self.tcx, def_id)\n            && self.frame.facts.in_place_adapters.contains(&span)\n            && false",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^rev_drops.rs$"],
  },
  {
    name: "flatten-count-by-reduce",
    breaks: "a flatten made in place is counted by `reduce`, never by `$count`, so the inner iterators' own `count` that `$flatMap` asks is never reached (#736)",
    file: "src/lower/mir/iter.rs",
    find: '            || self.compiled_step(iter_ty, iterator, "count") && (!in_place_flatten || inners_own);\n',
    replace: '            || self.compiled_step(iter_ty, iterator, "count") && !in_place_flatten;\n',
    tests: ["test/corpus.test.ts", "-t", "^flatten_count_takes_each_inner_count.rs$"],
  },
  {
    name: "flatten-count-always-own",
    breaks: "every flatten made in place is counted by `$count`, where JS's `flatMap` of inner iterators with no own `count` reads as `reduce` (#736)",
    file: "src/lower/mir/iter.rs",
    find: '            || self.compiled_step(iter_ty, iterator, "count") && (!in_place_flatten || inners_own);\n',
    replace: '            || self.compiled_step(iter_ty, iterator, "count");\n',
    tests: ["test/corpus.test.ts", "-t", "^flatten_count_takes_each_inner_count.rs$"],
    snapshots: true,
  },
  {
    name: "len-of-mut-items-left",
    breaks:
      "`len()` of a `by_ref()` of a `Vec`'s `IntoIter` reads `it.end - it.at` of the lent value, which has neither: `NaN` (#745)",
    file: "src/lower/mir/iter.rs",
    find: "                (Some(IterMethod::Len), None) if lends_self => Expr::call(Expr::member(it, \"len\"), Vec::new()),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^by_ref_lends.rs$|^by_ref_lends with"],
  },
  {
    name: "hint-of-mut-own-refused",
    breaks:
      "`size_hint()` of a `by_ref()` of the crate's iterator, `&mut I`'s, which passes on `I`'s own, is refused (#745)",
    file: "src/lower/mir/iter.rs",
    find: "        let stepped = matches!(std_method, Some(IterMethod::Next | IterMethod::SizeHint));\n",
    replace: "        let stepped = matches!(std_method, Some(IterMethod::Next));\n",
    tests: ["test/corpus.test.ts", "-t", "^by_ref_lends.rs$"],
  },
  {
    name: "steps-in-place-never",
    breaks:
      "no call steps an adapter made in place, its receiver's mark unread, so `v.iter().map(f).rfold(..)` steps std's compiled `Map`, whose back the recognizers' JS `.map()` hasn't (ADR 0871)",
    file: "src/lower/mir/iter.rs",
    find: "            && receiver.is_some_and(crate::lower::iterators::made_in_place)\n",
    replace: "            && receiver.is_some_and(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^(rev_in_place_through_adapters|flat_map_of_maps_from_its_back|range_own_steps).rs$"],
    snapshots: true,
  },
  {
    name: "adapter-hint-refused",
    breaks: "`size_hint()` of std's adapter of the crate's iterator, `own.filter(f)`'s, is refused where its JS has std's (#733)",
    file: "src/lower/mir/iter.rs",
    find: "            false => own || self.array_source(iter_ty).is_some() || self.adapted_hint_known(iter_ty),\n",
    replace: "            false => own || self.array_source(iter_ty).is_some(),\n",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_of_adapters.rs$"],
  },
  {
    name: "mapped-adapter-hint-refused",
    breaks: "`size_hint()` of an `enumerate()` of the crate's iterator, made a `map`, is refused (#733)",
    file: "src/lower/mir/iter.rs",
    find: "                    || source_ty.is_none() && self.adapted_hint_known(iter_ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_of_adapters.rs$"],
  },
  {
    name: "chars-hint-counted",
    breaks: "`size_hint()` of a map of a string's `chars()` is how many it has, `(5, Some(5))`, where Rust's is of its bytes, `(2, Some(6))` (#733)",
    file: "src/lower/mir/iter.rs",
    find: "        if of_text && !own && !self.struct_hint_known(iter_ty) {\n",
    replace: "        if false {\n",
    tests: ["test/diagnostics.test.ts", "-t", "size_hint of a map of a string's chars"],
  },
  {
    name: "chars-struct-hint-refused",
    breaks: "`size_hint()` of std's compiled flatten of `chars()`, held, is refused, though its struct's hint is std's",
    file: "src/lower/mir/iter.rs",
    find: "        if of_text && !own && !self.struct_hint_known(iter_ty) {\n",
    replace: "        if of_text && !own {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_flatten.rs$"],
  },
  {
    name: "filter-count-asks-no-hint",
    breaks: "a `filter()`'s `count()` of the crate's iterator asks its source's hint not first, as std's `Filter::count` does (#733)",
    file: "src/lower/mir/iter.rs",
    find: "            || filter_asks\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_of_adapters.rs$"],
  },
  {
    name: "mapped-hint-refused",
    breaks: "`size_hint()` of a map of the crate's iterator, or of what's given as it is, `I: Clone`'s, is refused (#385, #733)",
    file: "src/lower/mir/iter.rs",
    find: "                    || source_ty.is_some_and(|ty| self.adapted_hint_known(ty))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^lengths_and_backs_given_on.rs$|^adapter_size_hints.rs$"],
  },
  {
    name: "keyed-extreme-dropped-after",
    breaks: "`max_by_key` of a key that drops over the crate's iterator, given it closed, drops it after the call too, twice (#789)",
    file: "src/lower/mir/iter.rs",
    find: "            && (self.consumer_closes(method, receiver, false, dropping) || self.keyed_consumer_closes(method, arg_tys))\n",
    replace: "            && self.consumer_closes(method, receiver, false, dropping)\n",
    tests: ["test/corpus.test.ts", "-t", "^max_by_key_of_a_dropping_key_over_its_iterator.rs$"],
  },
  {
    name: "generic-own-iterator-into-iter-refused",
    breaks:
      "generic code's `for _ in w` over the crate's own generic iterator struct, a `Wrapper<I>` whose items are `I::Item`, isn't known to be its own iterator, so it's refused, \"calling `IntoIterator::into_iter`\" (#806)",
    file: "src/lower/mir/iter.rs",
    find: "        if self.range_kind(iter_ty).is_some() || self.recognition().iterates_as_itself(receiver_ty) {\n",
    replace: "        if self.range_kind(iter_ty).is_some() || self.implements_iterator(receiver_ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_for_over_own_iterator.rs$"],
  },
  {
    name: "next-back-of-marked-unread",
    breaks: "`next_back` of a variable made knowing its back where it's held reads its type alone, which doesn't, and is refused (#718)",
    file: "src/lower/mir/iter.rs",
    find: "self.has_a_back(iter_ty, &it) || std_method == Some(IterMethod::NextBack) && knows_back(&it);",
    replace: "self.has_a_back(iter_ty, &it);",
    tests: ["test/corpus.test.ts", "-t", "^skip_and_take_stepped_from_their_back_in_place.rs$"],
  },
  {
    name: "in-place-given-unmarked",
    breaks:
      "an adapter made in place isn't marked so by its maker, so a variable's `rev()` made in place, stepped from its back past a `skip`, is given `Rev`'s compiled back, not refused (#730)",
    file: "src/lower/mir/iter.rs",
    find: "            true => made.in_place(),\n            false => made,\n",
    replace: "            true => made,\n            false => made,\n",
    tests: ["test/diagnostics.test.ts", "-t", "held rev of the crate"],
  },
  {
    name: "copied-items-mapped-backless",
    breaks: "a `cloned()` of a chain the crate steps from its back, which knows no length, is JS's `map` of its clones, which has no back, and the back step throws (#850)",
    file: "src/lower/mir/iter.rs",
    find: "        let back = self.knows_its_back(output);\n",
    replace: "        let back = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_of_a_chain_stepped_from_its_back.rs$"],
  },
  {
    name: "slice-count-folded",
    breaks: "a `Vec`'s `IntoIter`'s `count` of items with a destructor is a `reduce` dropping each as it's counted, where std's is its `len()`, then it dropped as one slice, so a second panic drops one more before it aborts (a wrong answer, #862)",
    file: "src/lower/mir/iter.rs",
    find: "            || slice_count\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_count_second_panic_aborts.rs$"],
  },
  {
    name: "chain-other-side-not-made-an-iterator",
    breaks: "a chain cloned apart whose other side is given as what makes an iterator, `chain(&b)`, is refused, where its iterator is taken where the chain is made (#1002)",
    file: "src/lower/mir/iter.rs",
    find: "                && self.implements_iterator(into) =>\n",
    replace: "                && self.implements_iterator(into)\n                && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_of_a_slice_cloned_apart.rs$"],
  },
  {
    name: "copied-source-unknowing",
    breaks: "a `cloned()` or a `copied()` made knowing its back is `$mapEnds` of its source as it is, a `step_by(2)`'s forward-only `values().filter(..)`, whose `rfold` throws `it.back is not a function` (#851)",
    file: "src/lower/mir/iter.rs",
    find: "            Some(source) if back => self.as_double_ended(it, source.peel_refs(), span)?,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^copies_of_forward_forms_stepped_from_their_back.rs$"],
  },
  {
    name: "holding-loses-its-length-mark",
    breaks: "`$holding(adapter, drop)` of a map made knowing its length doesn't keep its mark, so a range's map whose closure drops, held, read by index by a zip, is refused as an `ExactSizeIterator` (#851)",
    file: "src/lower/mir/iter.rs",
    find: "        Ok(match knows_length {\n",
    replace: "        Ok(match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_by_index_of_holding_maps.rs$"],
  },
  {
    name: "cloned-chain-sides-backless",
    breaks: "a chain cloned apart stepped from its back makes its sides without their backs, a range's `$lazyRange`, which `$lazyChain` can't step from its back (#1002)",
    file: "src/lower/mir/iter.rs",
    find: "                    Parted::Chain if back => {\n",
    replace: "                    Parted::Chain if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_chain_stepped_from_its_back.rs$"],
  },

  {
    name: "consumed-given-function-panic-unguarded",
    breaks: "an iterator with a destructor a std method takes by value isn't dropped where the function it's given panics, `for_each(f)`'s, as the method's unwinding drops it (#1079)",
    file: "src/lower/mir/iter.rs",
    find: "        match receiver_ty.has_param() || self.steps_may_panic(receiver_ty) || given_may_panic {\n",
    replace: "        match receiver_ty.has_param() || self.steps_may_panic(receiver_ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^consumed_iterator_dropped_as_its_function_panics.rs$"],
  },
  {
    name: "consumed-unit-result-held",
    breaks: "a `()` a std method gives, `for_each`'s, of an iterator dropped after it is held, `let result; try { result = it.forEach(f) }`, a name read by nothing (#1079)",
    file: "src/lower/mir/iter.rs",
    find: "        let result = match output.is_unit() {\n",
    replace: "        let result = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^consumed_iterator_dropped_as_its_function_panics.rs$"],
    snapshots: true,
  },
  {
    name: "from-iter-iterator-undropped",
    breaks:
      "`Vec::from_iter(it)` of an iterator with a destructor never drops it, where `it.collect()` drops it as it's done (#838)",
    file: "src/lower/mir/iter.rs",
    find: "            return self.mir_iter_call(collect, &[given], values, output, span, made);\n",
    replace:
      "            return self.mir_iter_method(collect, (std_item(tcx, StdItem::Iterator), &[given]), values, output, span, made);\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_of_the_crates_iterator.rs$"],
  },
  {
    name: "from-iter-of-anything",
    breaks:
      "`Vec::from_iter` of an `Option` is collected as if it were an iterator",
    file: "src/lower/mir/iter.rs",
    find: "        if self.iterator_item(given).is_some() {\n",
    replace: "        if true {\n",
    tests: ["test/diagnostics.test.ts", "-t", "from_iter of an Option"],
  },
  {
    name: "from-iter-stepped-as-given",
    breaks:
      "`Vec::from_iter(iter)` of a type parameter's `IntoIterator` takes its iterator as `I` itself, not `I::IntoIter`, which has no items and no in-place facts of its own: a panic stopping it drops nothing, or it's refused (#800)",
    file: "src/lower/mir/iter.rs",
    find: "        let stepped = crate::lower::recognition::iterator_type_of(tcx, self.typing_env, given);\n",
    replace: "        let stepped = given;\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_panic_drops_collected.rs$"],
  },
  {
    name: "extend-taken-to-close",
    breaks: "`v.extend(it)` of an iterator with a `Drop` of items with one is taken to close it, which no `extend` does, so it's never dropped (#1120)",
    file: "src/lower/mir/iter.rs",
    find: "        if !extends\n",
    replace: "        if true\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_consumed_by_extend_unzip_partition_eq.rs$"],
  },
  {
    name: "std-consumer-undropped",
    breaks: "an iterator with a destructor a std consumer takes by value, `m.extend(it)`'s of a map or `it.partition(f)`'s, isn't dropped after it: never dropped, or refused (#1120, #1122)",
    file: "src/lower/mir/iter.rs",
    find: "        let Some(at) = self.consumed_at(def_id, arg_tys) else {\n",
    replace: "        let Some(at) = self.consumed_at(def_id, arg_tys).filter(|_| false) else {\n",
    tests: ["test/corpus.test.ts", "-t", "^(iterator_consumed_by_extend_unzip_partition_eq|held_skip_take).rs$"],
    snapshots: true,
  },
  {
    name: "source-own-count-unasked",
    breaks: "`repeat_n(x, n).count()` is counted by `reduce`, cloning each, not by its own `count`, its length (#1124)",
    file: "src/lower/mir/iter.rs",
    find: "        let source_own = self.recognition().source_writes(iter_ty, OwnStep::Count);\n",
    replace: "        let source_own = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_own_steps_clone_none.rs$"],
  },
  {
    name: "chars-count-reduced",
    breaks: "a count of a string's `chars()` made here reads `s[Symbol.iterator]().reduce((n) => n + 1, 0)`, not `Array.from(s).length`",
    file: "src/lower/mir/iter.rs",
    find: "        if body.is_empty()\n            && let Some(s) =\n",
    replace: "        if false\n            && let Some(s) =\n",
    tests: ["test/lowering-strings.test.ts", "-t", "a string's count of UTF-16 units is its length"],
  },
  {
    name: "chars-own-hint-refused",
    breaks: "`size_hint()` of a `chars()` itself, `$chars`' std's, is refused again",
    file: "src/lower/mir/iter.rs",
    find: "            text.is_str_chars(ty) || text.is_char_indices(ty)\n        };\n",
    replace: "            false && (text.is_str_chars(ty) || text.is_char_indices(ty))\n        };\n",
    tests: ["test/corpus.test.ts", "-t", "^chars_size_hint.rs$"],
  },
  {
    name: "in-place-adapter-hint-unasked",
    breaks: "`take(2)` of a type parameter's iterator, made in generic code where its hint is asked, is JS's own adapter, whose hint is `(0, None)`, where std's is `(2, Some(2))` (#1236)",
    file: "src/lower/mir/iter.rs",
    find: "        if let Some(hint) = self.in_place_adapter_hint(&it, source_ty) {\n",
    replace: "        if let Some(hint) = None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "^take_and_filter_of_a_type_parameter_hint_as_std\\.rs$"],
  },
  {
    name: "boxed-function-panic-unseen",
    breaks: "a by-value consumer given a boxed function, `Box<dyn FnMut>`, that may panic isn't taken to unwind through its call, so the iterator it took isn't dropped as it unwinds (#1232)",
    file: "src/lower/mir/iter.rs",
    find: "            let ty = ty.boxed_ty().unwrap_or(ty);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^boxed_function_panics_drop_the_consumed_iterator.rs$"],
  },
];
