// Mutations of src/lower/mir/iter/sources.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "given-by-ref-unlent",
    breaks:
      "`by_ref().sum()` of a type parameter given as it is calls its own `fold`, which takes it and drops it before the sum's done (#994)",
    file: "src/lower/mir/iter/sources.rs",
    find: "                ty::Ref(_, _, Mutability::Mut) => self.lent_iterator(it),\n",
    replace: "                ty::Ref(_, _, Mutability::Mut) => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^by_ref_of_held_iterator.rs$"],
  },
  {
    name: "held-into-iter-param-from-js",
    breaks:
      "a collection's `extend` of an `IntoIterator` type parameter held as it is steps its caller's own value as a JS iterable (#966)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            None if matches!(ty.kind(), ty::Param(_))\n                && let Some(items) = self.items_by_dictionary(value.clone(), ty, span)? =>\n",
    replace: "            None if false\n                && let Some(items) = self.items_by_dictionary(value.clone(), ty, span)? =>\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_named_by_another_bound.rs$"],
  },
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
    snapshots: true,
  },
  {
    name: "step-by-uncounted",
    breaks:
      "a range's `step_by(s)` made in place has no count: its `len()` is refused and its `collect()` asks no room (#246)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            && let Some((source_value, step)) = crate::lower::iterators::stepped_by_parts(it)\n",
    replace: "            && let Some((source_value, step)) = crate::lower::iterators::stepped_by_parts(it).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "repeat-n-uncounted",
    breaks:
      "`repeat_n(x, n)` made in place has no count, so its `collect()` asks no room: `capacity overflow` unasked (#246)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            && let Some([_, n, ..]) = crate::lower::iterators::repeating_n_parts(it)\n",
    replace: "            && let Some([_, n, ..]) = crate::lower::iterators::repeating_n_parts(it).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
    snapshots: true,
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
    find: '            && method == "map"\n            && let [f] = args.as_slice()\n',
    replace: '            && method == "mapped"\n            && let [f] = args.as_slice()\n',
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-take-unbounded",
    breaks:
      "`len()` of a `take(n)` made in place is more than `n`: `v.iter().take(2).map(f).len()` is 5, not 2 (ADR 0986)",
    file: "src/lower/mir/iter/sources.rs",
    find: '                false => math("min", vec![len, n]),\n',
    replace: '                false => math("max", vec![len, n]),\n',
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-skip-unsaturated",
    breaks:
      "`len()` of a `skip(n)` past its end, made in place, is below zero: `-4`, not `0` (ADR 0986)",
    file: "src/lower/mir/iter/sources.rs",
    find: '                true => math("max", vec![Expr::bin(Op::Sub, len, n), Expr::int(0)]),\n',
    replace: "                true => Expr::bin(Op::Sub, len, n),\n",
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
    find: "                false => self.js_iterator(value),\n",
    replace: "                false => value,\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "takes the crate's own from behind",
    ],
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
    find: "            None if let Some(it) = self.given_js_iterator(value.clone(), ty, false, false) => Ok(match ty.kind() {\n",
    replace:
      "            None if let Some(it) = self.given_js_iterator(value.clone(), ty, false, false).filter(|_| false) => Ok(match ty.kind() {\n",
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
    find: "            let items = self.recognition().steps_its_array(ty);\n",
    replace: "            let items = false;\n",
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
  {
    name: "parameters-read-callback-wrapped",
    breaks: "an adapter's callback of a binding that says it reads only its parameters is wrapped in an arrow, `items.map((item) => measure(item))` (rd648)",
    file: "src/lower/mir/iter/sources.rs",
    find: "                bindings::reads_only_its_parameters(self.tcx, def_id)\n",
    replace: "                false\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "reads only its parameters is its function"],
  },
  {
    name: "len-in-place-held-rev-read",
    breaks:
      "the `size_hint()` of a reversed range held, std's compiled `Rev`, is counted from its own fields as a range's, `it.end - it.start`, which it hasn't: `NaN` (#745)",
    file: "src/lower/mir/iter/sources.rs",
    find: "                _ if it.reads_same() => crate::lower::iterators::steps(it).ok_or(Unknown)?,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^by_ref_lends.rs$|^by_ref_lends with"],
  },
  {
    name: "zip-len-the-longer",
    breaks: "`len()` of a zip made here is the longer of its sides' (#740)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            return Ok(Expr::call(Expr::member(Expr::var(\"Math\"), \"min\"), vec![a, b]));\n",
    replace: "            return Ok(Expr::call(Expr::member(Expr::var(\"Math\"), \"max\"), vec![a, b]));\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_from_its_back.rs$"],
  },
  {
    name: "step-by-len-of-items-uncounted",
    breaks: "`len()` of a `step_by(n)` of a slice's items made in place is counted only of a range's, and refused (#811)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            && let Some((source_value, step)) = crate::lower::iterators::stepped_by_parts(it)\n",
    replace: "            && self.range_kind(source.peel_refs()).is_some()\n            && let Some((source_value, step)) = crate::lower::iterators::stepped_by_parts(it)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_and_counted_steps_from_their_back.rs$"],
  },
  {
    name: "step-by-of-one-divided",
    breaks: "a `step_by(1)`'s count is `Math.ceil(n / 1)`, where it's its source's (#811)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            if step.as_int() == Some(1) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_and_counted_steps_from_their_back|collected_counts_ask_room).rs$"],
    snapshots: true,
  },
  {
    name: "counted-made-knowing-unread",
    breaks:
      "a `skip(n)` or a `take(n)` the crate steps from its back, made knowing it where it's made, isn't read as knowing it where a variable holds it: `held.rev()` of a `let held = v.into_iter().skip(1)` is refused (#718)",
    file: "src/lower/mir/iter/sources.rs",
    find: "        if r::counted_source(self.tcx, base).is_some() && crate::lower::iterators::was_made_knowing_its_back(it) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
  },
  {
    name: "count-read-through-a-call-read-again",
    breaks:
      "`repeat_n(1u16, counted(n)).map(f).collect()` reads its count twice, `$roomFor(counted(n), 2)` and the iterator's, so `counted` runs twice where Rust runs it once (#1044)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            true => Err(RunsCode),\n",
    replace: "            true => Ok(count.clone()),\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "repeat-n-of-a-cloned-value-uncounted",
    breaks:
      "`repeat_n(Dropped(1), usize::MAX).collect()`, `$repeatingN(x, n, clone, drop)`, has no count read, so no room is asked and JS collects until memory runs out where std panics `capacity overflow` (#1044)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            && let Some([_, n, ..]) = crate::lower::iterators::repeating_n_parts(it)\n",
    replace: "            && let Some([_, n]) = crate::lower::iterators::repeating_n_parts(it)\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
    snapshots: true,
  },
  {
    name: "held-length-uncounted",
    breaks: "a variable its maker made knowing its length isn't counted by its own `len()` in place (#851)",
    file: "src/lower/mir/iter/sources.rs",
    find: "        if matches!(it.kind, js::ExprKind::Var(_))\n            && !self.steps_an_array(ty)\n",
    replace: "        if false\n            && !self.steps_an_array(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^(owned_maps_asked_their_length|cloned_items_from_both_ends).rs$"],
    snapshots: true,
  },
  {
    name: "held-compiled-length-counted",
    breaks: "a variable holding std's compiled `Map` of a generic `ExactSizeIterator` is counted by its own `len()`, which it hasn't unless given it, so its `size_hint()` throws `it.len is not a function` (#851)",
    file: "src/lower/mir/iter/sources.rs",
    find: "            && !self.compiled_struct(ty, it)\n            && self.knows_its_length_of(it, ty)\n",
    replace: "            && self.knows_its_length_of(it, ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^owned_maps_asked_their_length.rs$"],
  },
];
