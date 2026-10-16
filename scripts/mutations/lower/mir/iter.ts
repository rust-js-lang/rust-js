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
    find: "            if matches!(receiver_ty.kind(), ty::Param(_)) {\n",
    replace: "            if false && matches!(receiver_ty.kind(), ty::Param(_)) {\n",
    tests: ["test/lowering.test.ts","-t","stepped by hand"],
  },
  {
    name: "mir-rev-items-forward",
    breaks: "`rev()` of items made already gives them in order",
    file: "src/lower/mir/iter.rs",
    find: "                        Some(IterMethod::Rev) => Expr::call(Expr::member(items, \"toReversed\"), Vec::new()),\n",
    replace: "                        Some(IterMethod::Rev) => items,\n",
    tests: ["test/corpus.test.ts","-t","linked_lists"],
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
    find: "                } else if self.is_map(output) {\n",
    replace: "                } else if false && self.is_map(output) {\n",
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
    find: "                            _ => crate::lower::iterators::iterated(receiver),\n",
    replace: "                            _ => crate::js::Expr::call_helper(\"$rest\", vec![receiver]),\n",
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
    "find": "                            it if std_method == Some(IterMethod::Rev)\n                                && crate::lower::iterators::is_stepped_items(&it) =>\n",
    "replace": "                            it if false =>\n",
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
    "find": "            Some(IterMethod::Eq | IterMethod::Ne) => {",
    "replace": "            Some(IterMethod::Eq | IterMethod::Ne) if false => {",
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
    "name": "mir-from-iter-string-refused",
    "breaks": "`String::from_iter(items)` from MIR is refused",
    "file": "src/lower/mir/iter.rs",
    "find": "        if std_method == Some(IterMethod::FromIter) && self.is_lang_adt(output, LangItem::String) {\n",
    "replace": "        if std_method == Some(IterMethod::FromIter) && false {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a string from items from MIR"
    ]
  },
  {
    "name": "mir-from-iter-iterator-joined",
    "breaks": "`String::from_iter` of an iterator from MIR calls its `join`, which a JS iterator has none of",
    "file": "src/lower/mir/iter.rs",
    "find": "            let items = match given.is_array() || given.is_slice() || self.is_vec_like(given) {\n",
    "replace": "            let items = match true {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a string from items from MIR"
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
    "find": "                if self.is_std_type(output, StdItem::BinaryHeap) {\n",
    "replace": "                if false && self.is_std_type(output, StdItem::BinaryHeap) {\n",
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
    find: "                    true => Expr::call_helper(\"$nextSome\", vec![method(\"filter\", it, vec![test])]),\n",
    replace: "                    true => method(\"find\", it, vec![test]),\n",
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
    find: "                || (self.recognition().is_peekable(iter_ty)\n",
    replace: "                || (false\n",
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
    breaks: "MIR's generic `fold` of an iterator its callers give a destructor drops it never",
    file: "src/lower/mir/iter.rs",
    find: "                self.require_no_drops(receiver_ty);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses a destructor given where a generic fold"],
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
    find: "                let own = arg_tys.get(2).is_some_and(|ty| self.is_crate_fn_item(*ty));\n",
    replace: "                let own = arg_tys.get(1).is_some_and(|ty| self.is_crate_fn_item(*ty));\n",
    tests: ["test/lowering.test.ts","-t","a fold given a function of the crate's"],
  },
  {
    name: "mir-rev-of-counted-refused",
    breaks: "MIR's `rev` of an array's items skipped and taken from is refused",
    file: "src/lower/mir/iter.rs",
    find: "                        None if std_method == Some(IterMethod::Rev) && counted_chain(&receiver) => sliced(&receiver),\n",
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
    find: "                        Some(_) if matches!(receiver.kind, js::ExprKind::Var(_) | js::ExprKind::Member(..)) => {\n",
    replace: "                        Some(_) if false => {\n",
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
    find: "                self.drop_value(held, receiver_ty, span, made)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_drop_consumed.rs$"],
  },
  {
    name: "adapter-iterator-dropped",
    breaks: "an adapter of an iterator with a destructor, `map(f)`, drops it as soon as it's made, though the adapter holds it",
    file: "src/lower/mir/iter.rs",
    find: "                if output.walk().any(|part| part.as_type() == Some(receiver_ty)) {\n",
    replace: "                if false {\n",
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
    find: "                false => self.array_source(iter_ty).is_some() || self.is_unknown(iter_ty),\n",
    replace: "                false => self.array_source(iter_ty).is_some() || matches!(iter_ty.kind(), ty::Param(_)),\n",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_into_iter.rs$"],
  },
];
