// Mutations of crates/rust-js-lower/src/iterators/mir_steps/sources.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "given-by-ref-unlent",
    breaks:
      "`by_ref().sum()` of a type parameter given as it is calls its own `fold`, which takes it and drops it before the sum's done (#994)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "                ty::Ref(_, _, Mutability::Mut) => self.lent_iterator(it, ty),\n",
    replace: "                ty::Ref(_, _, Mutability::Mut) => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^self_given_on_as_into_iter\\.rs$"],
    snapshots: true,
  },
  {
    name: "step-by-count-unfolded",
    breaks:
      "a `step_by` of numbers known, `(0..10).step_by(3)` collected, asks room for `Math.ceil(10 / 3)`, not the 4 that surely fits",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            && let Some((source_value, step)) = crate::iterators::stepped_by_parts(it)\n",
    replace: "            && let Some((source_value, step)) = crate::iterators::stepped_by_parts(it).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
  },
  {
    name: "repeat-n-uncounted",
    breaks:
      "`repeat_n(x, n)` made in place has no count, so its `collect()` asks no room: `capacity overflow` unasked (#246)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            && let Some([x, n, rest @ ..]) = crate::iterators::repeating_n_parts(it)\n",
    replace: "            && let Some([x, n, rest @ ..]) = crate::iterators::repeating_n_parts(it).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
    snapshots: true,
  },
  {
    name: "given-copied-lengthless",
    breaks:
      "a `copied()` of an iterator generic code is given as it is, bound by `ExactSizeIterator`, is the JS iterator `$given` makes without its `len`, whose `len()` throws (#387)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            && let Some(it) = self.given_stepped(value.clone(), ty, self.knows_its_back(output), true)\n",
    replace:
      "            && let Some(it) = self.given_stepped(value.clone(), ty, self.knows_its_back(output), false)\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_given_len.rs$"],
  },
  {
    name: "given-copied-backless",
    breaks:
      "a `copied()` of an iterator generic code is given as it is, made knowing its length, is the JS iterator `$given` makes without its back: as programs run it, a clone of it steps its back by another route, so only how its JS reads changes, what the corpus's snapshot of it holds (#1983, #2261)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            && let Some(it) = self.given_stepped(value.clone(), ty, self.knows_its_back(output), true)\n",
    replace: "            && let Some(it) = self.given_stepped(value.clone(), ty, false, true)\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_of_a_generic_copied_stepped_from_its_back.rs$"],
    snapshots: true,
  },
  {
    name: "len-in-place-take-unbounded",
    breaks:
      "`len()` of a `take(n)` made in place is more than `n`: `v.iter().take(2).map(f).len()` is 5, not 2 (ADR 0986)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: '                false => math("min", vec![len, n]),\n',
    replace: '                false => math("max", vec![len, n]),\n',
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-skip-unsaturated",
    breaks:
      "`len()` of a `skip(n)` past its end, made in place, is below zero: `-4`, not `0` (ADR 0986)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: '                true => math("max", vec![Expr::bin(Op::Sub, len, n), Expr::int(0)]),\n',
    replace: "                true => Expr::bin(Op::Sub, len, n),\n",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-rev-unread",
    breaks:
      "`len()` of a `rev()` made in place isn't its source's: `(1..4).rev().len()` is refused (ADR 0986)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        if let Some(source) = r::reversed_source(self.tcx, ty) {\n            let source_value = match",
    replace:
      "        if let Some(source) = r::reversed_source(self.tcx, ty).filter(|_| false) {\n            let source_value = match",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-enumerate-unread",
    breaks:
      "`len()` of an `enumerate()` made in place isn't its source's: `v.iter().enumerate().len()` is refused (ADR 0986)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        if let Some(source) = r::enumerated_source(self.tcx, ty) {\n",
    replace:
      "        if let Some(source) = r::enumerated_source(self.tcx, ty).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-generic-unread",
    breaks:
      "`len()` of a `take(2)` of a generic `ExactSizeIterator` is refused (ADR 0986)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        if (self.is_unknown(ty) || r::dyn_iter_trait(self.tcx, ty).is_some()) && self.knows_its_length(ty) {\n",
    replace:
      "        if false && (self.is_unknown(ty) || r::dyn_iter_trait(self.tcx, ty).is_some()) && self.knows_its_length(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
  },
  {
    name: "len-in-place-generic-wrapped",
    breaks:
      "`len()` of a generic one is read through `Iterator.from(it)`, which wraps it, not of `it` (ADR 0986)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            let given = crate::iterators::iterated(it.clone());\n",
    replace: "            let given = it.clone();\n",
    tests: ["test/corpus.test.ts", "-t", "^len_made_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "mir-len-whole",
    breaks: "`it.len()` of a stepped `$iter` counts what it's given already",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: '        Expr::bin(Op::Sub, Expr::member(it.clone(), "end"), Expr::member(it, "at"))\n',
    replace: '        Expr::member(Expr::member(it, "items"), "length")\n',
    tests: ["test/corpus.test.ts", "-t", "iterator_back_ends"],
  },
  {
    name: "mir-made-array-iterator-from",
    breaks:
      "a `split`'s array MIR steps is `Iterator.from(…)`, so its chain stays a JS iterator's",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        } else if made_array(&value) || self.steps_an_array(ty) || self.counted_where_kept(ty) {\n",
    replace: "        } else if self.steps_an_array(ty) || self.counted_where_kept(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(boxed_array_iter|calculator).rs$"],
    snapshots: true,
  },
  {
    name: "deque-iter-generic",
    breaks:
      "a `VecDeque`'s iterator from MIR is `Iterator.from(prev)`, collected `Array.from(..)`, not `prev.slice()`, and its `next_back()` refused",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            || self.recognition().is_tree_iter(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "iter_next_back"],
  },
  {
    name: "mir-next-back-split",
    breaks: "MIR's `next_back()` of a `str::split` is refused",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            || self.is_str_split(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "iter_next_back"],
  },
  {
    name: "mir-f64-max-wrapped",
    breaks:
      "MIR's `fold(0.0, f64::max)` gives `reduce` an arrow calling `$f64Max`",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: '            _ if matches!(f.helper_name(), Some("$f64Max" | "$f64Min")) => f,\n',
    replace: "",
    tests: ["test/traits.test.ts", "-t", "MIR dictionaries are explicit"],
  },
  {
    name: "mir-generic-iterator-unwrapped",
    breaks:
      "MIR's generic iterator is stepped as given, an array's `drop` missing",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "                false => self.js_iterator(value),\n",
    replace: "                false => value,\n",
    tests: ["test/corpus.test.ts", "-t", "^by_ref_lends.rs$"],
    snapshots: true,
  },
  {
    name: "fresh-iter-len-stepped",
    breaks:
      "`v.iter().len()` is what's left of a `$iter` made for it, `$iter(v).end - $iter(v).at`, not `v.length`",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        if crate::iterators::is_stepped_items(&it)\n",
    replace: "        if false\n",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
    snapshots: true,
  },
  {
    name: "args-not-stepped",
    breaks:
      "`env::args()` held is stepped as a JS iterator, which knows no length, not a `$iter` of its array",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            || self.recognition().is_process_items(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
    snapshots: true,
  },
  {
    name: "given-iterator-adapted-as-js",
    breaks:
      "an adapter's `I` summed is `Iterator.from` of it, not stepped by its dictionary",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            None if let Some(it) = self.given_js_iterator(value.clone(), ty, false, false) => Ok(match ty.kind() {\n",
    replace:
      "            None if let Some(it) = self.given_js_iterator(value.clone(), ty, false, false).filter(|_| false) => Ok(match ty.kind() {\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "rev-of-array-not-array",
    breaks:
      "a `rev()` of an array's iterator isn't seen to be its array reversed, and `next_back` of it is refused",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            || crate::recognition::reversed_source(self.tcx, ty).is_some_and(|source| self.steps_an_array(source))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(rposition_of_lazy|std_rev).rs$"],
  },
  {
    name: "try-for-each-callback-unshifted",
    breaks:
      "`try_for_each`'s function is given what's made where its item goes",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "                params.insert(0, made.into());\n",
    replace: "                params.push(made.into());\n",
    tests: ["test/corpus.test.ts", "-t", "^try_for_each.rs$"],
  },
  {
    name: "string-drain-no-back",
    breaks: "a string Drain's next_back is an array's",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            || self.recognition().is_string_drain(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "str_odds"],
  },
  {
    name: "mut-range-given-raw",
    breaks: "a &mut range given as an iterator is stepped where it is",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            ty::Ref(_, inner, Mutability::Mut) if self.range_kind(*inner).is_some() => *inner,",
    replace:
      "            ty::Ref(_, inner, Mutability::Mut) if false && self.range_kind(*inner).is_some() => *inner,",
    tests: ["test/corpus.test.ts", "-t", "^references_given_to_generics.rs$"],
  },
  {
    name: "range-collect-room-unasked",
    breaks:
      "`(0..n).map(f).collect()` past `isize::MAX` bytes maps on, no panic",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        let size = item.and_then(|item| self.room_size_of(&count, item));\n",
    replace:
      "        let size = item.and_then(|item| self.room_size_of(&count, item)).filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^asked_length_overflow.rs$"],
  },
  {
    name: "made-closure-made-again",
    breaks:
      "a closure a call makes, given to `fold`, is made again for each item, losing what it changes of its own (#259)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            js::ExprKind::Arrow(params, _) if params.len() <= 2 => f,\n            _ if own => f,\n",
    replace:
      "            js::ExprKind::Arrow(params, _) if params.len() <= 2 => f,\n            js::ExprKind::Var(_) if own => f,\n",
    tests: ["test/corpus.test.ts", "-t", "^(closures_made_and_folded).rs$"],
  },
  {
    name: "counted-range-from-any-start",
    breaks:
      "`(1..4).map(f).collect()` is `Array.from({ length: 4 }, ..)`, its items from 0, not 1 (#361)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "|| range.start.as_int() != Some(0) ",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^range_map_collected.rs$"],
  },
  {
    name: "counted-range-inclusive",
    breaks:
      "`(0..=3).map(f).collect()` is `Array.from({ length: 3 }, ..)`, one item short (#361)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "|| range.inclusive ",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^range_map_collected.rs$"],
  },
  {
    name: "counted-range-reversed",
    breaks:
      "`(0..4).rev().map(f).collect()` is `Array.from({ length: 4 }, ..)`, its items from the front (#361)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: 'if name != "map" || range.back ',
    replace: 'if name != "map" ',
    tests: ["test/corpus.test.ts", "-t", "^range_map_collected.rs$"],
  },
  {
    name: "stepped-slices-unmade",
    breaks:
      "a slice's iterator only stepped is `$iter(items)` and `$next(it)`, helpers a page's inline script can't carry, not `items.values()` (rd270)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            let items = self.recognition().steps_its_array(ty) && (of_items || made_array(&value));\n",
    replace: "            let items = false;\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "only stepped"],
  },
  {
    name: "std-iterator-unsaid",
    breaks:
      "a std iterator's value isn't said to be a JS iterator, so `step_by(step())` of a held `take_while` makes it one again, dropping what's left before the statement ends (generated program 12454000)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
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
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "                bindings::reads_only_its_parameters(self.tcx, def_id)\n",
    replace: "                false\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "reads only its parameters is its function"],
  },
  {
    name: "len-in-place-held-rev-read",
    breaks:
      "the `size_hint()` of a reversed range held, std's compiled `Rev`, is counted from its own fields as a range's, `it.end - it.start`, which it hasn't: `NaN` (#745)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "                _ if it.reads_same() => crate::iterators::steps(it).ok_or(Unknown)?,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^by_ref_lends.rs$|^by_ref_lends with"],
  },
  {
    name: "zip-len-the-longer",
    breaks: "`len()` of a zip made here is the longer of its sides' (#740)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "                (Some(a), Some(b)) => Expr::int(a.min(b)),\n                _ => Expr::builtin(Expr::js_global(\"Math\"), \"min\", vec![a, b]),\n",
    replace: "                (Some(a), Some(b)) => Expr::int(a.max(b)),\n                _ => Expr::builtin(Expr::js_global(\"Math\"), \"max\", vec![a, b]),\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_from_its_back.rs$"],
  },
  {
    name: "step-by-len-of-items-uncounted",
    breaks: "`len()` of a `step_by(n)` of a slice's items made in place is counted only of a range's, and refused (#811)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            && let Some((source_value, step)) = crate::iterators::stepped_by_parts(it)\n",
    replace: "            && self.range_kind(source.peel_refs()).is_some()\n            && let Some((source_value, step)) = crate::iterators::stepped_by_parts(it)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_and_counted_steps_from_their_back.rs$"],
  },
  {
    name: "step-by-of-one-divided",
    breaks: "a `step_by(1)`'s count is `Math.ceil(n / 1)`, where it's its source's (#811)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            if step.as_int() == Some(1) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_and_counted_steps_from_their_back|collected_counts_ask_room).rs$"],
    snapshots: true,
  },
  {
    name: "counted-made-knowing-unread",
    breaks:
      "a `skip(n)` or a `take(n)` the crate steps from its back, made knowing it where it's made, isn't read as knowing it where a variable holds it: `held.rev()` of a `let held = v.into_iter().skip(1)` is refused (#718)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        if r::counted_source(self.tcx, base).is_some() && crate::iterators::was_made_knowing_its_back(it) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
  },
  {
    name: "count-of-a-call-read-again",
    breaks:
      "a count whose reading runs code, `repeat_n(1u16, *n.as_ref().unwrap())`'s or a range bound's call, is read again where the iterator is made of the same text: a collect's room and an `eq`'s lengths run it twice (#1044, #1441, #1483)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        Ok(len) if len.has_effects() => Err(Uncounted::RunsCode),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(collect_room_counted_by_the_program|counted_where_made_runs_each_part_once).rs$"],
  },
  {
    name: "count-leaves-unread-code-out",
    breaks:
      "`repeat_n(made(4), 3).size_hint()`, counted as `3` where it isn't made after, never runs `made(4)` (#1485)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        let unread = |part: &Expr| match discards && part.has_effects() {\n",
    replace: "        let unread = |part: &Expr| match false && part.has_effects() {\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_where_made_runs_each_part_once.rs$"],
  },
  {
    name: "range-count-reads-its-end-first",
    breaks:
      "`(made(6)..made(11)).step_by(2).len()`, `Math.max(made(11) - made(6), 0)`, runs the end's code before the start's (#1485)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "                && end.has_effects()\n",
    replace: "                && end.has_effects()\n                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_where_made_runs_each_part_once.rs$"],
  },
  {
    name: "repeat-n-of-a-cloned-value-uncounted",
    breaks:
      "`repeat_n(Dropped(1), usize::MAX).collect()`, `$repeatingN(x, n, clone, drop)`, has no count read, so no room is asked and JS collects until memory runs out where std panics `capacity overflow` (#1044)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            && let Some([x, n, rest @ ..]) = crate::iterators::repeating_n_parts(it)\n",
    replace: "            && let Some([x, n, rest @ ..]) = crate::iterators::repeating_n_parts(it).filter(|parts| parts.len() == 2)\n",
    tests: ["test/corpus.test.ts", "-t", "^collected_counts_ask_room.rs$"],
    snapshots: true,
  },
  {
    name: "held-length-uncounted",
    breaks: "a variable its maker made knowing its length isn't counted by its own `len()` in place (#851)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        if matches!(it.kind, js::ExprKind::Var(_))\n            && !self.steps_an_array(ty)\n",
    replace: "        if false\n            && !self.steps_an_array(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^(owned_maps_asked_their_length|cloned_items_from_both_ends).rs$"],
    snapshots: true,
  },
  {
    name: "held-compiled-length-counted",
    breaks: "a variable holding std's compiled `Map` of a generic `ExactSizeIterator` is counted by its own `len()`, which it hasn't unless given it, so its `size_hint()` throws `it.len is not a function` (#851)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            && !self.compiled_struct(ty, it)\n            && self.knows_its_length_of(it, ty)\n",
    replace: "            && self.knows_its_length_of(it, ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^owned_maps_asked_their_length.rs$"],
  },
  {
    name: "heap-iter-uncounted-where-kept",
    breaks: "a heap's `iter()`, in a crate that reads a capacity, is JS's own iterator of its array, which std's compiled `Skip` asks a hint of and is given none, its capacity 64 where std's is 38 (#1215)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        } else if made_array(&value) || self.steps_an_array(ty) || self.counted_where_kept(ty) {\n",
    replace: "        } else if made_array(&value) || self.steps_an_array(ty) {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "capacity_of_what_std_hints_less"],
  },
  {
    name: "js-iterator-of-dictionary-unasked",
    breaks:
      "an `eq` or a `cmp` given a `&'a I` bounded `&'a I: IntoIterator` steps the value itself, not its dictionary's `into_iter`: `\"abc\".chars().eq(word)` `false` where Rust gives `true` (#1264)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        if let Some(items) = self.items_by_dictionary(value.clone(), ty, span)? {\n            return Ok(items);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_of_a_reference_given_to_generic_code.rs$"],
  },
  {
    name: "chain-uncounted",
    breaks: "a chain's count isn't the sum of its halves', so a collect of one asks the room its hint asks, never `$roomFor(n + 2, size)` (#1441)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "r::chained_sources(self.tcx, ty),",
    replace: "r::chained_sources(self.tcx, ty).filter(|_| false),",
    tests: ["test/vec-capacity.test.ts", "-t", "a collect whose count surely fits"],
  },
  {
    name: "std-iterator-held-as-its-items",
    breaks:
      "a std iterator of an array's items given as it is, a variable's `$iter` that's been stepped, is said to be over the array's items from their start, so lent as a `dyn` and only stepped it's the array's own iterator, `a.values()`, of a `$iter`, which throws (#1985)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            let items = self.recognition().steps_its_array(ty) && (of_items || made_array(&value));\n",
    replace: "            let items = self.recognition().steps_its_array(ty);\n",
    tests: ["test/corpus.test.ts", "-t", "^stepped_iterator_lent_as_a_dyn.rs$"],
  },
  {
    name: "clone-as-its-source-uncounted",
    breaks: "a `cloned()` of what needs no clone, which is its source in JS, isn't counted as its source, so `len()` of a `rev()` of a `cloned()` of a slice's map is refused (#1978)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            r::copied_source(self.tcx, ty).or_else(|| r::cloned_source(self.tcx, ty).filter(|_| self.is_its_source(ty)))\n",
    replace: "            r::copied_source(self.tcx, ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^len_of_a_reversed_copy_of_a_map.rs$"],
  },
  {
    name: "mut-dyn-self-unlent",
    breaks: "a step of a `Self` that's a `&mut dyn Iterator` itself, `(&mut d).find(..)` or `.rfind(..)`, is its referent's, never lent as std's unsized `&mut I` lends it, so a `take(3)`'s `rfind` asks its own `try_rfold`, the crate's `nth_back(1)`, where std steps it by `next_back`, `nth_back(2)` (#2180)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "        if lent && !values.is_empty() {\n",
    replace: "        if lent && !values.is_empty() && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_dyn_lent_steps_as_std_does.rs$"],
  },
  {
    name: "box-of-std-iterator-as-its-value",
    breaks: "a `Box` of std's adapter is stepped as the adapter itself, so a boxed `map`'s `count()` or `fold()` asks its own `fold`, reaching the crate's, which std's `Box<I>` never asks (#2179)",
    file: "crates/rust-js-lower/src/iterators/mir_steps/sources.rs",
    find: "            None if ty.peel_refs().is_box()\n",
    replace: "            None if false && ty.peel_refs().is_box()\n",
    tests: ["test/corpus.test.ts", "-t", "^box_of_an_adapter_steps_as_std_box_does.rs$"],
  },
];
