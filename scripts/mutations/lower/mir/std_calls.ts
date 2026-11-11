// Mutations of src/lower/mir/std_calls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "string-write-through-a-call-uncaught",
    breaks: "`write!(*t.as_mut().unwrap(), ..)` of what fails throws past its `fmt::Result`, the place reached through a call not held (#1011)",
    file: "src/lower/mir/std_calls.rs",
    find: "                    let place = match fmt && !fixed_place(&place) {\n",
    replace: "                    let place = match false && !fixed_place(&place) {\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_error_as_a_value.rs$"],
  },
  {
    name: "wrapping-op-assign-written-through",
    breaks:
      "a `Wrapping`'s `op=` of a call's operand, through MIR's own `&mut b0`, is `$assign(b0, ..)`, written into the array its copies share: `17 (17, 17)` where Rust gives `1 (1, 17)` (#396)",
    file: "src/lower/mir/std_calls.rs",
    find: "                            if let Some(of) = own.map(|of| Expr::var(&state.locals.names[of])) {\n",
    replace:
      "                            if let Some(of) = own.filter(|_| false).map(|of| Expr::var(&state.locals.names[of])) {\n",
    tests: ["test/corpus.test.ts", "-t", "^wrapping_assign_of_a_copy.rs$"],
  },
  {
    name: "wrapping-assigned-to-its-ref",
    breaks:
      "a `Wrapping`'s `op=` through a `&mut` a variable holds assigns the variable, not what it points at: a caller's never changes, and MIR's borrow is a `const` assigned (#309)",
    file: "src/lower/mir/std_calls.rs",
    find: "                            (place, reference)\n",
    replace: "                            (place, false)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^wrapping_assigned_through_a_ref.rs$",
    ],
  },
  {
    name: "mir-radix-argument-unseen",
    breaks:
      "a `{:x}` argument from MIR isn't one `format_args!` shows by its spec",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::FmtDisplay | Std::FmtDebug | Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {\n",
    replace:
      "            Std::FmtDisplay | Std::FmtDebug | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {\n",
    tests: ["test/corpus.test.ts", "-t", "radix_negative"],
  },
  {
    name: "mir-sort-unsorted",
    breaks: "`v.sort()` from MIR leaves the items as they were",
    file: "src/lower/mir/std_calls.rs",
    find: "                this.sort_values(key, items, arg_tys[0], generic_args, span, out)?\n",
    replace: "                items\n",
    tests: ["test/corpus.test.ts", "-t", "generic_borrow"],
  },
  {
    name: "mir-view-whole",
    breaks: "`&mut v[a..b]` from MIR is all of `v`, not a view of those items",
    file: "src/lower/mir/std_calls.rs",
    find: '} else { Value::View(list, js::Span::NONE) })\n',
    replace: "} else { Value::Expr(list[0].clone()) })\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_loop_bounds"],
  },
  {
    name: "mir-rc-call-refused",
    breaks: "an `Rc`'s function from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                this.rc_values(op, (exprs, arg_tys, output), span, out)?\n",
    replace:
      '                return Err(this.unsupported(span, "an Rc\'s"));\n',
    tests: ["test/corpus.test.ts", "-t", "as_deref_counted"],
  },
  {
    name: "mir-to-int-unchecked-uncast",
    breaks:
      "`x.to_int_unchecked()` under MIR reaches `number_call`, which has none",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Number(NumOp::ToIntUnchecked) => lower(move |this, _, values, _| {\n",
    replace:
      "            Std::Number(NumOp::ToIntUnchecked) if false => lower(move |this, _, values, _| {\n",
    tests: ["test/corpus.test.ts", "-t", "integers_128"],
  },
  {
    name: "mir-copies-shared",
    breaks: "`vec![p; 3]` of a struct is three names for one object",
    file: "src/lower/mir/std_calls.rs",
    find: "this.copies_values((item, arg_tys[0]), (n, known), span, out)?",
    replace:
      "this.copies_values((item, tcx.types.unit), (n, known), span, out)?",
    tests: ["test/corpus.test.ts", "-t", "copy_mutation"],
  },
  {
    name: "mir-non-zero-new-refused",
    breaks: "`NonZero::new(n)` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::NonZeroNew => lower(move |this, _, values, out| {\n",
    replace:
      "            Std::NonZeroNew if false => lower(move |this, _, values, out| {\n",
    tests: ["test/corpus.test.ts", "-t", "nonzero"],
  },
  {
    name: "mir-string-edit-refused",
    breaks: "`s.insert(i, c)` and the rest of a string's edits are refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::StringEdit(edit) => lower(move |this, state, values, out| {\n",
    replace:
      "            Std::StringEdit(edit) if false => lower(move |this, state, values, out| {\n",
    tests: ["test/corpus.test.ts", "-t", "text_edits|string_editing"],
  },
  {
    name: "mir-string-range-swapped",
    breaks: "`s.replace_range(a..b, t)` takes its range's end for its start",
    file: "src/lower/mir/std_calls.rs",
    find: "                    given.splice(0..0, [start, end.unwrap_or_else(Expr::undefined)]);\n",
    replace:
      "                    given.splice(0..0, [end.unwrap_or_else(Expr::undefined), start]);\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "text_edits|string_editing|replace_range_panic|string_drain_from_past_len",
    ],
  },
  {
    name: "mir-with-capacity-refused",
    breaks: "`String::with_capacity(n)` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::StringWithCapacity => lower(move |this, state, values, out| {\n",
    replace:
      "            Std::StringWithCapacity if false => lower(move |this, state, values, out| {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "text_edits|string_editing|text_aliases|string_insert_boundary",
    ],
  },
  {
    name: "mir-make-mut-refused",
    breaks: "`Rc::make_mut(&mut rc)` of a local `Rc` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Rc(RcOp::MakeMut) => lower(move |this, state, values, out| {\n",
    replace:
      "            Std::Rc(RcOp::MakeMut) if false => lower(move |this, state, values, out| {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "rc_clone_sees_owners|rc_make_mut_clone_panics",
    ],
  },
  {
    name: "mir-type-name-refused",
    breaks: "`type_name::<T>()` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::TypeName { .. } => lower(move |this, state, values, out| {\n",
    replace:
      "            Std::TypeName { .. } if false => lower(move |this, state, values, out| {\n",
    tests: ["test/corpus.test.ts", "-t", "type_facts"],
  },
  {
    name: "mir-tree-range-refused",
    breaks: "a B-tree's `range` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Map(MapOp::TreeRange { mutable }) => lower(",
    replace:
      "            Std::Map(MapOp::TreeRange { mutable }) if mutable => lower(",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a B-tree's range from MIR",
    ],
  },
  {
    name: "mir-copy-within-refused",
    breaks: "`v.copy_within(a..b, at)` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Slice(SliceOp::CopyWithin) => lower(",
    replace: "            Std::Slice(SliceOp::CopyWithin) if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "slice_methods"],
  },
  {
    name: "mir-vec-extract-if-refused",
    breaks: "`v.extract_if(.., f)` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Slice(SliceOp::ExtractIf { range }) => lower(",
    replace:
      "            Std::Slice(SliceOp::ExtractIf { range }) if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "extract_if"],
  },
  {
    name: "mir-map-extract-if-refused",
    breaks: "a map's `extract_if(f)` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Map(MapOp::ExtractIf) => lower(",
    replace: "            Std::Map(MapOp::ExtractIf) if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "extract_if"],
  },
  {
    name: "mir-remainder-made-stepped",
    breaks:
      "a chunks' `remainder()` of one made there from MIR is `$iter(..).items.remainder`, not the array's",
    file: "src/lower/mir/std_calls.rs",
    find: "                if let Some(items) = made {\n                    exprs[0] = items;\n                }\n",
    replace: "",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a chunks' remainder from MIR",
    ],
  },
  {
    name: "mir-remainder-stepped-plain",
    breaks:
      "a stepped chunks' `remainder()` from MIR reads `.remainder` of its `$iter`, which has none",
    file: "src/lower/mir/std_calls.rs",
    find: "                let stepping = made.is_none();\n",
    replace: "                let stepping = false;\n",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a chunks' remainder from MIR",
    ],
  },
  {
    name: "mir-slice-views-refused",
    breaks:
      "a slice op giving a `&mut` view of items from MIR, `split_at_mut`, is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "if !inner.is_slice() && !inner.is_array())",
    replace: ")",
    tests: ["test/corpus.test.ts", "-t", "split_at_mut_past_len"],
  },
  {
    name: "mir-cell-giving-slice-refused",
    breaks:
      "`get_disjoint_mut` from MIR, which makes its items' boxes itself, is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                ) && (known.gives_its_cell() || !output.walk().any(|arg| {\n",
    replace: "                ) && (false || !output.walk().any(|arg| {\n",
    tests: ["test/corpus.test.ts", "-t", "slice_disjoint"],
  },
  {
    name: "mir-push-str-place-made-twice",
    breaks:
      "`v.push_mut(s).push('b')` from MIR reads and writes `$pushMut(v, s, true).value`, pushing twice",
    file: "src/lower/mir/std_calls.rs",
    find: "                    js::ExprKind::Member(object, field) if !object.reads_same() => {\n",
    replace:
      "                    js::ExprKind::Member(object, field) if false && !object.reads_same() => {\n",
    tests: ["test/corpus.test.ts", "-t", "vec_push_mut"],
  },
  {
    name: "mir-deque-range-mut-refused",
    breaks: "a deque's `range_mut(a..b)` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Slice(SliceOp::DequeRangeMut) => lower(",
    replace:
      "            Std::Slice(SliceOp::DequeRangeMut) if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "deque_range_mut"],
  },
  {
    name: "mir-deque-range-mut-unboxed",
    breaks:
      "a deque's `range_mut` of numbers from MIR gives the numbers, not boxes on them, so writing one changes nothing",
    file: "src/lower/mir/std_calls.rs",
    find: '                match this.mut_ref_form(generic_args.type_at(0)).is_cell() {\n                    true => Expr::call_helper("$mutItems", vec![view]),\n',
    replace:
      '                match false {\n                    true => Expr::call_helper("$mutItems", vec![view]),\n',
    tests: ["test/corpus.test.ts", "-t", "deque_range_mut"],
  },
  {
    name: "mir-tree-range-mut-refused",
    breaks: "a B-tree's `range_mut` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Map(MapOp::TreeRange { mutable }) => lower(",
    replace:
      "            Std::Map(MapOp::TreeRange { mutable }) if !mutable => lower(",
    tests: ["test/corpus.test.ts", "-t", "btree_range_mut"],
  },
  {
    name: "mir-range-edit-refused",
    breaks:
      "`v.extend_from_within(a..b)` and `v.splice(a..b, items)` from MIR are refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Text(op @ (TextOp::Splice | TextOp::ExtendFromWithin)) => lower(",
    replace:
      "            Std::Text(op @ (TextOp::Splice | TextOp::ExtendFromWithin)) if false => lower(",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a vec's range pushed again or replaced",
    ],
  },
  {
    name: "mir-option-place-refused",
    breaks: "`o.get_or_insert(x)` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::OptionPlace(op) => lower(",
    replace: "            Std::OptionPlace(op) if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "option_result_methods"],
  },
  {
    name: "mir-cow-refused",
    breaks: "a `Cow`'s `into_owned()` or `to_mut()` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Cow(op) => lower(",
    replace: "            Std::Cow(op) if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "cows"],
  },
  {
    name: "mir-heap-refused",
    breaks: "a `BinaryHeap`'s `append` or `peek_mut` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::Heap(op) => lower(",
    replace: "            Std::Heap(op) if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "heap_methods|heap_peek_mut"],
  },
  {
    name: "mir-wrapping-op-refused",
    breaks: "`a * Wrapping(2)` from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::WrappingOp(op, assign) => lower(",
    replace: "            Std::WrappingOp(op, assign) if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "wrapping_type"],
  },
  {
    name: "mir-user-write-refused",
    breaks:
      "`writeln!(out, ..)` to a writer of the crate's own from MIR is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::UserWrite => lower(",
    replace: "            Std::UserWrite if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "user_fmt_write"],
  },
  {
    name: "mir-split-off-unwritten",
    breaks: "MIR's `s.split_off(..n)` leaves `s` as it was",
    file: "src/lower/mir/std_calls.rs",
    find: "                out.push(StmtKind::Assign(place, left).at(this.js_span(span)));\n                Expr::index(split, Expr::int(1))\n",
    replace: "                Expr::index(split, Expr::int(1))\n",
    tests: ["test/corpus.test.ts", "-t", "slice_split_off"],
  },
  {
    name: "mir-split-off-held",
    breaks:
      "MIR's `r.split_off_mut(..n)`, `r` a `&mut` to `rest`, gives `r` what's left",
    file: "src/lower/mir/std_calls.rs",
    find: "                        if matches!(place.kind, js::ExprKind::Var(_)) && this.borrows_variable(def_id, span, 0) =>\n",
    replace:
      "                        if matches!(place.kind, js::ExprKind::Var(_)) =>\n",
    tests: ["test/lowering.test.ts", "-t", "split_off through a held"],
  },
  {
    name: "mir-uninit-drop-skipped",
    breaks: "MIR's `assume_init_drop()` of a slice drops nothing",
    file: "src/lower/mir/std_calls.rs",
    find: "                this.flush(state, out)?;\n                this.drop_value(slots, ty, span, out)?;\n",
    replace: "                this.flush(state, out)?;\n",
    tests: ["test/corpus.test.ts", "-t", "slice_uninit"],
  },
  {
    name: "mir-uninit-write-clone-copies",
    breaks: "MIR's `write_clone_of_slice` copies, not clones",
    file: "src/lower/mir/std_calls.rs",
    find: "                this.uninit_write_values(clone, values, generic_args.type_at(0), span, out)?\n",
    replace:
      "                this.uninit_write_values(false, values, generic_args.type_at(0), span, out)?\n",
    tests: ["test/corpus.test.ts", "-t", "slice_uninit"],
  },
  {
    name: "mir-str-part-range-unread",
    breaks:
      "MIR's `&mut s[a..b]` gives `$strPart` the range itself, not its bounds",
    file: "src/lower/mir/std_calls.rs",
    find: "                if matches!(op, TextOp::StrPart { bounds: false } | TextOp::StrGetMut) {\n",
    replace: "                if matches!(op, TextOp::StrGetMut) {\n",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-str-part-case-handle",
    breaks:
      "MIR's `s[a..b].make_ascii_uppercase()` changes a handle on the part, not `s`",
    file: "src/lower/mir/std_calls.rs",
    find: "                            && let Some((whole, mut bounds)) = str_part_of(place) =>\n",
    replace:
      "                            && let Some((whole, mut bounds)) = str_part_of(place).filter(|_| false) =>\n",
    tests: ["test/lowering-strings.test.ts", "-t", "part changed in place"],
  },
  {
    name: "mir-str-edit-part-refused",
    breaks:
      "MIR's string edit of a part a call gives, `tail(&mut s).make_ascii_uppercase()`, is refused",
    file: "src/lower/mir/std_calls.rs",
    find: '                            let cell = this.spill("part", (**cell).clone(), out);\n                            Expr::held(cell)\n',
    replace:
      '                            return Err(this.unsupported(span, "changing this string, from its MIR"));\n',
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-make-mut-field-refused",
    breaks: "MIR's `Rc::make_mut(&mut copy.pages)`, of a field, is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                        Some(name) if fixed_place(&place) => {\n",
    replace:
      "                        Some(name) if matches!(place.kind, js::ExprKind::Var(_)) => {\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "mir-failing-write-whole",
    breaks:
      "MIR's `write!` of what may fail writes it whole, what came before the error lost",
    file: "src/lower/mir/std_calls.rs",
    find: "                    if fmt && fixed_place(&place) {\n",
    replace: "                    if false && fixed_place(&place) {\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "mir-json-text-pretty-lost",
    breaks: "MIR's `serde_json::to_string_pretty` writes it compact",
    file: "src/lower/mir/std_calls.rs",
    find: "                this.json_text(value, ty, pretty, span)?\n",
    replace: "                this.json_text(value, ty, false, span)?\n",
    tests: ["test/serde.test.ts", "-t", "ordinary renamed structs"],
  },
  {
    name: "mir-deque-remove-refused",
    breaks: "MIR's `VecDeque::remove` is refused",
    file: "src/lower/mir/std_calls.rs",
    find: '                Expr::call_helper("$removeOpt", exprs)\n',
    replace:
      '                return Err(this.unsupported(span, &format!("`remove` of {}, from its MIR", exprs.len())));\n',
    tests: ["test/lowering-collections.test.ts", "-t", "a deque's remove"],
  },
  {
    name: "mir-push-str-lost",
    breaks: "`write!` or `push_str` to a `String` from MIR writes nothing",
    file: "src/lower/mir/std_calls.rs",
    find: "                        false => crate::lower::display::append_written(&place, value, false, this.js_span(span), out),\n",
    replace: "                        false => {}\n",
    tests: ["test/corpus.test.ts", "-t", "string_pushes"],
  },
  {
    name: "mir-format-failing-unmarked",
    breaks:
      "`format!` of a `Display` that fails throws a `fmt::Error`, not std's panic",
    file: "src/lower/mir/std_calls.rs",
    find: "                    if fails {\n                        return Ok(Value::Fmt(Std::FmtNew, output, joined));\n",
    replace:
      "                    if false && fails {\n                        return Ok(Value::Fmt(Std::FmtNew, output, joined));\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_format"],
  },
  {
    name: "mir-ptr-eq-refused",
    breaks: "MIR's `ptr::eq` of JS objects is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                    _ => Expr::bin(Op::Eq, a, b),\n",
    replace:
      '                    _ => return Err(this.unsupported(span, if a.reads_same() && b.reads_same() { "ptr::eq" } else { "ptr::eq of these" })),\n',
    tests: ["test/lowering-bindings.test.ts", "-t", "ptr::eq of JS objects"],
  },
  {
    name: "format-arguments-in-place",
    breaks:
      "a format string's argument that does something is evaluated where the template formats it, after a `Debug` of an earlier one that prints",
    file: "src/lower/mir/std_calls.rs",
    find: "                if this.formats_out_of_order(&items, &template, span)? {\n",
    replace:
      "                if false && this.formats_out_of_order(&items, &template, span)? {\n",
    tests: ["test/corpus.test.ts", "-t", "^format_arguments_first.rs$"],
  },
  {
    name: "own-write-all-unwrapped",
    breaks:
      "std's `write_all` of the crate's writer that fails throws out of where its result is a value",
    file: "src/lower/mir/std_calls.rs",
    find: "                    true => this.fmt_try(vec![StmtKind::Return(Some(call)).at(js::Span::NONE)]),\n",
    replace: "                    true => call,\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "extend-from-within-aliased",
    breaks: "`extend_from_within` pushes the same objects again, not clones",
    file: "src/lower/mir/std_calls.rs",
    find: "                    Some(item) if op == TextOp::ExtendFromWithin && this.needs_clone(item) => {\n",
    replace:
      "                    Some(item) if false && this.needs_clone(item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_edit_drops.rs$"],
  },
  {
    name: "make-mut-of-borrow-boxed",
    breaks:
      "`Rc::make_mut(&mut rc)` of a variable is given a box for it, and refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                        Value::Ref(place) => place,\n                        value => this.value_expr(value, span)?,\n",
    replace:
      "                        value => this.value_expr(value, span)?,\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_clone_sees_owners.rs$"],
  },
  {
    name: "make-mut-through-mut-refused",
    breaks: "`Rc::make_mut` of an `Rc` a `&mut` argument reaches is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "                if !boxed && (state.body.local_kind(local) == LocalKind::Arg || local_ty.is_ref()) {\n",
    replace:
      "                if state.body.local_kind(local) == LocalKind::Arg || local_ty.is_ref() {\n",
    tests: ["test/crates.test.ts", "-t", "three crates: rc_generic"],
  },
  {
    name: "fmt-to-held-formatter-pretty",
    breaks:
      "what a `fmt` trait's method writes to a held `Formatter` is written pretty, where the handle keeps no options",
    file: "src/lower/mir/std_calls.rs",
    find: "                let pretty = crate::lower::display::Pretty::Plain;\n",
    replace:
      "                let pretty = crate::lower::display::Pretty::Always;\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_to_held_formatter.rs$"],
  },
  {
    name: "fmt-arg-through-refs",
    breaks:
      "a `&mut` shown is a handle on its place, held and read through its getter",
    file: "src/lower/mir/std_calls.rs",
    find: "                    _ => this.shown_through(value, ty),\n",
    replace: "                    _ => (value, ty),\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_local"],
    snapshots: true,
  },
  {
    name: "ptr-eq-of-handles-identity",
    breaks:
      "`ptr::eq` of two pointers to Rust's values compares their handles by `===`, so the same item reached through another array's reference isn't equal",
    file: "src/lower/mir/std_calls.rs",
    find: "                    Some((pointee, _)) if this.compared_by_address(arg_tys[0]) => {",
    replace:
      "                    Some((pointee, _)) if false && this.compared_by_address(arg_tys[0]) => {",
    tests: ["test/corpus.test.ts", "-t", "^ptr_eq_through_refs.rs$"],
  },
  {
    name: "cut-string-reread-after-change",
    breaks:
      "a strip's string is read again where it's tested and cut, after its variable is given another",
    file: "src/lower/mir/std_calls.rs",
    find: "                            if holders.iter().any(|&held| state.locals.names[held] == *name));",
    replace:
      "                            if holders.len() < usize::MAX || name.is_empty());",
    tests: ["test/lowering-string-places.test.ts", "-t", "string changes"],
  },
  {
    name: "places-sliced-in-bytes",
    breaks:
      "a slice at places in units is `$strSlice`, which counts them as bytes",
    file: "src/lower/mir/std_calls.rs",
    find: "                    true => this.sliced_in_units(items, (range, arg_tys[1]), span, out)?,",
    replace:
      "                    true => this.slice_range_values((op, false), items, (range, arg_tys[1]), span, out)?,",
    tests: ["test/lowering-string-places.test.ts", "-t", "stepped past"],
  },
  {
    name: "pushed-component-raw",
    breaks: "`buf.push(c)` of a `Component` pushes its object",
    file: "src/lower/mir/std_calls.rs",
    find: "                    StringEdit::PathPush | StringEdit::SetFileName | StringEdit::SetExtension\n",
    replace: "                    StringEdit::Clear\n",
    tests: ["test/corpus.test.ts", "-t", "^path_components.rs$"],
  },
  {
    name: "string-with-capacity-of-no-size",
    breaks: "`String::with_capacity(usize::MAX)` is no panic",
    file: "src/lower/mir/std_calls.rs",
    find: "                this.room_for((None, n), Expr::int(1), span, out);\n",
    replace:
      "                this.room_for((None, n), Expr::int(0), span, out);\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_overflow.rs$"],
  },
  {
    name: "string-reserve-of-its-cell",
    breaks: "`s.reserve(n)` reads the room of its `&mut`'s cell, not its text",
    file: "src/lower/mir/std_calls.rs",
    find: "                        Value::Ref(place) => place,\n                        cell => Expr::held(this.value_expr(cell, span)?),\n",
    replace:
      "                        Value::Ref(place) => Expr::held(place),\n                        cell => Expr::held(this.value_expr(cell, span)?),\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_overflow.rs$"],
  },
  {
    name: "once-set-never-known-empty",
    breaks:
      "a `OnceCell`'s `set` is never of a cell known empty, so React's `payload._ioInfo = ioInfo` is `if (payload._ioInfo === undefined) ..`",
    file: "src/lower/mir/std_calls.rs",
    find: "                    (self.thir_call(def_id, span)).is_some_and(|(fun, _)| self.frame.facts.set_empty.contains(&fun));\n",
    replace:
      "                    (self.thir_call(def_id, span)).is_some_and(|(fun, _)| self.frame.facts.set_empty.contains(&fun) && false);\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_fields.rs$"],
    snapshots: true,
  },
  {
    name: "once-set-result-always-read",
    breaks:
      "a `set`'s `Result` nothing reads is made, `const unset = .. ; if (unset) ..`, where it's `if (node.cache === undefined) ..`",
    file: "src/lower/mir/std_calls.rs",
    find: "                let read = !self.result_unread(state);\n",
    replace: "                let read = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
    snapshots: true,
  },
  {
    name: "view-made-at-once",
    breaks: "`&mut v[a..b]` is made a view where it's borrowed, so `v[a..b].reverse()` reverses a `Proxy`, where it's `$reverseRange(v, a, b)` (ADR 1073)",
    file: "src/lower/mir/std_calls.rs",
    find: '} else { Value::View(list, js::Span::NONE) })\n',
    replace: '} else { Value::Expr(this.view_call("$view", list)) })\n',
    tests: ["test/corpus.test.ts", "-t", "^slice_range_reverse.rs$"],
    snapshots: true,
  },
  {
    name: "result-unread-untold",
    breaks:
      "a cell's `replace` whose old value nothing reads isn't told so, and reads it, `const old = metadata.action;`, a getter's read of a struct JS made (rd418, ADR 1022, amended)",
    file: "src/lower/mir/std_calls.rs",
    find: "            unread: self.result_unread(state),\n",
    replace: "            unread: false,\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "a replace of a JS-made struct's cell field whose old value nothing reads"],
  },
  {
    name: "nullable-lend-stores-as-given",
    breaks:
      "a nullable thread-local's lend isn't said to store `null` for `None`, so its module's variable written through it is stored as it is (#541)",
    file: "src/lower/mir/std_calls.rs",
    find: "                .or(cells.nullable_lent.get(&state.at))\n",
    replace: "",
    tests: ["test/lowering-nullable-borrows.test.ts", "-t", "written through a borrow"],
  },
  {
    name: "string-extend-unreserved",
    breaks: "a `String`'s `extend` by `char`s doesn't ask their iterator's hint first, as std's `Extend<char>` reserves by it (a wrong answer, #849)",
    file: "src/lower/mir/std_calls.rs",
    find: "                            this.extended_reserved(value, into, arg_tys[1], place.clone())\n",
    replace: "                            value\n",
    tests: ["test/corpus.test.ts", "-t", "^extend_reserves_by_hint.rs$"],
  },
  {
    name: "clone-from-vec-replaced",
    breaks: "`w.clone_from(&v)` of a `Vec` of items with a destructor clones every item, then drops the old `Vec`, not std's `SpecCloneIntoVec` order (#919)",
    file: "src/lower/mir/std_calls.rs",
    find: "                    if let Some(into) = this.clone_into(ty, span)? {\n",
    replace: "                    if let Some(into) = this.clone_into(ty, span)?.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(vec_clone_from_in_place|clone_from_std_own).rs$"],
  },
  {
    name: "clone-from-option-replaced",
    breaks: "`w.clone_from(&v)` of an `Option<Vec<D>>` of two `Some`s is `*w = v.clone()`, every clone before the old one's drop, not its `Vec`'s in place (#922)",
    file: "src/lower/mir/std_calls.rs",
    find: "                    let inner = this.option_of(ty).filter(|&inner| !this.boxed_payload(inner));\n",
    replace: "                    let inner = this.option_of(ty).filter(|&inner| false && !this.boxed_payload(inner));\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_std_own.rs$"],
  },
  {
    name: "clone-from-forwarded-unrefused",
    breaks: "`clone_from` of an `Option` of a type whose own `clone_from` is the crate's is `*dest = source.clone()`, which never calls it (#922)",
    file: "src/lower/mir/std_calls.rs",
    find: "                    if this.forwards_clone_from(ty, span)? {\n",
    replace: "                    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_option_own.rs$"],
  },
  {
    name: "generic-clone-from-place-not-given-back",
    breaks: "a generic `clone_from` of a local, given its dictionary's `cloneFrom` in a box of it, never takes back what it wrote: a `None` written over by a `Some` stays `None` (#928)",
    file: "src/lower/mir/std_calls.rs",
    find: "                                StmtKind::Assign(to.clone(), Expr::member(Expr::var(name), js::HELD))\n",
    replace: "                                StmtKind::Expr(Expr::member(Expr::var(name), js::HELD))\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_generic_std_own.rs$"],
  },
  {
    name: "generic-clone-from-box-made-again",
    breaks: "a generic `clone_from` of what its `&mut T` points at gives its dictionary a box of a copy, `{ value: dest.value }`, not the box it was given (#928)",
    file: "src/lower/mir/std_calls.rs",
    find: "                        Value::Ref(place) => this.box_of_place(state, place),\n",
    replace: "                        Value::Ref(_) => None,\n",
    tests: ["test/lowering-std-unwinds.test.ts", "-t", "generic clone_from"],
  },
  {
    name: "deque-range-mut-generic-unboxed",
    breaks: "`VecDeque::range_mut` in generic code steps the items, not cells of them, which generic code writes as boxes: JS throws (#1007)",
    file: "src/lower/mir/std_calls.rs",
    find: "                match this.mut_ref_form(generic_args.type_at(0)).is_cell() {\n",
    replace: "                match this.mut_ref_form(generic_args.type_at(0)).is_cell() && !this.is_unknown(generic_args.type_at(0)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_mut_lent_by_std.rs$"],
  },
  {
    name: "fmt-as-str-of-formatted",
    breaks:
      "`as_str()` of arguments made with a value to format is `Some` of their text, so `format_args!(\"{count} items\").as_str()` is `Some(\"3 items\")` where native Rust's is `None`, and anyhow's `format_err` takes the literal's way (#1091)",
    file: "src/lower/mir/std_calls.rs",
    find: "                Some(false) => lower(|_, _, _, _| Ok(Value::Expr(Expr::undefined()))),\n",
    replace: "                Some(false) => lower(|_, _, values, _| Ok(values.next().expect(\"the arguments\"))),\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_arguments_as_str.rs$"],
  },
  {
    name: "fmt-as-str-of-elsewhere-taken",
    breaks: "`as_str()` of arguments made in another function, whose maker nothing here says, is `Some` of their text, `None` or not (#1091)",
    file: "src/lower/mir/std_calls.rs",
    find: "            Std::FmtAsStr => match this.arguments_made(_state.body, _state.at) {\n",
    replace: "            Std::FmtAsStr => match this.arguments_made(_state.body, _state.at).or(Some(true)) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "as_str\\(\\) of arguments made elsewhere"],
  },
  {
    name: "lent-iterator-place-made-a-js-iterator",
    breaks:
      "a `&mut` to a `Range` that's a `Vec`'s item, `&mut items[1]`, is a new JS iterator of it, `Iterator.from($index(items, 1))`, which `r.next()` steps apart from the item (#638)",
    file: "src/lower/mir/std_calls.rs",
    find: "                    && !lent_place\n",
    replace: "",
    tests: ["test/mut-ref-matrix.test.ts", "-t", "of iterator, held as item"],
  },
  {
    name: "mir-std-iterator-array",
    breaks: "`text.bytes()` is an array MIR steps as an iterator",
    file: "src/lower/mir/std_calls.rs",
    find: "                if (self.implements_iterator(output) || std_iterator)\n",
    replace:
      "                if false && (self.implements_iterator(output) || std_iterator)\n",
    tests: ["test/lowering-loops.test.ts", "-t", "stepped by hand"],
  },
  {
    name: "mir-boxed-iterator-wrapped",
    breaks:
      "`Box::new(Countdown(3))` is wrapped as an array's iterator, stepping its struct's fields",
    file: "src/lower/mir/std_calls.rs",
    find: "                    && !output.is_box()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "dyn_iterators"],
  },
  {
    name: "mir-std-param-output-an-iterator",
    breaks:
      "a `T` a std call gives, `unwrap_or(fallback)`'s, is wrapped `Iterator.from(..)` from MIR, which throws of a number",
    file: "src/lower/mir/std_calls.rs",
    find: "                    && !matches!(output.kind(), ty::Param(_))\n                    && !output.is_box()",
    replace: "                    && !output.is_box()",
    tests: ["test/corpus.test.ts", "-t", "generic_from_str"],
  },
  {
    name: "result-method-write-result-object",
    breaks:
      "`map` of an `io::Result<usize>` into an `io::Result<()>` gives a `{ TAG }` object, which a caller takes for its error",
    file: "src/lower/mir/std_calls.rs",
    find: '                Value::Expr(Expr::call_helper("$writeOf", vec![e]))\n',
    replace: "                Value::Expr(e)\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "std-iterator-of-item-unmade",
    breaks:
      "std's iterator of another's items, `option::IntoIter<I::Item>`, is left the array it's made as, which `collect` can't step",
    file: "src/lower/mir/std_calls.rs",
    find: "                if (self.implements_iterator(output) || std_iterator)\n",
    replace: "                if self.implements_iterator(output)\n",
    tests: [
      "test/corpus.test.ts",
      "test/iterator-forms.test.ts",
      "-t",
      "^boxed_option_unwraps.rs$|^option_iterator_of_generic_items.rs$|^generic_accumulations with",
    ],
  },
  {
    name: "mir-str-deref-mut-refused",
    breaks:
      "MIR's `&mut str` of a `String`, `s.make_ascii_uppercase()`'s, is refused",
    file: "src/lower/mir/std_calls.rs",
    find: "        if self.is_cell(output) {\n            return Some(Value::Ref(place.clone()));\n",
    replace:
      "        if false && self.is_cell(output) {\n            return Some(Value::Ref(place.clone()));\n",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "pin-mut-gives-a-handle",
    breaks:
      "`*pin += 1` of a pinned `&mut` to a number writes through a handle, not the number",
    file: "src/lower/mir/std_calls.rs",
    find: "        if !matches!(known, Std::Same | Std::Pin(crate::lower::std_types::pin::PinOp::Mut)) {\n",
    replace: "        if !matches!(known, Std::Same) {\n",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
    snapshots: true,
  },
  {
    name: "box-as-mut-object-a-handle",
    breaks:
      "a `Box`'s `as_mut()` of an object is a handle on its place, so `b.as_mut().push(2)` pushes onto the handle",
    file: "src/lower/mir/std_calls.rs",
    find: "        (known == Std::Same && matches!(output.kind(), ty::Ref(_, _, ty::Mutability::Mut)))\n            .then(|| Value::Expr(place.clone()))\n",
    replace: "        None\n",
    tests: ["test/corpus.test.ts", "-t", "^box_as_mut_and_ref.rs$"],
  },
  {
    name: "splice-eager-where-items-drop",
    breaks: "a `splice` of items with a destructor is put in place where it's made, so what it took out drops after the replacement's steps, native Rust's other order (#1149, #1189)",
    file: "src/lower/mir/std_calls.rs",
    find: "                    let lazy = op == TextOp::Splice && this.drops(output) != crate::lower::drops::Drops::Nothing;\n",
    replace: "                    let lazy = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^splice_drops_and_fills_as_std.rs$"],
  },
  {
    name: "retain-closure-taken-for-unwinding-never",
    breaks:
      "a closure given a `String`'s `retain` is taken never to unwind, so its panic leaves the text it had (#1171)",
    file: "src/lower/mir/std_calls.rs",
    find: "                        ty::Closure(id, _) => !this.krate.never_unwinding.contains(&id),\n",
    replace: "                        ty::Closure(..) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^edits_that_panic_keep_what_std_keeps.rs$"],
  },
  {
    name: "placeholder-dyn-debug-made-a-pair",
    breaks: "`{:#?}` of `&x as &dyn Debug`, shown where it's made, makes the pair and shows it through its `impl` (#1335)",
    file: "src/lower/mir/std_calls.rs",
    find: "                        Value::DynDebug(from, _, value) if known == Std::FmtDebug => {\n",
    replace: "                        Value::DynDebug(from, _, value) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(dyn_debug_shown_where_rust_calls_fmt|pretty_debug)\\.rs$"],
    snapshots: true,
  },
];
