// Mutations of src/lower/mir/iter.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-enumerate-swapped",
    breaks: "`enumerate` gives `(item, index)`, not `(index, item)`",
    file: "src/lower/mir/iter.rs",
    find: "                let pair = Expr::array(vec![Expr::var(\"i\"), Expr::var(\"x\")]);\n",
    replace: "                let pair = Expr::array(vec![Expr::var(\"x\"), Expr::var(\"i\")]);\n",
    tests: ["test/corpus.test.ts", "-t", "labeled_blocks"],
  },
  {
    name: "mir-sum-from-one",
    breaks: "`sum` of an iterator starts from one",
    file: "src/lower/mir/iter.rs",
    find: "                    Some(IterMethod::Sum) => (Op::Add, 0),\n",
    replace: "                    Some(IterMethod::Sum) => (Op::Add, 1),\n",
    tests: ["test/corpus.test.ts", "-t", "assert_failure"],
  },
  {
    name: "mir-max-is-min",
    breaks: "`max()` of an iterator from MIR is its least item",
    file: "src/lower/mir/iter.rs",
    find: "                self.extreme_of(std_method == Some(IterMethod::Max), items, item, span)?\n",
    replace: "                self.extreme_of(std_method == Some(IterMethod::Min), items, item, span)?\n",
    tests: ["test/corpus.test.ts","-t","enum_order|opaque_iterators|generic_as_ref|returned_references"],
  },
  {
    name: "mir-mut-loop-items-unhandled",
    breaks: "`for x in &mut v` of numbers gives numbers, whose `value` writes nothing",
    file: "src/lower/mir/iter.rs",
    find: "            if handles {\n",
    replace: "            if false && handles {\n",
    tests: ["test/corpus.test.ts","-t","mut_ref_loop"],
  },
  {
    name: "mir-generic-into-iter-unwrapped",
    breaks: "`for x in items` of a generic `IntoIterator` given an array steps the array",
    file: "src/lower/mir/iter.rs",
    find: "            if self.recognition().is_given_iterable(receiver_ty) {\n                return Ok(Some(crate::lower::iterators::given_into_iter(value)));\n",
    replace: "            if self.recognition().is_given_iterable(receiver_ty) {\n                return Ok(Some(value));\n",
    tests: ["test/lowering.test.ts","-t","stepped by hand"],
  },
  {
    name: "mir-rev-items-forward",
    breaks: "`rev()` of items made already gives them in order",
    file: "src/lower/mir/iter.rs",
    find: "            Some(IterMethod::Rev) => Expr::call(Expr::member(items, \"toReversed\"), Vec::new()),\n",
    replace: "            Some(IterMethod::Rev) => items,\n",
    tests: ["test/corpus.test.ts", "-t", "^(boxed_array_iter|fused_iterator|str_bytes|sales_report|matrix_strings).rs$"],
  },
  {
    name: "mir-rev-own-next-back-unused",
    breaks: "`rev()` of an iterator of the crate's with its own `next_back` is refused",
    file: "src/lower/mir/iter.rs",
    find: "                    Some(double_ended) if std_method == Some(IterMethod::Rev) => self\n",
    replace: "                    Some(double_ended) if false && std_method == Some(IterMethod::Rev) => self\n",
    tests: ["test/corpus.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-next-back-front",
    breaks: "`it.next_back()` of a `$iter` gives its first item",
    file: "src/lower/mir/iter.rs",
    find: "                (Some(IterMethod::NextBack), None) if array => Expr::call_helper(\"$nextBack\", vec![it]),\n",
    replace: "                (Some(IterMethod::NextBack), None) if array => Expr::call_helper(\"$next\", vec![it]),\n",
    tests: ["test/corpus.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-len-whole",
    breaks: "`it.len()` of a stepped `$iter` counts what it's given already",
    file: "src/lower/mir/iter.rs",
    find: "        Expr::bin(Op::Sub, Expr::member(it.clone(), \"end\"), Expr::member(it, \"at\"))\n",
    replace: "        Expr::member(Expr::member(it, \"items\"), \"length\")\n",
    tests: ["test/corpus.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-size-hint-map-unseen",
    breaks: "`v.iter().map(f).size_hint()` is refused, though a `map`'s is its source's",
    file: "src/lower/mir/iter.rs",
    find: "                && method == \"map\"\n",
    replace: "                && method == \"map\" && false\n",
    tests: ["test/corpus.test.ts","-t","size_hint"],
  },
  {
    name: "mir-collect-map-refused",
    breaks: "`collect` into a map or a set is refused",
    file: "src/lower/mir/iter.rs",
    find: "        } else if self.is_map(output) {\n",
    replace: "        } else if false && self.is_map(output) {\n",
    tests: ["test/corpus.test.ts","-t","value_keys|user_hash|range_into_set"],
  },
  {
    name: "mir-made-array-iterator-from",
    breaks: "a `split`'s array MIR steps is `Iterator.from(…)`, so its chain stays a JS iterator's",
    file: "src/lower/mir/iter.rs",
    find: "        } else if made_array(&value) || self.steps_an_array(ty) {\n",
    replace: "        } else if self.steps_an_array(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_array_iter.rs$"],
    snapshots: true,
  },
  {
    name: "mir-rev-of-fresh-iter-rest",
    breaks: "`rev()` of a `split` just made is `$rest($iter(…))` reversed, not the array's",
    file: "src/lower/mir/iter.rs",
    find: "                _ => crate::lower::iterators::iterated(receiver),\n",
    replace: "                _ => crate::js::Expr::call_helper(\"$rest\", vec![receiver]),\n",
    tests: ["test/lowering.test.ts","-t","string's parts from MIR"],
  },
  {
    "name": "deque-iter-generic",
    "breaks": "a `VecDeque`'s iterator from MIR is `Iterator.from(prev)`, collected `.toArray()`, not `prev.slice()`, and its `next_back()` refused",
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
    "breaks": "a crate function given as a callback from MIR is `(item) => is_rule(item)`, not `is_rule`",
    "file": "src/lower/mir/iter.rs",
    "find": "            js::ExprKind::Arrow(params, _) if params.len() <= 1 => f,\n            js::ExprKind::Var(_) if own => f,\n",
    "replace": "            js::ExprKind::Arrow(params, _) if params.len() <= 1 => f,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a crate function given as a callback is passed as itself"
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
    "name": "mir-range-back-forward",
    "breaks": "`(0..n).rev().collect()` from MIR is `$range(0, n)`, forwards",
    "file": "src/lower/mir/iter.rs",
    "find": "        if range.back {\n",
    "replace": "        if !range.back {\n",
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
    "find": "        if !numbers || maps",
    "replace": "        if maps.is_empty() || !numbers || maps",
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
    "find": "            || self.recognition().is_str_chars(ty)\n",
    "replace": "",
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
    "find": "                it if std_method == Some(IterMethod::Rev) && crate::lower::iterators::is_stepped_items(&it) => {\n",
    "replace": "                it if false => {\n",
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
    "find": "(Some(IterMethod::Sum), true) => Expr::num(-0.0),",
    "replace": "(Some(IterMethod::Sum), true) => Expr::num(0.0),",
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
    "find": "            Some(method @ (IterMethod::Eq | IterMethod::Ne)) => {",
    "replace": "            Some(method @ (IterMethod::Eq | IterMethod::Ne)) if false => {",
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
    "find": "            || self.recognition().is_slice_iter_mut(ty)\n",
    "replace": "",
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
    "find": "                Some(n) => Expr::int(n + 1),\n                None => Expr::bin(js::Op::Add, range.end.clone(), Expr::int(1)),\n",
    "replace": "                Some(_) | None => Expr::bin(js::Op::Add, range.end.clone(), Expr::int(1)),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range mapped and collected"
    ]
  },
  {
    "name": "mir-user-len-refused",
    "breaks": "an iterator of the crate's `len()` from MIR is refused, not its `size_hint()`'s",
    "file": "src/lower/mir/iter.rs",
    "find": "            if self.is_user_iterator(iter_ty) && std_method == Some(IterMethod::Len) && !own_len()? {\n",
    "replace": "            if false && self.is_user_iterator(iter_ty) && std_method == Some(IterMethod::Len) && !own_len()? {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an iterator's len from MIR"
    ]
  },
  {
    "name": "mir-user-len-own-skipped",
    "breaks": "an iterator's own `len()` from MIR is std's of its `size_hint()`, not its own",
    "file": "src/lower/mir/iter.rs",
    "find": "&& std_method == Some(IterMethod::Len) && !own_len()? {\n",
    "replace": "&& std_method == Some(IterMethod::Len) {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an iterator's len from MIR"
    ]
  },
  {
    "name": "mir-counted-chain-len-refused",
    "breaks": "`xs.iter().skip(1).len()` from MIR is refused, not `xs.slice(1).length`",
    "file": "src/lower/mir/iter.rs",
    "find": "                (Some(IterMethod::Len), None) if counted_chain(&it) => {\n",
    "replace": "                (Some(IterMethod::Len), None) if false && counted_chain(&it) => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "str_bytes"
    ]
  },
  {
    "name": "mir-user-iterator-std-methods-refused",
    "breaks": "`map`, `fuse` or `collect` of the crate's iterator from MIR, which its impl takes from `Iterator`, is refused",
    "file": "src/lower/mir/iter.rs",
    "find": "            && (std_method == Some(IterMethod::Next) || self.recognition().user_impl_defines(trait_id, iter_ty, name));\n",
    "replace": ";\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "the crate's iterator from MIR"
    ]
  },
  {
    "name": "mir-user-iterator-own-method-std",
    "breaks": "what the crate's iterator writes itself, its own `count`, from MIR is std's",
    "file": "src/lower/mir/iter.rs",
    "find": "            && (std_method == Some(IterMethod::Next) || self.recognition().user_impl_defines(trait_id, iter_ty, name));\n",
    "replace": "            && std_method == Some(IterMethod::Next);\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "the crate's iterator from MIR"
    ]
  },
  {
    "name": "mir-user-by-ref-wrapped",
    "breaks": "`by_ref()` of the crate's iterator from MIR is a JS iterator over it, which its own `next` doesn't step",
    "file": "src/lower/mir/iter.rs",
    "find": "        if method == Some(IterMethod::ByRef) && self.is_user_iterator(iter_ty) {\n",
    "replace": "        if false && method == Some(IterMethod::ByRef) && self.is_user_iterator(iter_ty) {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "stepped through by_ref from MIR"
    ]
  },
  {
    "name": "mir-next-through-mut-refused",
    "breaks": "`next` of a `&mut` to the crate's iterator from MIR is refused, not its own",
    "file": "src/lower/mir/iter.rs",
    "find": "            && through.is_ref()\n",
    "replace": "            && through.is_ref() && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "stepped through by_ref from MIR"
    ]
  },
  {
    "name": "mir-collect-heap-unheaped",
    "breaks": "`collect()` into a `BinaryHeap` from MIR is its items as they came, not in heap order",
    "file": "src/lower/mir/iter.rs",
    "find": "        Ok(if self.is_std_type(output, StdItem::BinaryHeap) {\n",
    "replace": "        Ok(if false && self.is_std_type(output, StdItem::BinaryHeap) {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "heap_peek_mut"
    ]
  },
  {
    name: "mir-find-boxed-unboxed",
    breaks: "MIR's `find` of what looks like `None` gives it unboxed, `None` where it found `Some(None)`",
    file: "src/lower/mir/iter.rs",
    find: "                    (None, true) => Expr::call_helper(\"$nextSome\", vec![method(\"filter\", it, vec![test])]),\n",
    replace: "                    (None, true) => method(\"find\", it, vec![test]),\n",
    tests: ["test/corpus.test.ts", "-t", "lazy_chains"],
  },
  {
    name: "mir-next-back-tree",
    breaks: "MIR's `next_back()` of a B-tree's `iter()` is refused",
    file: "src/lower/mir/iter.rs",
    find: "            || self.recognition().is_tree_iter(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "iter_next_back"],
  },
  {
    name: "mir-next-back-split",
    breaks: "MIR's `next_back()` of a `str::split` is refused",
    file: "src/lower/mir/iter.rs",
    find: "            || self.is_str_split(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "iter_next_back"],
  },
  {
    name: "mir-next-back-peekable",
    breaks: "MIR's `next_back()` of a `Peekable` of an array's is refused",
    file: "src/lower/mir/iter.rs",
    find: "            || (self.recognition().is_peekable(iter_ty)\n",
    replace: "            || (false\n",
    tests: ["test/corpus.test.ts", "-t", "iter_next_back"],
  },
  {
    name: "mir-next-back-map-all",
    breaks: "MIR's `v.iter().map(f).next_back()` is refused",
    file: "src/lower/mir/iter.rs",
    find: "                        && crate::lower::iterators::is_items_map(&it)\n",
    replace: "                        && false\n",
    tests: ["test/lowering.test.ts", "-t", "next_back of a map"],
  },
  {
    name: "mir-filter-discarded-undropped",
    breaks: "MIR's `filter` of items with a destructor leaves those it discards undropped",
    file: "src/lower/mir/iter.rs",
    find: "                    Some(item) => self.dropping_discarded(test, item, false, span, made)?,\n",
    replace: "                    Some(_) => test,\n",
    tests: ["test/corpus.test.ts", "-t", "collect_drops"],
  },
  {
    name: "mir-fold-drops-unrequired",
    breaks: "MIR's generic `peekable` of an iterator its callers give a destructor drops it never",
    file: "src/lower/mir/iter.rs",
    find: "            self.require_consumed_drops_none(receiver, output);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses a destructor given where a generic peekable"],
  },
  {
    name: "mir-f64-max-wrapped",
    breaks: "MIR's `fold(0.0, f64::max)` gives `reduce` an arrow calling `$f64Max`",
    file: "src/lower/mir/iter.rs",
    find: "            _ if matches!(f.helper_name(), Some(\"$f64Max\" | \"$f64Min\")) => f,\n",
    replace: "",
    tests: ["test/traits.test.ts","-t","MIR dictionaries are explicit"],
  },
  {
    name: "mir-fold-own-of-init",
    breaks: "MIR's `fold` asks whether what it starts from is the crate's function, not its callback",
    file: "src/lower/mir/iter.rs",
    find: "        let own = arg_tys.get(2).is_some_and(|ty| self.is_crate_fn_item(*ty));\n        let mut args = vec![self.binary_callback(f, own), init];\n",
    replace: "        let own = arg_tys.get(1).is_some_and(|ty| self.is_crate_fn_item(*ty));\n        let mut args = vec![self.binary_callback(f, own), init];\n",
    tests: ["test/lowering.test.ts","-t","a fold given a function of the crate's"],
  },
  {
    name: "mir-rev-of-counted-refused",
    breaks: "MIR's `rev` of an array's items skipped and taken from is refused",
    file: "src/lower/mir/iter.rs",
    find: "            None if std_method == Some(IterMethod::Rev) && counted_chain(&receiver) => sliced(&receiver),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","rev of skipped and taken"],
  },
  {
    name: "mir-generic-iterator-unwrapped",
    breaks: "MIR's generic iterator is stepped as given, an array's `drop` missing",
    file: "src/lower/mir/iter.rs",
    find: "            None if matches!(ty.kind(), ty::Param(_)) && !is_js_iterator(&value) => Ok(self.js_iterator(value)),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","takes the crate's own from behind"],
  },
  {
    name: "mir-opaque-iterator-unrevealed",
    breaks: "MIR's iterator of the crate's behind `impl Iterator` is given as the struct, not a JS iterator",
    file: "src/lower/mir/iter.rs",
    find: "        let ty = self.reveal(ty);\n        match self.range_kind(ty) {\n",
    replace: "        match self.range_kind(ty) {\n",
    tests: ["test/lowering.test.ts","-t","takes the crate's own from behind"],
  },
  {
    name: "mir-len-closure-type-shown",
    breaks: "MIR's refused `len()` names its closures' types, where they're written",
    file: "src/lower/mir/iter.rs",
    find: "                        ty::Adt(adt, _) => self.tcx.def_path_str(adt.did()),\n",
    replace: "",
    tests: ["test/diagnostics.test.ts","-t","len of a chain whose closure prints"],
  },
  {
    name: "mir-rev-of-kept-iterator-wrapped",
    breaks: "`rev()` of an iterator a variable holds wraps its `$iter` again, `it.toReversed()`, which throws",
    file: "src/lower/mir/iter.rs",
    find: "            Some(_) if matches!(receiver.kind, js::ExprKind::Var(_) | js::ExprKind::Member(..)) => match std_method {\n",
    replace: "            Some(_) if false => match std_method {\n",
    tests: [
      "test/lowering.test.ts",
      "-t",
      "char ranges step by code point"
    ]
  },
  {
    "name": "mir-iterable-collected-lazy",
    "breaks": "a binding's list collected stays `Iterator.from(nodes).map(f).toArray()`, not `Array.from(nodes).map(f)`",
    "file": "src/lower/mir/iter.rs",
    "find": "None => match self.iterable_items(&it, iter_ty) {",
    "replace": "None => match None::<Expr> {",
    "tests": [
      "test/compiler.test.ts",
      "-t",
      "methods are their type's object|webapi crate's bindings"
    ]
  },
  {
    "name": "mir-iterable-collected-generic",
    "breaks": "a generic iterator collected is `Array.from(items)`, empty of an iterator with only a `next`",
    "file": "src/lower/mir/iter.rs",
    "find": "        if matches!(source.kind(), ty::Param(_) | ty::Alias(..)) {\n            return None;\n        }\n        collected(",
    "replace": "        collected(",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "only a next"
    ]
  },
  {
    "name": "identity-map-step",
    "breaks": "a map by an untagged variant constructor stays a step, `.map((value) => value)`",
    "file": "src/lower/mir/iter.rs",
    "find": "                f if f.is_identity() => it,\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "gives the value itself is none"
    ]
  },
  {
    name: "fresh-iter-len-stepped",
    breaks: "`v.iter().len()` is what's left of a `$iter` made for it, `$iter(v).end - $iter(v).at`, not `v.length`",
    file: "src/lower/mir/iter.rs",
    find: "        if crate::lower::iterators::is_stepped_items(&it)\n",
    replace: "        if false\n",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
    snapshots: true,
  },
  {
    name: "args-not-stepped",
    breaks: "`env::args()` held is stepped as a JS iterator, which knows no length, not a `$iter` of its array",
    file: "src/lower/mir/iter.rs",
    find: "            || self.recognition().is_process_items(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
    snapshots: true,
  },
  {
    name: "given-iterator-next-as-js",
    breaks: "`next` of an adapter's `I` steps it as a JS iterator, not by its dictionary, which a range isn't",
    file: "src/lower/mir/iter.rs",
    find: "            IterMethod::Next => Some(Expr::call(Expr::member(dictionary, \"next\"), vec![receiver])),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-iterator-into-iter-as-js",
    breaks: "a `for` over an adapter's `I`, or `by_ref()` of it, makes a JS iterator of it, `Iterator.from` of a range",
    file: "src/lower/mir/iter.rs",
    find: "            IterMethod::ByRef | IterMethod::IntoIter => Some(receiver),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-iterator-adapted-as-js",
    breaks: "an adapter's `I` summed is `Iterator.from` of it, not stepped by its dictionary",
    file: "src/lower/mir/iter.rs",
    find: "            None if let Some(dictionary) = self.iterator_evidence(ty) => {\n",
    replace: "            None if let Some(dictionary) = self.iterator_evidence(ty).filter(|_| false) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-iterator-unboxed",
    breaks: "an adapter's `I` of a generic item, made a JS iterator, keeps its `Some` boxed",
    file: "src/lower/mir/iter.rs",
    find: "                let boxed = self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));\n",
    replace: "                let boxed = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "consumed-iterator-undropped",
    breaks: "an iterator of the crate's with a destructor, consumed by `collect()`, is never dropped, so what its drop does doesn't happen",
    file: "src/lower/mir/iter.rs",
    find: "            self.drop_value(held, receiver_ty, span, made)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_drop_consumed.rs$"],
  },
  {
    name: "adapter-iterator-dropped",
    breaks: "an adapter of an iterator with a destructor, `scan(..)`, drops it as soon as it's made, though the adapter holds it",
    file: "src/lower/mir/iter.rs",
    find: "        let holds = output.walk().any(|part| part.as_type() == Some(receiver));\n",
    replace: "        let holds = false;\n",
    tests: ["test/diagnostics.test.ts", "-t", "an adapter of an iterator with a destructor"],
  },
  {
    name: "projection-item-not-an-iterator",
    breaks: "an iterator whose `Item` is another's associated type, `A::Item`, isn't taken as one, and `for _ in &mut it` is refused",
    file: "src/lower/mir/iter.rs",
    find: "                .is_some_and(|tr| !iterator_traits.contains(&tr)),\n",
    replace: "                .is_some_and(|_| false),\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_of_projection.rs$"],
  },
  {
    name: "size-hint-of-projection-refused",
    breaks: "`size_hint()` of a generic `I::IntoIter` is refused, where a type parameter's is its JS value's",
    file: "src/lower/mir/iter.rs",
    find: "        self.array_source(stepped).is_some() || self.is_unknown(stepped)\n",
    replace: "        self.array_source(stepped).is_some() || matches!(stepped.kind(), ty::Param(_))\n",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_into_iter.rs$"],
  },
  {
    name: "double-ended-next-back-refused",
    breaks: "`next_back()` of a generic `T: DoubleEndedIterator` is refused",
    file: "src/lower/mir/iter.rs",
    find: "            || self.knows_its_back(iter_ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "double-ended-rev-refused",
    breaks: "`rev()` of a generic `T: DoubleEndedIterator` is refused",
    file: "src/lower/mir/iter.rs",
    find: "            if std_method == Some(IterMethod::Rev) && self.knows_its_back(iter_ty) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "last-of-destructors-uncounted",
    breaks: "`last()` of items with a destructor collects them, and drops none it passes",
    file: "src/lower/mir/iter.rs",
    find: "            Some(drop) => Ok(Expr::call_helper(\"$lastDropping\", vec![it, drop])),\n",
    replace: "            Some(_) => Ok(self.last_of(Expr::call(Expr::member(it, \"toArray\"), Vec::new()), item, made)),\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
  {
    name: "take-of-owned-refused",
    breaks: "`take` of an iterator that owns items with a destructor is refused",
    file: "src/lower/mir/iter.rs",
    find: "        if method.is_some_and(IterMethod::holds_its_source) {\n",
    replace: "        if false && method.is_some_and(IterMethod::holds_its_source) {\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
  {
    name: "take-of-lent-refused",
    breaks: "`by_ref().take(n)` of items with a destructor is refused, where what it hasn't given is its lender's",
    file: "src/lower/mir/iter.rs",
    find: "                true => Ok(it),\n                false => self.closing(it, iter_ty, span),\n",
    replace: "                true => Err(self.unsupported(span, \"a lent adapter\")),\n                false => self.closing(it, iter_ty, span),\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
  {
    name: "skip-of-owned-unskipped",
    breaks: "`skip` of an iterator that owns items with a destructor is JS's `drop`, which drops none it skips",
    file: "src/lower/mir/iter.rs",
    find: "                Expr::call_helper(\"$skipDropping\", vec![it, n, drop])\n",
    replace: "                {\n                    let _ = drop;\n                    Expr::call(Expr::member(it, \"drop\"), vec![n])\n                }\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
  {
    "name": "rev-chain-refused",
    "breaks": "`rev()` of a lazy chain over a slice, `v.iter().map(f).rev()`, is refused, though its closures run in the same order over the items reversed",
    "file": "src/lower/mir/iter.rs",
    "find": "                && let Some(reversed) = reversed_inward(&receiver) =>\n",
    "replace": "                && let Some(reversed) = None::<Expr> =>\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "rev_of_lazy_chains"
    ]
  },
  {
    "name": "rev-chain-index-from-back",
    "breaks": "an `enumerate().rev()`'s index is counted over the items reversed, `(2, 5)` where it's `(0, 5)`",
    "file": "src/lower/mir/iter.rs",
    "find": "        js::ExprKind::Arrow(params, _) => params.len() == 1,\n",
    "replace": "        js::ExprKind::Arrow(..) => true,\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "rev_of_lazy_chains"
    ]
  },
  {
    "name": "fold-start-left-out",
    "breaks": "`fold(None, f)`'s `reduce(f, undefined)` loses its start, so an empty slice's throws and another's starts from its first item",
    "file": "src/lower/mir/iter.rs",
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
    "file": "src/lower/mir/iter.rs",
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
    "file": "src/lower/mir/iter.rs",
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
    "file": "src/lower/mir/iter.rs",
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
    file: "src/lower/mir/iter.rs",
    find: "            && let Some(dictionary) = self.collected_evidence(output, item)\n",
    replace: "            && let Some(dictionary) = None::<Expr>\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect.rs$"],
  },
  {
    name: "find-dropping-undropped",
    breaks: "`find` of items with a destructor drops none it passes over",
    file: "src/lower/mir/iter.rs",
    find: "                    (Some(item), _) => self.dropping_pick(\"$findDropping\", it, test, item, span)?,\n",
    replace: "                    (Some(_), _) => method(\"find\", it, vec![test]),\n",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
  {
    name: "nth-dropping-refused",
    breaks: "`nth` of items with a destructor is refused",
    file: "src/lower/mir/iter.rs",
    find: "                    | IterMethod::Nth\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
  {
    name: "rev-iterator-from-unreversed",
    breaks: "`rev()` of a chain over a string's `char_indices()`, writeable's, is refused",
    file: "src/lower/mir/iter.rs",
    find: "    if let Ok(source) = crate::lower::iterators::from_iterator(e.clone()) {\n",
    replace: "    if let Ok(source) = Err::<Expr, Expr>(e.clone()) {\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_of_char_indices.rs$"],
  },
  {
    name: "filter-map-method-leaves",
    breaks: "`filter_map` of items with a destructor, a method of a chain, is refused",
    file: "src/lower/mir/iter.rs",
    find: "        if keeps_each || crate::lower::recognition::leaves_none(name) {\n",
    replace: "        if keeps_each {\n",
    tests: ["test/corpus.test.ts", "-t", "^store_get_mut.rs$"],
  },
  {
    name: "back-step-receiver-wrapped",
    breaks: "`nth_back` and `rfold` of an iterator that knows its back wrap it again, `$nthBack($iter(it), ..)`, which `$iter` gives back as it is (ADR 0456)",
    file: "src/lower/mir/iter.rs",
    find: "        let it = match self.range_kind(iter_ty).is_none() && self.has_a_back(iter_ty) {\n",
    replace: "        let it = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
    snapshots: true,
  },
  {
    name: "back-step-unboxed",
    breaks: "`nth_back` of items that could look like `None` gives `None` for a `Some(None)`",
    file: "src/lower/mir/iter.rs",
    find: "                true => Expr::helper(\"$some\"),\n",
    replace: "                true => Expr::undefined(),\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "back-step-drops-undone",
    breaks: "what `nth_back` and `rfind` pass over of items with a destructor isn't dropped",
    file: "src/lower/mir/iter.rs",
    find: "        given.extend(drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^back_steps_drop.rs$"],
  },
  {
    name: "by-ref-back-lost",
    breaks: "`by_ref()` of an iterator that knows its back is lent without it, so a map of it can't be stepped from its back",
    file: "src/lower/mir/iter.rs",
    find: "            Some(IterMethod::ByRef) if self.knows_its_back(iter_ty) => {\n",
    replace: "            Some(IterMethod::ByRef) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "known-length-as-array",
    breaks: "`len()` of a generic iterator that knows its back and its length reads it as an array's, `it.end - it.at`, which a range's has not",
    file: "src/lower/mir/iter.rs",
    find: "                (Some(IterMethod::Len), None) if self.knows_its_length(iter_ty) => {\n",
    replace: "                (Some(IterMethod::Len), None) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^exact_size_len.rs$"],
  },
  {
    name: "from-iter-unlowered",
    breaks: "`Vec::from_iter(items)`, which `items.collect()` is, is refused",
    file: "src/lower/mir/iter.rs",
    find: "            return self.collected_from(values, receiver_ty, output, span);\n",
    replace: "            return Ok(None);\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_called.rs$"],
  },
  {
    name: "from-iter-array-stepped",
    breaks: "`String::from_iter` of an array steps it as an iterator, which an array isn't, and JS throws",
    file: "src/lower/mir/iter.rs",
    find: "            return Ok(Some(self.collected_into(items, output, item, None, span)?));\n",
    replace: "            return Ok(Some(self.collected_into(items, output, item, Some(iter_ty), span)?));\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_called.rs$"],
  },
  {
    name: "from-iter-of-anything",
    breaks: "`Vec::from_iter` of an `Option` is collected as if it were an iterator",
    file: "src/lower/mir/iter.rs",
    find: "        if !matches!(given.kind(), ty::Param(_)) && !self.implements_iterator(given) {\n            return Ok(None);\n        }\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "from_iter of an Option"],
  },
  {
    name: "try-fold-unlowered",
    breaks: "`try_fold` is refused",
    file: "src/lower/mir/iter.rs",
    find: "                self.try_folded(it, init, f, (arg_tys, output), span)?\n",
    replace: "                return Ok(None);\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-result-as-option",
    breaks: "`try_fold` into a `Result` stops at a `None`, which an `Ok` never is, and goes on past an `Err`",
    file: "src/lower/mir/iter.rs",
    find: "            None if self.recognition().is_result(output) => \"$tryFoldOk\",\n",
    replace: "            None if self.recognition().is_result(output) => \"$tryFoldSome\",\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-boxed-some",
    breaks: "`try_fold` into an `Option<()>`, whose `Some` is boxed, is stepped as one whose `Some` is its value",
    file: "src/lower/mir/iter.rs",
    find: "            Some(made) if !self.boxed_payload(made) => \"$tryFoldSome\",\n",
    replace: "            Some(_) => \"$tryFoldSome\",\n",
    tests: ["test/diagnostics.test.ts", "-t", "try_fold into a boxed Option"],
  },
  {
    name: "try-fold-leaves-items-refused",
    breaks: "`try_fold` of items with a destructor is refused, though it hands each to its closure and leaves the rest to the iterator",
    file: "src/lower/mir/iter.rs",
    find: "                    | IterMethod::TryFold\n                    | IterMethod::Collect\n",
    replace: "                    | IterMethod::Collect\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-lent",
    breaks: "`try_fold` of a `&mut` is given `$lent(it)`, which its helper, closing nothing, needn't be",
    file: "src/lower/mir/iter.rs",
    find: " && std_method != Some(IterMethod::TryFold);\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
    snapshots: true,
  },
  {
    name: "lazy-peekable-refused",
    breaks: "`peekable()` of a lazy iterator is refused",
    file: "src/lower/mir/iter.rs",
    find: "                Err(lazy) if std_method == Some(IterMethod::Peekable) => {\n",
    replace: "                Err(lazy) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_lazy.rs$"],
  },
  {
    name: "rev-range-chain-refused",
    breaks: "`rev()` of a lazy chain over a range is refused",
    file: "src/lower/mir/iter.rs",
    find: "    if let Some(range) = crate::lower::iterators::stepped_range(e)\n        && !range.back\n    {\n",
    replace: "    if let Some(range) = crate::lower::iterators::stepped_range(e)\n        && false\n    {\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_range.rs$"],
  },
  {
    name: "rev-range-chain-inclusive-as-exclusive",
    breaks: "`rev()` of a chain over `a..=b` steps it from `b - 1`, leaving `b` out",
    file: "src/lower/mir/iter.rs",
    find: "            true => crate::lower::std_types::range::RangeKind::Inclusive,\n",
    replace: "            true => crate::lower::std_types::range::RangeKind::Exclusive,\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_range.rs$"],
  },
  {
    name: "consumed-generic-refused",
    breaks: "generic code that consumes an iterator its caller gives a drop of, `items.into_iter().last()`, takes none, and a caller's destructor is refused",
    file: "src/lower/mir/iter.rs",
    find: "            true => self.given_drop(receiver).is_some(),\n",
    replace: "            true => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^extreme_drops.rs$"],
  },
  {
    name: "max-min-leave-drops",
    breaks: "`max()` and `min()` of items with a destructor are refused, though they drop each they pass over",
    file: "src/lower/mir/iter.rs",
    find: "                    | IterMethod::Max\n                    | IterMethod::Min\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extreme_drops.rs$"],
  },
  {
    name: "crate-iterator-unclosed",
    breaks: "a crate's iterator with a destructor an adapter holds isn't closed by its own drop",
    file: "src/lower/mir/iter.rs",
    find: "                    || self.is_user_iterator(ty) && self.drops(ty) != crate::lower::drops::Drops::Nothing\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "eq-other-not-closing",
    breaks: "`eq`'s other side that owns items with a destructor isn't closed to drop what's left",
    file: "src/lower/mir/iter.rs",
    find: "            Some(drop) => Expr::call_helper(\"$closing\", vec![self.js_iterator(other), drop]),\n",
    replace: "            Some(_) => other,\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "fuse-as-itself",
    breaks: "`fuse()` of the crate's iterator is it, which starts again after its `None`",
    file: "src/lower/mir/iter.rs",
    find: "            Some(IterMethod::Fuse) => self.fused(it, iter_ty, span)?,\n",
    replace: "            Some(IterMethod::Fuse) => it,\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
  },
  {
    name: "fuse-size-hint-refused",
    breaks: "`size_hint()` of a `Fuse` is refused",
    file: "src/lower/mir/iter.rs",
    find: "        let stepped = self.recognition().fuse_of(iter_ty).unwrap_or(iter_ty);\n",
    replace: "        let stepped = iter_ty;\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
  },
  {
    name: "held-source-own-drop-unclosed",
    breaks: "an adapter of the crate's iterator with a `Drop`, whose items have none, never closes it, so it's never dropped",
    file: "src/lower/mir/iter.rs",
    find: "        if method.is_some_and(IterMethod::holds_its_source) {\n",
    replace: "        if dropping.is_some() && method.is_some_and(IterMethod::holds_its_source) {\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
  },
];
