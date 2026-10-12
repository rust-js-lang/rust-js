// Mutations of src/lower/mir.rs (ADR 0364).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-fold-out-of-order",
    breaks: "temporaries that do something are folded where they're read, not in the order they were made",
    file: "src/lower/mir.rs",
    find: "        let ordered = acting.windows(2).all(|w| w[0] < w[1]);\n",
    replace: "        let ordered = true;\n",
    tests: ["test/corpus.test.ts", "-t", "order"],
  },
  {
    name: "mir-assert-inverted",
    breaks: "a MIR `Assert` throws when its condition holds, not when it fails",
    file: "src/lower/mir.rs",
    find: "                    true => Expr::unary(js::UnaryOp::Not, cond),\n",
    replace: "                    true => cond,\n",
    tests: ["test/corpus.test.ts", "-t", "bitwise_operators"],
  },
  {
    name: "mir-folded-call-uncaught",
    breaks: "a call written where it's used, which may panic as something's there to drop, isn't in a `try`",
    file: "src/lower/mir.rs",
    find: "            self.unwinding_to(state, unwind.filter(|_| ran), made, &mut out)?;\n",
    replace: "            self.unwinding_to(state, None, made, &mut out)?;\n",
    tests: ["test/corpus.test.ts","-t","drop|refcell_"],
  },
  {
    name: "mir-const-param-unread",
    breaks: "a const generic parameter is a constant MIR can't name, not its caller's value",
    file: "src/lower/mir.rs",
    find: "                ty::ConstKind::Param(_) => return self.const_arg(ct, c.span),\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","const_generic_values"],
  },
  {
    name: "mir-static-mut-not-a-place",
    breaks: "a `static mut` is its value, not the place its `{ value }` holds, so writes to it are lost",
    file: "src/lower/mir.rs",
    find: "                Ok(Value::Ref(Expr::member(self.fn_ref(def_id), \"value\")))\n",
    replace: "                Ok(Value::Expr(self.fn_ref(def_id)))\n",
    tests: ["test/corpus.test.ts","-t","static_mut_shared"],
  },
  {
    name: "mir-promoted-const-unseen",
    breaks: "a promoted `&LOG` of a thread local is a constant MIR can't name",
    file: "src/lower/mir.rs",
    find: "    (Some(local) == referred).then_some(c)\n",
    replace: "    (Some(local) == referred).then_some(c).filter(|_| false)\n",
    tests: ["test/corpus.test.ts","-t","thread_local"],
  },
  {
    name: "mir-box-not-copied-back",
    breaks: "a box given for a `&mut` number isn't copied back, so the caller never sees what the call wrote",
    file: "src/lower/mir.rs",
    find: "                                    false => StmtKind::Assign(place, back).at(js_span),\n",
    replace: "                                    false => StmtKind::Expr(Expr::undefined()).at(js_span),\n",
    tests: ["test/corpus.test.ts","-t","mut_number_args"],
  },
  {
    name: "mir-branch-continue-is-break",
    breaks: "`?` goes on with an `Err` or a `None`, and returns early with what's inside",
    file: "src/lower/mir.rs",
    find: "                    0 => negate(failed),\n                    _ => failed,\n",
    replace: "                    0 => failed,\n                    _ => negate(failed),\n",
    tests: ["test/corpus.test.ts","-t","question_early_returns"],
  },
  {
    name: "mir-residual-err-lost",
    breaks: "`?` of an `Err` returns nothing, not the `Err`",
    file: "src/lower/mir.rs",
    find: "                    Some(_) => Expr::undefined(),\n                    None => tried,\n",
    replace: "                    Some(_) => Expr::undefined(),\n                    None => Expr::undefined(),\n",
    tests: ["test/corpus.test.ts","-t","question_early_returns"],
  },
  {
    name: "mir-early-exit-not-negated",
    breaks: "a branch that returns, written first, is taken when its test fails, not when it holds",
    file: "src/lower/mir.rs",
    find: "            let mut out = vec![StmtKind::If(negate(test), otherwise, None).at(span)];\n",
    replace: "            let mut out = vec![StmtKind::If(test, otherwise, None).at(span)];\n",
    tests: ["test/lowering.test.ts","-t","branches from MIR read"],
  },
  {
    name: "mir-push-str-lost",
    breaks: "`write!` or `push_str` to a `String` from MIR writes nothing",
    file: "src/lower/mir.rs",
    find: "                super::display::append_written(&place, value, false, self.js_span(span), out);\n",
    replace: "                let _ = (place, value);\n",
    tests: ["test/corpus.test.ts", "-t", "string_pushes"],
  },
  {
    name: "mir-fmt-result-unwrapped",
    breaks: "`write!(s, ..).unwrap()` unwraps the nothing an `Ok` `fmt::Result` is in JS, as if it were a `Result`",
    file: "src/lower/mir.rs",
    find: "        if arg_tys.first().is_some_and(|ty| self.is_fmt_result(ty.peel_refs()))\n",
    replace: "        if false && arg_tys.first().is_some_and(|ty| self.is_fmt_result(ty.peel_refs()))\n",
    tests: ["test/corpus.test.ts","-t","string_pushes"],
  },
  {
    name: "mir-comb-subject-dropped",
    breaks: "an `Option`'s combinator from MIR is given its argument where its subject goes",
    file: "src/lower/mir.rs",
    find: "                self.comb_values(comb, (exprs, arg_tys), made, generic_args, span, out)?\n",
    replace: "                self.comb_values(comb, (exprs[1..].to_vec(), &arg_tys[1..]), made, generic_args, span, out)?\n",
    tests: ["test/corpus.test.ts","-t","option_as_deref|combinators_owned_drops"],
  },
  {
    name: "mir-map-call-refused",
    breaks: "a map's method from MIR is refused, as if THIR alone lowered it",
    file: "src/lower/mir.rs",
    find: "                self.map_values(op, (exprs, arg_tys), generic_args, state.discarding, span, out)?\n",
    replace: "                return Err(self.unsupported(span, \"a map's method\"));\n",
    tests: ["test/corpus.test.ts","-t","map_index_missing"],
  },
  {
    name: "mir-handle-deref-unread",
    breaks: "`*h.count` of a handle a struct holds is the handle, not the number it's to",
    file: "src/lower/mir.rs",
    find: "                if !std::mem::take(&mut placed) {\n                    value = Expr::member(value, \"value\");\n                }\n",
    replace: "                let _ = std::mem::take(&mut placed);\n",
    tests: ["test/corpus.test.ts","-t","mut_number_handles"],
  },
  {
    name: "mir-borrowed-result-given-value",
    breaks: "a `&mut` whose borrow a call's result holds is given the number, not a handle on its place",
    file: "src/lower/mir.rs",
    find: "                exprs.push(Expr::handle(place));\n                continue;\n",
    replace: "                exprs.push(place);\n                continue;\n",
    tests: ["test/corpus.test.ts","-t","mut_number_handles"],
  },
  {
    name: "mir-iter-comb-not-lazy",
    breaks: "an iterator's combinator from MIR takes its items as an array, though they're a JS iterator",
    file: "src/lower/mir.rs",
    find: "                self.iter_comb(comb, it, exprs.into_iter(), generic_args, iter_ty, true, span, out)?\n",
    replace: "                self.iter_comb(comb, it, exprs.into_iter(), generic_args, iter_ty, false, span, out)?\n",
    tests: ["test/corpus.test.ts","-t","iter_flatten"],
  },
  {
    name: "mir-radix-argument-unseen",
    breaks: "a `{:x}` argument from MIR isn't one `format_args!` shows by its spec",
    file: "src/lower/mir.rs",
    find: "            Std::FmtDisplay | Std::FmtDebug | Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {\n",
    replace: "            Std::FmtDisplay | Std::FmtDebug | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {\n",
    tests: ["test/corpus.test.ts","-t","radix_negative"],
  },
  {
    name: "mir-sort-unsorted",
    breaks: "`v.sort()` from MIR leaves the items as they were",
    file: "src/lower/mir.rs",
    find: "                self.sort_values(key, items, arg_tys[0], generic_args, span, out)?\n",
    replace: "                items\n",
    tests: ["test/corpus.test.ts","-t","generic_borrow"],
  },
  {
    name: "mir-swap-one-way",
    breaks: "`mem::swap(&mut a, &mut b)` of two places writes `b` into `a`, and leaves `b`",
    file: "src/lower/mir.rs",
    find: "                out.push(StmtKind::Assign(b, old).at(js_span));\n",
    replace: "                let _ = (b, old);\n",
    tests: ["test/corpus.test.ts","-t","swap_replace"],
  },
  {
    name: "mir-view-whole",
    breaks: "`&mut v[a..b]` from MIR is all of `v`, not a view of those items",
    file: "src/lower/mir.rs",
    find: "                    self.view_call(if checked { \"$viewGet\" } else { \"$view\" }, list)\n",
    replace: "                    list.remove(0)\n",
    tests: ["test/corpus.test.ts","-t","mut_ref_loop_bounds"],
  },
  {
    name: "mir-fn-value-undefined",
    breaks: "a function as a value from MIR is `undefined`, not the function",
    file: "src/lower/mir.rs",
    find: "                    Some(f) => Ok(Value::Expr(f)),\n",
    replace: "                    Some(_) => Ok(Value::Expr(Expr::undefined())),\n",
    tests: ["test/corpus.test.ts", "-t", "fn_items_as_values"],
  },
  {
    name: "mir-called-value-returns-nothing",
    breaks: "the arrow that calls a std function as a value returns nothing",
    file: "src/lower/mir.rs",
    find: "        body.push(StmtKind::Return(Some(value)).at(self.js_span(span)));\n        Ok(Expr::arrow(params.into_iter().map(Into::into).collect(), body))\n",
    replace: "        let _ = value;\n        Ok(Expr::arrow(params.into_iter().map(Into::into).collect(), body))\n",
    tests: ["test/corpus.test.ts","-t","as_deref|str_circumfix|std_function_value_panic"],
  },
  {
    name: "mir-fn-pointer-call-refused",
    breaks: "a call through a function pointer from MIR is refused",
    file: "src/lower/mir.rs",
    find: "            return Ok(Value::Expr(Expr::call(callee, exprs)));\n        };\n",
    replace: "            let _ = (callee, exprs);\n            return Err(self.unsupported(span, \"a call\"));\n        };\n",
    tests: ["test/corpus.test.ts","-t","static_fns"],
  },
  {
    name: "mir-ctor-call-unapplied",
    breaks: "a constructor called from MIR is its arrow, not what it makes",
    file: "src/lower/mir.rs",
    find: "            return Ok(Value::Expr(super::calls::apply(made, exprs)));\n",
    replace: "            let _ = exprs;\n            return Ok(Value::Expr(made));\n",
    tests: ["test/corpus.test.ts","-t","function_values"],
  },
  {
    name: "mir-rc-call-refused",
    breaks: "an `Rc`'s function from MIR is refused",
    file: "src/lower/mir.rs",
    find: "                self.rc_values(op, (exprs, arg_tys), span, out)?\n",
    replace: "                return Err(self.unsupported(span, \"an Rc's\"));\n",
    tests: ["test/corpus.test.ts","-t","as_deref_counted"],
  },
  {
    name: "mir-copy-shared",
    breaks: "a `Copy` struct read from a variable is the same JS object, so a change to the copy changes it",
    file: "src/lower/mir.rs",
    find: "            Value::Expr(e) if copied => {\n",
    replace: "            Value::Expr(e) if false && copied => {\n",
    tests: ["test/corpus.test.ts","-t","array_map"],
  },
  {
    name: "mir-drop-uses-unnoted",
    breaks: "a generic function from MIR isn't given the drop functions it uses, so what it drops isn't",
    file: "src/lower/mir.rs",
    find: "        self.note_drop_uses(def_id);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","arm_temporaries"],
  },
  {
    name: "mir-object-unboxed",
    breaks: "an object given where a generic `&mut T` goes isn't boxed, so the callee reads `value` of it",
    file: "src/lower/mir.rs",
    find: "                Value::Expr(object) if fn_id.is_some_and(|fn_id| self.param_is_box(fn_id, i)) => object,\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","generic_traits|mut_ref_as_generic|generic_mut_ref"],
  },
  {
    name: "mir-static-read-movable",
    breaks: "a read of a static, which any call may change, is moved past a call that changes it",
    file: "src/lower/mir.rs",
    find: "                stable &= !state.borrowed_names.contains(name) && state.own_names.contains(name);\n",
    replace: "                stable &= !state.borrowed_names.contains(name);\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-shared-temporary-a-place",
    breaks: "a shared reference to a temporary's read of a static is read where it's used, after what changes the static",
    file: "src/lower/mir.rs",
    find: "                            if e.reads_same() && movable(state, &Value::Expr(e.clone())) =>\n",
    replace: "                            if e.reads_same() =>\n",
    tests: ["test/lowering.test.ts","-t","a nested pattern of a captured item from MIR"],
  },
  {
    name: "mir-index-mut-a-value",
    breaks: "`v[i] += 1` of numbers through `index_mut` changes a copy of the element",
    file: "src/lower/mir.rs",
    find: "            return Ok(Value::Ref(Expr::index(items, index)));\n",
    replace: "            return Ok(Value::Expr(Expr::index(items, index)));\n",
    tests: ["test/corpus.test.ts","-t","compound_assignment_order"],
  },
  {
    name: "mir-item-handles-skipped",
    breaks: "`iter_mut()` of strings gives the strings, not handles on them, so a write to one is lost",
    file: "src/lower/mir.rs",
    find: "            if let Some(handles) = self.item_handles_of(known, exprs, receiver, span, out)? {\n",
    replace: "            if let Some(handles) = self.item_handles_of(known, exprs, receiver, span, out)?.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_std_items"],
  },
  {
    name: "mir-cell-deref-skipped",
    breaks: "a reborrow through a handle on a `Box<dyn Trait>` is the handle, not the pair it holds",
    file: "src/lower/mir.rs",
    find: "                            && !matches!(ty.ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if self.is_cell_pointee(*inner)))\n",
    replace: "                            && true)\n",
    tests: ["test/corpus.test.ts","-t","dyn_mut"],
  },
  {
    name: "mir-union-field-read",
    breaks: "a union's field read from its MIR crashes rust-js, not refused",
    file: "src/lower/mir.rs",
    find: "            (ty::Adt(adt, _), _) if adt.is_union() => Err(self.unsupported(span, \"unions\")),\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","union_const"],
  },
  {
    name: "mir-copied-closure-shared",
    breaks: "a closure that changes what it captured, copied, shares its captures, as a JS function does",
    file: "src/lower/mir.rs",
    find: "            && state.locals.reads[place.local] > 1\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts","-t","copied_closure"],
  },
  {
    name: "mir-unsize-unchecked",
    breaks: "a struct whose last field is a `dyn`, unsized, is compiled, and its `dyn` read as the value",
    file: "src/lower/mir.rs",
    find: "                            self.check_unsize(*to, span)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","unsized_struct_dyn"],
  },
  {
    name: "mir-boxed-option-unrefused",
    breaks: "a std call's `Option` of what looks like `None` is taken as `None`",
    file: "src/lower/mir.rs",
    find: "        if self.refuses_boxed_option(known, output) {\n",
    replace: "        if false && self.refuses_boxed_option(known, output) {\n",
    tests: ["test/corpus.test.ts","-t","as_deref_none_like"],
  },
  {
    name: "mir-takes-drops-unchecked",
    breaks: "`clear` of a generic `Vec` drops its items unrun",
    file: "src/lower/mir.rs",
    find: "        self.check_takes_drops(known, def_id, arg_tys, drained, span)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","drop_impl_clear"],
  },
  {
    name: "mir-mut-borrowed-unchanged",
    breaks: "`e += inc()` reads `e` before the closure that changes it runs",
    file: "src/lower/mir.rs",
    find: "                            locals.changed[borrowed.local] = true;\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","assignment_order"],
  },
  {
    name: "mir-changed-operand-stable",
    breaks: "an operand a call may change is read before the call",
    file: "src/lower/mir.rs",
    find: "                place.projection.is_empty() && !state.locals.borrowed[place.local] && !state.locals.changed[place.local]\n",
    replace: "                place.projection.is_empty() && !state.locals.borrowed[place.local]\n",
    tests: ["test/corpus.test.ts","-t","assignment_order"],
  },
  {
    name: "mir-whole-mut-borrow-unwritten",
    breaks: "`Box::new_uninit()`, written through `write`, is a `const` assigned again",
    file: "src/lower/mir.rs",
    find: "                            locals.writes[borrowed.local] += 1;\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","box_uninit"],
  },
  {
    name: "mir-iterator-ref-refused",
    breaks: "a `&mut` to a std iterator is a handle on it, which JS can't step",
    file: "src/lower/mir.rs",
    find: "                    Value::Ref(iterator) => Ok(iterator),\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","generic_associated_types"],
  },
  {
    name: "mir-format-width-dropped",
    breaks: "`{:5?}` from MIR pads nothing",
    file: "src/lower/mir.rs",
    find: "                                None => spec.width.map(|w| Expr::int(w.into())),\n",
    replace: "                                None => None,\n",
    tests: ["test/corpus.test.ts","-t","debug_options"],
  },
  {
    name: "mir-format-precision-dropped",
    breaks: "`{:.1?}` from MIR shows every digit",
    file: "src/lower/mir.rs",
    find: "                                None => spec.precision.map(|p| Expr::int(p.into())),\n",
    replace: "                                None => None,\n",
    tests: ["test/corpus.test.ts","-t","debug_options"],
  },
  {
    name: "mir-format-width-argument-dropped",
    breaks: "`{:>w$}` from MIR pads nothing",
    file: "src/lower/mir.rs",
    find: "                            let width = match spec.width_from {\n                                Some(from) => Some(item(from).ok_or_else(bad)?.2),\n",
    replace: "                            let width = match spec.width_from {\n                                Some(_) => None,\n",
    tests: ["test/corpus.test.ts","-t","format_width_arguments"],
  },
  {
    name: "mir-shift-amount-bigint",
    breaks: "`22i32 >> by` of an `i64` `by` shifts a number by a BigInt",
    file: "src/lower/mir.rs",
    find: "super::std_types::number::shift_amount_of(*op, self.value_expr(r, span)?, ty, r_ty)",
    replace: "{ let _ = r_ty; self.value_expr(r, span)? }",
    tests: ["test/corpus.test.ts","-t","wrapping"],
  },
  {
    name: "mir-assign-shift-amount-bigint",
    breaks: "`c <<= &3i64` of a `u32` shifts a number by a BigInt",
    file: "src/lower/mir.rs",
    find: "            let rhs = super::std_types::number::shift_amount_of(op, rhs, ty, arg_tys[1]);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","generic_bitwise_operators"],
  },
  {
    name: "mir-std-iterator-array",
    breaks: "`text.bytes()` is an array MIR steps as an iterator",
    file: "src/lower/mir.rs",
    find: "                    if self.implements_iterator(output)\n",
    replace: "                    if false && self.implements_iterator(output)\n",
    tests: ["test/lowering.test.ts","-t","stepped by hand"],
  },
  {
    name: "mir-to-int-unchecked-uncast",
    breaks: "`x.to_int_unchecked()` under MIR reaches `number_call`, which has none",
    file: "src/lower/mir.rs",
    find: "            Std::Number(NumOp::ToIntUnchecked) => {\n",
    replace: "            Std::Number(NumOp::ToIntUnchecked) if false => {\n",
    tests: ["test/corpus.test.ts","-t","integers_128"],
  },
  {
    name: "mir-static-copy-shared",
    breaks: "`let mut p = ORIGIN;` of a `Copy` static changes the static",
    file: "src/lower/mir.rs",
    find: "if place.is_indirect() || !state.pending",
    replace: "if !state.pending",
    tests: ["test/corpus.test.ts","-t","statics"],
  },
  {
    name: "mir-object-replace-rebinds",
    breaks: "`*self = v` through a `&mut` to an object rebinds `self`, not the object",
    file: "src/lower/mir.rs",
    find: "            && self.is_object(pointee)\n",
    replace: "            && false && self.is_object(pointee)\n",
    tests: ["test/corpus.test.ts","-t","replace_through_mut"],
  },
  {
    name: "mir-handle-boxed-by-value",
    breaks: "a handle `first_mut().unwrap()` made is boxed by its value, and writes to the box are lost",
    file: "src/lower/mir.rs",
    find: "            if !fixed_place(&place) && !matches!(place.kind, js::ExprKind::Undefined) {\n",
    replace: "            if false && !fixed_place(&place) && !matches!(place.kind, js::ExprKind::Undefined) {\n",
    tests: ["test/corpus.test.ts","-t","std_mut_items"],
  },
  {
    name: "mir-unit-place-refused",
    breaks: "`pass(&mut unit)` of a `()` is refused as a place that isn't fixed",
    file: "src/lower/mir.rs",
    find: "            if !fixed_place(&place) && !matches!(place.kind, js::ExprKind::Undefined) {\n",
    replace: "            if !fixed_place(&place) {\n",
    tests: ["test/corpus.test.ts","-t","mut_ref_as_generic"],
  },
  {
    name: "mir-format-failing-unmarked",
    breaks: "`format!` of a `Display` that fails throws a `fmt::Error`, not std's panic",
    file: "src/lower/mir.rs",
    find: "                if fails {\n                    return Ok(Value::Fmt(Std::FmtNew, output, joined));\n",
    replace: "                if false && fails {\n                    return Ok(Value::Fmt(Std::FmtNew, output, joined));\n",
    tests: ["test/corpus.test.ts","-t","fmt_error_format"],
  },
  {
    name: "mir-print-failing-unwrapped",
    breaks: "`println!` of a `Display` that fails throws, writing nothing of what it wrote",
    file: "src/lower/mir.rs",
    find: "            Std::Format | Std::Print { .. } => matches!(values.first(), Some(Value::Fmt(Std::FmtNew, ..))),\n",
    replace: "            Std::Format => matches!(values.first(), Some(Value::Fmt(Std::FmtNew, ..))),\n",
    tests: ["test/corpus.test.ts","-t","fmt_error_println"],
  },
  {
    name: "mir-to-string-failing-unwrapped",
    breaks: "`to_string()` of a `Display` that fails throws a `fmt::Error`, not std's panic",
    file: "src/lower/mir.rs",
    find: "            Std::ToString => first_ty().is_some_and(|ty| self.fmt_may_fail(self.display_trait(), ty)),\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","fmt_error_to_string"],
  },
  {
    name: "mir-user-iterator-arg-unwrapped",
    breaks: "an iterator of the crate's given where a generic iterator goes is its struct, which JS can't step",
    file: "src/lower/mir.rs",
    find: "                if matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(def_id, input, given) {\n",
    replace: "                if false && matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(def_id, input, given) {\n",
    tests: ["test/corpus.test.ts","-t","generic_iterator_rest_lazy"],
  },
  {
    name: "mir-escape-iterator-wrapped",
    breaks: "`escape_ascii()`'s text is made a JS iterator, and shown as one",
    file: "src/lower/mir.rs",
    find: "                        && !super::recognition::is_text_escape(tcx, output) =>\n",
    replace: "                        && (true || !super::recognition::is_text_escape(tcx, output)) =>\n",
    tests: ["test/corpus.test.ts","-t","slice_bytes_text"],
  },
  {
    name: "mir-made-before-block-scoped",
    breaks: "what's made before a labeled block and read after it is declared inside it",
    file: "src/lower/mir.rs",
    find: "        if let Some(last) = state.pending.iter().rposition(|(local, _)| !read.contains(local)) {\n",
    replace: "        if false && let Some(last) = state.pending.iter().rposition(|(local, _)| !read.contains(local)) {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-object-copied-back-rebound",
    breaks: "an object given in a box for a generic `&mut T` is copied back by assigning its variable, a `const`",
    file: "src/lower/mir.rs",
    find: "            let object = !matches!(value, Value::Ref(_));\n",
    replace: "            let object = false && !matches!(value, Value::Ref(_));\n",
    tests: ["test/corpus.test.ts","-t","evidence_paths"],
  },
  {
    name: "mir-boxed-iterator-wrapped",
    breaks: "`Box::new(Countdown(3))` is wrapped as an array's iterator, stepping its struct's fields",
    file: "src/lower/mir.rs",
    find: "                        && !output.is_box()\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","dyn_iterators"],
  },
  {
    name: "mir-rc-dyn-unpaired",
    breaks: "`Rc<dyn Any>` made from MIR is the value, not its pair with its dictionary",
    file: "src/lower/mir.rs",
    find: "                let to_dyn = matches!(self.pointee(*to).kind(), ty::Dynamic(..));\n",
    replace: "                let to_dyn = to.builtin_deref(true).is_some_and(|p| matches!(p.kind(), ty::Dynamic(..)));\n",
    tests: ["test/corpus.test.ts","-t","dyn_any_generic"],
  },
  {
    name: "mir-peek-steps",
    breaks: "`p.peek()` of a `Peekable` steps it",
    file: "src/lower/mir.rs",
    find: "StepOp::Peek => self.helper(Helper::Peek, \"$peek\", vec![it]),",
    replace: "StepOp::Peek => self.helper(Helper::Next, \"$next\", vec![it]),",
    tests: ["test/corpus.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-next-if-eq-unequal",
    breaks: "`p.next_if_eq(&x)` takes what isn't `x`",
    file: "src/lower/mir.rs",
    find: "Expr::bin(Op::Eq, Expr::var(\"item\"), x)",
    replace: "Expr::bin(Op::Ne, Expr::var(\"item\"), x)",
    tests: ["test/corpus.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-as-slice-whole",
    breaks: "`it.as_slice()` of a stepped `$iter` is all its items",
    file: "src/lower/mir.rs",
    find: "vec![Expr::member(it.clone(), \"at\"), Expr::member(it, \"end\")],",
    replace: "vec![Expr::int(0), Expr::member(it, \"end\")],",
    tests: ["test/corpus.test.ts","-t","iterator_back_ends"],
  },
  {
    name: "mir-default-size-hint-one",
    breaks: "std's `size_hint()` of an iterator of the crate's is `(1, None)`",
    file: "src/lower/mir.rs",
    find: "Expr::array(vec![Expr::int(0), Expr::undefined()])",
    replace: "Expr::array(vec![Expr::int(1), Expr::undefined()])",
    tests: ["test/corpus.test.ts","-t","size_hint"],
  },
  {
    name: "mir-entry-number-unhandled",
    breaks: "`*m.entry(k).or_insert(0) += 1` adds to the value read, writing nothing back",
    file: "src/lower/mir.rs",
    find: "            if !self.is_cell(output) {\n",
    replace: "            if true || !self.is_cell(output) {\n",
    tests: ["test/corpus.test.ts","-t","entry_counts"],
  },
  {
    name: "mir-kept-entry-parts",
    breaks: "an entry kept in a variable is taken apart as `[m, k]`, not `$entry`'s `_0`",
    file: "src/lower/mir.rs",
    find: "                    let parts = Expr::member(entry, \"_0\");\n",
    replace: "                    let parts = entry;\n",
    tests: ["test/corpus.test.ts","-t","entry_kept"],
  },
  {
    name: "mir-zero-sized-const-refused",
    breaks: "`Self` of a unit struct, a constant of nothing, is refused",
    file: "src/lower/mir.rs",
    find: "                if let mir::ConstValue::ZeroSized = value {\n",
    replace: "                if false && let mir::ConstValue::ZeroSized = value {\n",
    tests: ["test/corpus.test.ts","-t","expression_values"],
  },
  {
    name: "mir-promoted-variant-refused",
    breaks: "`&None` of generic code, a promoted rustc can't evaluate, is refused",
    file: "src/lower/mir.rs",
    find: "            && let Some((made, variant)) = referred_variant(tcx, &state.promoted[promoted])\n",
    replace: "            && let Some((made, variant)) = referred_variant(tcx, &state.promoted[promoted]).filter(|_| false)\n",
    tests: ["test/corpus.test.ts","-t","mut_ref_box_parts"],
  },
  {
    name: "mir-dyn-debug-made-early",
    breaks: "`{:#?}` of a `&dyn Debug` shows it as `{:?}` does, made before its spec is known",
    file: "src/lower/mir.rs",
    find: "                        if self.is_dyn_debug(*to) && !self.is_dyn_debug(from) {\n",
    replace: "                        if false && self.is_dyn_debug(*to) && !self.is_dyn_debug(from) {\n",
    tests: ["test/corpus.test.ts","-t","pretty_debug"],
  },
  {
    name: "mir-take-field-reordered",
    breaks: "a tuple's call taken before its later calls lets a flush run them first",
    file: "src/lower/mir.rs",
    find: "            && (movable(state, &items[field]) || items[field + 1..].iter().all(|item| movable(state, item)));\n",
    replace: "            && true;\n",
    tests: ["test/corpus.test.ts","-t","byte_strings|statics"],
  },
  {
    name: "mir-out-of-order-fields-undeclared",
    breaks: "a tuple read out of order is read by a name nothing declares",
    file: "src/lower/mir.rs",
    find: "                let rest = state.pending.split_off(at + 1);\n                self.flush(state, out)?;\n                state.pending = rest;\n",
    replace: "                let _ = at;\n",
    tests: ["test/corpus.test.ts","-t","range_inclusive_next_back"],
  },
  {
    name: "mir-copies-shared",
    breaks: "`vec![p; 3]` of a struct is three names for one object",
    file: "src/lower/mir.rs",
    find: "self.copies_values((item, arg_tys[0]), n, false, span, out)?",
    replace: "self.copies_values((item, tcx.types.unit), n, false, span, out)?",
    tests: ["test/corpus.test.ts","-t","copy_mutation"],
  },
  {
    name: "mir-non-zero-new-refused",
    breaks: "`NonZero::new(n)` is refused",
    file: "src/lower/mir.rs",
    find: "            Std::NonZeroNew => {\n",
    replace: "            Std::NonZeroNew if false => {\n",
    tests: ["test/corpus.test.ts","-t","nonzero"],
  },
  {
    name: "mir-string-edit-refused",
    breaks: "`s.insert(i, c)` and the rest of a string's edits are refused",
    file: "src/lower/mir.rs",
    find: "            Std::StringEdit(edit) => {\n",
    replace: "            Std::StringEdit(edit) if false => {\n",
    tests: ["test/corpus.test.ts","-t","text_edits|string_editing"],
  },
  {
    name: "mir-string-range-swapped",
    breaks: "`s.replace_range(a..b, t)` takes its range's end for its start",
    file: "src/lower/mir.rs",
    find: "                    given.splice(0..0, [start, end.unwrap_or_else(Expr::undefined)]);\n",
    replace: "                    given.splice(0..0, [end.unwrap_or_else(Expr::undefined), start]);\n",
    tests: ["test/corpus.test.ts","-t","text_edits|string_editing|replace_range_panic|string_drain_from_past_len"],
  },
  {
    name: "mir-with-capacity-refused",
    breaks: "`String::with_capacity(n)` is refused",
    file: "src/lower/mir.rs",
    find: "            Std::StringWithCapacity => {\n",
    replace: "            Std::StringWithCapacity if false => {\n",
    tests: ["test/corpus.test.ts","-t","text_edits|string_editing|text_aliases|string_insert_boundary"],
  },
  {
    name: "mir-make-mut-refused",
    breaks: "`Rc::make_mut(&mut rc)` of a local `Rc` is refused",
    file: "src/lower/mir.rs",
    find: "            Std::Rc(RcOp::MakeMut) => {\n",
    replace: "            Std::Rc(RcOp::MakeMut) if false => {\n",
    tests: ["test/corpus.test.ts","-t","rc_clone_sees_owners|rc_make_mut_clone_panics"],
  },
  {
    name: "mir-promoted-call-refused",
    breaks: "`&size_of::<T>()` of generic code, a promoted call rustc can't evaluate, is refused",
    file: "src/lower/mir.rs",
    find: "            && let Some((def_id, args, output)) = referred_call(&state.promoted[promoted])\n",
    replace: "            && let Some((def_id, args, output)) = referred_call(&state.promoted[promoted]).filter(|_| false)\n",
    tests: ["test/corpus.test.ts","-t","type_facts"],
  },
  {
    name: "mir-type-name-refused",
    breaks: "`type_name::<T>()` is refused",
    file: "src/lower/mir.rs",
    find: "            Std::TypeName { .. } => {\n",
    replace: "            Std::TypeName { .. } if false => {\n",
    tests: ["test/corpus.test.ts","-t","type_facts"],
  },
  {
    name: "mir-index-from-end-front",
    breaks: "`[.., last]` of a slice reads its item counted from the front",
    file: "src/lower/mir.rs",
    find: "                    None => Expr::bin(Op::Sub, Expr::member(base.clone(), \"length\"), Expr::int(offset.into())),\n",
    replace: "                    None => Expr::int(offset.into()),\n",
    tests: ["test/corpus.test.ts","-t","slice_patterns"],
  },
  {
    name: "mir-subslice-whole",
    breaks: "`[first, rest @ ..]`'s rest is the whole slice",
    file: "src/lower/mir.rs",
    find: "                Expr::call(Expr::member(base, \"slice\"), range)\n",
    replace: "                { let _ = range; base }\n",
    tests: ["test/corpus.test.ts","-t","slice_patterns"],
  },
  {
    name: "mir-fake-raw-refused",
    breaks: "a slice pattern's length test, a fake raw borrow, is refused",
    file: "src/lower/mir.rs",
    find: "            Rvalue::RawPtr(mir::RawPtrKind::FakeForPtrMetadata, place) => self.mir_place(state, *place, span, out)?,\n",
    replace: "            Rvalue::RawPtr(mir::RawPtrKind::FakeForPtrMetadata, place) if false => self.mir_place(state, *place, span, out)?,\n",
    tests: ["test/corpus.test.ts","-t","slice_patterns"],
  },
  {
    name: "mir-mut-subslice-unrefused",
    breaks: "a `&mut` to a slice pattern's rest writes its copy",
    file: "src/lower/mir.rs",
    find: "                        .any(|elem| matches!(elem, PlaceElem::Subslice { .. }))\n",
    replace: "                        .any(|elem| false && matches!(elem, PlaceElem::Subslice { .. }))\n",
    tests: ["test/corpus.test.ts","-t","slice_pattern_mut_rest"],
  },
  {
    name: "mir-untagged-variant-tagged",
    breaks: "an untagged enum's variant is tested by a tag it has none of",
    file: "src/lower/mir.rs",
    find: "        if bindings::is_untagged(self.tcx, adt.did()) {\n            let test = self.untagged_variant_test(ty, def, &subject);\n",
    replace: "        if false && bindings::is_untagged(self.tcx, adt.did()) {\n            let test = self.untagged_variant_test(ty, def, &subject);\n",
    tests: ["test/corpus.test.ts","-t","untagged_enums"],
  },
  {
    name: "mir-element-made-as-const",
    breaks: "an element whose temporary is made before it's used is a `const`, which takes no props",
    file: "src/lower/mir.rs",
    find: "            if !named && (self.capture_jsx(&mut expr, &mut made) || self.capture_object(&mut expr, &mut made)) {\n",
    replace: "            if !named && self.capture_object(&mut expr, &mut made) {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-element-unread",
    breaks: "an element made where it's used is read by its temporary's name, never declared",
    file: "src/lower/mir.rs",
    find: "        if let Some(value) = state.in_place.get(&local) {\n",
    replace: "        if let Some(value) = state.in_place.get(&local).filter(|_| false) {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-omitted-prop-called",
    breaks: "a prop `jsx!` isn't given calls react's `__omitted`, which isn't JS's",
    file: "src/lower/mir.rs",
    find: "        if bindings::is_omitted(tcx, def_id) {\n",
    replace: "        if false && bindings::is_omitted(tcx, def_id) {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-jsx-shapeless",
    breaks: "a JSX call from MIR has no THIR call for its shape: props in the struct's order",
    file: "src/lower/mir.rs",
    find: "        calls.next().is_none().then_some(call)\n",
    replace: "        calls.next().is_none().then_some(call).filter(|_| false)\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-alike-reassigned",
    breaks: "a variable assigned again reads alike, so JSX reads it later than Rust does",
    file: "src/lower/mir.rs",
    find: "                || state.locals.writes[local] > 1 && !(state.locals.user[local] && once_each_way(mir_body, local))\n",
    replace: "",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-binding-component-called",
    breaks: "a component a JS module exports is its binding's value, not the tag JSX imports",
    file: "src/lower/mir.rs",
    find: "                            ty::FnDef(id, _) => cx.binding_component_of(id),\n",
    replace: "                            ty::FnDef(..) => None,\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-props-taken-whole",
    breaks: "props a pattern takes apart are taken whole, so a flattened field is all of them and a field's default is lost",
    file: "src/lower/mir.rs",
    find: "            params[i] = pattern;\n",
    replace: "",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-param-part-moved-again",
    breaks: "a part a parameter's pattern binds is moved out of the parameter again, which the pattern took apart",
    file: "src/lower/mir.rs",
    find: "        if let Some(&arg) = state.parts.get(&place.local)\n",
    replace: "        if let Some(&arg) = state.parts.get(&place.local).filter(|_| false)\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-pattern-names-renamed",
    breaks: "what a parameter's pattern binds is named again, unlike its pattern's names",
    file: "src/lower/mir.rs",
    find: "            if let Some((_, name)) = named.iter().find(|(l, _)| *l == local) {\n",
    replace: "            if let Some((_, name)) = named.iter().find(|(l, _)| *l == local).filter(|_| false) {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-constant-made-as-const",
    breaks: "a constant made before it's used is a `const`, which isn't the `undefined` a flattened struct's field left out is",
    file: "src/lower/mir.rs",
    find: "            _ => e.is_constant(),\n",
    replace: "            _ => false,\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-plain-of-bound-vars",
    breaks: "a local's type inside a binder, a function pointer's parameter, is asked whether it's `Freeze`, which crashes rustc",
    file: "src/lower/mir.rs",
    find: "                        && !t.has_escaping_bound_vars()\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","fn_pointer_of_borrowed_struct"],
  },
  {
    name: "mir-foreign-static-by-pointer",
    breaks: "a JS global a foreign static names, which MIR reads through a raw pointer, isn't its global",
    file: "src/lower/mir.rs",
    find: "        if let ty::Ref(_, _, ty::Mutability::Not) | ty::RawPtr(_, ty::Mutability::Not) = ty.kind()\n",
    replace: "        if let ty::Ref(_, _, ty::Mutability::Not) = ty.kind()\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-fn-item-referenced-undefined",
    breaks: "a function given by reference, `&hello`, a promoted constant, is `undefined`",
    file: "src/lower/mir.rs",
    find: "                    && fn_def(referred.ty()).is_some() =>\n",
    replace: "                    && fn_def(referred.ty()).is_some() && false =>\n",
    tests: ["test/corpus.test.ts","-t","fn_item_referenced"],
  },
  {
    name: "mir-module-name-made-as-const",
    breaks: "a static's component made before it's used is a `const` of a temporary's name, no tag",
    file: "src/lower/mir.rs",
    find: "            js::ExprKind::Var(name) => self.module_names.contains(name) || self.locals.alike.contains(name),\n",
    replace: "            js::ExprKind::Var(name) => self.locals.alike.contains(name),\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-untagged-into-called",
    breaks: "`into()` to an untagged enum calls `Into::into`, which isn't JS's",
    file: "src/lower/mir.rs",
    find: "        if values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) {\n",
    replace: "        if false && values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-alike-variable-made-as-const",
    breaks: "a variable nothing changes, a tag, made before it's used is a `const` of a temporary's name, no tag",
    file: "src/lower/mir.rs",
    find: "            js::ExprKind::Var(name) => self.module_names.contains(name) || self.locals.alike.contains(name),\n",
    replace: "            js::ExprKind::Var(name) => self.module_names.contains(name),\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-shared-capture-changes",
    breaks: "what a closure shares a reference to may change while it runs, so a tag of it is a temporary's `const`",
    file: "src/lower/mir.rs",
    find: "                && self.locals.alike.insert(name.clone())\n",
    replace: "                && false\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-object-made-as-const",
    breaks: "a style made before a prop's branch is a `const`, which takes no more fields",
    file: "src/lower/mir.rs",
    find: "            if !named && (self.capture_jsx(&mut expr, &mut made) || self.capture_object(&mut expr, &mut made)) {\n",
    replace: "            if !named && self.capture_jsx(&mut expr, &mut made) {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-named-local-in-place",
    breaks: "a temporary a branch's subject read by its name is written where it's read, so its name is never declared",
    file: "src/lower/mir.rs",
    find: "                    state.named.insert(place.local);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","nested_type_chain"],
  },
  {
    name: "mir-otherwise-tag-of-its-own",
    breaks: "a discriminated union's `otherwise` is tested by a tag of its own, which no object has",
    file: "src/lower/mir.rs",
    find: "            let key = bindings::tag_key(self.tcx, adt.did());\n            let tests = (adt.variants().iter())\n                .filter(|other| other.def_id != def.def_id)\n",
    replace: "            let key = bindings::tag_key(self.tcx, adt.did());\n            let tests = (adt.variants().iter())\n                .filter(|other| other.def_id == def.def_id)\n",
    tests: ["test/lowering.test.ts","-t","otherwise variant is any other"],
  },
  {
    name: "mir-flattened-read-boxed",
    breaks: "a read whose `Option` only `.flatten()` reads is an `Option` of an `Option`, refused",
    file: "src/lower/mir.rs",
    find: "            && self.flattened_by(state, destination)\n",
    replace: "            && false\n",
    tests: ["test/lowering.test.ts","-t","flatten of a read of items"],
  },
  {
    name: "mir-flattened-copy-reread",
    breaks: "a copy of a flattened read, the item itself, is boxed again",
    file: "src/lower/mir.rs",
    find: "                    Some(Std::OptionCloned) => item_itself() && self.flattened_by(state, *made),\n",
    replace: "                    Some(Std::OptionCloned) => false,\n",
    tests: ["test/lowering.test.ts","-t","flatten of a read of items"],
  },
  {
    name: "mir-await-loop-kept",
    breaks: "an `.await`'s poll loop is lowered as it is, its `yield` none JS has",
    file: "src/lower/mir.rs",
    find: "            true => awaits::awaits(tcx, &mut body),\n",
    replace: "            true => Default::default(),\n",
    tests: ["test/lowering.test.ts", "-t", "an awaited reference given to a generic function is awaited|an async fn"],
  },
  {
    name: "mir-await-unawaited",
    breaks: "`.await` of a promise is the promise, not what it resolves to",
    file: "src/lower/mir.rs",
    find: "        Ok(Value::Expr(Expr::await_(self.value_expr(awaited, span)?)))\n",
    replace: "        Ok(Value::Expr(self.value_expr(awaited, span)?))\n",
    tests: ["test/compiler.test.ts","-t","async code becomes"],
  },
  {
    name: "mir-async-closure-called-body",
    breaks: "an `async` closure is an arrow that calls an async arrow, not the async arrow",
    file: "src/lower/mir.rs",
    find: "            && let Some(mut body) = called_async_body(&mut stmts)\n",
    replace: "            && let Some(mut body) = called_async_body(&mut stmts).filter(|_| false)\n",
    tests: ["test/compiler.test.ts","-t","async code becomes"],
  },
  {
    name: "mir-await-result-named",
    breaks: "what `.await` gives is a variable of the desugaring's name, `result`",
    file: "src/lower/mir.rs",
    find: "                && !info.source_info.span.is_desugaring(rustc_span::DesugaringKind::Await)\n",
    replace: "",
    tests: ["test/compiler.test.ts","-t","async code becomes"],
  },
  {
    name: "mir-async-fn-arrow-called",
    breaks: "an `async fn` is refused, not its coroutine's body",
    file: "src/lower/mir.rs",
    find: "        if is_async {\n",
    replace: "        if false {\n",
    tests: ["test/compiler.test.ts","-t","async code becomes"],
  },
  {
    name: "mir-async-closure-call-refused",
    breaks: "calling an `async` closure, `AsyncFn::async_call`, is refused",
    file: "src/lower/mir.rs",
    find: "                || tcx.async_fn_trait_kind_from_def_id(fn_trait).is_some())\n",
    replace: "                || false)\n",
    tests: ["test/lowering.test.ts", "-t", "an async closure"],
  },
  {
    name: "mir-on-load-returns",
    breaks: "an `on_load!` body from MIR returns at its module's top, which JS has no `return` at",
    file: "src/lower/mir.rs",
    find: "                        stmt.kind = StmtKind::Break(Some(label.clone()));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","on_load body's loop"],
  },
  {
    name: "mir-imported-binding-called-static",
    breaks: "a JS function a variable holds, `js::import!(f).await`'s, is called as its static import",
    file: "src/lower/mir.rs",
    find: "        if let Operand::Copy(_) | Operand::Move(_) = func\n",
    replace: "        if false && let Operand::Copy(_) | Operand::Move(_) = func\n",
    tests: ["test/bindings.test.ts","-t","js::import! loads a module"],
  },
  {
    name: "mir-flattened-struct-nested",
    breaks: "a struct with flattened fields made outside JSX holds them as fields, `{ template, files }`",
    file: "src/lower/mir.rs",
    find: "            if bindings::has_flatten(self.tcx, ty) {\n",
    replace: "            if false && bindings::has_flatten(self.tcx, ty) {\n",
    tests: ["test/lowering.test.ts","-t","a struct of flattened fields made outside JSX"],
  },
  {
    name: "mir-jsx-props-flattened",
    breaks: "a component's props with a flattened field are flattened before JSX takes them apart",
    file: "src/lower/mir.rs",
    find: "            && !state.jsx_props.contains(&place.local)\n",
    replace: "",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-local-functions-undeclared",
    breaks: "a function written in a body is declared nowhere from MIR, so what calls it reads nothing",
    file: "src/lower/mir.rs",
    find: "        self.place_written(written, &mut out);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","a function written in a body is declared"],
  },
  {
    name: "mir-early-exit-panic-last",
    breaks: "a check that panics is written after what goes on, `if (n === 6) { return n; } fail()`, not `if (n !== 6) { fail() } return n;`",
    file: "src/lower/mir.rs",
    find: "        Some(otherwise) if panics == [false, true] => {\n",
    replace: "        Some(otherwise) if panics == [false, true] && false => {\n",
    tests: ["test/lowering.test.ts","-t","what reads the same anywhere"],
  },
  {
    name: "mir-panics-through-calls-not",
    breaks: "a panic whose message a call makes isn't seen as one, its check written last",
    file: "src/lower/mir.rs",
    find: "            TerminatorKind::Goto { target }\n            | TerminatorKind::Call {\n                target: Some(target), ..\n            }\n",
    replace: "            TerminatorKind::Goto { target }\n",
    tests: ["test/lowering.test.ts","-t","what reads the same anywhere"],
  },
  {
    name: "mir-plain-field-as-const",
    breaks: "a field of plain data read twice is a `const _5 = route.title;`, not read where it's used",
    file: "src/lower/mir.rs",
    find: "                matches!(&root.kind, js::ExprKind::Var(name) if self.locals.plain.contains(name))\n",
    replace: "                false\n",
    tests: ["test/lowering.test.ts","-t","a match guard.s binding from MIR is what it binds"],
  },
  {
    name: "mir-payload-as-const",
    breaks: "`Some(title)`'s `title`, of a place that reads the same, is a `const title = …`, not the place",
    file: "src/lower/mir.rs",
    find: "            && (!state.locals.user[local] || (state.payloads.contains(&local) && !of_temporary))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local]\n            && !state.named",
    replace: "            && !state.locals.user[local]\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local]\n            && !state.named",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-payload-of-for-loop",
    breaks: "a `for` loop's item, its desugaring's `Some(x)`, is read in place, so the loop is MIR's `while (true)`",
    file: "src/lower/mir.rs",
    find: "                    if !p.span.is_desugaring(rustc_span::DesugaringKind::ForLoop) =>\n",
    replace: "                    if true =>\n",
    tests: ["test/lowering.test.ts","-t","loops from MIR read as a person writes them"],
  },
  {
    name: "mir-shared-ref-as-const",
    breaks: "a temporary's `&node.title` read twice is a `const _18 = node.title;`, not the place where it's read",
    file: "src/lower/mir.rs",
    find: "        let one = place.projection.is_empty()\n",
    replace: "        let one = false && place.projection.is_empty()\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-closure-parent-thir",
    breaks: "a closure's body is lowered with its parent's THIR, so its `if let` chain's tests stay nested",
    file: "src/lower/mir.rs",
    find: "        let outer = std::mem::replace(&mut self.thir, &body.thir);\n",
    replace: "        let outer = self.thir;\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-return-unwritten",
    breaks: "a `return` the program wrote is taken for the end the function falls off, and goes",
    file: "src/lower/mir.rs",
    find: "                stmt.span = written;\n",
    replace: "                let _ = (stmt, written);\n",
    tests: ["test/lowering.test.ts","-t","returning nothing returns where"],
  },
  {
    name: "mir-in-bounds-checked",
    breaks: "`v[0]` THIR knows is in bounds is `$index(v, 0)`",
    file: "src/lower/mir.rs",
    find: "            && (self.body_facts.in_bounds.contains(&fun) || self.in_bounds(items_id, index))\n",
    replace: "            && false\n",
    tests: ["test/lowering.test.ts","-t","string's parts from MIR"],
  },
  {
    name: "mir-closure-parent-facts",
    breaks: "a closure's body reads its parent's facts, by another body's ids",
    file: "src/lower/mir.rs",
    find: "        let outer_facts = std::mem::replace(&mut self.body_facts, &body.facts);\n",
    replace: "        let outer_facts = self.body_facts;\n",
    tests: ["test/lowering.test.ts","-t","string's parts from MIR"],
  },
  {
    name: "mir-prop-default-unread",
    breaks: "a props field with a default, left out, reads `undefined` from MIR",
    file: "src/lower/mir.rs",
    find: "            Some(default) => Expr::cond(Expr::bin(js::Op::Eq, read.clone(), Expr::undefined()), default, read),\n",
    replace: "            Some(_) => read,\n",
    tests: ["test/lowering.test.ts","-t","props field's default is read"],
  },
  {
    name: "mir-spawning-closure-arrow",
    breaks: "a closure that only spawns an async block is `() => { (async () => …)(); }`, not `async () => …`",
    file: "src/lower/mir.rs",
    find: "            && let js::ExprKind::AsyncArrow(inner, spawned) = &callee.kind\n            && inner.is_empty()\n        {\n            return Ok(Expr::async_arrow(params, spawned.clone()));\n",
    replace: "            && let js::ExprKind::AsyncArrow(inner, spawned) = &callee.kind\n            && inner.is_empty()\n            && false\n        {\n            return Ok(Expr::async_arrow(params, spawned.clone()));\n",
    tests: ["test/lowering.test.ts","-t","only spawns an async block"],
  },
  {
    name: "mir-payload-of-temporary-in-place",
    breaks: "`Some(current)` of a temporary is read as the temporary, `_5`, losing its name",
    file: "src/lower/mir.rs",
    find: "            && (!state.locals.user[local] || (state.payloads.contains(&local) && !of_temporary))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local]\n            && !state.named",
    replace: "            && (!state.locals.user[local] || state.payloads.contains(&local))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local]\n            && !state.named",
    tests: ["test/lowering.test.ts","-t","if-let's binding from MIR keeps"],
  },
  {
    name: "mir-payload-place-of-temporary",
    breaks: "`Some(first)` of a temporary holding a reference is read as the temporary, `_18`, losing its name",
    file: "src/lower/mir.rs",
    find: "            && (!state.locals.user[local] || (state.payloads.contains(&local) && !of_temporary))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local];\n",
    replace: "            && (!state.locals.user[local] || state.payloads.contains(&local))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local];\n",
    tests: ["test/lowering.test.ts","-t","let-else from MIR is a guard"],
  },
  {
    name: "mir-map-call-not-discarded",
    breaks: "a map's `insert` nothing reads is `$insert(m, k, v)`, not `m.set(k, v)`",
    file: "src/lower/mir.rs",
    find: "                self.map_values(op, (exprs, arg_tys), generic_args, state.discarding, span, out)?\n",
    replace: "                self.map_values(op, (exprs, arg_tys), generic_args, false, span, out)?\n",
    tests: ["test/lowering.test.ts","-t","loops one after another from MIR"],
  },
  {
    name: "mir-known-bounds-checked",
    breaks: "`args[i]` of `0..args.len()` THIR's facts know is in bounds is checked, `if (!(i < args.length)) throw`",
    file: "src/lower/mir.rs",
    find: "        matches!(msg, mir::AssertKind::BoundsCheck { .. })\n",
    replace: "        false && matches!(msg, mir::AssertKind::BoundsCheck { .. })\n",
    tests: ["test/lowering.test.ts","-t","early returns and index loops from MIR"],
  },
  {
    name: "mir-bool-switch-negated-first",
    breaks: "`if c { a } else { b }` is `if (!c) { b } else { a }`, MIR's test of `false` first",
    file: "src/lower/mir.rs",
    find: "                    && targets.iter().all(|(value, _)| value == 0)\n",
    replace: "                    && false\n",
    tests: ["test/lowering.test.ts","-t","value of two branches is returned"],
  },
  {
    name: "mir-switch-variant-order",
    breaks: "an enum's `match` tests its variants in their order, not its arms', `if (o == null)` first",
    file: "src/lower/mir.rs",
    find: "                    ordered.sort_by_key(|&(at, _)| at);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","value of two branches is returned"],
  },
  {
    name: "mir-early-exit-both-returning-flipped",
    breaks: "of two branches that both return, the second is first, `if (!c) { return b; } return a;`",
    file: "src/lower/mir.rs",
    find: "        Some(otherwise) if panics == [true, false] || exits(&then) => {\n",
    replace: "        Some(otherwise) if panics == [true, false] || exits(&then) && !exits(&otherwise) => {\n",
    tests: ["test/lowering.test.ts","-t","let-else from MIR"],
  },
  {
    name: "mir-booleans-unhinted",
    breaks: "a `bool` variable isn't known as one, `!!a || b`, not `a || b`",
    file: "src/lower/mir.rs",
    find: "        booleans.retain(|name| !others.contains(name));\n",
    replace: "        booleans.clear();\n",
    tests: ["test/lowering.test.ts","-t","value of two branches is returned"],
  },
  {
    name: "mir-negated-if-flipped",
    breaks: "`if !c { a } else { b }`, which MIR tests as `c`, is `if (c) { b }`, its second branch first",
    file: "src/lower/mir.rs",
    find: "                    && !matches!((written_at(state.body, *target), written_at(state.body, yes)), (Some(no), Some(yes)) if no < yes)\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","keeps its branch first"],
  },
  {
    name: "mir-captured-booleans-unknown",
    breaks: "a closure doesn't know a `bool` it captures as one, `!!force || n > 2`",
    file: "src/lower/mir.rs",
    find: "                    match ty.peel_refs().is_bool() {\n",
    replace: "                    match false {\n",
    tests: ["test/lowering.test.ts","-t","names its .&x. parameter"],
  },
  {
    name: "mir-bounds-check-inline",
    breaks: "an index nothing proves in bounds is checked by an `if … throw` of its own, not `$index(xs, i)`",
    file: "src/lower/mir.rs",
    find: "        _ if written.projection.last() == Some(&PlaceElem::Index(local)) => Some((local, true)),\n        _ => None,\n",
    replace: "        _ => None,\n",
    tests: ["test/lowering.test.ts","-t","nothing proves in bounds"],
  },
  {
    name: "mir-bounds-write-read",
    breaks: "an element written, `xs[i] = v`, is checked as one read, `$index(xs, i) = v`",
    file: "src/lower/mir.rs",
    find: "                value = match write && at + 1 == last {\n",
    replace: "                value = match false {\n",
    tests: ["test/lowering.test.ts","-t","nothing proves in bounds"],
  },
  {
    name: "mir-bounds-ref-unchecked-here",
    breaks: "a shared reference to an element, `&xs[i]`, is checked by an `if … throw`",
    file: "src/lower/mir.rs",
    find: "        | Rvalue::Ref(_, BorrowKind::Shared, read)\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","nothing proves in bounds"],
  },
  {
    name: "mir-bounds-deferred-counted",
    breaks: "a check moved to the read still counts its reads, a temporary left `const _7`",
    file: "src/lower/mir.rs",
    find: "                        || deferred_check(body, msg, *target).is_some() => {}\n",
    replace: "                        => {}\n",
    tests: ["test/lowering.test.ts","-t","nothing proves in bounds"],
  },
  {
    name: "mir-patterned-folded",
    breaks: "a temporary a `let` pattern of one field takes apart is folded, `const getProps = f().get_props`",
    file: "src/lower/mir.rs",
    find: "            && !state.patterned.contains(&local)\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","pattern of one field"],
  },
  {
    name: "mir-written-return-past-drops",
    breaks: "a `return` the program wrote, reached through drops, isn't known as written",
    file: "src/lower/mir.rs",
    find: "                && let [only] = preds[block].as_slice()\n",
    replace: "                && let [only] = preds[block].as_slice().get(..0).unwrap_or_default()\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-format-args-array",
    breaks: "a `format!`'s arguments read out of order are an array, `${_26[1]}:${_26[0]}`",
    file: "src/lower/mir.rs",
    find: "        if !in_order && !acting.is_empty() && !state.taken.contains(&place.local) {\n",
    replace: "        if false && !in_order && !acting.is_empty() && !state.taken.contains(&place.local) {\n",
    tests: ["test/lowering.test.ts","-t","arguments read out of order"],
  },
  {
    name: "mir-capture-copied-always",
    breaks: "a closure copies a variable written only before it, `const captured = index`",
    file: "src/lower/mir.rs",
    find: "                            && written_from(state.body, state.at, l)));\n",
    replace: "                            ));\n",
    tests: ["test/lowering.test.ts","-t","captures a variable itself where nothing"],
  },
  {
    name: "mir-capture-copied-never",
    breaks: "a closure made in a loop that changes what it captures reads the variable, every closure the last value",
    file: "src/lower/mir.rs",
    find: "        next.extend(data.terminator().successors());\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","captures a variable itself where nothing"],
  },
  {
    name: "mir-statics-closure-set-unseen",
    breaks: "a thread-local a closure sets is a `const`, which the closure can't set",
    file: "src/lower/mir.rs",
    find: "                js::each_block_mut(&mut function.body, &mut |stmts| {\n                    for stmt in stmts.iter() {\n                        if let StmtKind::Assign(target, _) = &stmt.kind\n",
    replace: "                js::statement_lists(&mut function.body, &mut |stmts| {\n                    for stmt in stmts.iter() {\n                        if let StmtKind::Assign(target, _) = &stmt.kind\n",
    tests: ["test/lowering.test.ts","-t","thread-local only its module"],
  },
  {
    name: "mir-promoted-closure-unsupported",
    breaks: "a promoted reference to a closure that captures nothing, `&|n| LAST.set(n)`, is unsupported",
    file: "src/lower/mir.rs",
    find: "                    && let Some(def_id) = referred_closure(&state.promoted[promoted]) =>\n",
    replace: "                    && let Some(def_id) = referred_closure(&state.promoted[promoted]).filter(|_| false) =>\n",
    tests: ["test/lowering.test.ts","-t","thread-local only its module"],
  },
  {
    "name": "mir-nullable-field-coalesced",
    "breaks": "a `#[rust_js::nullable]` field moved into another, `p.code`, is `p.code ?? null`",
    "file": "src/lower/mir.rs",
    "find": "                    bindings::is_nullable(self.tcx, &variant.fields[*field])\n",
    "replace": "                    false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a nullable field's None is null"
    ]
  },
  {
    "name": "mir-nullable-moved-through-unseen",
    "breaks": "a field's value moved through a temporary, as MIR moves it, is `?? null` again",
    "file": "src/lower/mir.rs",
    "find": "            Some(Rvalue::Use(operand, _)) => self.nulled(state, operand),\n",
    "replace": "            Some(Rvalue::Use(..)) => false,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a nullable field's None is null"
    ]
  },
  {
    "name": "mir-nullable-some-coalesced",
    "breaks": "a `Some(..)` given to a `#[rust_js::nullable]` field is `?? null` too",
    "file": "src/lower/mir.rs",
    "find": "                fields.len() == 1\n",
    "replace": "                fields.len() == 0\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a nullable field's None is null"
    ]
  },
  {
    "name": "mir-null-none-undefined",
    "breaks": "a `None` only a nullable field reads is `undefined`, and the field `(c ? v : undefined) ?? null`",
    "file": "src/lower/mir.rs",
    "find": "            && variant.as_u32() == 0\n",
    "replace": "            && variant.as_u32() == 9\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a nullable field of Some or None by a branch"
    ]
  },
  {
    "name": "mir-null-none-coalesced",
    "breaks": "a nullable field of an `Option` made `null` where it's `None` is `?? null` again",
    "file": "src/lower/mir.rs",
    "find": "        if state.null_nones.contains(&place.local) {\n",
    "replace": "        if false {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a nullable field of Some or None by a branch"
    ]
  },
  {
    "name": "mir-null-none-past-other-writes",
    "breaks": "an `Option` a call sets too, `and_then`'s `undefined`, is given to a nullable field as it is",
    "file": "src/lower/mir.rs",
    "find": "none && writes == locals.writes[local] && ",
    "replace": "none && ",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a nullable field of Some or None by a branch"
    ]
  },
  {
    "name": "async-params-whole",
    "breaks": "an `async fn`'s parameter pattern is `__arg0` and a `const params = __arg0.params;`",
    "file": "src/lower/mir.rs",
    "find": "            tidy::destructured_params(&mut params, &mut out);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an async fn's parameter pattern from MIR"
    ]
  },
  {
    "name": "mir-literal-lines-flattened",
    "breaks": "a string MIR's constant has, written across lines, is a string of `\\n`s",
    "file": "src/lower/mir.rs",
    "find": "text.contains('\\n') && super::literal_across_lines(tcx, c.span)",
    "replace": "text.contains('\\n') && false",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a string written across lines keeps its lines"
    ]
  },
  {
    "name": "mir-guard-conditions-unknown",
    "breaks": "a guarded arm's pattern test and its guard stay nested `if`s",
    "file": "src/lower/mir.rs",
    "find": "                        let guard = self.thir[arm].guard?;\n",
    "replace": "                        let guard = self.thir[arm].guard.filter(|_| false)?;\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match arm's guard from MIR is one test"
    ]
  },
  {
    "name": "while-hint-span-whole",
    "breaks": "a `while` in a closure isn't known as one: its desugared test's span, of another body, isn't the loop's",
    "file": "src/lower/mir.rs",
    "find": "                            && w.span.source_callsite().source_equal(written)\n",
    "replace": "                            && w.span.source_callsite() == written\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a while in a closure from MIR keeps its test|while"
    ]
  },
  {
    "name": "mir-format-lines-flattened",
    "breaks": "a format string MIR's template has, written across lines, is a template of `\\n`s",
    "file": "src/lower/mir.rs",
    "find": "let lines = bytes.contains(&b'\\n') && self.format_string_across_lines(span);",
    "replace": "let lines = false;",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a format string written across lines keeps its lines"
    ]
  },
  {
    "name": "let-pattern-deref-unpeeled",
    "breaks": "a `let` pattern through a reference isn't taken apart where it's made",
    "file": "src/lower/mir.rs",
    "find": "            while let thir::PatKind::Deref { subpattern, .. } = &pattern.kind {\n                pattern = subpattern;\n            }\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "let-pattern-ref-binding-unseen",
    "breaks": "a `let` pattern's binding by reference, `&(*_2).0`, isn't one it takes apart",
    "file": "src/lower/mir.rs",
    "find": "                    | Rvalue::Ref(_, BorrowKind::Shared, from),\n",
    "replace": ",\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "let-pattern-through-deref-unseen",
    "breaks": "a `let` pattern's field read through a reference, `(*_2).0`, isn't one it takes apart",
    "file": "src/lower/mir.rs",
    "find": "                && derefs.iter().all(|p| matches!(p, PlaceElem::Deref))\n",
    "replace": "                && derefs.is_empty()\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-self-named-self",
    "breaks": "a method's `self` from MIR is named `self`, not after its type",
    "file": "src/lower/mir.rs",
    "find": "ty::Adt(adt, _) if info.name.as_str() == \"self\" =>",
    "replace": "ty::Adt(adt, _) if false =>",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a method's self from MIR is named after its type"
    ]
  },
  {
    "name": "mir-own-name-called",
    "breaks": "`section.as_str()` from MIR is `Section.as_str(section)`, not `section`",
    "file": "src/lower/mir.rs",
    "find": "        if values.len() == 1 && self.gives_own_name(def_id) {\n",
    "replace": "        if values.len() == 1 && false {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a call giving each variant its own name"
    ]
  },
  {
    "name": "mir-closure-forwarding-unmarked",
    "breaks": "a closure from MIR that passes on what it's given to Rust's function stays `(x) => double(x)`, not `double`",
    "file": "src/lower/mir.rs",
    "find": "        let forwarding = self.closure_calls_rust(body, false);\n",
    "replace": "        let forwarding = false;\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "only passes its arguments on"
    ]
  },
  {
    "name": "mir-context-member-held",
    "breaks": "an imported context's provider MIR reads ahead of a branch is held, `const Component = THEME.Provider`",
    "file": "src/lower/mir.rs",
    "find": "            js::ExprKind::Member(..) if context_member => true,\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-checked-index-copied",
    "breaks": "a parameter MIR checks an element's place at is copied first, `const index = i;`",
    "file": "src/lower/mir.rs",
    "find": "                || matches!(&index.kind, js::ExprKind::Var(v) if self.locals.alike.contains(v))\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "checked element set is one statement"
    ]
  },
  {
    "name": "mir-nullable-clone-renulled",
    "breaks": "a clone of a nullable field given to one from MIR is `m.rule_id ?? null`",
    "file": "src/lower/mir.rs",
    "find": "        if itself\n",
    "replace": "        if false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "nullable field's clone"
    ]
  },
  {
    "name": "mir-binding-trait-dispatched",
    "breaks": "a binding default method called through a generic from MIR is a dictionary's, `{}.rule_id(error)`, which throws",
    "file": "src/lower/mir.rs",
    "find": "            && !bindings::is_binding(tcx, def_id)\n            && let Some(trait_id) = tcx.trait_of_assoc(def_id)\n",
    "replace": "            && let Some(trait_id) = tcx.trait_of_assoc(def_id)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "binding trait method called through a generic"
    ]
  },
  {
    "name": "mir-unread-base-dropped",
    "breaks": "a struct update giving every field from MIR drops its base, `{ url }`, losing what JS's object held besides",
    "file": "src/lower/mir.rs",
    "find": "            && let Some(spread) = self.written_base(state, ty, operands, span)\n",
    "replace": "            && let Some(spread) = None::<Place<'tcx>>\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "giving every field spreads its base"
    ]
  },
  {
    "name": "mir-written-base-unread-only",
    "breaks": "a struct update reading one field of its base from MIR is each field, `{ title, level: props.level }`, dropping what JS's object held besides",
    "file": "src/lower/mir.rs",
    "find": "            if !field || self.contains_mutated(t) && self.is_copy(t) {\n",
    "replace": "            if true {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "reading one field of its base"
    ]
  },
  {
    "name": "mir-branch-capture-unrecorded",
    "breaks": "what a branch's element reads, MIR's `let`, isn't known as MIR's, `let className$2; if (..) { .. }` kept",
    "file": "src/lower/mir.rs",
    "find": "                        StmtKind::Let(name, None) => {\n",
    "replace": "                        StmtKind::Let(name, None) if false => {\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-bool-ref-unknown",
    "breaks": "a state's `bool`, a `&bool` local, isn't known as one, `showHint ? hint : undefined`",
    "file": "src/lower/mir.rs",
    "find": "                ty::Ref(_, inner, ty::Mutability::Not) => inner.is_bool(),\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-imports-not-plain",
    "breaks": "an import's member, `HighlightStyle.define`, blocks what's read after it, `const _3 = [..]` held",
    "file": "src/lower/mir.rs",
    "find": "                .flat_map(|imports| imports.values().cloned()),\n",
    "replace": "                .flat_map(|imports| imports.values().cloned())\n                .filter(|_| false),\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-temporaries-unnamed",
    "breaks": "a temporary MIR holds is named `_4`, not `value` as THIR names it",
    "file": "src/lower/mir.rs",
    "find": "        tidy::readable_temporaries(&params, &mut out);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "held temporary is named as THIR"
    ]
  },
  {
    "name": "mir-stored-paths-unknown",
    "breaks": "a getter's `bool` stored in a local isn't known as one, `!!(element.isContentEditable || ..)`",
    "file": "src/lower/mir.rs",
    "find": "            let bool = state.body.local_decls[local].ty.is_bool();\n            *self.locals.paths.entry(path).or_insert(bool) &= bool;\n        }\n        let unread",
    "replace": "        }\n        let unread",
    "tests": ["test/lowering.test.ts", "-t", "or chain of a getter's bool"]
  },
  {
    "name": "mir-closure-concise-unmarked",
    "breaks": "a closure of one call that gives nothing isn't marked, so a held handler keeps its block",
    "file": "src/lower/mir.rs",
    "find": "        let concise = self.closure_calls_for_nothing(body);\n",
    "replace": "        let concise = false;\n",
    "tests": ["test/jsx.test.ts"]
  },
  {
    "name": "mir-call-paths-unrecorded",
    "breaks": "what a call gives isn't recorded a `bool`, `!!isItem(item) && ..`",
    "file": "src/lower/mir.rs",
    "find": ".or_else(|| tidy::call_path(e))",
    "replace": "",
    "tests": ["test/lowering.test.ts", "-t", "call giving a bool is one in a test"]
  },
  {
    "name": "mir-variant-test-not-bool",
    "breaks": "a variant's test function isn't known to give a `bool`, `!!(isValidElement(child) && ..)`",
    "file": "src/lower/mir.rs",
    "find": "                    self.locals.paths.entry(path).or_insert(true);\n",
    "replace": "",
    "tests": ["test/jsx.test.ts"]
  },
  {
    "name": "mir-written-fn-in-closure-at-top",
    "breaks": "a function written in a closure is declared at its function's top from MIR, not in the closure",
    "file": "src/lower/mir.rs",
    "find": "            .filter(|&&id| self.tcx.parent(id) == def_id)",
    "replace": "            .filter(|_| false)",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a function written in a closure is declared where it's written"
    ]
  },
  {
    "name": "mir-written-fn-before-all",
    "breaks": "a function written in a body is declared before all of it from MIR, not before what's written after it",
    "file": "src/lower/mir.rs",
    "find": ".position(|s| !s.span.is_none() && s.span.lo > at)",
    "replace": ".position(|_| true)",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a function written in a closure is declared where it's written"
    ]
  },
  {
    "name": "mir-async-closure-wild-param-kept",
    "breaks": "an `async` closure's `|_|` from MIR is `async (_) => ..`, not `async () => ..` as THIR's",
    "file": "src/lower/mir.rs",
    "find": "            && (name == \"_\" || name.starts_with(\"_$\"))\n",
    "replace": "            && name == \"never\"\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an async closure's ignored argument is no parameter"
    ]
  },
  {
    "name": "mir-array-items-not-plain",
    "breaks": "an array of JS's values isn't known to keep its items past calls, so `[...first, ...]` holds what's made after",
    "file": "src/lower/mir.rs",
    "find": "                self.locals.plain.insert(format!(\"...{name}\"));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arrays joined are one array of their items"
    ]
  },
  {
    "name": "mir-continue-alone-only",
    "breaks": "only an `if` of a `continue` alone is known written so, not one that ends with one",
    "file": "src/lower/mir.rs",
    "find": "let only = match (block.stmts.last(), block.expr) {",
    "replace": "let only = match (block.stmts.last().filter(|_| block.stmts.len() == 1), block.expr) {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "continues a loop's ifs end with are kept"
    ]
  },
  {
    "name": "mir-whole-cast-wrapped",
    "breaks": "a float THIR knows whole and in range, cast, is `$f64ToInt(index, ..)` from MIR, not `index`",
    "file": "src/lower/mir.rs",
    "find": "                } else if self.whole_cast_at(span) {",
    "replace": "                } else if false {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a whole float's cast is the float from MIR too"
    ]
  },
  {
    "name": "mir-take-variable-as-object",
    "breaks": "`mem::take(&mut current)` of a variable is `$take(current, [])` from MIR, not `const old = current; current = [];`",
    "file": "src/lower/mir.rs",
    "find": "Value::Expr(e) | Value::Place(e) if i < borrowed && variables && own(&e) => Value::Ref(e),",
    "replace": "Value::Expr(e) | Value::Place(e) if false && i < borrowed && variables && own(&e) => Value::Ref(e),",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "taking a variable assigns the variable"
    ]
  },
  {
    "name": "mir-mut-capture-copied",
    "breaks": "a closure's `&mut` capture of its maker's variable is a copy, `let current$1 = current`, so taking it only rebinds the copy",
    "file": "src/lower/mir.rs",
    "find": "                        upvars.get(i).map(|t| t.kind()),\n",
    "replace": "                        upvars.get(i).map(|t| t.kind()).filter(|_| false),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "taking a variable assigns the variable"
    ]
  },
  {
    "name": "mir-call-write-before-pending",
    "breaks": "what a call writes, a setter's `n.textContent = \"x\"`, runs before a value made ahead of it that reads it, `<p>{n.textContent}</p>` reads the new one",
    "file": "src/lower/mir.rs",
    "find": "if called.iter().any(|s| matches!(s.kind, StmtKind::Assign(..))) && !state.pending.is_empty() {",
    "replace": "if false && called.iter().any(|s| matches!(s.kind, StmtKind::Assign(..))) && !state.pending.is_empty() {",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-props-default-taken-whole",
    "breaks": "a component's props with a default taken whole from MIR are read without it, `props.target` undefined, not refused as THIR refuses",
    "file": "src/lower/mir.rs",
    "find": "            self.check_props_taken_apart(param.ty, param.pat.as_deref(), param.ty_span.unwrap_or(span))?;\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-default-base-called",
    "breaks": "an update's derived `Default` base is its `default()` called, which a generic component's `children: C` needs a dictionary for, refused",
    "file": "src/lower/mir.rs",
    "find": "            && let Some(fields) = self.default_base_at(span)?\n",
    "replace": "            && false\n            && let Some(fields) = self.default_base_at(span)?\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-default-base-fields-held",
    "breaks": "a field of an update's default base is read of an object held for it, `{ name: value.name, .. }`, not its default",
    "file": "src/lower/mir.rs",
    "find": "        if let Some(defaults) = state.defaults.get(&place.local)\n",
    "replace": "        if let Some(defaults) = state.defaults.get(&place.local).filter(|_| false)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an update's default base is the defaults it reads"
    ]
  },
  {
    "name": "mir-unread-param-named-temporary",
    "breaks": "a parameter the program names nothing and reads nowhere is `_2`, not `_`",
    "file": "src/lower/mir.rs",
    "find": "if !state.locals.user[local] && state.locals.reads[local] == 0 && !state.locals.borrowed[local] {",
    "replace": "if false && !state.locals.user[local] && state.locals.reads[local] == 0 && !state.locals.borrowed[local] {",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-plain-fields-unknown",
    "breaks": "a plain field of a struct holding a `Cell`, `q.title`, isn't known to read the same",
    "file": "src/lower/mir.rs",
    "find": "            self.locals.plain.extend(fields);\n",
    "replace": "            drop(fields);\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-plain-through-mut",
    "breaks": "a field through a `&mut`, `e.n`, is taken as unchanged, read after what sets it",
    "file": "src/lower/mir.rs",
    "find": "            let through_mut = matches!(\n",
    "replace": "            let through_mut = false && matches!(\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-props-with-drops-whole",
    "breaks": "props with a destructor are taken whole, `{ label, rest }`, their `Rest` a field of its own, not `...rest`",
    "file": "src/lower/mir.rs",
    "find": "            if self.has_drops(pat.ty) && !self.binds_dropped_fields(pat) {",
    "replace": "            if self.has_drops(pat.ty) {",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-props-drop-reads-counted",
    "breaks": "the drops of a parameter's moved-out parts count as reads of it, so its pattern isn't taken, `{ label, rest }`",
    "file": "src/lower/mir.rs",
    "find": "            let drops = 2 * dropped.iter().filter(|&&local| local == arg).count();",
    "replace": "            let drops = 0 * dropped.iter().filter(|&&local| local == arg).count();",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-rest-field-read",
    "breaks": "a `Rest` of props read as a field of its own, `p.rest`, is `undefined`, not refused",
    "file": "src/lower/mir.rs",
    "find": "            PlaceElem::Field(_, field_ty) if bindings::is_rest(self.tcx, field_ty) => {",
    "replace": "            PlaceElem::Field(_, field_ty) if false && bindings::is_rest(self.tcx, field_ty) => {",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-let-rest-whole",
    "breaks": "a `let`'s pattern's rest holds every field, `...rest` with what the pattern leaves, `..`",
    "file": "src/lower/mir.rs",
    "find": "            .filter(|&(i, _)| !bindings::is_rest_field(tcx, ty, i))\n",
    "replace": "            .filter(|&(i, _)| false && !bindings::is_rest_field(tcx, ty, i))\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-match-flattened-kept",
    "breaks": "a `match`'s binding of flattened props is the props whole, their other fields among them",
    "file": "src/lower/mir.rs",
    "find": "        if state.binding_var\n            && let Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) = rvalue\n",
    "replace": "        if false\n            && let Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) = rvalue\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-division-checked-twice",
    "breaks": "a `%` or `/` from MIR is checked before the division checks it, `if ((2 === -1) & \u2026) throw`",
    "file": "src/lower/mir.rs",
    "find": "        AssertKind::DivisionByZero(_)\n            | AssertKind::RemainderByZero(_)\n            | AssertKind::Overflow(BinOp::Div | BinOp::Rem, ..)\n",
    "replace": "        AssertKind::Overflow(BinOp::Shl, ..)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "division by a constant from MIR"
    ]
  },
  {
    "name": "mir-frozen-borrow-a-variable",
    "breaks": "a match guard's shared borrow of `k.level` makes `k` a variable, its payloads held, `const t = k._0; return t;`",
    "file": "src/lower/mir.rs",
    "find": "&& !place.is_indirect() && !frozen {",
    "replace": "&& !place.is_indirect() {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match guard's binding from MIR"
    ]
  },
  {
    "name": "mir-chars-as-str-refused",
    "breaks": "`c.as_str()` of a string's `chars()` from MIR is refused",
    "file": "src/lower/mir.rs",
    "find": "                if op == StepOp::AsStr\n",
    "replace": "                if false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "the rest of a string's chars from MIR"
    ]
  },
  {
    "name": "mir-tried-named-value",
    "breaks": "`let f = xs.find(..)?;` from MIR holds the value as `value`, not `f`",
    "file": "src/lower/mir.rs",
    "find": "                && let Some((bound, from)) = continued_binding(state, local)\n",
    "replace": "                && let Some((bound, from)) = continued_binding(state, local).filter(|_| false)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let of an Option's . from MIR"
    ]
  },
  {
    "name": "mir-return-written-by-storage",
    "breaks": "the value a `?` goes on to is returned at the `?`'s span, a `return` the program wrote, so its `?` stays nested, not a guard",
    "file": "src/lower/mir.rs",
    "find": "                        .filter(|s| !matches!(s.kind, StatementKind::StorageLive(_) | StatementKind::StorageDead(_)))\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a function's .s from MIR are guards"
    ]
  },
  {
    "name": "mir-arm-written-at-test",
    "breaks": "an arm that tests a field first, `Done { ok: true }`, is placed where its test is spanned, the `match`, first",
    "file": "src/lower/mir.rs",
    "find": "    written_within(body, block, 4)\n",
    "replace": "    written_within(body, block, 0)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match's arms from MIR are in written order"
    ]
  },
  {
    "name": "mir-range-bound-first",
    "breaks": "a range pattern's test from MIR reads its bound first, `2 <= k`",
    "file": "src/lower/mir.rs",
    "find": "&& state.ranges.iter().any(|range| range.contains(span))",
    "replace": "&& false",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range pattern's tests from MIR"
    ]
  },
  {
    "name": "mir-range-unsigned-floor-tested",
    "breaks": "`0..=4` of a `u8` from MIR tests `k >= 0`, which always holds",
    "file": "src/lower/mir.rs",
    "find": ".is_some_and(|n| n.is_null())",
    "replace": ".is_some_and(|n| n.is_null() && false)",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range pattern's tests from MIR"
    ]
  },
  {
    "name": "mir-fn-value-param-value",
    "breaks": "`.map(Vec::len)` from MIR names its arrow's parameter `value`, not by its type, `x`",
    "file": "src/lower/mir.rs",
    "find": "self.fresh(self.parameter_name(inputs[0].peel_refs()))",
    "replace": "self.fresh(\"value\")",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "mir-arm-binding-held",
    "breaks": "an arm binding the value whole, `x if x.len() > 1`, from MIR holds `const x = s;`",
    "file": "src/lower/mir.rs",
    "find": "            thir::PatKind::Binding { .. } => Some(arm.pattern.span),\n",
    "replace": "            thir::PatKind::Binding { .. } => None::<rustc_span::Span>,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a string's or-pattern from MIR"
    ]
  },
  {
    "name": "mir-enum-cast-refused",
    "breaks": "`ordering as i32` from MIR is refused, its discriminant read as a value",
    "file": "src/lower/mir.rs",
    "find": "                    Value::Discriminant(value, enum_ty) => {\n                        return Ok(Value::Expr(self.cast(value, enum_ty, *to, span)?));\n                    }\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an enum cast to a number from MIR"
    ]
  },
  {
    "name": "mir-tree-range-refused",
    "breaks": "a B-tree's `range` from MIR is refused",
    "file": "src/lower/mir.rs",
    "find": "            Std::Map(MapOp::TreeRange { mutable }) => {",
    "replace": "            Std::Map(MapOp::TreeRange { mutable }) if mutable => {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a B-tree's range from MIR"
    ]
  },
  {
    "name": "mir-item-handles-a-js-iterator",
    "breaks": "`iter_mut()`'s handles from MIR are `Iterator.from($mutItems(v))`, its count a `reduce`, not `$mutItems(v).filter(f).length`",
    "file": "src/lower/mir.rs",
    "find": "                    true => self.std_iterator(handles, output),\n",
    "replace": "                    true => self.js_iterator(handles),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iter_mut chains from MIR"
    ]
  },
  {
    "name": "mir-let-else-continue-unwritten",
    "breaks": "a `let \u2026 else { continue; }` from MIR isn't one the program wrote: `if (current) { .. }` around the rest, not the guard",
    "file": "src/lower/mir.rs",
    "find": "            .chain(self.let_else_continues())\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let-else's written continue"
    ]
  },
  {
    "name": "mir-alike-once-each-way-unseen",
    "breaks": "a variable each of a `match`'s arms assigns, `let variant = match ..`, isn't alike, so JSX holds `const Icon = variant.Icon` first, not `variant.Icon && <variant.Icon />`",
    "file": "src/lower/mir.rs",
    "find": "                || state.locals.writes[local] > 1 && !(state.locals.user[local] && once_each_way(mir_body, local))\n",
    "replace": "                || state.locals.writes[local] > 1\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-alike-machine-once",
    "breaks": "a local MIR assigns twice on separate ways, a `vec!`'s box, reads alike, and what's around it is held in `const`s first",
    "file": "src/lower/mir.rs",
    "find": "                || state.locals.writes[local] > 1 && !(state.locals.user[local] && once_each_way(mir_body, local))\n",
    "replace": "                || state.locals.writes[local] > 1 && !once_each_way(mir_body, local)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a vec.s specs from MIR stay in place"
    ]
  },
  {
    "name": "mir-switch-cases-ungrouped",
    "breaks": "an or-pattern's values that go to one block are a test each from MIR, the block copied into each, one copy's `const right` an assignment of nothing declared",
    "file": "src/lower/mir.rs",
    "find": "                    match grouped.iter_mut().find(|(_, t)| *t == target) {\n",
    "replace": "                    match None::<&mut (Expr, BasicBlock)> {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an or-pattern's values from MIR"
    ]
  },
  {
    "name": "mir-std-param-output-an-iterator",
    "breaks": "a `T` a std call gives, `unwrap_or(fallback)`'s, is wrapped `Iterator.from(..)` from MIR, which throws of a number",
    "file": "src/lower/mir.rs",
    "find": "                        && !matches!(output.kind(), ty::Param(_))\n                        && !output.is_box()",
    "replace": "                        && !output.is_box()",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "generic_from_str"
    ]
  },
  {
    "name": "mir-cmp-max-refused",
    "breaks": "`cmp::max(a, b)` of what isn't a number is refused from MIR, not `Ord::max(a, b)`'s",
    "file": "src/lower/mir.rs",
    "find": "        if known.is_none()\n            && let Some(name) = self.recognition().cmp_extreme(def_id)\n",
    "replace": "        if false\n            && let Some(name) = self.recognition().cmp_extreme(def_id)\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "std_methods"
    ]
  },
  {
    "name": "mir-dyn-error-from-refused",
    "breaks": "`\"too big\".into()` into a `Box<dyn Error>` is refused from MIR, not `{ value, impl: $stringError() }`",
    "file": "src/lower/mir.rs",
    "find": "            && let Some(dictionary) = self.dyn_error_from(to, from, span)?\n            && let [value] = &values[..]\n",
    "replace": "            && let Some(dictionary) = self.dyn_error_from(to, from, span)?\n            && let [value] = &values[..0]\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "dyn_display"
    ]
  },
  {
    "name": "mir-call-refusal-own-words",
    "breaks": "MIR's refusal of a call it can't lower says so in its own words, not the one a `compile-fail` case expects of rust-js",
    "file": "src/lower/mir.rs",
    "find": "&format!(\"calling `{}`\", self.tcx.def_path_str(def_id))",
    "replace": "&format!(\"calling `{}`, from its MIR\", self.tcx.def_path_str(def_id))",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "str_circumfix_closure"
    ]
  },
  {
    "name": "mir-copy-within-refused",
    "breaks": "`v.copy_within(a..b, at)` from MIR is refused",
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
    "find": "                match self.is_boxable(generic_args.type_at(0)) {\n                    true => self.helper(Helper::MutItems, \"$mutItems\", vec![view]),\n",
    "replace": "                match false {\n                    true => self.helper(Helper::MutItems, \"$mutItems\", vec![view]),\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "deque_range_mut"
    ]
  },
  {
    "name": "mir-tree-range-mut-refused",
    "breaks": "a B-tree's `range_mut` from MIR is refused",
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
    "find": "            Std::Text(op @ (TextOp::Splice | TextOp::ExtendFromWithin)) => {\n",
    "replace": "            Std::Text(op @ (TextOp::Splice | TextOp::ExtendFromWithin)) if false => {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a vec's range pushed again or replaced"
    ]
  },
  {
    "name": "mir-combinator-user-iterator-fields",
    "breaks": "the crate's iterator given to `chain` from MIR is its fields, `[0]`, not its items",
    "file": "src/lower/mir.rs",
    "find": "                    .map(|(e, &ty)| match self.is_user_iterator(ty) {\n",
    "replace": "                    .map(|(e, &ty)| match false && self.is_user_iterator(ty) {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "the crate's iterator from MIR"
    ]
  },
  {
    "name": "mir-option-place-refused",
    "breaks": "`o.get_or_insert(x)` from MIR is refused",
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
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
    "file": "src/lower/mir.rs",
    "find": "            Std::UserWrite => {\n",
    "replace": "            Std::UserWrite if false => {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "user_fmt_write"
    ]
  },
  {
    "name": "mir-param-dictionary-of-ref-refused",
    "breaks": "`w.write_char(c)` of a generic writer from MIR, its dictionary's, is refused",
    "file": "src/lower/mir.rs",
    "find": "                        || generic_args.types().next().is_some_and(|t| matches!(t.peel_refs().kind(), ty::Param(_))))))\n",
    "replace": "                        || false)))\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "generic_writers"
    ]
  },
  {
    name: "mir-split-off-unwritten",
    breaks: "MIR's `s.split_off(..n)` leaves `s` as it was",
    file: "src/lower/mir.rs",
    find: "                out.push(StmtKind::Assign(place, left).at(self.js_span(span)));\n                Expr::index(split, Expr::int(1))\n",
    replace: "                Expr::index(split, Expr::int(1))\n",
    tests: ["test/corpus.test.ts", "-t", "slice_split_off"],
  },
  {
    name: "mir-split-off-held",
    breaks: "MIR's `r.split_off_mut(..n)`, `r` a `&mut` to `rest`, gives `r` what's left",
    file: "src/lower/mir.rs",
    find: "                        if matches!(place.kind, js::ExprKind::Var(_)) && self.borrows_variable(def_id, span, 0) =>\n",
    replace: "                        if matches!(place.kind, js::ExprKind::Var(_)) =>\n",
    tests: ["test/lowering.test.ts", "-t", "split_off through a held"],
  },
  {
    name: "mir-uninit-drop-skipped",
    breaks: "MIR's `assume_init_drop()` of a slice drops nothing",
    file: "src/lower/mir.rs",
    find: "                self.flush(state, out)?;\n                self.drop_value(slots, ty, span, out)?;\n",
    replace: "                self.flush(state, out)?;\n",
    tests: ["test/corpus.test.ts", "-t", "slice_uninit"],
  },
  {
    name: "mir-uninit-write-clone-copies",
    breaks: "MIR's `write_clone_of_slice` copies, not clones",
    file: "src/lower/mir.rs",
    find: "                self.uninit_write_values(clone, values, generic_args.type_at(0), span, out)?\n",
    replace: "                self.uninit_write_values(false, values, generic_args.type_at(0), span, out)?\n",
    tests: ["test/corpus.test.ts", "-t", "slice_uninit"],
  },
  {
    name: "mir-str-deref-mut-refused",
    breaks: "MIR's `&mut str` of a `String`, `s.make_ascii_uppercase()`'s, is refused",
    file: "src/lower/mir.rs",
    find: "        if known == Std::Same\n            && self.is_cell(output)\n",
    replace: "        if false\n            && self.is_cell(output)\n",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-str-part-range-unread",
    breaks: "MIR's `&mut s[a..b]` gives `$strPart` the range itself, not its bounds",
    file: "src/lower/mir.rs",
    find: "                if matches!(op, TextOp::StrPart { bounds: false } | TextOp::StrGetMut) {\n",
    replace: "                if matches!(op, TextOp::StrGetMut) {\n",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-str-part-case-handle",
    breaks: "MIR's `s[a..b].make_ascii_uppercase()` changes a handle on the part, not `s`",
    file: "src/lower/mir.rs",
    find: "                            && let Some((whole, mut bounds)) = str_part_of(place) =>\n",
    replace: "                            && let Some((whole, mut bounds)) = str_part_of(place).filter(|_| false) =>\n",
    tests: ["test/lowering.test.ts", "-t", "part changed in place"],
  },
  {
    name: "mir-str-edit-part-refused",
    breaks: "MIR's string edit of a part a call gives, `tail(&mut s).make_ascii_uppercase()`, is refused",
    file: "src/lower/mir.rs",
    find: "                        let cell = self.spill(\"part\", (**cell).clone(), out);\n                        Expr::member(cell, \"value\")\n",
    replace: "                        return Err(self.unsupported(span, \"changing this string, from its MIR\"));\n",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-call-cell-refused",
    breaks: "MIR's `&mut` a call gives the cell of, `&mut s[7..]` returned, is refused",
    file: "src/lower/mir.rs",
    find: "                        js::ExprKind::Call(..) | js::ExprKind::Object(_) => true,\n",
    replace: "                        js::ExprKind::Object(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-str-box-not-copied-back",
    breaks: "MIR's `shout(&mut s)` of a `&mut str` boxes `s` and doesn't copy it back",
    file: "src/lower/mir.rs",
    find: "!pointee.is_sized(self.tcx, self.typing_env) && !pointee.is_str())",
    replace: "!pointee.is_sized(self.tcx, self.typing_env))",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-reborrow-cell-folded",
    breaks: "MIR's `&mut *s` of `leak()`'s cell is its text, `\"cd\"`, not the cell",
    file: "src/lower/mir.rs",
    find: "                                    let kind = js::ExprKind::Member(Box::new(cell), \"value\".into());\n                                    Expr {\n                                        kind,\n                                        ..Expr::undefined()\n                                    }\n",
    replace: "                                    Expr::member(cell, \"value\")\n",
    tests: ["test/corpus.test.ts", "-t", "leaked_string_written"],
  },
  {
    name: "mir-object-cell-refused",
    breaks: "MIR's cell made where it's kept, `vec![s.leak()]`'s, is refused",
    file: "src/lower/mir.rs",
    find: "                        js::ExprKind::Call(..) | js::ExprKind::Object(_) => true,\n",
    replace: "                        js::ExprKind::Call(..) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "leaked_string_written"],
  },
  {
    name: "mir-box-items-of-handle",
    breaks: "MIR's `for n in &mut boxed` steps a handle on the box, not its array",
    file: "src/lower/mir.rs",
    find: "                    Value::Ref(place) if known == Std::Pointee => Ok(place),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "boxed_array_iter"],
  },
  {
    name: "mir-kept-ref-unfixed",
    breaks: "MIR's `let first = &mut h.list[0]` reads `h.list` where it's used, after it's assigned again",
    file: "src/lower/mir.rs",
    find: "                Value::Ref(self.fixed_kept(state, target, out)?)\n",
    replace: "                Value::Ref(target)\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_local"],
  },
  {
    name: "mir-ref-of-ref-unaliased",
    breaks: "MIR's `*outer = &mut ys`, `outer` a `&mut` to `list`, assigns `outer`",
    file: "src/lower/mir.rs",
    find: "                state.refs.insert(place.local, list);\n                return Ok(());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_local"],
  },
  {
    name: "mir-through-alias-handle",
    breaks: "MIR's `&mut outer[0]` through `outer`, a `&mut` to `list`, indexes a handle on `list`",
    file: "src/lower/mir.rs",
    find: "                            if !place.projection.is_empty()\n",
    replace: "                            if false\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_local"],
  },
  {
    name: "mir-kept-ref-var-unheld",
    breaks: "MIR's `let first = &mut list[0]`, `list` a `&mut` given another later, reads `list` where it's used",
    file: "src/lower/mir.rs",
    find: "                .is_none_or(|(l, _)| state.locals.writes[l] <= 1 || !state.body.local_decls[l].ty.is_ref()),\n",
    replace: "                .is_none_or(|_| true),\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_local"],
  },
  {
    name: "mir-option-as-mut-refused",
    breaks: "MIR's `o.as_mut()` of an `Option<Box<T>>` or of a number is refused",
    file: "src/lower/mir.rs",
    find: "            if known == Std::Pointee\n                && let Some(item) = self.option_of(receiver)\n",
    replace: "            if false\n                && let Some(item) = self.option_of(receiver)\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-field-value-collapsed",
    breaks: "MIR's `&mut list.value`, a struct's field named `value`, given as `list` itself",
    file: "src/lower/mir.rs",
    find: "                        js::ExprKind::Var(name) => self.locals.cells.contains(name),\n",
    replace: "                        js::ExprKind::Var(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-borrow-root-unheld",
    breaks: "MIR's `&mut list.value` of a `list` moved on later reads `list` where it's used",
    file: "src/lower/mir.rs",
    find: "                        && state.body.local_decls[place.local].ty.is_ref();\n                    let target = match root {\n",
    replace: "                        && false;\n                    let target = match root {\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-generic-result-boxed",
    breaks: "MIR's `pick(..).n += 5`, a generic `&mut T` of an object, reads the box's `n`",
    file: "src/lower/mir.rs",
    find: "            let called = self.generic_result_values((def_id, generic_args), called, span)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-generic-method-result-boxed",
    breaks: "MIR's `checked.push(1)` through the crate's generic `deref_mut` pushes to its box",
    file: "src/lower/mir.rs",
    find: "                let call = self.generic_result_values(self.callee(def_id, generic_args), call, span)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-param-type-unchecked",
    breaks: "MIR's parameter of a `&mut &mut [i32]` compiles, given a copy its `*s = ..` never reaches the caller",
    file: "src/lower/mir.rs",
    find: "            self.check_value_ty(mir_body.local_decls[local].ty, def_span)?;\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "slice reference is refused from MIR"],
  },
  {
    name: "mir-cell-compare-handles",
    breaks: "MIR's `p > q` of `&mut` numbers compares the handles",
    file: "src/lower/mir.rs",
    find: "            && generic_args.types().any(|t| self.is_cell(t))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "mir-through-cell-ref-unwrapped",
    breaks: "MIR's `&*r` of a `&&mut i32` gives the number, not its cell, `refs.contains(&&mut 2)`",
    file: "src/lower/mir.rs",
    find: "                                && !self.is_cell(pointee) =>\n",
    replace: "                                =>\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "mir-borrowed-const",
    breaks: "MIR's `w` a `ref mut n` of its payload writes is a `const`",
    file: "src/lower/mir.rs",
    find: "                match state.locals.borrowed[local] {\n",
    replace: "                match false {\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "mir-assign-op-handle-refused",
    breaks: "MIR's `*m.entry(k).or_insert(10) += &v`, through a handle a call gave, is refused",
    file: "src/lower/mir.rs",
    find: "            && (matches!(place, Value::Ref(_)) || self.is_cell(arg_tys[0]))\n",
    replace: "            && matches!(place, Value::Ref(_))\n",
    tests: ["test/corpus.test.ts", "-t", "prepared_places"],
  },
  {
    name: "mir-variable-type-unchecked",
    breaks: "MIR's variable of a struct of a `&mut &mut [i32]` compiles, `*h.s = ..` changing a copy",
    file: "src/lower/mir.rs",
    find: "        self.check_held_drops(ty, span)?;\n        self.check_value_ty(ty, span)\n",
    replace: "        self.check_held_drops(ty, span)\n",
    tests: ["test/lowering.test.ts", "-t", "type JS can't hold is refused from MIR"],
  },
  {
    name: "mir-variable-drops-unchecked",
    breaks: "MIR's set of keys with a destructor says only that it's of a type JS can't hold",
    file: "src/lower/mir.rs",
    find: "        self.check_held_drops(ty, span)?;\n        self.check_value_ty(ty, span)\n",
    replace: "        self.check_value_ty(ty, span)\n",
    tests: ["test/corpus.test.ts", "-t", "map_key_drops"],
  },
  {
    name: "mir-flush-discriminant",
    breaks: "a discriminant pending when MIR flushes, a loop's `next()` under a drop flag's unwind, is read as a value and refused",
    file: "src/lower/mir.rs",
    find: "            if let Value::Discriminant(subject, _) = &value\n                && subject.reads_same()\n",
    replace: "            if let Value::Discriminant(subject, _) = &value\n                && false\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_borrows"],
  },
  {
    name: "mir-mut-temp-place-held",
    breaks: "MIR's `&mut` temporary of a guard's `value` is a `const` of it, not the place",
    file: "src/lower/mir.rs",
    find: "            state.in_place.insert(local, target.clone());\n            return Ok(());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
  {
    name: "mir-guard-rc-assign-refused",
    breaks: "MIR's write of a `Weak` to a guard's `value` is refused",
    file: "src/lower/mir.rs",
    find: "                if matches!(&target.kind, js::ExprKind::Member(_, field) if field == \"value\") {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
  {
    name: "mir-make-mut-field-refused",
    breaks: "MIR's `Rc::make_mut(&mut copy.pages)`, of a field, is refused",
    file: "src/lower/mir.rs",
    find: "                    Some(name) if fixed_place(&place) => state",
    replace: "                    Some(name) if matches!(place.kind, js::ExprKind::Var(_)) => state",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "mir-mut-temp-expr-spilled",
    breaks: "MIR's `&mut copy.pages` temporary is a `const` of the `Rc`, which `make_mut` can't give another",
    file: "src/lower/mir.rs",
    find: "            && let Value::Place(target) | Value::Expr(target) = &value\n",
    replace: "            && let Value::Place(target) = &value\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "mir-counted-rc-plain",
    breaks: "MIR's `Arc::strong_count(&owner)` is read where it's shown, after a later `upgrade()` counted another",
    file: "src/lower/mir.rs",
    find: "                            && self.counted_rc(t).is_none()\n                            && self.weak_of(t).is_none()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "mir-iter-made-dropped",
    breaks: "MIR's `last()` of what looks like `None` reads items it never holds",
    file: "src/lower/mir.rs",
    find: "                    out.extend(made);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
  },
  {
    name: "mir-map-unrecorded",
    breaks: "MIR's map temporary isn't recorded, so `next_back` of it is refused",
    file: "src/lower/mir.rs",
    find: "            state.maps.insert(local, expr.clone());\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "next_back of a map"],
  },
  {
    name: "mir-closure-tuple-refused",
    breaks: "MIR's `f(x)` of a generic `F: Fn`, its arguments a tuple made before, is refused",
    file: "src/lower/mir.rs",
    find: "                    if tuple.reads_same()\n",
    replace: "                    if false && tuple.reads_same()\n",
    tests: ["test/corpus.test.ts", "-t", "generic_trait_methods"],
  },
  {
    name: "mir-failing-write-whole",
    breaks: "MIR's `write!` of what may fail writes it whole, what came before the error lost",
    file: "src/lower/mir.rs",
    find: "                if matches!(given, Value::Fmt(Std::FmtNew, ..)) && fixed_place(&place) {\n",
    replace: "                if false && fixed_place(&place) {\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "mir-failing-result-methods-refused",
    breaks: "MIR's `r.is_err()` of a `write!` that may fail is refused",
    file: "src/lower/mir.rs",
    find: "                return Ok(Value::Expr(self.fmt_result_values(answer, result, message, span, out)));\n",
    replace: "                return Err(self.unsupported(span, \"methods of a `fmt::Result` that may fail, from its MIR\"));\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "mir-failing-question-refused",
    breaks: "MIR's `write!(s, ..)?` that may fail is refused",
    file: "src/lower/mir.rs",
    find: "                self.fmt_check(tried, self.js_span(span), out);\n",
    replace: "                return Err(self.unsupported(span, \"`?` of a `fmt::Result` that may fail, from its MIR\"));\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "mir-fmt-result-arguments",
    breaks: "MIR's `{:?}` of a `fmt::Result` takes `Argument::new_debug` for a method of the result's",
    file: "src/lower/mir.rs",
    find: "            && tcx\n                .impl_of_assoc(def_id)\n",
    replace: "            && true || tcx\n                .impl_of_assoc(def_id)\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "mir-same-trait-dispatched",
    breaks: "MIR's `b.as_ref()` of a `Box<dyn Any>` asks for `AsRef`'s evidence, not the same JS value",
    file: "src/lower/mir.rs",
    find: "        if !matches!(known, Some(Std::Any(_) | Std::Same))\n",
    replace: "        if !matches!(known, Some(Std::Any(_)))\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_any"],
  },
  {
    name: "mir-async-captured-refused",
    breaks: "MIR's `async fn` that captures its array parameter first is refused",
    file: "src/lower/mir.rs",
    find: "        .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_only_vars() || copy(value)))\n",
    replace: "        .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_only_vars()))\n",
    tests: ["test/corpus.test.ts", "-t", "future_sizes"],
  },
  {
    name: "mir-skip-while-skipped-undropped",
    breaks: "MIR's `skip_while` of items with a destructor leaves those it skips undropped",
    file: "src/lower/mir.rs",
    find: "                    exprs.insert(0, self.dropping_discarded(test, item, true, span, out)?);\n",
    replace: "                    exprs.insert(0, test);\n",
    tests: ["test/corpus.test.ts", "-t", "drop_skipped_items"],
  },
  {
    name: "mir-skip-while-takes-refused",
    breaks: "MIR's `skip_while` of an owning iterator of what has a destructor is refused",
    file: "src/lower/mir.rs",
    find: "        let drained = known == Std::IterComb(IterComb::SkipWhile);\n",
    replace: "        let drained = false;\n",
    tests: ["test/corpus.test.ts", "-t", "drop_skipped_items"],
  },
  {
    name: "mir-closure-unmade",
    breaks: "MIR's closure holding a value with a destructor can't be dropped: where it's made isn't known",
    file: "src/lower/mir.rs",
    find: "        self.closure_made(def_id, held);\n",
    replace: "        self.closure_made(def_id, Vec::new());\n",
    tests: ["test/corpus.test.ts", "-t", "closure_drops"],
  },
  {
    name: "mir-call-takes-unchecked",
    breaks: "MIR's `m.get(\"a\")` through the crate's `Borrow`, and `v.extend(bag)` of its `IntoIterator`, compile, wrong",
    file: "src/lower/mir.rs",
    find: "        self.check_call_takes(def_id, generic_args, span)?;\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses a map's key looked up"],
  },
  {
    name: "mir-untagged-alike-matched",
    breaks: "MIR's match of an untagged enum's variants of one kind takes the first for both",
    file: "src/lower/mir.rs",
    find: "                if bindings::is_untagged(self.tcx, adt.did())\n                    && let Some(other) = self.untagged_alike(*enum_ty, adt.variant(variant))\n",
    replace: "                if false\n                    && let Some(other) = self.untagged_alike(*enum_ty, adt.variant(variant))\n",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses an untagged enum's variants"],
  },
  {
    name: "mir-dyn-drops-lost",
    breaks: "MIR's `Box<dyn Send>` of a value with a destructor never drops it",
    file: "src/lower/mir.rs",
    find: "                                super::drops::Drops::Unsupported(..) => true,\n                            }\n",
    replace: "                                super::drops::Drops::Unsupported(..) => false,\n                            } && false\n",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses a Box<dyn Send>"],
  },
  {
    name: "mir-whole-assign-through-mut",
    breaks: "MIR's `*f = g` of a closure through a `&mut` only gives the parameter another",
    file: "src/lower/mir.rs",
    find: "            && matches!(target.kind, js::ExprKind::Var(ref name) if *name == state.locals.names[base.local])\n",
    replace: "            && false\n",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses a new closure assigned"],
  },
  {
    name: "mir-closure-drops-unchecked",
    breaks: "MIR's closure holding a destructor, given away uncalled, is never dropped",
    file: "src/lower/mir.rs",
    find: "        self.check_closure_drops()?;\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses a Fn closure holding"],
  },
  {
    name: "mir-pretty-dyn-debug-plain",
    breaks: "MIR's `{:#?}` of a `&dyn Debug` made elsewhere shows it plain",
    file: "src/lower/mir.rs",
    find: "                            if spec.alternate && kind == Std::FmtDebug && self.is_dyn_debug(ty) {\n",
    replace: "                            if false {\n",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses .....? of a kept"],
  },
  {
    name: "mir-json-text-pretty-lost",
    breaks: "MIR's `serde_json::to_string_pretty` writes it compact",
    file: "src/lower/mir.rs",
    find: "                self.json_text(value, ty, pretty, span)?\n",
    replace: "                self.json_text(value, ty, false, span)?\n",
    tests: ["test/serde.test.ts", "-t", "ordinary renamed structs"],
  },
  {
    name: "mir-json-calls-refused",
    breaks: "MIR's `serde_json::Value` operations and `to_value` are refused",
    file: "src/lower/mir.rs",
    find: "            return Ok(Value::Expr(self.json_call_values(operation, exprs, span)?));\n",
    replace: "            return Err(self.unsupported(span, &format!(\"serde_json of {}, from its MIR\", exprs.len())));\n",
    tests: ["test/serde.test.ts", "-t", "f32 is written"],
  },
  {
    name: "mir-fn-directive-lost",
    breaks: "MIR's function loses its `js::directive!`",
    file: "src/lower/mir.rs",
    find: "        out.splice(0..0, self.directives(body.def_id));\n",
    replace: "",
    tests: ["test/modules.test.ts","-t","js::directive! in a body"],
  },
  {
    name: "mir-async-closure-directive-lost",
    breaks: "MIR's async closure loses its `js::directive!`",
    file: "src/lower/mir.rs",
    find: "            body.splice(0..0, directives);\n",
    replace: "",
    tests: ["test/modules.test.ts","-t","js::directive! in a body"],
  },
  {
    name: "mir-closure-directive-lost",
    breaks: "MIR's closure loses its `js::directive!`",
    file: "src/lower/mir.rs",
    find: "        stmts.splice(0..0, directives);\n",
    replace: "",
    tests: ["test/modules.test.ts","-t","js::directive! in a body"],
  },
  {
    name: "mir-spawning-closure-directive-lost",
    breaks: "MIR's closure that only spawns loses its `js::directive!`",
    file: "src/lower/mir.rs",
    find: "            && directives.is_empty()\n",
    replace: "",
    tests: ["test/modules.test.ts","-t","js::directive! in a body"],
  },
  {
    name: "mir-cannot-throw-unwinds",
    breaks: "MIR's call of what can't throw cleans up, its library function taking a drop",
    file: "src/lower/mir.rs",
    find: "                *unwind = UnwindAction::Unreachable;\n",
    replace: "",
    tests: ["test/bindings.test.ts","-t","a binding that can't throw"],
  },
  {
    name: "mir-quiet-local-borrowed",
    breaks: "MIR's quiet thread-local's borrows stay `$withBorrow`, its cell kept",
    file: "src/lower/mir.rs",
    find: "            && matches!(&key.kind, js::ExprKind::Var(v) if quiet.contains(v.as_str()))\n",
    replace: "            && matches!(&key.kind, js::ExprKind::Var(v) if quiet.contains(v.as_str()) && false)\n",
    tests: ["test/modules.test.ts","-t","thread-local only read and set"],
  },
  {
    name: "mir-quiet-number-local-given-value",
    breaks: "MIR's thread-local number changed by `with_borrow_mut` is given its value, a change lost",
    file: "src/lower/mir.rs",
    find: "            && let [key, _] = args.as_slice()\n",
    replace: "            && let [key, ..] = args.as_slice()\n",
    tests: ["test/modules.test.ts","-t","MIR's thread-local number"],
  },
  {
    name: "mir-quiet-local-borrow-not-inlined",
    breaks: "MIR's quiet thread-local's borrow stays a called arrow",
    file: "src/lower/mir.rs",
    find: "                ..super::calls::apply(f, vec![Expr::member(key, \"value\")])\n",
    replace: "                ..Expr::call(f, vec![Expr::member(key, \"value\")])\n",
    tests: ["test/modules.test.ts","-t","thread-local only read and set"],
  },
  {
    name: "mir-store-span-lost",
    breaks: "MIR's values map to no Rust in the source map",
    file: "src/lower/mir.rs",
    find: "                e.span = self.js_span(span);\n",
    replace: "",
    tests: ["test/modules.test.ts","-t","methods across modules"],
  },
  {
    name: "mir-return-span-of-end",
    breaks: "MIR's `return` maps to where the body ends, which the source map has nothing of",
    file: "src/lower/mir.rs",
    find: "                    Some(value) if !value.span.is_none() => value.span,\n",
    replace: "                    Some(value) if false => value.span,\n",
    tests: ["test/modules.test.ts","-t","methods across modules"],
  },
  {
    name: "mir-params-suffixed",
    breaks: "MIR's closure parameter keeps the suffix its function's names gave it",
    file: "src/lower/mir.rs",
    find: "        tidy::unsuffixed_params(&mut out, self.module_names);\n",
    replace: "",
    tests: ["test/link.test.ts","-t","module-load statements"],
  },
  {
    name: "mir-if-span-of-condition",
    breaks: "MIR's `if` maps to its condition, not `if c`",
    file: "src/lower/mir.rs",
    find: "                        span: self.js_span(arm.unwrap_or_else(|| self.if_span(span))),\n",
    replace: "                        span: self.js_span(arm.unwrap_or(span)),\n",
    tests: ["test/emission.test.ts","-t","fib-mir"],
  },
  {
    name: "mir-match-test-span-of-match",
    breaks: "MIR's test of a `match` arm maps to its `match`, not the arm's pattern",
    file: "src/lower/mir.rs",
    find: "                    let arm = written_at(state.body, target).and_then(|at| self.arm_span(span, at));\n",
    replace: "                    let arm: Option<Span> = None;\n",
    tests: ["test/emission.test.ts","-t","fib-mir"],
  },
  {
    name: "mir-array-constant-index-checked",
    breaks: "MIR's constant index within an array is checked, `$index(items, 0)`",
    file: "src/lower/mir.rs",
    find: "                    _ if within => Expr::index(items, index),\n",
    replace: "",
    tests: ["test/traits.test.ts","-t","MIR dictionaries are explicit"],
  },
  {
    name: "mir-deque-remove-refused",
    breaks: "MIR's `VecDeque::remove` is refused",
    file: "src/lower/mir.rs",
    find: "                self.helper(Helper::RemoveOpt, \"$removeOpt\", exprs)\n",
    replace: "                return Err(self.unsupported(span, &format!(\"`remove` of {}, from its MIR\", exprs.len())));\n",
    tests: ["test/lowering.test.ts","-t","a deque's remove"],
  },
  {
    name: "mir-named-divisor-unknown",
    breaks: "MIR's division by a named constant that can't panic is `$div`",
    file: "src/lower/mir.rs",
    find: "                    _ => self.binary(*op, l, r, known, ty, span)?,\n",
    replace: "                    _ => self.binary(*op, l, r, None.or(known.filter(|_| false)), ty, span)?,\n",
    tests: ["test/lowering.test.ts","-t","a division by a named constant"],
  },
  {
    name: "mir-every-loop-valued",
    breaks: "every MIR loop counts as one a `break` gives a value",
    file: "src/lower/mir.rs",
    find: "matches!(e.kind, thir::ExprKind::Loop { .. }) && !e.ty.is_unit() && !e.ty.is_never()",
    replace: "matches!(e.kind, thir::ExprKind::Loop { .. })",
    tests: ["test/lowering.test.ts"],
  },
  {
    name: "mir-untagged-into-through-trait-call",
    breaks: "MIR's `into()` an untagged enum calls the crate's `from`, as the trait's call it resolves to",
    file: "src/lower/mir.rs",
    find: "        if values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) {\n",
    replace: "        if values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) && values.is_empty() {\n",
    tests: ["test/lowering.test.ts","-t","into an untagged enum is the value"],
  },
  {
    name: "mir-label-of-scope",
    breaks: "MIR's labeled loop in a scope of its own is named `bbN`, not its label",
    file: "src/lower/mir.rs",
    find: "                        self.strip(value),\n",
    replace: "                        value,\n",
    tests: ["test/lowering.test.ts","-t","returns in a loop are where"],
  },
  {
    name: "mir-shown-option-applied",
    breaks: "MIR's `Option` shown is an arrow called with it, not held first",
    file: "src/lower/mir.rs",
    find: "                    true => self.spill(\"value\", value, out),\n",
    replace: "                    true => value,\n",
    tests: ["test/serde.test.ts","-t","a Value's reads"],
  },
  {
    name: "mir-trait-generic-iter-lent",
    breaks: "MIR's trait method given a `&mut` to a generic iterator is lowered, a range given it throwing",
    file: "src/lower/mir.rs",
    find: "        if self.tcx.trait_of_assoc(def_id).is_some() || self.tcx.trait_impl_of_assoc(def_id).is_some() {\n            for param in &self.thir.params {\n",
    replace: "        if false {\n            for param in &self.thir.params {\n",
    tests: ["test/diagnostics.test.ts","-t","a &mut to a generic iterator in a trait's method"],
  },
  {
    name: "mir-ptr-eq-value-unchecked",
    breaks: "MIR's `ptr::eq` of a Rust value isn't refused as THIR refuses it",
    file: "src/lower/mir.rs",
    find: "                && self.recognition().classify(id, args) == Some(Std::PtrEq)\n",
    replace: "                && false\n",
    tests: ["test/diagnostics.test.ts","-t","ptr::eq of a Rust struct"],
  },
  {
    name: "mir-raw-ptr-object-refused",
    breaks: "MIR's pointer to a JS object, `ptr::eq`'s, is refused",
    file: "src/lower/mir.rs",
    find: "            Rvalue::RawPtr(_, place) if self.is_js_object(place.ty(decls, tcx).ty) => {\n",
    replace: "            Rvalue::RawPtr(_, place) if false && self.is_js_object(place.ty(decls, tcx).ty) => {\n",
    tests: ["test/lowering.test.ts","-t","ptr::eq of JS objects"],
  },
  {
    name: "mir-ptr-eq-refused",
    breaks: "MIR's `ptr::eq` of JS objects is refused",
    file: "src/lower/mir.rs",
    find: "                Expr::bin(Op::Eq, a, b)\n            }\n            // What was there, taken out, as THIR's.\n",
    replace: "                return Err(self.unsupported(span, if a.reads_same() && b.reads_same() { \"ptr::eq\" } else { \"ptr::eq of these\" }));\n            }\n            // What was there, taken out, as THIR's.\n",
    tests: ["test/lowering.test.ts","-t","ptr::eq of JS objects"],
  },
  {
    name: "mir-generic-iter-field-lent",
    breaks: "MIR's generic iterator a field holds is stepped through a `&mut`, a range given it throwing",
    file: "src/lower/mir.rs",
    find: "                    && place.projection.iter().any(|elem| matches!(elem, PlaceElem::Field(..)))\n",
    replace: "                    && false\n",
    tests: ["test/diagnostics.test.ts","-t","a &mut to a generic iterator in a field"],
  },
  {
    name: "mir-constant-written-shared",
    breaks: "MIR's named constant a local copies is the constant itself though the local is written, changing it",
    file: "src/lower/mir.rs",
    find: "            && only_read(state.body, place.local)\n",
    replace: "            && true\n",
    tests: ["test/corpus.test.ts","-t","bitwise_operators"],
  },
  {
    name: "mir-variant-cell-field-read",
    breaks: "MIR's `Cell` a variant's field holds is read as a cell, `slot.n.value`",
    file: "src/lower/mir.rs",
    find: "                Ok(self.held(read, self.field_ty(&variant.fields[field], args)))\n",
    replace: "                Ok(read)\n",
    tests: ["test/lowering.test.ts","-t","a cell in a field is the property it holds"],
  },
  {
    name: "mir-cell-handle-root-followed",
    breaks: "MIR's handle on a `Cell` field reads the field of what its reference is set to after",
    file: "src/lower/mir.rs",
    find: "                    && (self.is_std_type(pointee, StdItem::Cell) || self.plain_ref_cell(pointee))\n",
    replace: "                    && false\n",
    tests: ["test/lowering.test.ts","-t","a Cell a let takes apart"],
  },
  {
    name: "mir-returned-local-copied",
    breaks: "MIR's variable a function gives is copied, `return { ...p }`",
    file: "src/lower/mir.rs",
    find: "                if *place == Place::from(mir::RETURN_PLACE)\n",
    replace: "                if false\n",
    tests: ["test/lowering.test.ts","-t","a variable a function gives is handed over uncopied"],
  },
  {
    name: "mir-nullable-made-coalesced",
    breaks: "MIR's `Some` of a function given a nullable parameter is `(e) => .. ?? null`",
    file: "src/lower/mir.rs",
    find: "                            _ if value.is_constant() || value.is_made_here() => value,\n",
    replace: "                            _ if value.is_constant() => value,\n",
    tests: ["test/compiler.test.ts","-t","the webapi crate's bindings become plain JS"],
  },
  {
    name: "mir-writer-unshaped",
    breaks: "MIR's writer is `let f = \"\"; f += s; return f`, not `return s`",
    file: "src/lower/mir.rs",
    find: "                false => tidy::writer_body(&mut out, name),\n",
    replace: "                false => {}\n",
    tests: ["test/lowering.test.ts","-t","a writer from its MIR returns the string it writes"],
  },
  {
    name: "mir-derived-params-unnamed",
    breaks: "MIR's derived impl's parameters are `_1`, not `point`",
    file: "src/lower/mir.rs",
    find: "                && (!info.source_info.span.in_external_macro(sm) || body.local_kind(place.local) == LocalKind::Arg)\n",
    replace: "                && !info.source_info.span.in_external_macro(sm)\n",
    tests: ["test/lowering.test.ts","-t","a writer from its MIR returns the string it writes"],
  },
  {
    name: "mir-writer-options-dropped",
    breaks: "MIR's writer takes no options, a `{:>6}` of it unpadded",
    file: "src/lower/mir.rs",
    find: "let options = (formatter.is_some() && self.writers_take_options())",
    replace: "let options = (formatter.is_some() && false)",
    tests: ["test/lowering.test.ts","-t","a writer from its MIR takes its Formatter's options"],
  },
  {
    name: "mir-writer-fields-plain",
    breaks: "MIR's derived `Debug` gives its fields none of its options",
    file: "src/lower/mir.rs",
    find: "                            if state.writer.is_some() {\n",
    replace: "                            if false {\n",
    tests: ["test/lowering.test.ts","-t","a writer from its MIR takes its Formatter's options"],
  },
  {
    name: "mir-writer-error-ignored",
    breaks: "MIR's writer's `Err(fmt::Error)` is nothing, the writer going on",
    file: "src/lower/mir.rs",
    find: "let thrown = Expr::call(Expr::var(\"$fmtError\"), Vec::new());",
    replace: "let thrown = Expr::undefined();",
    tests: ["test/corpus.test.ts","-t","fmt_error"],
  },
  {
    name: "mir-writer-written-lost",
    breaks: "MIR's failing writer loses what it wrote before the error",
    file: "src/lower/mir.rs",
    find: "            match self.writer_fails() {\n",
    replace: "            match false {\n",
    tests: ["test/corpus.test.ts","-t","fmt_error"],
  },
  {
    name: "mir-writer-written-whole",
    breaks: "MIR's `write!` of what may fail in a writer is written whole, what came before it lost",
    file: "src/lower/mir.rs",
    find: "let pieces = values.iter().any(|v| matches!(v, Value::Fmt(Std::FmtNew, ..)));",
    replace: "let pieces = false;",
    tests: ["test/corpus.test.ts","-t","fmt_error"],
  },
  {
    name: "mir-writer-questions-thir",
    breaks: "MIR leaves a writer asking its `Formatter` to THIR",
    file: "src/lower/mir.rs",
    find: "                    if whole && at == 0 && super::recognition::formatter_query(self.tcx, def_id).is_some() {\n",
    replace: "                    if false {\n",
    tests: ["test/lowering.test.ts","-t","a writer from its MIR answers what its Formatter is asked"],
  },
  {
    name: "mir-writer-builder-kept-stepped",
    breaks: "MIR's `Debug` builder kept in a variable a loop steps shows one step",
    file: "src/lower/mir.rs",
    find: "if let Some(&local) = builders.keys().find(|&&local| named(local)) {",
    replace: "if let Some(&local) = builders.keys().find(|_| false) {",
    tests: ["test/diagnostics.test.ts","-t","a Debug builder kept in a variable"],
  },
  {
    name: "mir-writer-builder-entries-stepped",
    breaks: "MIR's `entries(v.iter())` is `Array.from($iter(v))`",
    file: "src/lower/mir.rs",
    find: "true => super::iterators::iterated(value),",
    replace: "true => value,",
    tests: ["test/lowering.test.ts","-t","a writer's Debug builder from its MIR is the string its chain writes"],
  },
  {
    name: "mir-writer-formatter-not-given",
    breaks: "MIR refuses a writer giving its `Formatter` to the crate's generic `fmt::Write` code",
    file: "src/lower/mir.rs",
    find: "if whole && self.given_formatter(def_id, generic_args, at) {",
    replace: "if false {",
    tests: ["test/corpus.test.ts","-t","generic_writers"],
  },
  {
    name: "mir-promoted-functions-refused",
    breaks: "MIR refuses a promoted value of functions, `&[(\"double\", double)]`",
    file: "src/lower/mir.rs",
    find: "            None if let Const::Unevaluated(uv, _) = c.const_\n",
    replace: "            None if false && let Const::Unevaluated(uv, _) = c.const_\n",
    tests: ["test/corpus.test.ts","-t","static_fns"],
  },
  {
    name: "mir-initializer-asked-visibility",
    breaks: "MIR asks a `const { .. }` block's visibility, which rustc has none of",
    file: "src/lower/mir.rs",
    find: "                export: matches!(self.tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)\n",
    replace: "                export: true\n",
    tests: ["test/corpus.test.ts","-t","const_block_values"],
  },
  {
    name: "mir-fn-directives-dropped",
    breaks: "a function's `js::directive!` is written nowhere",
    file: "src/lower/mir.rs",
    find: "        out.splice(0..0, self.directives(body.def_id));\n",
    replace: "        out.splice(0..0, Vec::new());\n",
    tests: ["test/modules.test.ts","-t","in a body is the body"],
  },
  {
    name: "mir-constant-returned-shared",
    breaks: "MIR's named constant a function gives is the constant itself, which its caller changes",
    file: "src/lower/mir.rs",
    find: "            && place.local != mir::RETURN_PLACE\n            && let Rvalue::Use(Operand::Constant(c), _) = rvalue\n",
    replace: "            && let Rvalue::Use(Operand::Constant(c), _) = rvalue\n",
    tests: ["test/corpus.test.ts", "-t", "const_returned_copied"],
  },
  {
    name: "mir-ref-mut-payload-refused",
    breaks: "MIR refuses \`ref mut x\` of an \`Option<&mut T>\`'s payload, a \`&mut &mut T\`",
    file: "src/lower/mir.rs",
    find: "            if steady_place(&part) {\n",
    replace: "            if false && steady_place(&part) {\n",
    tests: ["test/corpus.test.ts", "-t", "ref_mut_of_mut_payload"],
  },
  {
    name: "mir-some-value-unsteady",
    breaks: "MIR refuses \`ref mut x\` of a boxed \`Some\`'s payload, \`$someValue(o)\`",
    file: "src/lower/mir.rs",
    find: "        js::ExprKind::Call(..) => super::std_types::option::some_value_of(place).is_some_and(steady_place),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "ref_mut_of_mut_payload"],
  },
  {
    name: "mir-unread-ref-handled",
    breaks: "MIR's \`&mut\` nothing reads, \`let _ = &mut p.y\` through \`deref_mut\`, refused as a handle on a call's place",
    file: "src/lower/mir.rs",
    find: "                Value::Ref(mut place) => {\n                    while let js::ExprKind::Member(object, _) = place.kind {\n",
    replace: "                Value::Ref(mut place) if false => {\n                    while let js::ExprKind::Member(object, _) = place.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "unread_mut_through_deref"],
  },
  {
    name: "mir-made-array-unsliced",
    breaks: "MIR refuses a slice pattern's rest of an array made where it's taken apart",
    file: "src/lower/mir.rs",
    find: "            PlaceElem::ConstantIndex { from_end: true, .. } | PlaceElem::Subslice { .. } => {\n",
    replace: "            PlaceElem::ConstantIndex { from_end: true, .. } | PlaceElem::Subslice { .. } if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "slice_pattern_of_made_array"],
  },
];
