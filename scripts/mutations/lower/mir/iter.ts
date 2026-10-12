// Mutations of src/lower/mir/iter.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-enumerate-swapped",
    breaks: "`enumerate` gives `(item, index)`, not `(index, item)`",
    file: "src/lower/mir/iter.rs",
    find: "                let pair = Expr::array(vec![Expr::var(\"i\"), Expr::var(\"x\")]);\n",
    replace: "                let pair = Expr::array(vec![Expr::var(\"x\"), Expr::var(\"i\")]);\n",
    tests: ["test/mir.test.ts", "-t", "labeled_blocks"],
  },
  {
    name: "mir-sum-from-one",
    breaks: "`sum` of an iterator starts from one",
    file: "src/lower/mir/iter.rs",
    find: "                    \"sum\" => (Op::Add, 0),\n",
    replace: "                    \"sum\" => (Op::Add, 1),\n",
    tests: ["test/mir.test.ts", "-t", "assert_failure"],
  },
  {
    name: "mir-max-is-min",
    breaks: "`max()` of an iterator from MIR is its least item",
    file: "src/lower/mir/iter.rs",
    find: "                self.extreme_of(name == \"max\", items, item, span)?\n",
    replace: "                self.extreme_of(name == \"min\", items, item, span)?\n",
    tests: ["test/mir.test.ts","-t","enum_order|opaque_iterators|generic_as_ref|returned_references"],
  },
  {
    name: "mir-mut-loop-items-unhandled",
    breaks: "`for x in &mut v` of numbers gives numbers, whose `value` writes nothing",
    file: "src/lower/mir/iter.rs",
    find: "            if handles {\n",
    replace: "            if false && handles {\n",
    tests: ["test/mir.test.ts","-t","mut_ref_loop"],
  },
  {
    name: "mir-array-iterator-not-stepped-items",
    breaks: "std's iterator over an array from MIR is `Iterator.from`, whose clone can't know where it is",
    file: "src/lower/mir/iter.rs",
    find: "        match self.array_source(ty) {\n            Some(_) => match from_iterator(value) {\n",
    replace: "        match self.array_source(ty).filter(|_| false) {\n            Some(_) => match from_iterator(value) {\n",
    tests: ["test/mir.test.ts","-t","iterator_clones"],
  },
  {
    name: "mir-generic-into-iter-unwrapped",
    breaks: "`for x in items` of a generic `IntoIterator` given an array steps the array",
    file: "src/lower/mir/iter.rs",
    find: "            if matches!(receiver_ty.kind(), ty::Param(_)) {\n",
    replace: "            if false && matches!(receiver_ty.kind(), ty::Param(_)) {\n",
    tests: ["test/lowering.test.ts","-t","stepped by hand"],
  },
  {
    name: "mir-rev-items-forward",
    breaks: "`rev()` of items made already gives them in order",
    file: "src/lower/mir/iter.rs",
    find: "                        \"rev\" => Expr::call(Expr::member(items, \"toReversed\"), Vec::new()),\n",
    replace: "                        \"rev\" => items,\n",
    tests: ["test/mir.test.ts","-t","linked_lists"],
  },
  {
    name: "mir-rev-range-forward",
    breaks: "`(1..4).rev()` steps the range from its start",
    file: "src/lower/mir/iter.rs",
    find: "                        vec![receiver, Expr::var(\"$rangeNextBack\")],\n",
    replace: "                        vec![receiver, Expr::var(\"$rangeNext\")],\n",
    tests: ["test/mir.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-rev-inclusive-refused",
    breaks: "`(250..=255u8).rev()` is refused",
    file: "src/lower/mir/iter.rs",
    find: "                (\"rev\", Some(RangeKind::Inclusive)) => {\n",
    replace: "                (\"rev\", Some(RangeKind::Inclusive)) if false => {\n",
    tests: ["test/mir.test.ts","-t","byte_ascii"],
  },
  {
    name: "mir-rev-own-next-back-unused",
    breaks: "`rev()` of an iterator of the crate's with its own `next_back` is refused",
    file: "src/lower/mir/iter.rs",
    find: "                    Some(double_ended) if name == \"rev\" => self\n",
    replace: "                    Some(double_ended) if false && name == \"rev\" => self\n",
    tests: ["test/mir.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-next-back-front",
    breaks: "`it.next_back()` of a `$iter` gives its first item",
    file: "src/lower/mir/iter.rs",
    find: "                (\"next_back\", None) if array => self.helper(Helper::NextBack, \"$nextBack\", vec![it]),\n",
    replace: "                (\"next_back\", None) if array => self.helper(Helper::Next, \"$next\", vec![it]),\n",
    tests: ["test/mir.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-range-next-back-front",
    breaks: "`range.next_back()` gives its start",
    file: "src/lower/mir/iter.rs",
    find: "                    self.helper(Helper::RangeNextBack, \"$rangeNextBack\", vec![it])\n",
    replace: "                    self.helper(Helper::RangeNext, \"$rangeNext\", vec![it])\n",
    tests: ["test/mir.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-inclusive-next-back-refused",
    breaks: "`next_back()` of an inclusive range is refused",
    file: "src/lower/mir/iter.rs",
    find: "                (\"next_back\", Some(RangeKind::Inclusive)) => {\n",
    replace: "                (\"next_back\", Some(RangeKind::Inclusive)) if false => {\n",
    tests: ["test/mir.test.ts","-t","range_inclusive_next_back"],
  },
  {
    name: "mir-len-whole",
    breaks: "`it.len()` of a stepped `$iter` counts what it's given already",
    file: "src/lower/mir/iter.rs",
    find: "                (\"len\", None) if array => Expr::bin(Op::Sub, Expr::member(it.clone(), \"end\"), Expr::member(it, \"at\")),\n",
    replace: "                (\"len\", None) if array => Expr::member(Expr::member(it, \"items\"), \"length\"),\n",
    tests: ["test/mir.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-size-hint-map-unseen",
    breaks: "`v.iter().map(f).size_hint()` is refused, though a `map`'s is its source's",
    file: "src/lower/mir/iter.rs",
    find: "                && method == \"map\"\n",
    replace: "                && method == \"map\" && false\n",
    tests: ["test/mir.test.ts","-t","size_hint"],
  },
  {
    name: "mir-collect-map-refused",
    breaks: "`collect` into a map or a set is refused",
    file: "src/lower/mir/iter.rs",
    find: "                } else if self.is_map(output) {\n",
    replace: "                } else if false && self.is_map(output) {\n",
    tests: ["test/mir.test.ts","-t","value_keys|user_hash|range_into_set"],
  },
  {
    name: "mir-object-keys-iterated",
    breaks: "`Object.keys(o)`, an array, is taken as a JS iterator and stepped as one",
    file: "src/lower/mir/iter.rs",
    find: "            js::ExprKind::Var(v) if v == \"Object\" => false,\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","a for over Object.keys"],
  },
  {
    name: "mir-made-array-iterator-from",
    breaks: "a `split`'s array MIR steps is `Iterator.from(…)`, so its chain stays a JS iterator's",
    file: "src/lower/mir/iter.rs",
    find: "            None if made_array(&value)\n                || self.recognition().is_deque_iter(ty)\n",
    replace: "            None if false\n                || self.recognition().is_deque_iter(ty)\n",
    tests: ["test/lowering.test.ts","-t","string's parts from MIR"],
  },
  {
    name: "mir-rev-of-fresh-iter-rest",
    breaks: "`rev()` of a `split` just made is `$rest($iter(…))` reversed, not the array's",
    file: "src/lower/mir/iter.rs",
    find: "                            _ => crate::lower::iterators::iterated(receiver),\n",
    replace: "                            _ => self.helper(Helper::Rest, \"$rest\", vec![receiver]),\n",
    tests: ["test/lowering.test.ts","-t","string's parts from MIR"],
  },
  {
    "name": "deque-iter-generic",
    "breaks": "a `VecDeque`'s iterator from MIR is `Iterator.from(prev)`, collected `.toArray()`, not `prev.slice()`",
    "file": "src/lower/mir/iter.rs",
    "find": " || self.recognition().is_deque_iter(ty)",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a VecDeque's items collected from MIR"
    ]
  },
  {
    "name": "mir-crate-fn-item-callback-wrapped",
    "breaks": "a crate function given as a callback from MIR is `(item) => is_rule(item)`, not `is_rule` as THIR has it",
    "file": "src/lower/mir/iter.rs",
    "find": "            js::ExprKind::Var(_) if own => f,\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a crate function given as a callback is passed as itself"
    ]
  },
  {
    "name": "mir-adapter-not-js-iterator",
    "breaks": "an adapter of a JS iterator, `.map(f)` of a range's, is made one again, `Iterator.from(..)`",
    "file": "src/lower/mir/iter.rs",
    "find": "            _ if matches!(name.as_str(), \"map\" | \"filter\" | \"flatMap\" | \"take\" | \"drop\") => is_js_iterator(object),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an adapter of a JS iterator is not made one again"
    ]
  },
  {
    "name": "mir-mapped-range-lazy",
    "breaks": "a range mapped and collected stays `$iterator({ start, end }, $rangeNext).map(f).toArray()`, not `$range(a, b).map(f)`",
    "file": "src/lower/mir/iter.rs",
    "find": "match self.mapped_range(&it, iter_ty) {",
    "replace": "match None::<Expr> {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range mapped and collected is its array mapped"
    ]
  },
  {
    "name": "mir-range-inclusive-lazy",
    "breaks": "`(1..=n).collect()` from MIR stays `$iterator(\u2026, $rangeInclusiveNext).toArray()`, not `$range(1, n + 1)`",
    "file": "src/lower/mir/iter.rs",
    "find": "            \"$rangeInclusiveNext\" => (true, false),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range mapped and collected"
    ]
  },
  {
    "name": "mir-range-back-forward",
    "breaks": "`(0..n).rev().collect()` from MIR is `$range(0, n)`, forwards",
    "file": "src/lower/mir/iter.rs",
    "find": "        if back {\n",
    "replace": "        if !back {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range mapped and collected"
    ]
  },
  {
    "name": "mir-range-unmapped-lazy",
    "breaks": "a range collected unmapped from MIR stays a lazy `$iterator(\u2026).toArray()`",
    "file": "src/lower/mir/iter.rs",
    "find": "            || (s.as_str(), e.as_str()) != (\"start\", \"end\")\n",
    "replace": "            || maps.is_empty()\n            || (s.as_str(), e.as_str()) != (\"start\", \"end\")\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range mapped and collected"
    ]
  },
  {
    "name": "mir-chars-next-back-refused",
    "breaks": "`next_back()` of a string's `chars()` from MIR is refused as a lazy iterator's",
    "file": "src/lower/mir/iter.rs",
    "find": "self.array_source(iter_ty).is_some() || self.recognition().is_str_chars(iter_ty);",
    "replace": "self.array_source(iter_ty).is_some();",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "the rest of a string's chars from MIR"
    ]
  },
  {
    "name": "mir-rev-of-fresh-copied",
    "breaks": "`xs.iter().rev()` from MIR copies the array first, `$rest($iter(xs)).toReversed()`",
    "file": "src/lower/mir/iter.rs",
    "find": "it if name == \"rev\" && crate::lower::iterators::is_stepped_items(&it) => {",
    "replace": "it if false => {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "mir-float-sum-from-zero",
    "breaks": "a float's sum from MIR starts at 0, an empty sum `0`, not Rust's -0.0",
    "file": "src/lower/mir/iter.rs",
    "find": "(\"sum\", true) => Expr::num(-0.0),",
    "replace": "(\"sum\", true) => Expr::num(0.0),",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a float's sum from MIR keeps the sign"
    ]
  },
  {
    "name": "mir-iter-eq-refused",
    "breaks": "`a.eq(b)` of two iterators from MIR is refused",
    "file": "src/lower/mir/iter.rs",
    "find": "            \"eq\" | \"ne\" => {",
    "replace": "            \"eq\" | \"ne\" if false => {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterators compared equal from MIR"
    ]
  },
  {
    "name": "mir-iter-mut-a-js-iterator",
    "breaks": "a slice's `iter_mut()` from MIR is `Iterator.from(v)`, its chain lazy, `$lent(Iterator.from(v)).find(f)`, not `v.find(f)`",
    "file": "src/lower/mir/iter.rs",
    "find": "                || self.recognition().is_slice_iter_mut(ty) =>\n",
    "replace": "                || false =>\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iter_mut chains from MIR"
    ]
  },
  {
    "name": "mir-mapped-range-filtered",
    "breaks": "`(0..n).filter(f).collect()` from MIR is `$range(0, n).filter(f)`, an array of all n numbers made to keep few",
    "file": "src/lower/mir/iter.rs",
    "find": "            && name == \"map\"\n",
    "replace": "            && (name == \"map\" || name == \"filter\")\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range made where it's stepped"
    ]
  },
  {
    "name": "mir-mapped-range-end-unfolded",
    "breaks": "`(1..=3).map(f).collect()` from MIR is `$range(1, 3 + 1).map(f)`, not `$range(1, 4)`",
    "file": "src/lower/mir/iter.rs",
    "find": "                Some(n) => Expr::int(n + 1),\n                None => Expr::bin(js::Op::Add, end.clone(), Expr::int(1)),\n",
    "replace": "                Some(_) | None => Expr::bin(js::Op::Add, end.clone(), Expr::int(1)),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range mapped and collected"
    ]
  },
];
