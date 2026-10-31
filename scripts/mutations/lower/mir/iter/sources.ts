// Mutations of src/lower/mir/iter/sources.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "step-by-count-unfolded",
    breaks:
      "a `step_by` of numbers known, `(0..10).step_by(3)` collected, asks room for `Math.ceil(10 / 3)`, not the 4 that surely fits",
    file: "src/lower/mir/iter/sources.rs",
    find: "            if let (Some(count), Some(step)) = (count.as_int(), step.as_int())\n",
    replace:
      "            if let (Some(count), Some(step), false) = (count.as_int(), step.as_int(), true)\n",
    tests: ["test/corpus.test.ts", "-t", "^step_by_zero\\.rs$"],
    snapshots: true,
  },
  {
    name: "take-of-endless-uncounted",
    breaks:
      "`repeat(x).take(n)` made in place has no count, as `repeat` has none, so its `collect()` asks no room (#246)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            if !skip && self.recognition().endless_by_type(source) {\n",
    replace:
      "            if false && !skip && self.recognition().endless_by_type(source) {\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "step-by-uncounted",
    breaks:
      "a range's `step_by(s)` made in place has no count: its `len()` is refused and its `collect()` asks no room (#246)",
    file: "src/lower/mir/iter/sources.rs",
    find: '            && callee.helper_name() == Some("$stepBy")\n',
    replace: '            && callee.helper_name() == Some("$stepByNot")\n',
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "repeat-n-uncounted",
    breaks:
      "`repeat_n(x, n)` made in place has no count, so its `collect()` asks no room: `capacity overflow` unasked (#246)",
    file: "src/lower/mir/iter/sources.rs",
    find: '            && callee.helper_name() == Some("$repeatingN")\n',
    replace: '            && callee.helper_name() == Some("$repeatingNot")\n',
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "given-copied-lengthless",
    breaks:
      "a `copied()` of an iterator generic code is given as it is, bound by `ExactSizeIterator`, is the JS iterator `$given` makes without its `len`, whose `len()` throws (#387)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            && let Some(it) = self.given_js_iterator(value.clone(), ty, false, true)\n",
    replace:
      "            && let Some(it) = self.given_js_iterator(value.clone(), ty, false, false)\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_given_len.rs$"],
  },
  {
    name: "len-in-place-map-unread",
    breaks:
      "`len()` of a `map` made in place isn't its source's: `v.iter().map(f).len()` is refused (ADR 0986)",
    file: "src/lower/mir/iter/sources.rs",
    find: '            && method == "map"\n            && let [f] = args.as_slice()\n            && !f.has_effects()\n',
    replace:
      '            && method == "mapped"\n            && let [f] = args.as_slice()\n            && !f.has_effects()\n',
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-take-unbounded",
    breaks:
      "`len()` of a `take(n)` made in place is more than `n`: `v.iter().take(2).map(f).len()` is 5, not 2 (ADR 0986)",
    file: "src/lower/mir/iter/sources.rs",
    find: '                false => math("min", vec![len, n.clone()]),\n',
    replace: '                false => math("max", vec![len, n.clone()]),\n',
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-skip-unsaturated",
    breaks:
      "`len()` of a `skip(n)` past its end, made in place, is below zero: `-4`, not `0` (ADR 0986)",
    file: "src/lower/mir/iter/sources.rs",
    find: '                true => math("max", vec![Expr::bin(Op::Sub, len, n.clone()), Expr::int(0)]),\n',
    replace: "                true => Expr::bin(Op::Sub, len, n.clone()),\n",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-rev-unread",
    breaks:
      "`len()` of a `rev()` made in place isn't its source's: `(1..4).rev().len()` is refused (ADR 0986)",
    file: "src/lower/mir/iter/sources.rs",
    find: "        if let Some(source) = r::reversed_source(self.tcx, ty) {\n            let source_value = match",
    replace:
      "        if let Some(source) = r::reversed_source(self.tcx, ty).filter(|_| false) {\n            let source_value = match",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-enumerate-unread",
    breaks:
      "`len()` of an `enumerate()` made in place isn't its source's: `v.iter().enumerate().len()` is refused (ADR 0986)",
    file: "src/lower/mir/iter/sources.rs",
    find: "        if let Some(source) = r::enumerated_source(self.tcx, ty) {\n",
    replace:
      "        if let Some(source) = r::enumerated_source(self.tcx, ty).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-generic-unread",
    breaks:
      "`len()` of a `take(2)` of a generic `ExactSizeIterator` is refused (ADR 0986)",
    file: "src/lower/mir/iter/sources.rs",
    find: "        if (self.is_unknown(ty) || r::dyn_iter_trait(self.tcx, ty).is_some()) && self.knows_its_length(ty) {\n",
    replace:
      "        if false && (self.is_unknown(ty) || r::dyn_iter_trait(self.tcx, ty).is_some()) && self.knows_its_length(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-generic-wrapped",
    breaks:
      "`len()` of a generic one is read through `Iterator.from(it)`, which wraps it, not of `it` (ADR 0986)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            let given = crate::lower::iterators::iterated(it.clone());\n",
    replace: "            let given = it.clone();\n",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "mir-len-whole",
    breaks: "`it.len()` of a stepped `$iter` counts what it's given already",
    file: "src/lower/mir/iter/sources.rs",
    find: '        Expr::bin(Op::Sub, Expr::member(it.clone(), "end"), Expr::member(it, "at"))\n',
    replace: '        Expr::member(Expr::member(it, "items"), "length")\n',
    tests: ["test/corpus.test.ts", "-t", "iterator_back_ends"],
  },
  {
    name: "mir-made-array-iterator-from",
    breaks:
      "a `split`'s array MIR steps is `Iterator.from(…)`, so its chain stays a JS iterator's",
    file: "src/lower/mir/iter/sources.rs",
    find: "        } else if made_array(&value) || self.steps_an_array(ty) {\n",
    replace: "        } else if self.steps_an_array(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(boxed_array_iter|calculator).rs$"],
    snapshots: true,
  },
  {
    name: "deque-iter-generic",
    breaks:
      "a `VecDeque`'s iterator from MIR is `Iterator.from(prev)`, collected `Array.from(..)`, not `prev.slice()`, and its `next_back()` refused",
    file: "src/lower/mir/iter/sources.rs",
    find: " || self.recognition().is_deque_iter(ty)",
    replace: "",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a VecDeque's items collected from MIR",
    ],
  },
  {
    name: "mir-crate-fn-item-callback-wrapped",
    breaks:
      "a crate function given as a callback from MIR is `(item) => is_rule(item)`, not `is_rule`",
    file: "src/lower/mir/iter/sources.rs",
    find: "            js::ExprKind::Arrow(params, _) if params.len() <= 1 => f,\n            _ if own => f,\n",
    replace:
      "            js::ExprKind::Arrow(params, _) if params.len() <= 1 => f,\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a crate function given as a callback is passed as itself",
    ],
  },
  {
    name: "mir-range-back-forward",
    breaks: "`(0..n).rev().collect()` from MIR is `$range(0, n)`, forwards",
    file: "src/lower/mir/iter/sources.rs",
    find: "        if range.back {\n",
    replace: "        if !range.back {\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "a range mapped and collected",
    ],
  },
  {
    name: "mir-range-unmapped-lazy",
    breaks:
      "a range collected unmapped from MIR stays a lazy `Array.from($iterator(\u2026))`",
    file: "src/lower/mir/iter/sources.rs",
    find: "        if !numbers || maps",
    replace: "        if maps.is_empty() || !numbers || maps",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "a range mapped and collected",
    ],
  },
  {
    name: "mir-chars-next-back-refused",
    breaks:
      "`next_back()` of a string's `chars()` from MIR is refused as a lazy iterator's",
    file: "src/lower/mir/iter/sources.rs",
    find: "            || self.recognition().is_str_chars(ty)\n",
    replace: "",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "the rest of a string's chars from MIR",
    ],
  },
  {
    name: "mir-iter-mut-a-js-iterator",
    breaks:
      "a slice's `iter_mut()` from MIR is `Iterator.from(v)`, its chain lazy, `$lent(Iterator.from(v)).find(f)`, not `v.find(f)`",
    file: "src/lower/mir/iter/sources.rs",
    find: "            || self.recognition().is_slice_iter_mut(ty)\n",
    replace: "",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "iter_mut chains from MIR",
    ],
  },
  {
    name: "mir-mapped-range-filtered",
    breaks:
      "`(0..n).filter(f).collect()` from MIR is `$range(0, n).filter(f)`, an array of all n numbers made to keep few",
    file: "src/lower/mir/iter/sources.rs",
    find: '            && name == "map"\n',
    replace: '            && (name == "map" || name == "filter")\n',
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "a range made where it's stepped",
    ],
  },
  {
    name: "mir-mapped-range-end-unfolded",
    breaks:
      "`(1..=3).map(f).collect()` from MIR is `$range(1, 3 + 1).map(f)`, not `$range(1, 4)`",
    file: "src/lower/mir/iter/sources.rs",
    find: "                Some(n) => Expr::int(n + 1),\n                None => Expr::bin(js::Op::Add, range.end.clone(), Expr::int(1)),\n",
    replace:
      "                Some(_) | None => Expr::bin(js::Op::Add, range.end.clone(), Expr::int(1)),\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "a range mapped and collected",
    ],
  },
  {
    name: "mir-next-back-tree",
    breaks: "MIR's `next_back()` of a B-tree's `iter()` is refused",
    file: "src/lower/mir/iter/sources.rs",
    find: "            || self.recognition().is_tree_iter(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "iter_next_back"],
  },
  {
    name: "mir-next-back-split",
    breaks: "MIR's `next_back()` of a `str::split` is refused",
    file: "src/lower/mir/iter/sources.rs",
    find: "            || self.is_str_split(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "iter_next_back"],
  },
  {
    name: "mir-f64-max-wrapped",
    breaks:
      "MIR's `fold(0.0, f64::max)` gives `reduce` an arrow calling `$f64Max`",
    file: "src/lower/mir/iter/sources.rs",
    find: '            _ if matches!(f.helper_name(), Some("$f64Max" | "$f64Min")) => f,\n',
    replace: "",
    tests: ["test/traits.test.ts", "-t", "MIR dictionaries are explicit"],
  },
  {
    name: "mir-generic-iterator-unwrapped",
    breaks:
      "MIR's generic iterator is stepped as given, an array's `drop` missing",
    file: "src/lower/mir/iter/sources.rs",
    find: "            None if matches!(ty.kind(), ty::Param(_)) && !is_js_iterator(&value) => Ok(self.js_iterator(value)),\n",
    replace: "",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "takes the crate's own from behind",
    ],
  },
  {
    name: "mir-iterable-collected-generic",
    breaks:
      "a generic iterator collected is `Array.from(items)`, empty of an iterator with only a `next`",
    file: "src/lower/mir/iter/sources.rs",
    find: "        if crate::lower::recognition::known_by_bounds(source) {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "only a next"],
  },
  {
    name: "fresh-iter-len-stepped",
    breaks:
      "`v.iter().len()` is what's left of a `$iter` made for it, `$iter(v).end - $iter(v).at`, not `v.length`",
    file: "src/lower/mir/iter/sources.rs",
    find: "        if crate::lower::iterators::is_stepped_items(&it)\n",
    replace: "        if false\n",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
    snapshots: true,
  },
  {
    name: "args-not-stepped",
    breaks:
      "`env::args()` held is stepped as a JS iterator, which knows no length, not a `$iter` of its array",
    file: "src/lower/mir/iter/sources.rs",
    find: "            || self.recognition().is_process_items(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
    snapshots: true,
  },
  {
    name: "given-iterator-adapted-as-js",
    breaks:
      "an adapter's `I` summed is `Iterator.from` of it, not stepped by its dictionary",
    file: "src/lower/mir/iter/sources.rs",
    find: "            None if let Some(it) = self.given_js_iterator(value.clone(), ty, false, false) => Ok(it),\n",
    replace:
      "            None if let Some(it) = self.given_js_iterator(value.clone(), ty, false, false).filter(|_| false) => Ok(it),\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "projection-item-not-an-iterator",
    breaks:
      "an iterator whose `Item` is another's associated type, `A::Item`, isn't taken as one, and `for _ in &mut it` is refused",
    file: "src/lower/mir/iter/sources.rs",
    find: "                .is_some_and(|tr| !iterator_traits.contains(&tr)),\n",
    replace: "                .is_some_and(|_| false),\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_of_projection.rs$"],
  },
  {
    name: "rev-of-array-not-array",
    breaks:
      "a `rev()` of an array's iterator isn't seen to be its array reversed, and `next_back` of it is refused",
    file: "src/lower/mir/iter/sources.rs",
    find: "            || crate::lower::recognition::reversed_source(self.tcx, ty).is_some_and(|source| self.steps_an_array(source))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(rposition_of_lazy|std_rev).rs$"],
  },
  {
    name: "try-for-each-callback-unshifted",
    breaks:
      "`try_for_each`'s function is given what's made where its item goes",
    file: "src/lower/mir/iter/sources.rs",
    find: "                params.insert(0, made.into());\n",
    replace: "                params.push(made.into());\n",
    tests: ["test/corpus.test.ts", "-t", "^try_for_each.rs$"],
  },
  {
    name: "string-drain-no-back",
    breaks: "a string Drain's next_back is an array's",
    file: "src/lower/mir/iter/sources.rs",
    find: "            || self.recognition().is_string_drain(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "str_odds"],
  },
  {
    name: "mut-range-given-raw",
    breaks: "a &mut range given as an iterator is stepped where it is",
    file: "src/lower/mir/iter/sources.rs",
    find: "            ty::Ref(_, inner, Mutability::Mut) if self.range_kind(*inner).is_some() => *inner,",
    replace:
      "            ty::Ref(_, inner, Mutability::Mut) if false && self.range_kind(*inner).is_some() => *inner,",
    tests: ["test/corpus.test.ts", "-t", "^references_given_to_generics.rs$"],
  },
  {
    name: "range-collect-room-unasked",
    breaks:
      "`(0..n).map(f).collect()` past `isize::MAX` bytes maps on, no panic",
    file: "src/lower/mir/iter/sources.rs",
    find: "        let size = item.and_then(|item| self.room_size_of(&count, item));\n",
    replace:
      "        let size = item.and_then(|item| self.room_size_of(&count, item)).filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^asked_length_overflow.rs$"],
  },
  {
    name: "made-closure-made-again",
    breaks:
      "a closure a call makes, given to `fold`, is made again for each item, losing what it changes of its own (#259)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            js::ExprKind::Arrow(params, _) if params.len() <= 2 => f,\n            _ if own => f,\n",
    replace:
      "            js::ExprKind::Arrow(params, _) if params.len() <= 2 => f,\n            js::ExprKind::Var(_) if own => f,\n",
    tests: ["test/corpus.test.ts", "-t", "^(closures_made_and_folded).rs$"],
  },
  {
    name: "counted-range-from-any-start",
    breaks:
      "`(1..4).map(f).collect()` is `Array.from({ length: 4 }, ..)`, its items from 0, not 1 (#361)",
    file: "src/lower/mir/iter/sources.rs",
    find: "|| range.start.as_int() != Some(0) ",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^range_map_collected.rs$"],
  },
  {
    name: "counted-range-inclusive",
    breaks:
      "`(0..=3).map(f).collect()` is `Array.from({ length: 3 }, ..)`, one item short (#361)",
    file: "src/lower/mir/iter/sources.rs",
    find: "|| range.inclusive ",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^range_map_collected.rs$"],
  },
  {
    name: "counted-range-reversed",
    breaks:
      "`(0..4).rev().map(f).collect()` is `Array.from({ length: 4 }, ..)`, its items from the front (#361)",
    file: "src/lower/mir/iter/sources.rs",
    find: 'if name != "map" || range.back ',
    replace: 'if name != "map" ',
    tests: ["test/corpus.test.ts", "-t", "^range_map_collected.rs$"],
  },
  {
    name: "stepped-slices-unmade",
    breaks:
      "a slice's iterator only stepped is `$iter(items)` and `$next(it)`, helpers a page's inline script can't carry, not `items.values()` (rd270)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            let slice = self.is_std_type(ty, StdItem::SliceIter);\n",
    replace:
      "            let slice = false && self.is_std_type(ty, StdItem::SliceIter);\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "only stepped"],
  },
  {
    name: "std-iterator-unsaid",
    breaks:
      "a std iterator's value isn't said to be a JS iterator, so `step_by(step())` of a held `take_while` makes it one again, dropping what's left before the statement ends (generated program 12454000)",
    file: "src/lower/mir/iter/sources.rs",
    find: "                facts: value.facts.with(js::Known::Iterator),\n",
    replace: "                facts: value.facts,\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^step_by_of_a_call_holds_its_items.rs$",
    ],
  },
];
