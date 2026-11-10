//! A std call lowered from its MIR, of its values and places (ADR 0364),
//! by its kind: `format_args!`'s parts, text, a map's, a `Vec`'s or a
//! slice's, an iterator's, a place's, a value's. Each lowers the calls it
//! knows and says `None` to the rest; `mir_std_call` asks each in turn, and
//! lowers what none knows of its values alone, by `std_values`.

use super::*;
use crate::lower::format_args::Piece;
use crate::lower::std_types::once::OnceOp;

mod fmt_parts;
mod iterators;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A std call's value as its `output` is held. A std iterator
    /// `mir_std_call` makes an array of, `bytes()`'s: a JS iterator, which
    /// MIR steps; not one that's the string it shows, an escape's, nor a `T`
    /// it gives, `unwrap_or`'s, its value. A write's result a `Result`'s
    /// method makes of a value's, `map` of an `io::Result<usize>`: `undefined`
    /// or its error, `$writeOf(..)` (ADR 0462).
    pub(super) fn std_value_as(&mut self, value: Value<'tcx>, output: Ty<'tcx>, arg_tys: &[Ty<'tcx>]) -> Value<'tcx> {
        // std's own iterator, of whatever items, `option::IntoIter<I::Item>`.
        let std_iterator = matches!(output.kind(), ty::Adt(adt, _) if crate::lower::recognition::is_std_crate(self.tcx, adt.did()))
            && self.iterator_item(output).is_some();
        let lent_place = self.std_lends_a_place(output);
        match value {
            Value::Expr(e)
                if (self.implements_iterator(output) || std_iterator)
                    && !lent_place
                    && !matches!(output.kind(), ty::Param(_))
                    && !output.is_box()
                    && self.range_kind(output).is_none()
                    && !self.is_user_iterator(output) =>
            {
                Value::Expr(self.std_iterator(e, output))
            }
            Value::Expr(e)
                if self.is_write_result(output)
                    && arg_tys
                        .first()
                        .is_some_and(|&from| self.recognition().is_result(from) && !self.is_write_result(from)) =>
            {
                Value::Expr(Expr::call_helper("$writeOf", vec![e]))
            }
            value => value,
        }
    }

    /// A std call that gives the `&mut` it's given, of what's in the same
    /// place: `&mut *s` of a `String`, `as_mut_str()`, a `&mut str` to all
    /// of it, the place the `&mut String` is; a pin's `as_mut()`, its; and a
    /// `Box`'s `as_mut()` of an object, `&mut Vec<T>` of a `&mut
    /// Box<Vec<T>>` (#116), the object in that place, as a `&mut` to one is
    /// (ADRs 0099, 0550). `None` of another.
    pub(super) fn same_reference(&self, known: Std, output: Ty<'tcx>, values: &[Value<'tcx>]) -> Option<Value<'tcx>> {
        let [Value::Ref(place)] = values else { return None };
        if !matches!(known, Std::Same | Std::Pin(crate::lower::std_types::pin::PinOp::Mut)) {
            return None;
        }
        let output = self.recognition().pinned(output).unwrap_or(output);
        if self.is_cell(output) {
            return Some(Value::Ref(place.clone()));
        }
        (known == Std::Same && matches!(output.kind(), ty::Ref(_, _, ty::Mutability::Mut)))
            .then(|| Value::Expr(place.clone()))
    }

    /// What MIR says of the call being lowered's arguments, past their
    /// values: which `Option`s are `null` already (`nulled`), whether its
    /// cell is one no borrow of which can meet another (`QuietCells`, ADR
    /// 0863, amended), a nullable thread-local's, its `None` `null` (ADR 0586,
    /// amended), read alike (#665), a `swap`'s flattened states (#687); and
    /// if nothing reads what it gives.
    pub(super) fn call_given<'n>(&self, state: &State<'_, 'tcx>, nulled: &'n [bool]) -> crate::lower::calls::Given<'n> {
        let cells = state.facts.quiet_cells();
        crate::lower::calls::Given {
            nulled,
            quiet: cells.calls.contains(&state.at),
            momentary: cells.momentary.contains(&state.at),
            none_form: cells
                .nullable
                .get(&state.at)
                .or(cells.nullable_lent.get(&state.at))
                .copied()
                .unwrap_or_default(),
            read_alike: cells.alike.contains(&state.at),
            unread: self.result_unread(state),
            flattened: cells.flattened.get(&state.at).copied().unwrap_or_default(),
            lent: state.facts.pointed().lent_calls.contains(&state.at),
        }
    }
}

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

/// What lowers a std call a handler took: its arm's work, given what it
/// may change then. A handler reads what it's given alone, `&self` and
/// `&State`, to take a call, so the one that takes it is chosen before
/// anything's changed (#191, ADR 0791).
pub(super) type StdLowering<'c, 'a, 'tcx> = Box<
    dyn for<'s> FnOnce(
            &mut FnCx<'a, 'tcx>,
            &mut State<'s, 'tcx>,
            &mut std::vec::IntoIter<Value<'tcx>>,
            &mut Vec<Stmt>,
        ) -> R<Value<'tcx>>
        + 'c,
>;

/// An arm's work, boxed: `StdLowering`.
fn lower<'c, 'a, 'tcx>(
    f: impl for<'s> FnOnce(
        &mut FnCx<'a, 'tcx>,
        &mut State<'s, 'tcx>,
        &mut std::vec::IntoIter<Value<'tcx>>,
        &mut Vec<Stmt>,
    ) -> R<Value<'tcx>>
    + 'c,
) -> StdLowering<'c, 'a, 'tcx> {
    Box::new(f)
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `format_args!`'s parts, kept until `Arguments::new` shows them, an
    /// assertion's failure, and a write to a writer of the crate's (ADRs 0054,
    /// 0058).
    pub(super) fn mir_fmt_call<'c>(
        &self,
        _state: &State<'_, 'tcx>,
        call: StdCall<'c, 'tcx>,
    ) -> Option<StdLowering<'c, 'a, 'tcx>> {
        let this = self;
        let StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
        } = call;
        let tcx = this.tcx;
        let first_ty = move || generic_args.types().next();
        Some(match known {
            // `format_args!`'s parts, kept until `Arguments::new` shows them.
            Std::FmtDisplay | Std::FmtDebug | Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {
                lower(move |this, _, values, _| {
                    // `{:x}`'s and the like, which `format_value` writes by its spec.
                    let ty = match known {
                        Std::FmtUsize => this.tcx.types.usize,
                        _ => first_ty().expect("a type argument"),
                    };
                    let value = match values.next().expect("an argument") {
                        // A `&dyn Debug`, shown as what it's made of is.
                        Value::Fmt(Std::FmtDebug, made, value) if known == Std::FmtDebug => {
                            return Ok(Value::Fmt(known, made, value));
                        }
                        value => this.value_expr(value, span)?,
                    };
                    // As it's shown, a handle the place it's on; a pointer's
                    // `{:p}`, its address, as it is.
                    let (value, ty) = match known {
                        Std::FmtPointer => (value, ty),
                        _ => this.shown_through(value, ty),
                    };
                    // One read twice, `v == null ? "None" : ..`, is held first by
                    // tidy, which sees where nothing it does moves (ADR 0377).
                    Ok(Value::Fmt(known, ty, value))
                })
            }
            // `as_str()` of an `Arguments` the body made: its literal's
            // text, `from_str("plain")`'s, or `None` of one made with
            // arguments, `new(..)`'s, as native Rust's (#1091). Of one made
            // elsewhere, a parameter's, nothing says which: refused.
            Std::FmtAsStr => match this.arguments_made(_state.body, _state.at) {
                Some(true) => lower(|_, _, values, _| Ok(values.next().expect("the arguments"))),
                Some(false) => lower(|_, _, _, _| Ok(Value::Expr(Expr::undefined()))),
                None => lower(move |this, _, _, _| {
                    Err(this.unsupported(
                        span,
                        "`as_str()` of a `fmt::Arguments` made in another function: whether it was made with arguments, which makes it `None`, isn't known here (#1091)",
                    ))
                }),
            },
            Std::FmtNew => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let template = values.next().expect("a template");
                    let items = values.next().expect("the arguments");
                    let Value::Expr(template) = template else {
                        return Err(this.unsupported(span, "this format string, from its MIR"));
                    };
                    let bytes = expr_bytes(&template)
                        .ok_or_else(|| this.unsupported(span, "this format string, from its MIR"))?;
                    let Value::List(mut items, _) = items else {
                        return Err(this.unsupported(span, "these format arguments, from its MIR"));
                    };
                    let template = crate::lower::format_args::decode_template(&bytes)
                        .ok_or_else(|| this.unsupported(span, "this format string"))?;
                    // As its format string is written: across lines, or
                    // `concat!`'s pieces (ADRs 0066, 0641).
                    let written = crate::lower::format_args::written_pieces(tcx, span, template);
                    let template: Vec<_> = written.iter().flat_map(|(pieces, _)| pieces.iter().cloned()).collect();
                    // Where each placeholder's argument is written (ADR 0641, amended).
                    let arguments = crate::lower::format_args::placeholder_arguments(tcx, span, &template);
                    let arguments = arguments.as_deref().unwrap_or_default();
                    let mut parts = this.written_parts(&items, &written, arguments, span)?;
                    // Rust evaluates each argument before it formats any, where
                    // the template formats each where it's written: where that
                    // would run one after formatting that runs code, each
                    // argument that does something is held first (ADR 0377).
                    if this.formats_out_of_order(&items, &template, span)? {
                        this.flush(state, out)?;
                        for item in items.iter_mut() {
                            if let Value::Fmt(_, _, value) = item
                                && value.has_effects()
                            {
                                *value = this.spill("_$", std::mem::replace(value, Expr::undefined()), out);
                            }
                        }
                        parts = this.written_parts(&items, &written, arguments, span)?;
                    }
                    // Of what may fail, its `Arguments` say so to `format!`'s or
                    // `print!`'s: `fmt_may_fail` of each, by its trait (ADR 0187).
                    let fails = items.iter().any(|item| match *item {
                        Value::Fmt(Std::FmtDisplay, ty, _) => this.fmt_may_fail(this.display_trait(), ty),
                        Value::Fmt(Std::FmtDebug, ty, _) => this.fmt_may_fail(this.debug_trait(), ty),
                        // A number's or a pointer's, unless a type of the crate's.
                        Value::Fmt(_, ty, _) => this.krate.any_failing && ty.peel_refs().is_adt(),
                        _ => false,
                    });
                    // What may fail is written piece by piece, so that what
                    // came before is written: its pieces kept apart (ADR 0187).
                    let joined = crate::lower::display::join_written(parts, fails);
                    if fails {
                        return Ok(Value::Fmt(Std::FmtNew, output, joined));
                    }
                    joined
                }))
            }),
            // `assert_failed(kind, &left, &right, None or Some(message))`: the
            // two values' `{:?}`, by their types (ADR 0060).
            Std::AssertFailed => lower(move |this, _, values, _| {
                Ok(Value::Expr({
                    let mut list = Vec::new();
                    for (i, value) in values.enumerate() {
                        let value = this.value_expr(value, span)?;
                        list.push(match i {
                            1 | 2 => this.debug_string(value, arg_tys[i].peel_refs(), span)?,
                            _ => value,
                        });
                    }
                    // A `None` message, `undefined`, is left out as preparation
                    // leaves out each trailing one.
                    Expr::call(Expr::helper("$assertFailed"), list)
                }))
            }),
            // A writer of the crate's own: what `write!` and `write_char` give
            // its `write_str` (ADR 0166).
            Std::UserWrite => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    let trait_id = tcx.trait_of_assoc(def_id).expect("`fmt::Write`'s method");
                    let write_str = trait_method(tcx, trait_id, "write_str");
                    this.flush(state, out)?;
                    this.trait_call(write_str, generic_args, exprs, span, out)?
                        .ok_or_else(|| this.unsupported(span, "this writer's `write_str`"))?
                }))
            }),
            // A writer of bytes of the crate's own: its dictionary's
            // `write_all` or `write_fmt`, given it in a box, as generic code
            // gives it (ADR 0392).
            Std::UserIoWrite => lower(move |this, _, values, _| {
                Ok(Value::Expr({
                    let mut exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    let trait_id = tcx.trait_of_assoc(def_id).expect("`io::Write`'s method");
                    let tr = ty::TraitRef::new_from_args(tcx, trait_id, generic_args);
                    let dictionary = this.dictionary(tr, span)?;
                    let receiver = exprs.remove(0);
                    let receiver = match this.mut_ref_form(tr.self_ty()).is_cell() {
                        true => receiver,
                        false => super::super::mut_refs::boxed(receiver),
                    };
                    exprs.insert(0, receiver);
                    let call = Expr::call(Expr::member(dictionary, bindings::fn_name(tcx, def_id)), exprs);
                    // std's `write_all` of the crate's own `write`, which may fail
                    // where any of the crate's may: its result a value (ADR 0462).
                    match this.krate.any_failing {
                        true => this.fmt_try(vec![StmtKind::Return(Some(call)).at(js::Span::NONE)]),
                        false => call,
                    }
                }))
            }),
            _ => return None,
        })
    }

    /// A string's or a `str`'s call: pushed to, sliced, edited in place
    /// (ADR 0063).
    pub(super) fn mir_text_call<'c>(
        &self,
        state: &State<'_, 'tcx>,
        call: StdCall<'c, 'tcx>,
    ) -> Option<StdLowering<'c, 'a, 'tcx>> {
        let this = self;
        let StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
        } = call;
        Some(match known {
            // `Display::fmt(&x, self.0)` of a `Formatter` held where it isn't
            // its writer's: what it writes, appended to the handle's text,
            // plain, as the handle keeps no options (ADR 0461).
            Std::FmtTo(operation) => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let shown = this.value_expr(values.next().expect("what's shown"), span)?;
                    let place = this
                        .text_place(values.next(), arg_tys[1])
                        .ok_or_else(|| this.unsupported(span, "this `Formatter`"))?;
                    let pretty = crate::lower::display::Pretty::Plain;
                    let written = this.written_of(operation, def_id, generic_args, vec![shown], &pretty, span, out)?;
                    this.flush(state, out)?;
                    crate::lower::display::append_written(&place, written, false, this.js_span(span), out);
                    Expr::undefined()
                }))
            }),
            // `s.push_str(t)`, `write!(s, ..)`: JS strings don't change, so `s`
            // gets a new one.
            Std::PushStr => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let place = this
                        .text_place(values.next(), arg_tys[0])
                        .ok_or_else(|| this.unsupported(span, "`push_str` on this, from its MIR"))?;
                    let given = values.next().expect("what's pushed");
                    // A `write!` of what may fail: piece by piece, what came before
                    // written, its `fmt::Result` the error, if any (ADR 0187); to a
                    // place reached through a call, `*s.as_mut().unwrap()`, held (#1011).
                    let fmt = matches!(given, Value::Fmt(Std::FmtNew, ..));
                    let place = match fmt && !fixed_place(&place) {
                        true => this.flush(state, out).map(|()| this.fixed_place_of(place, out))?,
                        false => place,
                    };
                    if fmt && fixed_place(&place) {
                        let value = this.value_expr(given, span)?;
                        this.flush(state, out)?;
                        let js_span = this.js_span(span);
                        let mut written = Vec::new();
                        crate::lower::display::append_written(&place, value, true, js_span, &mut written);
                        let caught = this.fresh("error");
                        let partial = Expr::call(Expr::helper("$fmtPartial"), vec![Expr::var(&caught)]);
                        let handler = vec![
                            StmtKind::Assign(place.clone(), Expr::bin(Op::Add, place.clone(), partial)).at(js_span),
                            StmtKind::Throw(Expr::var(&caught)).at(js_span),
                        ];
                        let tried = vec![StmtKind::TryCatch(written, Some(caught), handler).at(js_span)];
                        return Ok(Value::Expr(this.fmt_try(tried)));
                    }
                    let value = this.value_expr(given, span)?;
                    // `s.extend(items)`: their text, joined (ADR 0445).
                    let extended = (this.tcx.trait_of_assoc(def_id))
                        .is_some_and(|t| crate::lower::recognition::is_extend(this.tcx, t));
                    // Its hint asked first, as std's `Extend<char>` reserves by
                    // it (#849).
                    let value = match extended {
                        true => {
                            let into = arg_tys[0].peel_refs();
                            this.extended_reserved(value, into, arg_tys[1], place.clone())
                        }
                        false => value,
                    };
                    this.flush(state, out)?;
                    // Read and written: a place whose object is made here, a
                    // handle `push_mut` gives, `$pushMut(v, x).value`, held first,
                    // so it's made once.
                    let place = match place.kind {
                        js::ExprKind::Member(object, field) if !object.reads_same() => {
                            Expr::member(this.spill("value", *object, out), &field)
                        }
                        kind => Expr {
                            facts: crate::lower::iterators::its_facts(place.facts),
                            kind,
                            span: place.span,
                            held_as: Default::default(),
                        },
                    };
                    match extended {
                        true => this.extended_text(&place, value, arg_tys[1], span, out)?,
                        false => crate::lower::display::append_written(&place, value, false, this.js_span(span), out),
                    }
                    Expr::undefined()
                }))
            }),
            // `&v[a..b]`, `s.get(a..)`, `v.drain(..b)`: of the items and the range's
            // value. Not `&mut v[a..b]`, a view of them, not a copy.
            Std::Text(op @ (TextOp::Slice | TextOp::StrSlice | TextOp::StrGet | TextOp::SliceGet | TextOp::Drain))
                if !this
                    .tcx
                    .trait_of_assoc(def_id)
                    .is_some_and(|t| this.tcx.is_lang_item(t, LangItem::IndexMut)) =>
            {
                lower(move |this, state, values, out| {
                    Ok(Value::Expr({
                        let items = this.value_expr(values.next().expect("the items"), span)?;
                        let range = this.value_expr(values.next().expect("the range"), span)?;
                        // At places of the string in JS's units: `s.slice(a, b)` (ADR 0642).
                        match state.facts.offsets().call_in_units(state.at) {
                            true => this.sliced_in_units(items, (range, arg_tys[1]), span, out)?,
                            false => {
                                let in_bounds = this.indexes_in_bounds(def_id, span);
                                this.slice_range_values((op, in_bounds), items, (range, arg_tys[1]), span, out)?
                            }
                        }
                    }))
                })
            }
            // A strip whose `Option` is only tested and taken apart: its test
            // and its cut, of what holds its string and pattern there, or
            // of each held first (ADR 0642).
            Std::StripPrefix | Std::StripSuffix if let Some(cut) = state.facts.offsets().cut(state.at).cloned() => {
                lower(move |this, state, values, out| {
                    let holders = [cut.holders.clone(), cut.pattern_holders.clone()];
                    let units = cut.units;
                    let mut parts = Vec::new();
                    for (value, holders) in values.zip(holders) {
                        let part = this.value_expr(value, span)?;
                        // What reads the same wherever it's read: a constant, a
                        // part of plain data nothing changes, or what holds the
                        // string still where it's tested and cut.
                        let mut plain = part.reads_same();
                        part.visit_vars(&mut |name| plain &= this.plain_local(name));
                        let kept = plain
                            || matches!(&part.kind, js::ExprKind::Var(name)
                            if holders.iter().any(|&held| state.locals.names[held] == *name));
                        parts.push(match kept {
                            true => part,
                            false => {
                                this.flush(state, out)?;
                                this.spill("text", part, out)
                            }
                        });
                    }
                    let [s, p]: [Expr; 2] = parts.try_into().ok().expect("a string and a pattern");
                    let (test, cut) = crate::lower::std_types::text::cut(s, p, units, known == Std::StripPrefix);
                    Ok(Value::Cut(test, cut, VariantIdx::from_u32(1)))
                })
            }
            // A part of a string as a `&mut str` (ADR 0334), of the cell its
            // `&mut` is.
            Std::Text(op @ (TextOp::StrPart { .. } | TextOp::StrGetMut | TextOp::StrSplitAtMut { .. })) => {
                lower(move |this, _, values, out| {
                    Ok(Value::Expr({
                        let cell = this.value_expr(values.next().expect("the string"), span)?;
                        let mut given = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                        if matches!(op, TextOp::StrPart { bounds: false } | TextOp::StrGetMut) {
                            let range = given.remove(0);
                            let (start, end) = this.range_value_bounds(range, arg_tys[1], span, out)?;
                            given = std::iter::once(start).chain(end).collect();
                        }
                        this.str_part_values(op, cell, given)
                    }))
                })
            }
            // A `String`'s room asked, which std checks (ADR 0774), of its
            // text where it is.
            Std::Text(TextOp::Reserve) => lower(move |this, _, values, _| {
                Ok(Value::Expr({
                    let text = match values.next().expect("the string") {
                        Value::Ref(place) => place,
                        cell => Expr::held(this.value_expr(cell, span)?),
                    };
                    let n = this.value_expr(values.next().expect("the room"), span)?;
                    Expr::call(Expr::helper("$textRoomFor"), vec![text, n])
                }))
            }),
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
                lower(move |this, state, values, out| {
                    Ok(Value::Expr({
                        let exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                        // Of places in JS's units: JS's own (ADR 0642).
                        match state.facts.offsets().call_in_units(state.at) {
                            true => crate::lower::std_types::text::of_places(op, exprs).ok_or_else(|| {
                                this.unsupported(span, "this string's method, of places in JS's units")
                            })?,
                            false => this.text_values(op, exprs, arg_tys, generic_args, span, out)?,
                        }
                    }))
                })
            }
            // `s.insert(i, c)` and the rest: `s` given its new text; what it
            // took out, its value.
            // A `Vec`'s range replaced, `splice`, or pushed again,
            // `extend_from_within`: `range_edit`.
            Std::Text(op @ (TextOp::Splice | TextOp::ExtendFromWithin)) => lower(move |this, _, values, out| {
                Ok(Value::Expr({
                    let mut exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    let mut rest = exprs.split_off(2);
                    let [items, range]: [Expr; 2] = exprs.try_into().ok().expect("the items and a range");
                    // `splice`'s replacement, an iterable, the crate's own
                    // iterator stepped by its `next`; of one that holds what
                    // drops, a JS iterator of it closed to drop it; and what
                    // drops each item, which the `Splice` owns of what it
                    // took out (#1147, #1149).
                    if op == TextOp::Splice {
                        let replacement = rest.pop().expect("the replacement");
                        let items = this.iterable_items(replacement, arg_tys[2], span, out)?;
                        rest.push(match this.closing_drop(arg_tys[2], span)? {
                            Some(drop) => {
                                let it = this.js_iterator(items);
                                this.closed_by(it, drop, arg_tys[2])
                            }
                            None => items,
                        });
                        let item = this.slice_item(arg_tys[0]).expect("a `Vec`'s items");
                        rest.extend(this.std_drop_function(item, span)?);
                    }
                    // A `Splice` whose drop runs nothing, of an array's items
                    // that have no destructor, put in place where it's made,
                    // `$splice`; else std's, `$splicing` (#1189).
                    let lazy = op == TextOp::Splice && this.drops(output) != crate::lower::drops::Drops::Nothing;
                    let range =
                        this.slice_range_values((range_read(op), false), items, (range, arg_tys[1]), span, out)?;
                    let clone = match this.slice_item(arg_tys[0]) {
                        Some(item) if op == TextOp::ExtendFromWithin && this.needs_clone(item) => {
                            Some(this.clone_arg(item, span)?)
                        }
                        _ => None,
                    };
                    this.range_edit(op, range, rest, (clone, lazy))
                }))
            }),
            Std::StringEdit(edit) => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let place = match values.next().expect("the string") {
                        Value::Ref(place) if fixed_place(&place) => place,
                        // `s[a..b].make_ascii_uppercase()`: `s` given its new text,
                        // that part changed, `s = $asciiCase(s, true, a,
                        // b)` (ADR 0334).
                        Value::Ref(ref place)
                            if let StringEdit::AsciiCase { upper } = edit
                                && let Some((whole, mut bounds)) = str_part_of(place) =>
                        {
                            this.flush(state, out)?;
                            if bounds.first().and_then(Expr::as_int) == Some(0) && bounds.len() == 1 {
                                bounds.clear();
                            }
                            let mut list = vec![whole.clone(), Expr::bool(upper)];
                            list.extend(bounds);
                            let edited = Expr::call(Expr::helper("$asciiCase"), list);
                            out.push(StmtKind::Assign(whole, edited).at(this.js_span(span)));
                            return Ok(Value::Expr(Expr::undefined()));
                        }
                        // A part another call gives, `$strGetMut(..)`'s: held,
                        // its `value` the place.
                        Value::Ref(ref place)
                            if let js::ExprKind::Member(cell, field) = &place.kind
                                && field == js::HELD =>
                        {
                            this.flush(state, out)?;
                            let cell = this.spill("part", (**cell).clone(), out);
                            Expr::held(cell)
                        }
                        _ => return Err(this.unsupported(span, "changing this string, from its MIR")),
                    };
                    let mut given = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    // A path's, given as an `AsRef<Path>` or `AsRef<OsStr>`.
                    if matches!(
                        edit,
                        StringEdit::PathPush | StringEdit::SetFileName | StringEdit::SetExtension
                    ) {
                        given = (given.into_iter().zip(&arg_tys[1..]))
                            .map(|(value, &ty)| this.path_given(value, ty))
                            .collect();
                    }
                    if matches!(
                        edit,
                        StringEdit::Drain | StringEdit::ReplaceRange | StringEdit::ExtendFromWithin
                    ) {
                        let range = given.remove(0);
                        let (start, end) = this.range_value_bounds(range, arg_tys[1], span, out)?;
                        given.splice(0..0, [start, end.unwrap_or_else(Expr::undefined)]);
                    }
                    this.flush(state, out)?;
                    let (edited, taken) = this.string_edit_values(edit, place.clone(), given, out);
                    out.push(StmtKind::Assign(place, edited).at(this.js_span(span)));
                    taken.unwrap_or_else(Expr::undefined)
                }))
            }),
            // Its capacity is the engine's, but std checks the room it's
            // asked, of bytes (ADR 0774).
            Std::StringWithCapacity => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let mut n = this.value_expr(values.next().expect("the capacity"), span)?;
                    this.flush(state, out)?;
                    if !n.reads_same() {
                        n = this.spill("n", n, out);
                    }
                    this.room_for((None, n), Expr::int(1), span, out);
                    Expr::str("")
                }))
            }),
            _ => return None,
        })
    }

    /// A map's or a set's call of its places: a range of a tree, what's
    /// extracted, an entry (ADR 0059).
    pub(super) fn mir_map_call<'c>(
        &self,
        _state: &State<'_, 'tcx>,
        call: StdCall<'c, 'tcx>,
    ) -> Option<StdLowering<'c, 'a, 'tcx>> {
        let StdCall {
            known, arg_tys, span, ..
        } = call;
        Some(match known {
            // A B-tree's range of its keys (ADR 0325).
            Std::Map(MapOp::TreeRange { mutable }) => lower(move |this, _, values, out| {
                Ok(Value::Expr({
                    let [m, range]: [Value<'tcx>; 2] =
                        values.collect::<Vec<_>>().try_into().ok().expect("a tree and a range");
                    let (m, range) = (this.value_expr(m, span)?, this.value_expr(range, span)?);
                    this.tree_range_values((m, arg_tys[0]), (range, arg_tys[1]), mutable, span, out)?
                }))
            }),
            // A map's or a set's `extract_if`, a B-tree's of a range.
            Std::Map(MapOp::ExtractIf) => lower(move |this, _, values, out| {
                Ok(Value::Expr({
                    let mut exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    let f = exprs.pop().expect("the test");
                    let m = exprs.remove(0);
                    let range = match this.is_sorted(arg_tys[0].peel_refs()) {
                        true => Some((exprs.remove(0), arg_tys[1])),
                        false => None,
                    };
                    let f_ty = *arg_tys.last().expect("the test's type");
                    this.map_extract_if_values((m, arg_tys[0]), range, (f, f_ty), span, out)?
                }))
            }),
            _ => return None,
        })
    }

    /// A `Vec`'s, a slice's, a `VecDeque`'s or a heap's call of its places:
    /// sorted, split, viewed, written (ADRs 0025, 0324).
    pub(super) fn mir_vec_call<'c>(
        &self,
        state: &State<'_, 'tcx>,
        call: StdCall<'c, 'tcx>,
    ) -> Option<StdLowering<'c, 'a, 'tcx>> {
        let this = self;
        let StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
        } = call;
        let tcx = this.tcx;
        Some(match known {
            // `vec![..]`: the array its box was written with.
            Std::VecMacro => lower(move |this, _, values, _| Ok(Value::Expr(this.value_expr(values.next().expect("the array"), span)?))),
            // A deque's `range_mut(a..b)`: a view of those items, each a box
            // where it's one JS can't change in place.
            Std::Slice(SliceOp::DequeRangeMut) => lower(move |this, _, values, out| Ok(Value::Expr({
                let items = this.value_expr(values.next().expect("the items"), span)?;
                let range = this.value_expr(values.next().expect("the range"), span)?;
                let (start, end) = this.range_value_bounds(range, arg_tys[1], span, out)?;
                let mut list = vec![items, start];
                list.extend(end);
                let view = this.view_call("$view", list);
                match this.mut_ref_form(generic_args.type_at(0)).is_cell() {
                    true => Expr::call_helper("$mutItems", vec![view]),
                    false => view,
                }
            }))),
            // `v.sort()`, `v.sort_by_key(key)`: in place.
            Std::Sort | Std::SortByKey => lower(move |this, _, values, out| Ok(Value::Expr({
                let items = this.value_expr(values.next().expect("the items"), span)?;
                let key = match known {
                    Std::SortByKey => Some(this.value_expr(values.next().expect("the key"), span)?),
                    _ => None,
                };
                this.sort_values(key, items, arg_tys[0], generic_args, span, out)?
            }))),
            // `v.sort_unstable_by(f)` and a selection (ADR 0342).
            Std::Slice(op @ (SliceOp::SortUnstable { with } | SliceOp::SelectNth { with })) => lower(move |this, _, values, out| Ok(Value::Expr({
                let values = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                this.unstable_sort_values(op, with, values, generic_args, span, out)?
            }))),
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
                })) => lower(move |this, _, values, out| Ok(Value::Expr({
                let values = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                this.slice_values(op, values, false, generic_args, span, out)?
            }))),
            // A chunks' `remainder()`: from MIR, which steps a std iterator as
            // a `$iter` (`std_iterator`), its items'; of one made there, the
            // array's.
            Std::Slice(SliceOp::Remainder) => lower(move |this, _, values, out| Ok(Value::Expr({
                let mut exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
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
                this.slice_values(SliceOp::Remainder, exprs, stepping, generic_args, span, out)?
            }))),
            // `v.copy_within(a..b, at)` and `v.extract_if(range, f)`: of the
            // range's bounds.
            Std::Slice(SliceOp::CopyWithin) => lower(move |this, _, values, out| Ok(Value::Expr({
                let items = this.value_expr(values.next().expect("the items"), span)?;
                let range = this.value_expr(values.next().expect("the range"), span)?;
                let bounds = this.range_value_bounds(range, arg_tys[1], span, out)?;
                let dest = this.value_expr(values.next().expect("where to"), span)?;
                this.copy_within_values(items, bounds, dest)
            }))),
            Std::Slice(SliceOp::ExtractIf { range }) => lower(move |this, _, values, out| Ok(Value::Expr({
                let mut exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let f = exprs.pop().expect("the test");
                let items = exprs.remove(0);
                let bounds = match range {
                    true => this.range_value_bounds(exprs.remove(0), arg_tys[1], span, out)?,
                    false => (Expr::int(0), None),
                };
                this.extract_if_values(items, bounds, f, generic_args.type_at(0))
            }))),
            // A slice of `MaybeUninit`s written from a slice, and each item
            // dropped (ADR 0332).
            Std::Slice(SliceOp::UninitWrite { clone }) => lower(move |this, state, values, out| Ok(Value::Expr({
                let values = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                this.flush(state, out)?;
                this.uninit_write_values(clone, values, generic_args.type_at(0), span, out)?
            }))),
            Std::Slice(SliceOp::UninitDrop) => lower(move |this, state, values, out| Ok(Value::Expr({
                let slots = this.value_expr(values.next().expect("the slots"), span)?;
                let ty = Ty::new_slice(tcx, generic_args.type_at(0));
                this.flush(state, out)?;
                this.drop_value(slots, ty, span, out)?;
                Expr::undefined()
            }))),
            // `s.split_off(..n)` and the like: the slice's place given what's
            // left (ADR 0338).
            Std::Slice(SliceOp::SplitOff { end, mutable }) => lower(move |this, state, values, out| Ok(Value::Expr({
                // Of a `&mut [T]`, which a `&mut` to isn't a cell: the
                // variable `rest.split_off_mut(..)` borrows, not one holding a
                // `&mut` to it, which JS would assign instead.
                let place = match values.next().expect("the slice") {
                    Value::Ref(place) if fixed_place(&place) => place,
                    Value::Expr(place)
                        if matches!(place.kind, js::ExprKind::Var(_)) && this.borrows_variable(def_id, span, 0) =>
                    {
                        place
                    }
                    _ => return Err(this.unsupported(span, "splitting this slice")),
                };
                let bounds = match end {
                    None => {
                        let range = this.value_expr(values.next().expect("the range"), span)?;
                        Some(this.range_value_bounds(range, arg_tys[1], span, out)?)
                    }
                    Some(_) => None,
                };
                this.flush(state, out)?;
                let split =
                    this.split_off_values((end, mutable), place.clone(), bounds, generic_args.type_at(0), out);
                let left = Expr::index(split.clone(), Expr::int(0));
                out.push(StmtKind::Assign(place, left).at(this.js_span(span)));
                Expr::index(split, Expr::int(1))
            }))),
            // `&mut v[a..b]`, `get_mut(a..b)`: a view of those items (ADR 0335),
            // an unchecked one kept unmade until it's used (`Value::View`).
            Std::Slice(SliceOp::View { checked }) => lower(move |this, _, values, out| {
                let items = this.value_expr(values.next().expect("the items"), span)?;
                let range = this.value_expr(values.next().expect("the range"), span)?;
                let (start, end) = this.range_value_bounds(range, arg_tys[1], span, out)?;
                // `&mut v[..]`: all of it, which is `v`.
                if !checked && start.as_int() == Some(0) && end.is_none() {
                    return Ok(Value::Expr(items));
                }
                let list = [vec![items, start], end.into_iter().collect()].concat();
                Ok(if checked { Value::Expr(this.view_call("$viewGet", list)) } else { Value::View(list, js::Span::NONE) })
            }),
            // `vec![item; n]`: each a clone, of what it can't remake.
            // A count rustc knows, a constant's read by its name, is known
            // to fit where it does (ADR 0774, amended).
            Std::FromElem => {
                let known = match &state.body[state.at].terminator().kind {
                    TerminatorKind::Call { args, .. } => (args.get(1).and_then(|arg| arg.node.constant()))
                        .and_then(|count| count.const_.try_eval_target_usize(tcx, this.typing_env)),
                    _ => None,
                };
                lower(move |this, _, values, out| Ok(Value::Expr({
                    let item = this.value_expr(values.next().expect("the item"), span)?;
                    let n = this.value_expr(values.next().expect("the count"), span)?;
                    this.copies_values((item, arg_tys[0]), (n, known), span, out)?
                })))
            }
            // A `BinaryHeap`'s, and its `peek_mut()` guard's; a `pop()`'s
            // `Some` boxed where its item could look like `None` (ADR 0051).
            Std::Heap(op) => lower(move |this, _, values, _| Ok(Value::Expr({
                let exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let boxed = this.option_of(output).is_some_and(|item| this.boxed_payload(item));
                this.heap_values(op, exprs, (arg_tys[0], boxed), span)?
            }))),
            // What was there, taken out.
            Std::DequeRemove => lower(move |this, _, values, _| Ok(Value::Expr({
                let exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                Expr::call_helper("$removeOpt", exprs)
            }))),
            _ => return None,
        })
    }

    /// What changes a place: `mem::swap` and `replace`, an `Option`'s
    /// `take`, an `Rc`'s `make_mut`, a `Cow`'s, a `OnceCell`'s `set`
    /// (ADRs 0147, 0319, 0320, 0847).
    pub(super) fn mir_place_call<'c>(
        &self,
        state: &State<'_, 'tcx>,
        call: StdCall<'c, 'tcx>,
    ) -> Option<StdLowering<'c, 'a, 'tcx>> {
        let StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
            ..
        } = call;
        Some(match known {
            Std::Swap | Std::Replace | Std::MemTake | Std::OptionTake | Std::OptionReplace => {
                lower(move |this, state, values, out| {
                    let values: Vec<Value<'tcx>> = values.collect();
                    this.mir_swap_or_replace(state, (known, def_id), values, arg_tys, span, out)
                })
            }
            // `x.to_owned()` of a `T: Clone`: `x.clone()`'s copy.
            Std::ToOwnedClone => lower(move |this, _, values, out| {
                Ok(Value::Expr({
                    let value = this.value_expr(values.next().expect("the value"), span)?;
                    this.clone_value(value, arg_tys[0].peel_refs(), span, out)?
                }))
            }),
            // `dest.clone_from(&source)`: `mem::replace` of `dest` with a clone
            // of `source`, written as it writes, what it gives back dropped,
            // as `*dest = source.clone()` drops it.
            // std's own, where its order of clones and drops is seen, a
            // `Vec`'s, a `VecDeque`'s, a hashed map's, an array's, a `Box`'s
            // of one: `clone_into`'s, in place, `$cloneIntoVec(dest, source,
            // ..)` (#919, #922); an `Option`'s of one, its value's where both
            // are `Some`.
            Std::CloneFrom => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let ty = arg_tys[1].peel_refs();
                    let dest = values.next().expect("the destination");
                    let source = values.next().expect("the source");
                    let source = this.value_expr(source, span)?;
                    if let Some(into) = this.clone_into(ty, span)? {
                        let dest = this.dest_place(&dest, ty, span)?;
                        this.flush(state, out)?;
                        out.push(StmtKind::Expr(into.call(dest, source)).at(this.js_span(span)));
                        return Ok(Value::Expr(Expr::undefined()));
                    }
                    // Of an `Option`, its value's, where both are `Some`; of a
                    // value only the caller knows, a `T` or a `Box<T>`, or an
                    // `Option` of one, its dictionary's `clone_from`, where
                    // the caller's type has one, asked first (#928): given
                    // the box generic code has of `dest`, or one of its place
                    // here, written back.
                    let inner = this.option_of(ty).filter(|&inner| !this.boxed_payload(inner));
                    let to = this.dest_place(&dest, ty, span)?;
                    let given_box = match &dest {
                        Value::Ref(place) => this.box_of_place(state, place),
                        _ => Some(this.value_expr(dest.clone(), span)?),
                    };
                    let made = given_box.is_none().then(|| this.fresh("to"));
                    let pointer = given_box.unwrap_or_else(|| Expr::var(made.as_deref().expect("a box made")));
                    let held = inner.unwrap_or(ty);
                    if this
                        .written_over(held, (to.clone(), pointer.clone()), source.clone(), span)?
                        .is_some()
                    {
                        this.flush(state, out)?;
                        // Read again by what's asked and by the clone.
                        let source = match source.reads_same() {
                            true => source,
                            false => this.spill("source", source, out),
                        };
                        let (asked, written) = this
                            .written_over(held, (to.clone(), pointer), source.clone(), span)?
                            .expect("written over, as asked above");
                        // The box made of the place, where a dictionary's is
                        // given it, not `clone_into`'s, which writes the place.
                        let made = made.filter(|_| asked.is_some());
                        let some = |e: &Expr| Expr::bin(Op::LooseNe, e.clone(), Expr::null());
                        let both = inner.map(|_| Expr::bin(Op::And, some(&to), some(&source)));
                        let test = match (both, asked) {
                            (Some(both), Some(asked)) => Expr::bin(Op::And, both, asked),
                            (Some(test), None) | (None, Some(test)) => test,
                            (None, None) => unreachable!("`clone_into`'s is written above"),
                        };
                        let mut then = Vec::new();
                        if let Some(name) = &made {
                            then.push(
                                StmtKind::Const(name.clone(), crate::lower::mut_refs::boxed(to.clone()))
                                    .at(this.js_span(span)),
                            );
                        }
                        then.push(StmtKind::Expr(written).at(this.js_span(span)));
                        if let Some(name) = &made {
                            then.push(
                                StmtKind::Assign(to.clone(), Expr::member(Expr::var(name), js::HELD))
                                    .at(this.js_span(span)),
                            );
                        }
                        let mut replaced = Vec::new();
                        this.replace_with_clone(state, (dest, source), (ty, def_id), arg_tys, span, &mut replaced)?;
                        out.push(StmtKind::If(test, then, Some(replaced)).at(this.js_span(span)));
                        return Ok(Value::Expr(Expr::undefined()));
                    }
                    if this.forwards_clone_from(ty, span)? {
                        return Err(this.unsupported(
                            span,
                            &format!("`clone_from` of `{ty}`, whose std `clone_from` is its value's, which the crate's or std's own writes in place (#927): write the value's `clone_from` where both hold one, `match (&mut *dest, source) {{ (Some(to), Some(from)) => to.clone_from(from), _ => *dest = source.clone() }}`"),
                        ));
                    }
                    this.replace_with_clone(state, (dest, source), (ty, def_id), arg_tys, span, out)?;
                    Expr::undefined()
                }))
            }),
            // `Rc::make_mut(&mut rc)` of a local `Rc` of this body, or one a
            // `&mut` reaches, its place's box (ADR 0523): the place given the
            // one `$makeMut` gives. Through a shared reference the place isn't
            // here, which a variable given another would leave.
            Std::Rc(RcOp::MakeMut) => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    // `&mut rc` of a place: the place itself, not a box for it.
                    let place = match values.next().expect("the `Rc`") {
                        Value::Ref(place) => place,
                        value => this.value_expr(value, span)?,
                    };
                    let item = this.counted_rc(arg_tys[0].peel_refs());
                    // Of a variable of this body or a field of one, `copy.pages`.
                    let local = match root_var(&place) {
                        Some(name) if fixed_place(&place) => {
                            state.locals.names.iter_enumerated().find(|(_, n)| *n == name)
                        }
                        _ => None,
                    };
                    let (Some(item), Some((local, _))) = (item, local) else {
                        return Err(this.unsupported(span, "`Rc::make_mut` of an `Rc` that isn't a place here"));
                    };
                    let local_ty = state.body.local_decls[local].ty;
                    let boxed = matches!(local_ty.kind(), ty::Ref(_, rc, ty::Mutability::Mut) if this.counted_rc(*rc).is_some());
                    if !boxed && (state.body.local_kind(local) == LocalKind::Arg || local_ty.is_ref()) {
                        return Err(this.unsupported(span, "`Rc::make_mut` of an `Rc` that isn't a place here"));
                    }
                    this.flush(state, out)?;
                    this.make_mut_values(place, item, span, out)?
                }))
            }),
            // An `Rc`'s or a `Weak`'s, but `make_mut`, which writes its place.
            Std::Rc(op) if op != RcOp::MakeMut => lower(move |this, _, values, out| {
                Ok(Value::Expr({
                    let exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    this.rc_values(op, (exprs, arg_tys, output), span, out)?
                }))
            }),
            // A `Cow`'s `into_owned()`, its deref, and its `to_mut()` of what's
            // its own place.
            Std::Cow(op) => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let cow = match (op, values.next().expect("the `Cow`")) {
                        (CowOp::ToMut, Value::Ref(place) | Value::Expr(place)) if fixed_place(&place) => place,
                        (CowOp::ToMut, _) => {
                            return Err(this.unsupported(span, "`to_mut()` of this `Cow`, from its MIR"));
                        }
                        (_, value) => this.value_expr(value, span)?,
                    };
                    this.flush(state, out)?;
                    let cow_ty = match op {
                        CowOp::From { .. } => output,
                        _ => arg_tys[0].peel_refs(),
                    };
                    this.cow_values(op, cow, cow_ty, span, out)?
                }))
            }),
            // `o.get_or_insert(x)` and the like: the `Option`'s place given its
            // value where it has none, and a `&mut` to what it holds.
            Std::OptionPlace(op) => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let place = match values.next().expect("the `Option`") {
                        Value::Ref(place) if fixed_place(&place) => place,
                        _ => return Err(this.unsupported(span, "changing this `Option`, from its MIR")),
                    };
                    let item = this.option_of(arg_tys[0].peel_refs()).expect("an `Option`");
                    let given = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    this.flush(state, out)?;
                    this.option_place_values(op, place, item, (given, &arg_tys[1..]), span, out)?
                }))
            }),
            // A `OnceCell`'s `set`: of a cell the body's ranges show still
            // empty there, and of a `Result` read or not (ADR 0847).
            Std::Once(OnceOp::Set) => {
                let empty =
                    (self.thir_call(def_id, span)).is_some_and(|(fun, _)| self.frame.facts.set_empty.contains(&fun));
                let read = !self.result_unread(state);
                let item = generic_args.type_at(0);
                lower(move |this, state, values, out| {
                    let cell = this.value_expr(values.next().expect("the cell"), span)?;
                    let value = this.value_expr(values.next().expect("what it's set to"), span)?;
                    this.flush(state, out)?;
                    Ok(Value::Expr(this.once_set(
                        (cell, value),
                        item,
                        (empty, read),
                        span,
                        out,
                    )))
                })
            }
            _ => return None,
        })
    }

    /// A value's call: a number's, JSON's, a type's name, a pointer's
    /// equality (ADRs 0064, 0077).
    pub(super) fn mir_value_call<'c>(
        &self,
        state: &State<'_, 'tcx>,
        call: StdCall<'c, 'tcx>,
    ) -> Option<StdLowering<'c, 'a, 'tcx>> {
        let StdCall {
            known,
            generic_args,
            arg_tys,
            output,
            span,
            ..
        } = call;
        Some(match known {
            // An integer's `try_from` whose `Result` is only tested and its
            // `Ok` taken: its range test and the number (#366).
            Std::TryFromInt { into } if state.facts.tried_int(state.at) => {
                let (from, to) = self.tried_int_types(generic_args.type_at(usize::from(into)), arg_tys[0])?;
                lower(move |this, state, values, out| this.tried_int(state, values, (from, to), span, out))
            }
            // `serde_json::to_string(&v)` and `from_str(s)` (ADR 0081).
            Std::ToJson(pretty) => lower(move |this, _, values, _| {
                Ok(Value::Expr({
                    let value = this.value_expr(values.next().expect("the value"), span)?;
                    let ty = generic_args.types().next().expect("`to_string::<T>`");
                    this.json_text(value, ty, pretty, span)?
                }))
            }),
            Std::FromJson => lower(move |this, _, values, _| {
                Ok(Value::Expr({
                    let text = this.value_expr(values.next().expect("the text"), span)?;
                    let ty = generic_args.types().next().expect("`from_str::<T>`");
                    this.json_value(text, ty, span)?
                }))
            }),
            Std::NonZeroNew => lower(move |this, _, values, out| {
                Ok(Value::Expr({
                    let n = this.value_expr(values.next().expect("the number"), span)?;
                    this.non_zero_new(n, arg_tys[0], span, out)?
                }))
            }),
            // A `Wrapping`'s operator, `a * Wrapping(2)`, `w += x`: assigned
            // to the place itself, or through a `&mut` to it a variable holds,
            // a parameter's or MIR's borrow taken before an operand that's a
            // call, `w += x << 4`'s, as any `*r = v` is written (`assign`).
            Std::WrappingOp(op, assign) => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let (target, through) = match (assign, values.next().expect("the `Wrapping`")) {
                        // Through a variable the program wrote that holds a
                        // `&mut`, what it refers to is written in place. Not
                        // MIR's own `&mut b0` of `b0 += ..`: the variable it's
                        // of, rebound, so a `Copy` value's copies stay what
                        // they were (#396).
                        (true, Value::Ref(place) | Value::Expr(place)) if fixed_place(&place) => {
                            let local = match &place.kind {
                                js::ExprKind::Var(name) => (state.locals.names.iter_enumerated())
                                    .find(|(l, n)| *n == name && state.body.local_decls[*l].ty.is_ref())
                                    .map(|(l, _)| l),
                                _ => None,
                            };
                            let own = local.and_then(|l| super::queries::own_borrow_of_a_variable(state, l));
                            if let Some(of) = own.map(|of| Expr::var(&state.locals.names[of])) {
                                (of, false)
                            } else {
                                let reference = local.is_some();
                                if reference && !this.mut_ref_form(arg_tys[0].peel_refs()).written_in_place() {
                                    return Err(this.unsupported(span, "assigning a whole value through a `&mut`"));
                                }
                                (place, reference)
                            }
                        }
                        (true, _) => return Err(this.unsupported(span, "this assignment, from its MIR")),
                        (false, value) => (this.value_expr(value, span)?, false),
                    };
                    let target = match op.is_ok() && !target.reads_same() {
                        true => {
                            this.flush(state, out)?;
                            this.spill("wrapping", target, out)
                        }
                        false => target,
                    };
                    let rhs = values.next().map(|v| this.value_expr(v, span)).transpose()?;
                    this.flush(state, out)?;
                    let value = this.wrapping_value(op, arg_tys[0].peel_refs(), target.clone(), rhs, span)?;
                    if !assign {
                        value
                    } else {
                        let js_span = this.js_span(span);
                        out.push(match through {
                            true => StmtKind::Expr(Expr::call_helper("$assign", vec![target, value])).at(js_span),
                            false => StmtKind::Assign(target, value).at(js_span),
                        });
                        Expr::undefined()
                    }
                }))
            }),
            // rustc's name for the type; of a type parameter, the one its
            // caller gave (ADR 0145). What `type_name_of_val` is given runs.
            Std::TypeName { .. } => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    for value in values {
                        let value = this.value_expr(value, span)?;
                        if value.has_effects() {
                            this.flush(state, out)?;
                            out.push(StmtKind::Expr(value).at(this.js_span(span)));
                        }
                    }
                    let ty = generic_args.type_at(0);
                    this.type_fact_value(ty, crate::lower::recognition::TypeFact::Name, span)?
                }))
            }),
            // `x.to_int_unchecked()`: the cast `as` is, where it's in range.
            Std::Number(NumOp::ToIntUnchecked) => lower(move |this, _, values, _| {
                Ok(Value::Expr({
                    let value = this.value_expr(values.next().expect("the float"), span)?;
                    this.cast(value, arg_tys[0], output, span)?
                }))
            }),
            // Whether two JS objects are one, `a === b` (ADR 0285); of a Rust
            // value, refused before (`mir_function`).
            // Of two pointers to Rust's values, handles on places, by place, as
            // `==` of them is (ADR 0562).
            Std::PtrEq => lower(move |this, _, values, _| {
                Ok(Value::Expr({
                    let mut exprs = values
                        .map(|v| this.value_expr(v, span))
                        .collect::<R<Vec<_>>>()?
                        .into_iter();
                    let (a, b) = (exprs.next().expect("a"), exprs.next().expect("b"));
                    match crate::lower::pointers::raw_pointee(this.tcx, arg_tys[0]) {
                        Some((pointee, _)) if this.compared_by_address(arg_tys[0]) => {
                            this.pointers_equal(a, b, pointee, span)?
                        }
                        _ => Expr::bin(Op::Eq, a, b),
                    }
                }))
            }),
            Std::Number(op) => lower(move |this, _, values, out| {
                Ok(Value::Expr({
                    // `i32::from_str_radix(s, 16)`'s is what its `Result` holds.
                    let ty = match (op, output.kind()) {
                        (NumOp::FromStrRadix, ty::Adt(_, result)) => result.type_at(0),
                        (
                            NumOp::FromBytes { .. }
                            | NumOp::FloatFromBytes { .. }
                            | NumOp::FromBits
                            | NumOp::Bound { .. },
                            _,
                        ) => output,
                        _ => arg_tys.first().copied().map(|t| t.peel_refs()).expect("a number"),
                    };
                    let exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    this.number_call(op, exprs, ty, span, out)?
                }))
            }),
            _ => return None,
        })
    }
}
