//! A std call lowered from its MIR, of its values and places (ADR 0364),
//! by its kind: `format_args!`'s parts, text, a map's, a `Vec`'s or a
//! slice's, an iterator's, a place's, a value's. Each lowers the calls it
//! knows and says `None` to the rest; `mir_std_call` asks each in turn, and
//! lowers what none knows of its values alone, by `std_values`.

use super::*;

/// What's called, a std function `classify` knows, and its type's parts.
#[derive(Clone, Copy)]
pub(super) struct StdCall<'c, 'tcx> {
    pub(super) known: Std,
    pub(super) def_id: rustc_span::def_id::DefId,
    pub(super) generic_args: ty::GenericArgsRef<'tcx>,
    pub(super) arg_tys: &'c [Ty<'tcx>],
    pub(super) output: Ty<'tcx>,
    pub(super) span: Span,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `format_args!`'s parts, kept until `Arguments::new` shows them, an
    /// assertion's failure, and a write to a writer of the crate's (ADRs 0054,
    /// 0058).
    pub(super) fn mir_fmt_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        call: StdCall<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Value<'tcx>>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
        } = call;
        let tcx = self.tcx;
        let first_ty = || generic_args.types().next();
        Ok(Some(Value::Expr(match known {
            // `format_args!`'s parts, kept until `Arguments::new` shows them.
            Std::FmtDisplay | Std::FmtDebug | Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {
                // `{:x}`'s and the like, which `format_value` writes by its spec.
                let ty = match known {
                    Std::FmtUsize => self.tcx.types.usize,
                    _ => first_ty().expect("a type argument"),
                };
                let value = match values.next().expect("an argument") {
                    // A `&dyn Debug`, shown as what it's made of is.
                    Value::Fmt(Std::FmtDebug, made, value) if known == Std::FmtDebug => {
                        return Ok(Some(Value::Fmt(known, made, value)));
                    }
                    value => self.value_expr(value, span)?,
                };
                // One read twice, `v == null ? "None" : ..`, is held first by
                // tidy, which sees where nothing it does moves (ADR 0377).
                return Ok(Some(Value::Fmt(known, ty, value)));
            }
            Std::FmtNew => {
                let template = values.next().expect("a template");
                let items = values.next().expect("the arguments");
                let Value::Expr(template) = template else {
                    return Err(self.unsupported(span, "this format string, from its MIR"));
                };
                let bytes =
                    expr_bytes(&template).ok_or_else(|| self.unsupported(span, "this format string, from its MIR"))?;
                let Value::List(mut items) = items else {
                    return Err(self.unsupported(span, "these format arguments, from its MIR"));
                };
                let mut parts = self.formatted_parts(&items, &bytes, span)?;
                // Rust evaluates each argument before it formats any, where
                // the template formats each where it's written: where that
                // would run one after formatting that runs code, each
                // argument that does something is held first (ADR 0377).
                if self.formats_out_of_order(&items, &bytes, span)? {
                    self.flush(state, out)?;
                    for item in items.iter_mut() {
                        if let Value::Fmt(_, _, value) = item
                            && value.has_effects()
                        {
                            *value = self.spill("_$", std::mem::replace(value, Expr::undefined()), out);
                        }
                    }
                    parts = self.formatted_parts(&items, &bytes, span)?;
                }
                // Of what may fail, its `Arguments` say so to `format!`'s or
                // `print!`'s: `fmt_may_fail` of each, by its trait (ADR 0187).
                let fails = items.iter().any(|item| match *item {
                    Value::Fmt(Std::FmtDisplay, ty, _) => self.fmt_may_fail(self.display_trait(), ty),
                    Value::Fmt(Std::FmtDebug, ty, _) => self.fmt_may_fail(self.debug_trait(), ty),
                    // A number's or a pointer's, unless a type of the crate's.
                    Value::Fmt(_, ty, _) => self.krate.any_failing && ty.peel_refs().is_adt(),
                    _ => false,
                });
                // Written across lines, its line breaks as they are (ADR 0066).
                let lines = bytes.contains(&b'\n') && self.format_string_across_lines(span);
                let joined = crate::lower::display::join(parts).with_lines(lines);
                if fails {
                    return Ok(Some(Value::Fmt(Std::FmtNew, output, joined)));
                }
                joined
            }
            // `assert_failed(kind, &left, &right, None or Some(message))`: the
            // two values' `{:?}`, by their types (ADR 0060).
            Std::AssertFailed => {
                let mut list = Vec::new();
                for (i, value) in values.enumerate() {
                    let value = self.value_expr(value, span)?;
                    list.push(match i {
                        1 | 2 => self.debug_string(value, arg_tys[i].peel_refs(), span)?,
                        _ => value,
                    });
                }
                // A `None` message, `undefined`, is left out as preparation
                // leaves out each trailing one.
                Expr::call(Expr::helper("$assertFailed"), list)
            }
            // A writer of the crate's own: what `write!` and `write_char` give
            // its `write_str` (ADR 0166).
            Std::UserWrite => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let trait_id = tcx.trait_of_assoc(def_id).expect("`fmt::Write`'s method");
                let write_str = trait_method(tcx, trait_id, "write_str");
                self.flush(state, out)?;
                self.trait_call(write_str, generic_args, exprs, span, out)?
                    .ok_or_else(|| self.unsupported(span, "this writer's `write_str`"))?
            }
            // A writer of bytes of the crate's own: its dictionary's
            // `write_all` or `write_fmt`, given it in a box, as generic code
            // gives it (ADR 0392).
            Std::UserIoWrite => {
                let mut exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let trait_id = tcx.trait_of_assoc(def_id).expect("`io::Write`'s method");
                let tr = ty::TraitRef::new_from_args(tcx, trait_id, generic_args);
                let dictionary = self.dictionary(tr, span)?;
                let receiver = exprs.remove(0);
                let receiver = match self.is_cell_pointee(tr.self_ty()) {
                    true => receiver,
                    false => super::super::mut_refs::boxed(receiver),
                };
                exprs.insert(0, receiver);
                let call = Expr::call(Expr::member(dictionary, bindings::fn_name(tcx, def_id)), exprs);
                // std's `write_all` of the crate's own `write`, which may fail
                // where any of the crate's may: its result a value (ADR 0462).
                match self.krate.any_failing {
                    true => self.fmt_try(vec![StmtKind::Return(Some(call)).at(js::Span::NONE)]),
                    false => call,
                }
            }
            _ => return Ok(None),
        })))
    }

    /// Whether the template `bytes` formats `items` would evaluate one that
    /// does something after formatting that may run the program's code, a
    /// `Debug` of the crate's, where Rust evaluates it first. Of its parts
    /// formatted with a stand-in for each argument, what each formatting
    /// runs alone. MIR's folding keeps the arguments in the order given, and
    /// one read twice held.
    fn formats_out_of_order(&mut self, items: &[Value<'tcx>], bytes: &[u8], span: Span) -> R<bool> {
        let effectful: Vec<bool> = (items.iter())
            .map(|item| matches!(item, Value::Fmt(_, _, value) if value.has_effects()))
            .collect();
        if !effectful.contains(&true) {
            return Ok(false);
        }
        let stand_in = |i: usize| format!("_$argument{i}");
        let stood: Vec<Value<'tcx>> = (items.iter().enumerate())
            .map(|(i, item)| match item {
                Value::Fmt(kind, ty, _) => Value::Fmt(*kind, *ty, Expr::var(&stand_in(i))),
                other => other.clone(),
            })
            .collect();
        let mut formatted = false;
        for part in self.formatted_parts(&stood, bytes, span)? {
            let mut reads = false;
            part.visit_vars(&mut |var| reads |= (0..items.len()).any(|i| effectful[i] && var == stand_in(i)));
            if reads && formatted {
                return Ok(true);
            }
            formatted |= part.runs_program_code();
        }
        Ok(false)
    }

    /// Each part of a format string, `bytes`, its text and each argument of
    /// `items` formatted as its spec says.
    fn formatted_parts(&mut self, items: &[Value<'tcx>], bytes: &[u8], span: Span) -> R<Vec<Expr>> {
        let mut parts = Vec::new();
        for piece in crate::lower::format_args::decode_template(bytes)
            .ok_or_else(|| self.unsupported(span, "this format string"))?
        {
            parts.push(match piece {
                crate::lower::format_args::Piece::Text(text) => Expr::str(text),
                crate::lower::format_args::Piece::Argument(i, spec) => {
                    let bad = || self.unsupported(span, "this format argument, from its MIR");
                    let item = |i: usize| match items.get(i) {
                        Some(Value::Fmt(kind, ty, value)) => Some((*kind, *ty, value.clone())),
                        _ => None,
                    };
                    let (kind, ty, value) = item(i).ok_or_else(bad)?;
                    // `{:#?}` of a `&dyn Debug` made elsewhere, the string it
                    // showed there, not pretty (ADR 0137).
                    if spec.alternate && kind == Std::FmtDebug && self.is_dyn_debug(ty) {
                        return Err(self.unsupported(span, "`{:#?}` of a `&dyn Debug` made elsewhere"));
                    }
                    // `{:1$}`'s width, or `{:.*}`'s precision, another argument.
                    let width = match spec.width_from {
                        Some(from) => Some(item(from).ok_or_else(bad)?.2),
                        None => spec.width.map(|w| Expr::int(w.into())),
                    };
                    let precision = match spec.precision_from {
                        Some(from) => Some(item(from).ok_or_else(bad)?.2),
                        None => spec.precision.map(|p| Expr::int(p.into())),
                    };
                    self.format_value(value, (kind, ty), spec, (width, precision), span)?
                }
            });
        }
        Ok(parts)
    }

    /// A string's or a `str`'s call: pushed to, sliced, edited in place
    /// (ADR 0063).
    pub(super) fn mir_text_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        call: StdCall<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Value<'tcx>>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            span,
            ..
        } = call;
        Ok(Some(Value::Expr(match known {
            // `s.push_str(t)`, `write!(s, ..)`: JS strings don't change, so `s`
            // gets a new one.
            Std::PushStr => {
                let place = match values.next() {
                    Some(Value::Ref(place)) => place,
                    Some(Value::Expr(place))
                        if matches!(place.kind, js::ExprKind::Var(_) | js::ExprKind::Member(..)) =>
                    {
                        place
                    }
                    _ => return Err(self.unsupported(span, "`push_str` on this, from its MIR")),
                };
                let given = values.next().expect("what's pushed");
                // A `write!` of what may fail: piece by piece, what came before
                // written, its `fmt::Result` the error, if any (ADR 0187).
                if matches!(given, Value::Fmt(Std::FmtNew, ..)) && fixed_place(&place) {
                    let value = self.value_expr(given, span)?;
                    self.flush(state, out)?;
                    let js_span = self.js_span(span);
                    let mut written = Vec::new();
                    crate::lower::display::append_written(&place, value, true, js_span, &mut written);
                    let caught = self.fresh("error");
                    let partial = Expr::call(Expr::helper("$fmtPartial"), vec![Expr::var(&caught)]);
                    let handler = vec![
                        StmtKind::Assign(place.clone(), Expr::bin(Op::Add, place.clone(), partial)).at(js_span),
                        StmtKind::Throw(Expr::var(&caught)).at(js_span),
                    ];
                    let tried = vec![StmtKind::TryCatch(written, Some(caught), handler).at(js_span)];
                    return Ok(Some(Value::Expr(self.fmt_try(tried))));
                }
                let value = self.value_expr(given, span)?;
                // `s.extend(items)`: their text, joined (ADR 0445).
                let extended = (self.tcx.trait_of_assoc(def_id))
                    .is_some_and(|t| crate::lower::recognition::is_extend(self.tcx, t));
                let value = match extended {
                    true => self.joined_text(value, Some(arg_tys[1]), span)?,
                    false => value,
                };
                self.flush(state, out)?;
                // Read and written: a place whose object is made here, a
                // handle `push_mut` gives, `$pushMut(v, x).value`, held first,
                // so it's made once.
                let place = match place.kind {
                    js::ExprKind::Member(object, field) if !object.reads_same() => {
                        Expr::member(self.spill("value", *object, out), &field)
                    }
                    kind => Expr {
                        reads: false,
                        kind,
                        span: place.span,
                    },
                };
                crate::lower::display::append_written(&place, value, false, self.js_span(span), out);
                Expr::undefined()
            }
            // `&v[a..b]`, `s.get(a..)`, `v.drain(..b)`: of the items and the range's
            // value. Not `&mut v[a..b]`, a view of them, not a copy.
            Std::Text(op @ (TextOp::Slice | TextOp::StrSlice | TextOp::StrGet | TextOp::SliceGet | TextOp::Drain))
                if !self
                    .tcx
                    .trait_of_assoc(def_id)
                    .is_some_and(|t| self.tcx.is_lang_item(t, LangItem::IndexMut)) =>
            {
                let items = self.value_expr(values.next().expect("the items"), span)?;
                let range = self.value_expr(values.next().expect("the range"), span)?;
                self.slice_range_values(op, items, (range, arg_tys[1]), span, out)?
            }
            // A part of a string as a `&mut str` (ADR 0334), of the cell its
            // `&mut` is.
            Std::Text(op @ (TextOp::StrPart { .. } | TextOp::StrGetMut | TextOp::StrSplitAtMut { .. })) => {
                let cell = self.value_expr(values.next().expect("the string"), span)?;
                let mut given = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                if matches!(op, TextOp::StrPart { bounds: false } | TextOp::StrGetMut) {
                    let range = given.remove(0);
                    let (start, end) = self.range_value_bounds(range, arg_tys[1], span, out)?;
                    given = std::iter::once(start).chain(end).collect();
                }
                self.str_part_values(op, cell, given)
            }
            // A string's or a slice's method, but one of a range or a part of
            // it, which is lowered from its place.
            Std::Text(op)
                if !matches!(
                    op,
                    TextOp::Slice
                        | TextOp::StrSlice
                        | TextOp::StrGet
                        | TextOp::SliceGet
                        | TextOp::Drain
                        | TextOp::Splice
                        | TextOp::ExtendFromWithin
                        | TextOp::StrPart { .. }
                        | TextOp::StrGetMut
                        | TextOp::StrSplitAtMut { .. }
                ) =>
            {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.text_values(op, exprs, arg_tys, generic_args, span, out)?
            }
            // `s.insert(i, c)` and the rest: `s` given its new text; what it
            // took out, its value.
            // A `Vec`'s range replaced, `splice`, or pushed again,
            // `extend_from_within`: `range_edit`.
            Std::Text(op @ (TextOp::Splice | TextOp::ExtendFromWithin)) => {
                let mut exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let rest = exprs.split_off(2);
                let [items, range]: [Expr; 2] = exprs.try_into().ok().expect("the items and a range");
                let range = self.slice_range_values(range_read(op), items, (range, arg_tys[1]), span, out)?;
                let clone = match self.slice_item(arg_tys[0]) {
                    Some(item) if op == TextOp::ExtendFromWithin && self.needs_clone(item) => {
                        Some(self.clone_arg(item, span)?)
                    }
                    _ => None,
                };
                self.range_edit(op, range, rest, clone)
            }
            Std::StringEdit(edit) => {
                let place = match values.next().expect("the string") {
                    Value::Ref(place) if fixed_place(&place) => place,
                    // `s[a..b].make_ascii_uppercase()`: `s` given its new text,
                    // that part changed, `s = $asciiCase(s, true, a,
                    // b)` (ADR 0334).
                    Value::Ref(ref place)
                        if let StringEdit::AsciiCase { upper } = edit
                            && let Some((whole, mut bounds)) = str_part_of(place) =>
                    {
                        self.flush(state, out)?;
                        if bounds.first().and_then(Expr::as_int) == Some(0) && bounds.len() == 1 {
                            bounds.clear();
                        }
                        let mut list = vec![whole.clone(), Expr::bool(upper)];
                        list.extend(bounds);
                        let edited = Expr::call(Expr::helper("$asciiCase"), list);
                        out.push(StmtKind::Assign(whole, edited).at(self.js_span(span)));
                        return Ok(Some(Value::Expr(Expr::undefined())));
                    }
                    // A part another call gives, `$strGetMut(..)`'s: held,
                    // its `value` the place.
                    Value::Ref(ref place)
                        if let js::ExprKind::Member(cell, field) = &place.kind
                            && field == js::HELD =>
                    {
                        self.flush(state, out)?;
                        let cell = self.spill("part", (**cell).clone(), out);
                        Expr::held(cell)
                    }
                    _ => return Err(self.unsupported(span, "changing this string, from its MIR")),
                };
                let mut given = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                if matches!(
                    edit,
                    StringEdit::Drain | StringEdit::ReplaceRange | StringEdit::ExtendFromWithin
                ) {
                    let range = given.remove(0);
                    let (start, end) = self.range_value_bounds(range, arg_tys[1], span, out)?;
                    given.splice(0..0, [start, end.unwrap_or_else(Expr::undefined)]);
                }
                self.flush(state, out)?;
                let (edited, taken) = self.string_edit_values(edit, place.clone(), given, out);
                out.push(StmtKind::Assign(place, edited).at(self.js_span(span)));
                taken.unwrap_or_else(Expr::undefined)
            }
            // Its capacity is the engine's: what it's given runs, for what it does.
            Std::StringWithCapacity => {
                for value in values {
                    let value = self.value_expr(value, span)?;
                    if value.has_effects() {
                        self.flush(state, out)?;
                        out.push(StmtKind::Expr(value).at(self.js_span(span)));
                    }
                }
                Expr::str("")
            }
            _ => return Ok(None),
        })))
    }

    /// A map's or a set's call of its places: a range of a tree, what's
    /// extracted, an entry (ADR 0059).
    pub(super) fn mir_map_call(
        &mut self,
        _state: &mut State<'_, 'tcx>,
        call: StdCall<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Value<'tcx>>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let StdCall {
            known,
            generic_args,
            arg_tys,
            span,
            ..
        } = call;
        Ok(Some(Value::Expr(match known {
            // A B-tree's range of its keys (ADR 0325).
            Std::Map(MapOp::TreeRange { mutable }) => {
                let [m, range]: [Value<'tcx>; 2] =
                    values.collect::<Vec<_>>().try_into().ok().expect("a tree and a range");
                let (m, range) = (self.value_expr(m, span)?, self.value_expr(range, span)?);
                self.tree_range_values((m, arg_tys[0]), (range, arg_tys[1]), mutable, span, out)?
            }
            // A map's or a set's `extract_if`, a B-tree's of a range.
            Std::Map(MapOp::ExtractIf) => {
                let mut exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let f = exprs.pop().expect("the test");
                let m = exprs.remove(0);
                let range = match self.is_sorted(arg_tys[0].peel_refs()) {
                    true => Some((exprs.remove(0), arg_tys[1])),
                    false => None,
                };
                self.map_extract_if_values((m, arg_tys[0]), range, f, span, out)?
            }
            // A map's or a set's method, but an entry's value or a range, which
            // is lowered from its places.
            Std::Map(op)
                if !matches!(
                    op,
                    MapOp::OrInsert
                        | MapOp::OrInsertWith
                        | MapOp::OrDefault
                        | MapOp::TreeRange { .. }
                        | MapOp::ExtractIf
                ) =>
            {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.map_values(op, (exprs, arg_tys), generic_args, span, out)?
            }
            _ => return Ok(None),
        })))
    }

    /// A `Vec`'s, a slice's, a `VecDeque`'s or a heap's call of its places:
    /// sorted, split, viewed, written (ADRs 0025, 0324).
    pub(super) fn mir_vec_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        call: StdCall<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Value<'tcx>>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
        } = call;
        let tcx = self.tcx;
        Ok(Some(Value::Expr(match known {
            // `vec![..]`: the array its box was written with.
            Std::VecMacro => self.value_expr(values.next().expect("the array"), span)?,
            // A deque's `range_mut(a..b)`: a view of those items, each a box
            // where it's one JS can't change in place.
            Std::Slice(SliceOp::DequeRangeMut) => {
                let items = self.value_expr(values.next().expect("the items"), span)?;
                let range = self.value_expr(values.next().expect("the range"), span)?;
                let (start, end) = self.range_value_bounds(range, arg_tys[1], span, out)?;
                let mut list = vec![items, start];
                list.extend(end);
                let view = self.view_call("$view", list);
                match self.is_boxable(generic_args.type_at(0)) {
                    true => Expr::call_helper("$mutItems", vec![view]),
                    false => view,
                }
            }
            // `v.sort()`, `v.sort_by_key(key)`: in place.
            Std::Sort | Std::SortByKey => {
                let items = self.value_expr(values.next().expect("the items"), span)?;
                let key = match known {
                    Std::SortByKey => Some(self.value_expr(values.next().expect("the key"), span)?),
                    _ => None,
                };
                self.sort_values(key, items, arg_tys[0], generic_args, span, out)?
            }
            // `v.sort_unstable_by(f)` and a selection (ADR 0342).
            Std::Slice(op @ (SliceOp::SortUnstable { with } | SliceOp::SelectNth { with })) => {
                let values = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.unstable_sort_values(op, with, values, generic_args, span, out)?
            }
            // The rest of a slice's own that their operands say all of: not one
            // giving a `&mut`, a view, nor `remainder`, which asks whether its
            // iterator is stepped.
            Std::Slice(op)
                if !matches!(
                    op,
                    SliceOp::Remainder
                        | SliceOp::View { .. }
                        | SliceOp::DequeRangeMut
                        | SliceOp::ExtractIf { .. }
                        | SliceOp::SplitOff { .. }
                        | SliceOp::UninitWrite { .. }
                        | SliceOp::UninitDrop
                        | SliceOp::CopyWithin
                ) && (known.gives_its_cell() || !output.walk().any(|arg| {
                    // A `&mut` to items, a view of them (ADR 0335); not to one
                    // item, which a number's is a box of, but where the op
                    // makes that box itself, `split_first_mut`'s.
                    arg.as_type().is_some_and(|t| {
                        matches!(t.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if !inner.is_slice() && !inner.is_array())
                    })
                })) =>
            {
                let values = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.slice_values(op, values, false, generic_args, span, out)?
            }
            // A chunks' `remainder()`: from MIR, which steps a std iterator as
            // a `$iter` (`std_iterator`), its items'; of one made there, the
            // array's.
            Std::Slice(SliceOp::Remainder) => {
                let mut exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let made = match &exprs[0].kind {
                    js::ExprKind::Call(_, args) if is_stepped_items(&exprs[0]) && args.len() == 1 => {
                        Some(args[0].clone())
                    }
                    _ => None,
                };
                let stepping = made.is_none();
                if let Some(items) = made {
                    exprs[0] = items;
                }
                self.slice_values(SliceOp::Remainder, exprs, stepping, generic_args, span, out)?
            }
            // `v.copy_within(a..b, at)` and `v.extract_if(range, f)`: of the
            // range's bounds.
            Std::Slice(SliceOp::CopyWithin) => {
                let items = self.value_expr(values.next().expect("the items"), span)?;
                let range = self.value_expr(values.next().expect("the range"), span)?;
                let bounds = self.range_value_bounds(range, arg_tys[1], span, out)?;
                let dest = self.value_expr(values.next().expect("where to"), span)?;
                self.copy_within_values(items, bounds, dest)
            }
            Std::Slice(SliceOp::ExtractIf { range }) => {
                let mut exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let f = exprs.pop().expect("the test");
                let items = exprs.remove(0);
                let bounds = match range {
                    true => self.range_value_bounds(exprs.remove(0), arg_tys[1], span, out)?,
                    false => (Expr::int(0), None),
                };
                self.extract_if_values(items, bounds, f, generic_args.type_at(0))
            }
            // A slice of `MaybeUninit`s written from a slice, and each item
            // dropped (ADR 0332).
            Std::Slice(SliceOp::UninitWrite { clone }) => {
                let values = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.flush(state, out)?;
                self.uninit_write_values(clone, values, generic_args.type_at(0), span, out)?
            }
            Std::Slice(SliceOp::UninitDrop) => {
                let slots = self.value_expr(values.next().expect("the slots"), span)?;
                let ty = Ty::new_slice(tcx, generic_args.type_at(0));
                self.flush(state, out)?;
                self.drop_value(slots, ty, span, out)?;
                Expr::undefined()
            }
            // `s.split_off(..n)` and the like: the slice's place given what's
            // left (ADR 0338).
            Std::Slice(SliceOp::SplitOff { end, mutable }) => {
                // Of a `&mut [T]`, which a `&mut` to isn't a cell: the
                // variable `rest.split_off_mut(..)` borrows, not one holding a
                // `&mut` to it, which JS would assign instead.
                let place = match values.next().expect("the slice") {
                    Value::Ref(place) if fixed_place(&place) => place,
                    Value::Expr(place)
                        if matches!(place.kind, js::ExprKind::Var(_)) && self.borrows_variable(def_id, span, 0) =>
                    {
                        place
                    }
                    _ => return Err(self.unsupported(span, "splitting this slice")),
                };
                let bounds = match end {
                    None => {
                        let range = self.value_expr(values.next().expect("the range"), span)?;
                        Some(self.range_value_bounds(range, arg_tys[1], span, out)?)
                    }
                    Some(_) => None,
                };
                self.flush(state, out)?;
                let split =
                    self.split_off_values((end, mutable), place.clone(), bounds, generic_args.type_at(0), out);
                let left = Expr::index(split.clone(), Expr::int(0));
                out.push(StmtKind::Assign(place, left).at(self.js_span(span)));
                Expr::index(split, Expr::int(1))
            }
            // `&mut v[a..b]`, `get_mut(a..b)`: a view of those items (ADR 0335).
            Std::Slice(SliceOp::View { checked }) => {
                let items = self.value_expr(values.next().expect("the items"), span)?;
                let range = self.value_expr(values.next().expect("the range"), span)?;
                let (start, end) = self.range_value_bounds(range, arg_tys[1], span, out)?;
                // `&mut v[..]`: all of it, which is `v`.
                if !checked && start.as_int() == Some(0) && end.is_none() {
                    items
                } else {
                    let mut list = vec![items, start];
                    list.extend(end);
                    self.view_call(if checked { "$viewGet" } else { "$view" }, list)
                }
            }
            // `vec![item; n]`: each a clone, of what it can't remake.
            Std::FromElem => {
                let item = self.value_expr(values.next().expect("the item"), span)?;
                let n = self.value_expr(values.next().expect("the count"), span)?;
                self.copies_values((item, arg_tys[0]), n, span, out)?
            }
            // A `BinaryHeap`'s, and its `peek_mut()` guard's.
            Std::Heap(op) => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.heap_values(op, exprs, arg_tys[0], span)?
            }
            // What was there, taken out.
            Std::DequeRemove => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                Expr::call_helper("$removeOpt", exprs)
            }
            _ => return Ok(None),
        })))
    }

    /// An iterator's call `mir_iter_call` leaves: a combinator, a step
    /// through a `Peekable`, a range's, a stream's (ADRs 0062, 0071).
    pub(super) fn mir_iter_std_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        call: StdCall<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Value<'tcx>>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let StdCall {
            known,
            generic_args,
            arg_tys,
            output,
            span,
            ..
        } = call;
        Ok(Some(Value::Expr(match known {
            Std::Range(op) => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.range_call(op, exprs, arg_tys, span, out)?
            }
            // An `Option`'s or a `Result`'s combinator: of its arguments' values,
            // the subject one just made where it's a call's.
            Std::Comb(comb) => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let made = matches!(exprs[0].kind, js::ExprKind::Call(..));
                self.comb_values(comb, (exprs, arg_tys), made, generic_args, span, out)?
            }
            // An iterator's combinator, of a JS iterator, lazy as Rust's; one a
            // `&mut` lends, stepped through what can't close it.
            Std::IterComb(comb) => {
                let iter_ty = arg_tys[0].peel_refs();
                // Items with a destructor a combinator may leave undropped; not
                // one that leaves none, `partition`'s and `max_by`'s, nor an
                // adapter that holds what it steps, closed to drop it, which
                // drops each it discards (ADR 0465).
                let dropping = self
                    .iterator_item(iter_ty)
                    .filter(|&item| self.drops(item) != crate::lower::drops::Drops::Nothing);
                if dropping.is_some() && !comb.leaves_none() && !comb.holds_its_sources() {
                    return Err(self.unsupported(
                        span,
                        "an iterator's combinator of items with a destructor, from its MIR",
                    ));
                }
                let lent = matches!(arg_tys[0].kind(), ty::Ref(_, _, ty::Mutability::Mut));
                let mut exprs = values
                    .map(|v| match v {
                        Value::Ref(place) => Ok(place),
                        v => self.value_expr(v, span),
                    })
                    .collect::<R<Vec<_>>>()?;
                let receiver = exprs.remove(0);
                let it = self.as_js_iterator(receiver, iter_ty, span)?;
                let it = match lent {
                    true => self.lent_iterator(it),
                    false if comb.holds_its_sources() => self.closing(it, iter_ty, span)?,
                    false => it,
                };
                // Another iterator it's given, `chain(Counter(0))`'s, the
                // crate's: stepped by its own `next`, not its fields, as a
                // type parameter's given as it is is by its dictionary (ADR
                // 0487); one that owns items with a destructor, `zip(v)`'s,
                // `$closing`.
                let mut exprs = (exprs.into_iter().zip(&arg_tys[1..]))
                    .map(|(e, &ty)| {
                        let user = self.is_user_iterator(ty);
                        let e = match user || self.iterator_given_as_it_is(ty.peel_refs()) {
                            true => self.as_js_iterator(e, ty.peel_refs(), span)?,
                            false => e,
                        };
                        if !matches!(comb, IterComb::Zip | IterComb::Chain) {
                            return Ok(e);
                        }
                        Ok(match self.closing_drop(ty, span)? {
                            Some(drop) if user => Expr::call_helper("$closing", vec![e, drop]),
                            Some(drop) => Expr::call_helper("$closing", vec![self.js_iterator(e), drop]),
                            None => e,
                        })
                    })
                    .collect::<R<Vec<_>>>()?;
                if let Some(item) = dropping
                    && matches!(comb, IterComb::SkipWhile | IterComb::TakeWhile)
                {
                    let test = exprs.remove(0);
                    self.flush(state, out)?;
                    let discard_when = comb == IterComb::SkipWhile;
                    exprs.insert(0, self.dropping_discarded(test, item, discard_when, span, out)?);
                }
                self.iter_comb(comb, it, exprs.into_iter(), generic_args, iter_ty, span, out)?
            }
            // `cycle()` of another iterator than std's over an array, cloned
            // (ADR 0491).
            Std::Step(StepOp::Cycle)
                if !arg_tys
                    .first()
                    .is_some_and(|t| self.array_source(t.peel_refs()).is_some()) =>
            {
                let it = values.map(|v| self.value_expr(v, span)).next().expect("the iterator")?;
                self.cycled(it, arg_tys[0].peel_refs(), span)?
            }
            // A `Peekable`'s or a slice's or string's iterator's own: of a `$iter`, which
            // knows where it is (ADR 0071).
            Std::Step(
                op @ (StepOp::Peek
                | StepOp::PeekMut
                | StepOp::NextIf
                | StepOp::NextIfEq
                | StepOp::AsSlice
                | StepOp::AsStr
                | StepOp::Cycle),
            ) if op == StepOp::AsStr
                || arg_tys
                    .first()
                    .is_some_and(|t| self.array_source(t.peel_refs()).is_some() || self.is_peekable(t.peel_refs())) =>
            {
                let mut exprs = values
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?
                    .into_iter();
                let it = exprs.next().expect("the iterator");
                let boxed = self.option_of(output).is_some_and(|item| self.boxed_payload(item));
                match op {
                    StepOp::Peek if boxed => Expr::call_helper("$peekSome", vec![it]),
                    // A string's `chars()`' rest, as a string.
                    StepOp::AsStr => Expr::call_helper("$restStr", vec![it]),
                    StepOp::Peek => Expr::call_helper("$peek", vec![it]),
                    // A handle on a number or text, else the item itself.
                    StepOp::PeekMut => match self.iterator_item(arg_tys[0].peel_refs()) {
                        Some(item) if self.is_boxable(item) => Expr::call_helper("$peekCell", vec![it]),
                        _ => Expr::call_helper("$peek", vec![it]),
                    },
                    StepOp::Cycle => {
                        let iter_ty = arg_tys[0].peel_refs();
                        let owns = self.array_source(iter_ty).expect("matched");
                        // Its items, owned, which it would drop as it's dropped.
                        if owns
                            && self
                                .iterator_item(iter_ty)
                                .is_some_and(|item| self.drops(item) != crate::lower::drops::Drops::Nothing)
                        {
                            return Err(self.unsupported(span, "`cycle()` of an array's items with a destructor"));
                        }
                        let it = match it.reads_same() || is_stepped_items(&it) {
                            true => it,
                            false => self.spill("it", it, out),
                        };
                        let mut args = vec![self.stepped_left(it, iter_ty, false, span)?];
                        let item = self.iterator_item(iter_ty).expect("an iterator's item");
                        if owns && self.needs_clone(item) {
                            args.push(self.clone_fn("item", item, span)?);
                        }
                        Expr::call_helper("$cycle", args)
                    }
                    StepOp::NextIf => {
                        let f = exprs.next().expect("a test");
                        Expr::call_helper("$nextIf", vec![it, f])
                    }
                    StepOp::NextIfEq => {
                        let item = arg_tys[1];
                        if !self.eq_is_identity(item) {
                            return Err(self.unsupported(span, &format!("`next_if_eq` of `{}`s", item.peel_refs())));
                        }
                        let x = exprs.next().expect("a value");
                        let x = if x.reads_same() {
                            x
                        } else {
                            self.spill("expected", x, out)
                        };
                        let same = Expr::arrow(
                            vec!["item".into()],
                            vec![StmtKind::Return(Some(Expr::bin(Op::Eq, Expr::var("item"), x))).at(js::Span::NONE)],
                        );
                        Expr::call_helper("$nextIf", vec![it, same])
                    }
                    _ => {
                        let it = if it.reads_same() { it } else { self.spill("it", it, out) };
                        self.stepped_left(it, arg_tys[0].peel_refs(), false, span)?
                    }
                }
            }
            // A standard stream's, `writeln!(io::stderr(), ..)`.
            Std::Stream(op) => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.flush(state, out)?;
                self.stream_values(op, exprs, span, out)
            }
            // std's `size_hint()`, of an iterator of the crate's that keeps it:
            // `(0, None)` (ADR 0170).
            Std::SizeHint(false) => {
                let it = self.value_expr(values.next().expect("the iterator"), span)?;
                if it.has_effects() {
                    self.flush(state, out)?;
                    out.push(StmtKind::Expr(it).at(self.js_span(span)));
                }
                Expr::array(vec![Expr::int(0), Expr::undefined()])
            }
            _ => return Ok(None),
        })))
    }

    /// What changes a place: `mem::swap` and `replace`, an `Option`'s
    /// `take`, an `Rc`'s `make_mut`, a `Cow`'s (ADRs 0147, 0319, 0320).
    pub(super) fn mir_place_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        call: StdCall<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Value<'tcx>>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let StdCall {
            known,
            def_id,
            arg_tys,
            output,
            span,
            ..
        } = call;
        Ok(Some(Value::Expr(match known {
            Std::Swap | Std::Replace | Std::MemTake | Std::OptionTake | Std::OptionReplace => {
                let values: Vec<Value<'tcx>> = values.collect();
                return self
                    .mir_swap_or_replace(state, (known, def_id), values, arg_tys, span, out)
                    .map(Some);
            }
            // `dest.clone_from(&source)`: `mem::replace` of `dest` with a clone
            // of `source`, written as it writes, what it gives back dropped,
            // as `*dest = source.clone()` drops it.
            Std::CloneFrom => {
                let ty = arg_tys[1].peel_refs();
                let dest = values.next().expect("the destination");
                let source = values.next().expect("the source");
                let source = self.value_expr(source, span)?;
                let clone = self.clone_value(source, ty, span, out)?;
                let given = vec![dest, Value::Expr(clone)];
                let old = self.mir_swap_or_replace(state, (Std::Replace, def_id), given, arg_tys, span, out)?;
                let old = self.value_expr(old, span)?;
                if self.drops(ty) != crate::lower::drops::Drops::Nothing {
                    let old = self.droppable(old, ty, out);
                    self.drop_value(old, ty, span, out)?;
                } else if old.has_effects() {
                    out.push(StmtKind::Expr(old).at(self.js_span(span)));
                }
                Expr::undefined()
            }
            // `Rc::make_mut(&mut rc)` of a local `Rc` of this body: the local
            // given the one `$makeMut` gives. Through a reference the place
            // isn't here, which a variable given another would leave.
            Std::Rc(RcOp::MakeMut) => {
                let place = self.value_expr(values.next().expect("the `Rc`"), span)?;
                let item = self.counted_rc(arg_tys[0].peel_refs());
                // Of a variable of this body or a field of one, `copy.pages`.
                let local = match root_var(&place) {
                    Some(name) if fixed_place(&place) => state.locals.names.iter_enumerated().find(|(_, n)| *n == name),
                    _ => None,
                };
                let (Some(item), Some((local, _))) = (item, local) else {
                    return Err(self.unsupported(span, "`Rc::make_mut` of an `Rc` that isn't a place here"));
                };
                if state.body.local_kind(local) == LocalKind::Arg || state.body.local_decls[local].ty.is_ref() {
                    return Err(self.unsupported(span, "`Rc::make_mut` of an `Rc` that isn't a place here"));
                }
                self.counted_here(item, span)?;
                self.flush(state, out)?;
                self.make_mut_values(place, item, span, out)?
            }
            // An `Rc`'s or a `Weak`'s, but `make_mut`, which writes its place.
            Std::Rc(op) if op != RcOp::MakeMut => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.rc_values(op, (exprs, arg_tys), span, out)?
            }
            // A `Cow`'s `into_owned()`, its deref, and its `to_mut()` of what's
            // its own place.
            Std::Cow(op) => {
                let cow = match (op, values.next().expect("the `Cow`")) {
                    (CowOp::ToMut, Value::Ref(place) | Value::Expr(place)) if fixed_place(&place) => place,
                    (CowOp::ToMut, _) => return Err(self.unsupported(span, "`to_mut()` of this `Cow`, from its MIR")),
                    (_, value) => self.value_expr(value, span)?,
                };
                self.flush(state, out)?;
                let cow_ty = match op {
                    CowOp::From { .. } => output,
                    _ => arg_tys[0].peel_refs(),
                };
                self.cow_values(op, cow, cow_ty, span, out)?
            }
            // `o.get_or_insert(x)` and the like: the `Option`'s place given its
            // value where it has none, and a `&mut` to what it holds.
            Std::OptionPlace(op) => {
                let place = match values.next().expect("the `Option`") {
                    Value::Ref(place) if fixed_place(&place) => place,
                    _ => return Err(self.unsupported(span, "changing this `Option`, from its MIR")),
                };
                let item = self.option_of(arg_tys[0].peel_refs()).expect("an `Option`");
                let given = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.flush(state, out)?;
                self.option_place_values(op, place, item, (given, &arg_tys[1..]), span, out)?
            }
            _ => return Ok(None),
        })))
    }

    /// A value's call: a number's, JSON's, a type's name, a pointer's
    /// equality (ADRs 0064, 0077).
    pub(super) fn mir_value_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        call: StdCall<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Value<'tcx>>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let StdCall {
            known,
            generic_args,
            arg_tys,
            output,
            span,
            ..
        } = call;
        Ok(Some(Value::Expr(match known {
            // `serde_json::to_string(&v)` and `from_str(s)` (ADR 0081).
            Std::ToJson(pretty) => {
                let value = self.value_expr(values.next().expect("the value"), span)?;
                let ty = generic_args.types().next().expect("`to_string::<T>`");
                self.json_text(value, ty, pretty, span)?
            }
            Std::FromJson => {
                let text = self.value_expr(values.next().expect("the text"), span)?;
                let ty = generic_args.types().next().expect("`from_str::<T>`");
                self.json_value(text, ty, span)?
            }
            Std::NonZeroNew => {
                let n = self.value_expr(values.next().expect("the number"), span)?;
                self.non_zero_new(n, arg_tys[0], span, out)?
            }
            // A `Wrapping`'s operator, `a * Wrapping(2)`, `w += x`.
            Std::WrappingOp(op, assign) => {
                let target = match (assign, values.next().expect("the `Wrapping`")) {
                    (true, Value::Ref(place) | Value::Expr(place)) if fixed_place(&place) => place,
                    (true, _) => return Err(self.unsupported(span, "this assignment, from its MIR")),
                    (false, value) => self.value_expr(value, span)?,
                };
                let target = match op.is_ok() && !target.reads_same() {
                    true => {
                        self.flush(state, out)?;
                        self.spill("wrapping", target, out)
                    }
                    false => target,
                };
                let rhs = values.next().map(|v| self.value_expr(v, span)).transpose()?;
                self.flush(state, out)?;
                self.wrapping_values(op, assign, arg_tys[0].peel_refs(), target, rhs, span, out)?
            }
            // rustc's name for the type; of a type parameter, the one its
            // caller gave (ADR 0145). What `type_name_of_val` is given runs.
            Std::TypeName { .. } => {
                for value in values {
                    let value = self.value_expr(value, span)?;
                    if value.has_effects() {
                        self.flush(state, out)?;
                        out.push(StmtKind::Expr(value).at(self.js_span(span)));
                    }
                }
                let ty = generic_args.type_at(0);
                self.type_fact_value(ty, crate::lower::recognition::TypeFact::Name, span)?
            }
            // `x.to_int_unchecked()`: the cast `as` is, where it's in range.
            Std::Number(NumOp::ToIntUnchecked) => {
                let value = self.value_expr(values.next().expect("the float"), span)?;
                self.cast(value, arg_tys[0], output, span)?
            }
            // Whether two JS objects are one, `a === b` (ADR 0285); of a Rust
            // value, refused before (`mir_function`).
            Std::PtrEq => {
                let mut exprs = values
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?
                    .into_iter();
                let (a, b) = (exprs.next().expect("a"), exprs.next().expect("b"));
                Expr::bin(Op::Eq, a, b)
            }
            Std::Number(op) => {
                // `i32::from_str_radix(s, 16)`'s is what its `Result` holds.
                let ty = match (op, output.kind()) {
                    (NumOp::FromStrRadix, ty::Adt(_, result)) => result.type_at(0),
                    (NumOp::FromBytes { .. } | NumOp::FloatFromBytes { .. } | NumOp::FromBits, _) => output,
                    _ => arg_tys.first().copied().map(|t| t.peel_refs()).expect("a number"),
                };
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.number_call(op, exprs, ty, span, out)?
            }
            _ => return Ok(None),
        })))
    }
}
