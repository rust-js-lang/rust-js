// Mutations of src/lower/mir/std_calls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-comb-subject-dropped",
    breaks: "an `Option`'s combinator from MIR is given its argument where its subject goes",
    file: "src/lower/mir/std_calls.rs",
    find: "                self.comb_values(comb, (exprs, arg_tys), made, generic_args, span, out)?\n",
    replace: "                self.comb_values(comb, (exprs[1..].to_vec(), &arg_tys[1..]), made, generic_args, span, out)?\n",
    tests: ["test/corpus.test.ts","-t","option_as_deref|combinators_owned_drops"],
  },
  {
    name: "mir-map-call-refused",
    breaks: "a map's method from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                self.map_values(op, (exprs, arg_tys), generic_args, span, out)?\n",
    replace: "                return Err(self.unsupported(span, \"a map's method\"));\n",
    tests: ["test/corpus.test.ts","-t","map_index_missing"],
  },
  {
    name: "mir-radix-argument-unseen",
    breaks: "a `{:x}` argument from MIR isn't one `format_args!` shows by its spec",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::FmtDisplay | Std::FmtDebug | Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {\n",
    replace: "            Std::FmtDisplay | Std::FmtDebug | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {\n",
    tests: ["test/corpus.test.ts","-t","radix_negative"],
  },
  {
    name: "mir-sort-unsorted",
    breaks: "`v.sort()` from MIR leaves the items as they were",
    file: "src/lower/mir/std_calls.rs",
    find: "                self.sort_values(key, items, arg_tys[0], generic_args, span, out)?\n",
    replace: "                items\n",
    tests: ["test/corpus.test.ts","-t","generic_borrow"],
  },
  {
    name: "mir-view-whole",
    breaks: "`&mut v[a..b]` from MIR is all of `v`, not a view of those items",
    file: "src/lower/mir/std_calls.rs",
    find: "                    self.view_call(if checked { \"$viewGet\" } else { \"$view\" }, list)\n",
    replace: "                    list.remove(0)\n",
    tests: ["test/corpus.test.ts","-t","mut_ref_loop_bounds"],
  },
  {
    name: "mir-rc-call-refused",
    breaks: "an `Rc`'s function from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                self.rc_values(op, (exprs, arg_tys), span, out)?\n",
    replace: "                return Err(self.unsupported(span, \"an Rc's\"));\n",
    tests: ["test/corpus.test.ts","-t","as_deref_counted"],
  },
  {
    name: "mir-format-width-dropped",
    breaks: "`{:5?}` from MIR pads nothing",
    file: "src/lower/mir/std_calls.rs",
    find: "                        None => spec.width.map(|w| Expr::int(w.into())),\n",
    replace: "                        None => None,\n",
    tests: ["test/corpus.test.ts","-t","debug_options"],
  },
  {
    name: "mir-format-precision-dropped",
    breaks: "`{:.1?}` from MIR shows every digit",
    file: "src/lower/mir/std_calls.rs",
    find: "                        None => spec.precision.map(|p| Expr::int(p.into())),\n",
    replace: "                        None => None,\n",
    tests: ["test/corpus.test.ts","-t","debug_options"],
  },
  {
    name: "mir-format-width-argument-dropped",
    breaks: "`{:>w$}` from MIR pads nothing",
    file: "src/lower/mir/std_calls.rs",
    find: "                    let width = match spec.width_from {\n                        Some(from) => Some(item(from).ok_or_else(bad)?.2),\n",
    replace: "                    let width = match spec.width_from {\n                        Some(_) => None,\n",
    tests: ["test/corpus.test.ts","-t","format_width_arguments"],
  },
  {
    name: "mir-to-int-unchecked-uncast",
    breaks: "`x.to_int_unchecked()` under MIR reaches `number_call`, which has none",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Number(NumOp::ToIntUnchecked) => {\n",
    replace: "            Std::Number(NumOp::ToIntUnchecked) if false => {\n",
    tests: ["test/corpus.test.ts","-t","integers_128"],
  },
  {
    name: "mir-peek-steps",
    breaks: "`p.peek()` of a `Peekable` steps it",
    file: "src/lower/mir/std_calls.rs",
    find: "StepOp::Peek => Expr::call_helper(\"$peek\", vec![it]),",
    replace: "StepOp::Peek => Expr::call_helper(\"$next\", vec![it]),",
    tests: ["test/corpus.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-next-if-eq-unequal",
    breaks: "`p.next_if_eq(&x)` takes what isn't `x`",
    file: "src/lower/mir/std_calls.rs",
    find: "Expr::bin(Op::Eq, Expr::var(\"item\"), x)",
    replace: "Expr::bin(Op::Ne, Expr::var(\"item\"), x)",
    tests: ["test/corpus.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-default-size-hint-one",
    breaks: "std's `size_hint()` of an iterator of the crate's is `(1, None)`",
    file: "src/lower/mir/std_calls.rs",
    find: "Expr::array(vec![Expr::int(0), Expr::undefined()])",
    replace: "Expr::array(vec![Expr::int(1), Expr::undefined()])",
    tests: ["test/corpus.test.ts","-t","size_hint"],
  },
  {
    name: "mir-copies-shared",
    breaks: "`vec![p; 3]` of a struct is three names for one object",
    file: "src/lower/mir/std_calls.rs",
    find: "self.copies_values((item, arg_tys[0]), n, span, out)?",
    replace: "self.copies_values((item, tcx.types.unit), n, span, out)?",
    tests: ["test/corpus.test.ts","-t","copy_mutation"],
  },
  {
    name: "mir-non-zero-new-refused",
    breaks: "`NonZero::new(n)` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::NonZeroNew => {\n",
    replace: "            Std::NonZeroNew if false => {\n",
    tests: ["test/corpus.test.ts","-t","nonzero"],
  },
  {
    name: "mir-string-edit-refused",
    breaks: "`s.insert(i, c)` and the rest of a string's edits are refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::StringEdit(edit) => {\n",
    replace: "            Std::StringEdit(edit) if false => {\n",
    tests: ["test/corpus.test.ts","-t","text_edits|string_editing"],
  },
  {
    name: "mir-string-range-swapped",
    breaks: "`s.replace_range(a..b, t)` takes its range's end for its start",
    file: "src/lower/mir/std_calls.rs",
    find: "                    given.splice(0..0, [start, end.unwrap_or_else(Expr::undefined)]);\n",
    replace: "                    given.splice(0..0, [end.unwrap_or_else(Expr::undefined), start]);\n",
    tests: ["test/corpus.test.ts","-t","text_edits|string_editing|replace_range_panic|string_drain_from_past_len"],
  },
  {
    name: "mir-with-capacity-refused",
    breaks: "`String::with_capacity(n)` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::StringWithCapacity => {\n",
    replace: "            Std::StringWithCapacity if false => {\n",
    tests: ["test/corpus.test.ts","-t","text_edits|string_editing|text_aliases|string_insert_boundary"],
  },
  {
    name: "mir-make-mut-refused",
    breaks: "`Rc::make_mut(&mut rc)` of a local `Rc` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Rc(RcOp::MakeMut) => {\n",
    replace: "            Std::Rc(RcOp::MakeMut) if false => {\n",
    tests: ["test/corpus.test.ts","-t","rc_clone_sees_owners|rc_make_mut_clone_panics"],
  },
  {
    name: "mir-type-name-refused",
    breaks: "`type_name::<T>()` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::TypeName { .. } => {\n",
    replace: "            Std::TypeName { .. } if false => {\n",
    tests: ["test/corpus.test.ts","-t","type_facts"],
  },
  {
    "name": "mir-format-lines-flattened",
    "breaks": "a format string MIR's template has, written across lines, is a template of `\\n`s",
    "file": "src/lower/mir/std_calls.rs",
    "find": "let lines = bytes.contains(&b'\\n') && self.format_string_across_lines(span);",
    "replace": "let lines = false;",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a format string written across lines keeps its lines"
    ]
  },
  {
    "name": "mir-chars-as-str-refused",
    "breaks": "`c.as_str()` of a string's `chars()` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "                if op == StepOp::AsStr\n",
    "replace": "                if false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "the rest of a string's chars from MIR"
    ]
  },
  {
    "name": "mir-tree-range-refused",
    "breaks": "a B-tree's `range` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Map(MapOp::TreeRange { mutable }) => {",
    "replace": "            Std::Map(MapOp::TreeRange { mutable }) if mutable => {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a B-tree's range from MIR"
    ]
  },
  {
    "name": "mir-copy-within-refused",
    "breaks": "`v.copy_within(a..b, at)` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Slice(SliceOp::CopyWithin) => {\n",
    "replace": "            Std::Slice(SliceOp::CopyWithin) if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "slice_methods"
    ]
  },
  {
    "name": "mir-vec-extract-if-refused",
    "breaks": "`v.extract_if(.., f)` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Slice(SliceOp::ExtractIf { range }) => {\n",
    "replace": "            Std::Slice(SliceOp::ExtractIf { range }) if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "extract_if"
    ]
  },
  {
    "name": "mir-map-extract-if-refused",
    "breaks": "a map's `extract_if(f)` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Map(MapOp::ExtractIf) => {\n",
    "replace": "            Std::Map(MapOp::ExtractIf) if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "extract_if"
    ]
  },
  {
    "name": "mir-remainder-made-stepped",
    "breaks": "a chunks' `remainder()` of one made there from MIR is `$iter(..).items.remainder`, not the array's",
    "file": "src/lower/mir/std_calls.rs",
    "find": "                if let Some(items) = made {\n                    exprs[0] = items;\n                }\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a chunks' remainder from MIR"
    ]
  },
  {
    "name": "mir-remainder-stepped-plain",
    "breaks": "a stepped chunks' `remainder()` from MIR reads `.remainder` of its `$iter`, which has none",
    "file": "src/lower/mir/std_calls.rs",
    "find": "                let stepping = made.is_none();\n",
    "replace": "                let stepping = false;\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a chunks' remainder from MIR"
    ]
  },
  {
    "name": "mir-slice-views-refused",
    "breaks": "a slice op giving a `&mut` view of items from MIR, `split_at_mut`, is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "if !inner.is_slice() && !inner.is_array())",
    "replace": ")",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "split_at_mut_past_len"
    ]
  },
  {
    "name": "mir-cell-giving-slice-refused",
    "breaks": "`get_disjoint_mut` from MIR, which makes its items' boxes itself, is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "                ) && (known.gives_its_cell() || !output.walk().any(|arg| {\n",
    "replace": "                ) && (false || !output.walk().any(|arg| {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "slice_disjoint"
    ]
  },
  {
    "name": "mir-push-str-place-made-twice",
    "breaks": "`v.push_mut(s).push('b')` from MIR reads and writes `$pushMut(v, s, true).value`, pushing twice",
    "file": "src/lower/mir/std_calls.rs",
    "find": "                    js::ExprKind::Member(object, field) if !object.reads_same() => {\n",
    "replace": "                    js::ExprKind::Member(object, field) if false && !object.reads_same() => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "vec_push_mut"
    ]
  },
  {
    "name": "mir-deque-range-mut-refused",
    "breaks": "a deque's `range_mut(a..b)` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Slice(SliceOp::DequeRangeMut) => {\n",
    "replace": "            Std::Slice(SliceOp::DequeRangeMut) if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "deque_range_mut"
    ]
  },
  {
    "name": "mir-deque-range-mut-unboxed",
    "breaks": "a deque's `range_mut` of numbers from MIR gives the numbers, not boxes on them, so writing one changes nothing",
    "file": "src/lower/mir/std_calls.rs",
    "find": "                match self.is_boxable(generic_args.type_at(0)) {\n                    true => Expr::call_helper(\"$mutItems\", vec![view]),\n",
    "replace": "                match false {\n                    true => Expr::call_helper(\"$mutItems\", vec![view]),\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "deque_range_mut"
    ]
  },
  {
    "name": "mir-tree-range-mut-refused",
    "breaks": "a B-tree's `range_mut` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Map(MapOp::TreeRange { mutable }) => {\n",
    "replace": "            Std::Map(MapOp::TreeRange { mutable }) if !mutable => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "btree_range_mut"
    ]
  },
  {
    "name": "mir-range-edit-refused",
    "breaks": "`v.extend_from_within(a..b)` and `v.splice(a..b, items)` from MIR are refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Text(op @ (TextOp::Splice | TextOp::ExtendFromWithin)) => {\n",
    "replace": "            Std::Text(op @ (TextOp::Splice | TextOp::ExtendFromWithin)) if false => {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a vec's range pushed again or replaced"
    ]
  },
  {
    "name": "mir-option-place-refused",
    "breaks": "`o.get_or_insert(x)` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::OptionPlace(op) => {\n",
    "replace": "            Std::OptionPlace(op) if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "option_result_methods"
    ]
  },
  {
    "name": "mir-cow-refused",
    "breaks": "a `Cow`'s `into_owned()` or `to_mut()` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Cow(op) => {\n",
    "replace": "            Std::Cow(op) if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "cows"
    ]
  },
  {
    "name": "mir-heap-refused",
    "breaks": "a `BinaryHeap`'s `append` or `peek_mut` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Heap(op) => {\n",
    "replace": "            Std::Heap(op) if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "heap_methods|heap_peek_mut"
    ]
  },
  {
    "name": "mir-wrapping-op-refused",
    "breaks": "`a * Wrapping(2)` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::WrappingOp(op, assign) => {\n",
    "replace": "            Std::WrappingOp(op, assign) if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "wrapping_type"
    ]
  },
  {
    "name": "mir-stream-refused",
    "breaks": "`writeln!(io::stderr(), ..)` from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::Stream(op) => {\n",
    "replace": "            Std::Stream(op) if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "std_odds"
    ]
  },
  {
    "name": "mir-user-write-refused",
    "breaks": "`writeln!(out, ..)` to a writer of the crate's own from MIR is refused",
    "file": "src/lower/mir/std_calls.rs",
    "find": "            Std::UserWrite => {\n",
    "replace": "            Std::UserWrite if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "user_fmt_write"
    ]
  },
  {
    name: "mir-split-off-unwritten",
    breaks: "MIR's `s.split_off(..n)` leaves `s` as it was",
    file: "src/lower/mir/std_calls.rs",
    find: "                out.push(StmtKind::Assign(place, left).at(self.js_span(span)));\n                Expr::index(split, Expr::int(1))\n",
    replace: "                Expr::index(split, Expr::int(1))\n",
    tests: ["test/corpus.test.ts", "-t", "slice_split_off"],
  },
  {
    name: "mir-split-off-held",
    breaks: "MIR's `r.split_off_mut(..n)`, `r` a `&mut` to `rest`, gives `r` what's left",
    file: "src/lower/mir/std_calls.rs",
    find: "                        if matches!(place.kind, js::ExprKind::Var(_)) && self.borrows_variable(def_id, span, 0) =>\n",
    replace: "                        if matches!(place.kind, js::ExprKind::Var(_)) =>\n",
    tests: ["test/lowering.test.ts", "-t", "split_off through a held"],
  },
  {
    name: "mir-uninit-drop-skipped",
    breaks: "MIR's `assume_init_drop()` of a slice drops nothing",
    file: "src/lower/mir/std_calls.rs",
    find: "                self.flush(state, out)?;\n                self.drop_value(slots, ty, span, out)?;\n",
    replace: "                self.flush(state, out)?;\n",
    tests: ["test/corpus.test.ts", "-t", "slice_uninit"],
  },
  {
    name: "mir-uninit-write-clone-copies",
    breaks: "MIR's `write_clone_of_slice` copies, not clones",
    file: "src/lower/mir/std_calls.rs",
    find: "                self.uninit_write_values(clone, values, generic_args.type_at(0), span, out)?\n",
    replace: "                self.uninit_write_values(false, values, generic_args.type_at(0), span, out)?\n",
    tests: ["test/corpus.test.ts", "-t", "slice_uninit"],
  },
  {
    name: "mir-str-part-range-unread",
    breaks: "MIR's `&mut s[a..b]` gives `$strPart` the range itself, not its bounds",
    file: "src/lower/mir/std_calls.rs",
    find: "                if matches!(op, TextOp::StrPart { bounds: false } | TextOp::StrGetMut) {\n",
    replace: "                if matches!(op, TextOp::StrGetMut) {\n",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-str-part-case-handle",
    breaks: "MIR's `s[a..b].make_ascii_uppercase()` changes a handle on the part, not `s`",
    file: "src/lower/mir/std_calls.rs",
    find: "                            && let Some((whole, mut bounds)) = str_part_of(place) =>\n",
    replace: "                            && let Some((whole, mut bounds)) = str_part_of(place).filter(|_| false) =>\n",
    tests: ["test/lowering.test.ts", "-t", "part changed in place"],
  },
  {
    name: "mir-str-edit-part-refused",
    breaks: "MIR's string edit of a part a call gives, `tail(&mut s).make_ascii_uppercase()`, is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                        let cell = self.spill(\"part\", (**cell).clone(), out);\n                        Expr::held(cell)\n",
    replace: "                        return Err(self.unsupported(span, \"changing this string, from its MIR\"));\n",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-make-mut-field-refused",
    breaks: "MIR's `Rc::make_mut(&mut copy.pages)`, of a field, is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                    Some(name) if fixed_place(&place) => state",
    replace: "                    Some(name) if matches!(place.kind, js::ExprKind::Var(_)) => state",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "mir-failing-write-whole",
    breaks: "MIR's `write!` of what may fail writes it whole, what came before the error lost",
    file: "src/lower/mir/std_calls.rs",
    find: "                if matches!(given, Value::Fmt(Std::FmtNew, ..)) && fixed_place(&place) {\n",
    replace: "                if false && fixed_place(&place) {\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "mir-skip-while-skipped-undropped",
    breaks: "MIR's `skip_while` of items with a destructor leaves those it skips undropped",
    file: "src/lower/mir/std_calls.rs",
    find: "                    exprs.insert(0, self.dropping_discarded(test, item, discard_when, span, out)?);\n",
    replace: "                    let _ = (item, discard_when);\n                    exprs.insert(0, test);\n",
    tests: ["test/corpus.test.ts", "-t", "drop_skipped_items"],
  },
  {
    name: "mir-pretty-dyn-debug-plain",
    breaks: "MIR's `{:#?}` of a `&dyn Debug` made elsewhere shows it plain",
    file: "src/lower/mir/std_calls.rs",
    find: "                    if spec.alternate && kind == Std::FmtDebug && self.is_dyn_debug(ty) {\n",
    replace: "                    if false {\n",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses .....? of a kept"],
  },
  {
    name: "mir-json-text-pretty-lost",
    breaks: "MIR's `serde_json::to_string_pretty` writes it compact",
    file: "src/lower/mir/std_calls.rs",
    find: "                self.json_text(value, ty, pretty, span)?\n",
    replace: "                self.json_text(value, ty, false, span)?\n",
    tests: ["test/serde.test.ts", "-t", "ordinary renamed structs"],
  },
  {
    name: "mir-deque-remove-refused",
    breaks: "MIR's `VecDeque::remove` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                Expr::call_helper(\"$removeOpt\", exprs)\n",
    replace: "                return Err(self.unsupported(span, &format!(\"`remove` of {}, from its MIR\", exprs.len())));\n",
    tests: ["test/lowering.test.ts","-t","a deque's remove"],
  },
  {
    name: "mir-push-str-lost",
    breaks: "`write!` or `push_str` to a `String` from MIR writes nothing",
    file: "src/lower/mir/std_calls.rs",
    find: "                crate::lower::display::append_written(&place, value, false, self.js_span(span), out);\n",
    replace: "                let _ = (place, value);\n",
    tests: ["test/corpus.test.ts", "-t", "string_pushes"],
  },
  {
    name: "mir-format-failing-unmarked",
    breaks: "`format!` of a `Display` that fails throws a `fmt::Error`, not std's panic",
    file: "src/lower/mir/std_calls.rs",
    find: "                if fails {\n                    return Ok(Some(Value::Fmt(Std::FmtNew, output, joined)));\n",
    replace: "                if false && fails {\n                    return Ok(Some(Value::Fmt(Std::FmtNew, output, joined)));\n",
    tests: ["test/corpus.test.ts","-t","fmt_error_format"],
  },
  {
    name: "mir-ptr-eq-refused",
    breaks: "MIR's `ptr::eq` of JS objects is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                Expr::bin(Op::Eq, a, b)\n            }\n            Std::Number(op) => {\n",
    replace: "                return Err(self.unsupported(span, if a.reads_same() && b.reads_same() { \"ptr::eq\" } else { \"ptr::eq of these\" }));\n            }\n            Std::Number(op) => {\n",
    tests: ["test/lowering.test.ts","-t","ptr::eq of JS objects"],
  },
  {
    name: "format-arguments-in-place",
    breaks: "a format string's argument that does something is evaluated where the template formats it, after a `Debug` of an earlier one that prints",
    file: "src/lower/mir/std_calls.rs",
    find: "                if self.formats_out_of_order(&items, &bytes, span)? {\n",
    replace: "                if false && self.formats_out_of_order(&items, &bytes, span)? {\n",
    tests: ["test/corpus.test.ts", "-t", "^format_arguments_first.rs$"],
  },
  {
    name: "formatting-runs-nothing",
    breaks: "a `Debug` of the crate's isn't taken to run code, so an argument after it is made after it runs",
    file: "src/lower/mir/std_calls.rs",
    find: "            formatted |= part.runs_program_code();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^format_arguments_first.rs$"],
  },
  {
    name: "cycle-owned-items-shared",
    breaks: "`cycle()` of an iterator that owns its items gives the same one each time round, where Rust's clones them",
    file: "src/lower/mir/std_calls.rs",
    find: "                        if owns && self.needs_clone(item) {\n                            args.push(self.clone_fn(\"item\", item, span)?);\n",
    replace: "                        if false && owns && self.needs_clone(item) {\n                            args.push(self.clone_fn(\"item\", item, span)?);\n",
    tests: ["test/corpus.test.ts", "-t", "^iter_cycle.rs$"],
  },
  {
    name: "string-extend-unjoined",
    breaks: "`s.extend(chars)` adds their array, `s += [\"y\", \"z\"]`, not their text",
    file: "src/lower/mir/std_calls.rs",
    find: "                    true => self.joined_text(value, Some(arg_tys[1]), span)?,\n",
    replace: "                    true => value,\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_extend.rs$"],
  },
  {
    name: "partition-dropping-refused",
    breaks: "`partition` and `filter_map` of items with a destructor are refused, though they move each on",
    file: "src/lower/mir/std_calls.rs",
    find: "if dropping.is_some() && !comb.leaves_none() && !comb.holds_its_sources() {",
    replace: "if dropping.is_some() && !comb.holds_its_sources() {",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
  {
    name: "clone-from-old-undropped",
    breaks: "`clone_from` doesn't drop what was there, as `*dest = source.clone()` does",
    file: "src/lower/mir/std_calls.rs",
    find: "                    self.drop_value(old, ty, span, out)?;\n",
    replace: "                    let _ = old;\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from.rs$"],
  },
  {
    name: "clone-from-shared",
    breaks: "`clone_from` writes the source itself, not a clone, so a change to one is seen in the other",
    file: "src/lower/mir/std_calls.rs",
    find: "                let clone = self.clone_value(source, ty, span, out)?;\n                let given = vec![dest, Value::Expr(clone)];\n",
    replace: "                let given = vec![dest, Value::Expr(source)];\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from.rs$"],
  },
  {
    name: "own-write-all-unwrapped",
    breaks: "std's `write_all` of the crate's writer that fails throws out of where its result is a value",
    file: "src/lower/mir/std_calls.rs",
    find: "                    true => self.fmt_try(vec![StmtKind::Return(Some(call)).at(js::Span::NONE)]),\n",
    replace: "                    true => call,\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "zip-other-not-closing",
    breaks: "`zip`'s other side that owns items with a destructor isn't closed to drop them",
    file: "src/lower/mir/std_calls.rs",
    find: "                        Ok(match self.closing_drop(ty, span)? {\n",
    replace: "                        Ok(match None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "take-while-rejected-undropped",
    breaks: "the item `take_while` stops at isn't dropped",
    file: "src/lower/mir/std_calls.rs",
    find: "                    && matches!(comb, IterComb::SkipWhile | IterComb::TakeWhile)\n",
    replace: "                    && comb == IterComb::SkipWhile\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "cycle-of-other-refused",
    breaks: "`cycle()` of a range, the crate's iterator or a type parameter's is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                self.cycled(it, arg_tys[0].peel_refs(), span)?\n",
    replace: "                return Err(self.unsupported(span, \"cycle\"));\n",
    tests: ["test/corpus.test.ts", "-t", "^cycled_iterators.rs$"],
  },
  {
    name: "cycle-array-drops-allowed",
    breaks: "`cycle()` of an array's items with a destructor is allowed, and leaks what it holds",
    file: "src/lower/mir/std_calls.rs",
    find: "                            return Err(self.unsupported(span, \"`cycle()` of an array's items with a destructor\"));\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "cycling an array's items"],
  },
  {
    name: "extend-from-within-aliased",
    breaks: "`extend_from_within` pushes the same objects again, not clones",
    file: "src/lower/mir/std_calls.rs",
    find: "                    Some(item) if op == TextOp::ExtendFromWithin && self.needs_clone(item) => {\n",
    replace: "                    Some(item) if false && self.needs_clone(item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_edit_drops.rs$"],
  },
  {
    name: "peek-mut-of-numbers-unhandled",
    breaks: "`peek_mut` of numbers gives the number, which a write changes nowhere",
    file: "src/lower/mir/std_calls.rs",
    find: "                        Some(item) if self.is_boxable(item) => Expr::call_helper(\"$peekCell\", vec![it]),\n",
    replace: "                        Some(item) if false && self.is_boxable(item) => Expr::call_helper(\"$peekCell\", vec![it]),\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_and_peek_mut.rs$"],
  },
  {
    name: "make-mut-of-borrow-boxed",
    breaks: "`Rc::make_mut(&mut rc)` of a variable is given a box for it, and refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                    Value::Ref(place) => place,\n                    value => self.value_expr(value, span)?,\n",
    replace: "                    value => self.value_expr(value, span)?,\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_clone_sees_owners.rs$"],
  },
  {
    name: "make-mut-through-mut-refused",
    breaks: "`Rc::make_mut` of an `Rc` a `&mut` argument reaches is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                if !boxed && (state.body.local_kind(local) == LocalKind::Arg || local_ty.is_ref()) {\n",
    replace: "                if state.body.local_kind(local) == LocalKind::Arg || local_ty.is_ref() {\n",
    tests: ["test/crates.test.ts", "-t", "three crates: rc_generic"],
  },
  {
    name: "chained-into-iter-raw",
    breaks: "what `chain` or `zip` is given is passed as it is, a range's object or an `Option`",
    file: "src/lower/mir/std_calls.rs",
    find: "                        let e = self.iterable_items(e, ty, span, out)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_arguments.rs$"],
  },
  {
    name: "held-cycle-of-array-generated",
    breaks: "a held `Cycle` of an array's iterator is a generator, which keeps neither it cycles",
    file: "src/lower/mir/std_calls.rs",
    find: "                if self.held_adapter(output)\n                    || !arg_tys\n",
    replace: "                if !arg_tys\n",
    tests: ["test/corpus.test.ts", "-t", "^held_cycle.rs$"],
  },
  {
    name: "fmt-to-held-formatter-pretty",
    breaks: "what a `fmt` trait's method writes to a held `Formatter` is written pretty, where the handle keeps no options",
    file: "src/lower/mir/std_calls.rs",
    find: "                let pretty = crate::lower::display::Pretty::Plain;\n",
    replace: "                let pretty = crate::lower::display::Pretty::Always;\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_to_held_formatter.rs$"],
  },
];
