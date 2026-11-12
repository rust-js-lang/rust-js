//! A std call rust-js knows, given what MIR made of its arguments: the
//! value it gives as its output is held, and the one kind of
//! `std_calls`' that takes it. Split from `dispatch.rs` by concept
//! (#1309).

use super::*;
use crate::lower::temporary_names::Role;

/// One kind of std call's handler, `std_calls`': of what it reads alone, the
/// call's lowering where it takes the call, else `None` (#191).
type StdHandler<'a, 'tcx> = for<'s, 'c> fn(
    &FnCx<'a, 'tcx>,
    &State<'s, 'tcx>,
    std_calls::StdCall<'c, 'tcx>,
) -> Option<std_calls::StdLowering<'c, 'a, 'tcx>>;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A std call, `mir_std_call`'s, its value as its `output` is held,
    /// `std_value_as`'s.
    pub(super) fn mir_std_value(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (known, def_id, generic_args): (Std, rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        ((arg_tys, nulled), mut values, output): ((&[Ty<'tcx>], &[bool]), Vec<Value<'tcx>>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        // A view of part of a slice given alone to what has a form of its
        // own over a range of the slice, `v[a..b].reverse()`: that form, of
        // the slice and the range, `$reverseRange(v, a, b)`, no view made
        // (ADR 1073).
        if let [Value::View(..)] = values.as_slice()
            && let Some(helper) = crate::lower::std_types::slice::ranged(known)
            && let Some(Value::View(list, at)) = values.pop()
        {
            let ranged = Value::Expr(self.view_call(helper, list).or_at(at));
            return Ok(self.std_value_as(ranged, output, arg_tys));
        }
        // An iterator it takes by value, dropped after it (`consumed_by_std`).
        let std = (known, def_id, generic_args);
        if let Some(value) = self.consumed_by_std(state, std, (arg_tys, nulled), &mut values, output, span, out)? {
            return Ok(self.std_value_as(value, output, arg_tys));
        }
        let given = self.call_given(state, nulled);
        let value = self.grown_std_call(
            state,
            (known, def_id),
            generic_args,
            (arg_tys, given),
            values,
            output,
            span,
            out,
        )?;
        let value = self.write_value(state, (known, output), value, span, out)?;
        // What a place held, given back as it was held (#1238).
        if known.gives_back_held() {
            return Ok(value);
        }
        Ok(self.std_value_as(value, output, arg_tys))
    }

    /// A std function of its arguments' values, lowered once they're values
    /// (`std_values`).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower) fn std_by_values(
        &mut self,
        known: Std,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        (arg_tys, given): (&[Ty<'tcx>], crate::lower::calls::Given<'_>),
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let boxed = self.option_of(output).is_some_and(|inner| self.boxed_payload(inner));
        let call = crate::lower::calls::Call::given((def_id, generic_args), arg_tys, given, span);
        self.std_values(known, call, values, boxed, output, out)
    }

    /// A `Vec` the call of `def_id` may shorten, where capacity's kept (ADR
    /// 0419): what it has kept first, as Rust's shortening leaves its
    /// capacity, `$keepCapacity(v)`.
    fn keep_capacity(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        values: &mut [Value<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        if self.krate.keeps_capacity
            && let Some(at) = crate::lower::recognition::shortens_vec(self.tcx, def_id)
            && let Some(vector) = values.get(at).cloned()
        {
            let vector = self.value_expr(vector, span)?;
            self.flush(state, out)?;
            let vector = match vector.reads_same() {
                true => vector,
                false => self.spill(Role::Items, vector, out),
            };
            let kept = Expr::call(Expr::helper("$keepCapacity"), vec![vector.clone()]);
            out.push(StmtKind::Expr(kept).at(self.js_span(span)));
            values[at] = Value::Expr(vector);
        }
        Ok(())
    }

    /// `o.as_mut()` of an `Option`, `receiver`, of what a `&mut` to is a
    /// cell, a number or a `Box`: a handle on its place where it's `Some`; of
    /// a generic `T`'s, on what its `Some` holds (ADR 0051). Its place the
    /// `Option`'s, or through a handle a call gave on it, `child_mut(side)`'s,
    /// of type `given`, that handle's `value`, the handle held (#323). None
    /// of another receiver.
    fn option_as_mut(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (receiver, given): (Ty<'tcx>, Ty<'tcx>),
        values: &[Value<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Some(item) = self.option_of(receiver) else {
            return Ok(None);
        };
        let place = match values {
            [Value::Ref(place)] if fixed_place(place) => place.clone(),
            [handle] if self.is_cell(given) => {
                let handle = self.value_expr(handle.clone(), span)?;
                let handle = match handle.reads_same() {
                    true => handle,
                    false => {
                        self.flush(state, out)?;
                        self.spill(Role::Handle, handle, out)
                    }
                };
                Expr::held(handle)
            }
            _ => return Ok(None),
        };
        let none = Expr::bin(js::Op::LooseEq, place.clone(), Expr::null());
        let held = self.boxed_payload(item).then(|| self.some_value(place.clone()));
        let some = self.value_expr(Value::Ref(held.unwrap_or(place)), span)?;
        Ok(Some(Expr::cond(none, Expr::undefined(), some)))
    }

    /// Whether nothing reads what the call being lowered gives: its
    /// destination a temporary no statement reads, and whose drop reads
    /// nothing, as one that does nothing doesn't (`unread`): a `OnceCell`'s
    /// `set`'s `Result` (ADR 0847), a `Vec`'s `remove`'s item, a cell's
    /// `replace`'s old value (rd418, ADR 1022, amended).
    pub(in crate::lower::mir) fn result_unread(&self, state: &State<'_, 'tcx>) -> bool {
        matches!(&state.body[state.at].terminator().kind,
            mir::TerminatorKind::Call { destination, .. }
                if destination.projection.is_empty() && self.unread(state, destination.local))
    }

    /// A std function rust-js knows, given what MIR made of its arguments.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower::mir) fn mir_std_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (known, def_id): (Std, rustc_span::def_id::DefId),
        generic_args: ty::GenericArgsRef<'tcx>,
        (arg_tys, given): (&[Ty<'tcx>], crate::lower::calls::Given<'_>),
        mut values: Vec<Value<'tcx>>,
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let first_ty = || generic_args.types().next();
        let tcx = self.tcx;
        // `format!`, `print!` and `to_string()` of what may fail: in a `try`, a
        // `fmt::Error` std's panic (ADR 0187).
        let fails = match known {
            Std::Format | Std::Print { .. } => matches!(values.first(), Some(Value::Fmt(Std::FmtNew, ..))),
            Std::ToString => first_ty().is_some_and(|ty| self.fmt_may_fail(self.display_trait(), ty)),
            _ => false,
        };
        if fails {
            let exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let value = match known {
                Std::ToString => {
                    self.std_by_values(known, def_id, generic_args, (arg_tys, given), exprs, output, span, out)?
                }
                _ => exprs.into_iter().next().expect("the text"),
            };
            if matches!(known, Std::Print { .. }) {
                self.flush(state, out)?;
            }
            return Ok(Value::Expr(self.failing_values(known, Vec::new(), value, span, out)));
        }
        // One whose `Some` is boxed where it looks like `None` (ADR 0051).
        if self.refuses_boxed_option(known, output) {
            return Err(self.unsupported(span, "this call, for an `Option` of what could look like `None`"));
        }
        // A value with a destructor taken, refused; a chain that owns its items
        // is `mir_iter_call`'s, so none here does.
        self.check_takes_drops(known, def_id, arg_tys, span)?;
        self.keep_capacity(state, def_id, &mut values, span, out)?;
        // An index's: unchecked where the body's ranges show it in bounds,
        // and through `index_mut`, of a value JS can't change in place, the
        // item's place.
        if self.call_in_bounds(known, def_id, &values, span) || self.index_mut_place(state, known, def_id, output) {
            return self.index_call(state, known, (def_id, output, arg_tys[0]), values, span, out);
        }
        // `m.entry(k).or_insert(v)`: the value put in if the key's missing,
        // `$orInsert`, which is the value, an object; of a number,
        // `*m.entry(k).or_insert(0) += 1`, a handle on the key's value,
        // `$mutGet`'s (ADR 0152). The entry is `$entry(m, k)`, `{ TAG, _0: [m, k] }`.
        if let Std::Map(op @ (MapOp::OrInsert | MapOp::OrInsertWith { .. } | MapOp::OrDefault)) = known {
            return self.or_insert_values(state, op, values, (generic_args, arg_tys[0], output), span, out);
        }
        if let Some(same) = self.same_reference(known, output, &values) {
            return Ok(same);
        }
        // A `&mut` it made to an item JS can't change in place: a handle on
        // each (ADR 0152).
        if let Some(cell) = self.makes_items_of(def_id, output, generic_args, arg_tys) {
            let exprs = values
                .iter()
                .cloned()
                .map(|v| match v {
                    // A `Box<[T; N]>`'s, through a `&mut`: the array in its place.
                    Value::Ref(place) if known == Std::Pointee => Ok(place),
                    v => self.value_expr(v, span),
                })
                .collect::<R<Vec<_>>>()?;
            let receiver = arg_tys.first().map_or(tcx.types.unit, |t| t.peel_refs());
            if let Some(handles) = self.item_handles_of(known, exprs, receiver, span, out)? {
                return Ok(Value::Expr(match self.implements_iterator(output) {
                    true => self.std_iterator(handles, output),
                    false => handles,
                }));
            }
            if known == Std::Pointee
                && let Some(handle) = self.option_as_mut(state, (receiver, arg_tys[0]), &values, span, out)?
            {
                return Ok(Value::Expr(handle));
            }
            if !known.gives_its_cell() {
                let path = tcx.def_path_str(def_id);
                return Err(self.unsupported(span, &format!("a `{cell}` from `{path}` used as a value, from its MIR")));
            }
        }
        let mut values = values.into_iter();
        // Each kind in turn, which says `None` to what it doesn't know.
        let call = std_calls::StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
        };
        // The one kind that takes it, each asked of what it reads alone, so
        // none changes what's lowered before it's chosen (#191).
        let handlers: [(&str, StdHandler<'a, 'tcx>); 7] = [
            ("mir_fmt_call", Self::mir_fmt_call),
            ("mir_text_call", Self::mir_text_call),
            ("mir_map_call", Self::mir_map_call),
            ("mir_vec_call", Self::mir_vec_call),
            ("mir_iter_std_call", Self::mir_iter_std_call),
            ("mir_place_call", Self::mir_place_call),
            ("mir_value_call", Self::mir_value_call),
        ];
        let mut taken = (handlers.into_iter()).filter_map(|(name, handler)| Some((name, handler(self, state, call)?)));
        if let Some((name, lowering)) = taken.next() {
            let others: Vec<&str> = taken.map(|(other, _)| other).collect();
            assert!(others.is_empty(), "`{name}` and {others:?} each took one call");
            return lowering(self, state, &mut values, out);
        }
        Ok(Value::Expr(match known {
            // Panics end what runs: what's made before is made first.
            Std::Panic | Std::PanicFmt | Std::PanicDisplay { .. } | Std::BeginPanic => {
                self.flush(state, out)?;
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.std_by_values(known, def_id, generic_args, (arg_tys, given), exprs, output, span, out)?
            }
            _ if lowered_from_values(known) => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.std_by_values(known, def_id, generic_args, (arg_tys, given), exprs, output, span, out)?
            }
            _ => {
                let name = self.tcx.def_path_str(def_id);
                return Err(self.unsupported(span, &format!("`{name}`, from its MIR")));
            }
        }))
    }
}
