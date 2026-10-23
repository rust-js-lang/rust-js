//! std calls that change a place: a map's entry, `?` of a write's result,
//! and `mem::swap`, `mem::replace` and `take`.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `m.entry(k).or_insert(v)`, `or_insert_with(f)` or `or_default()`, of
    /// its entry's and value's `values`: a value, or a handle on a number's
    /// or text's (ADRs 0152, 0321, 0467).
    pub(super) fn or_insert_values(
        &mut self,
        state: &mut State<'_, 'tcx>,
        op: MapOp,
        values: Vec<Value<'tcx>>,
        (generic_args, output): (ty::GenericArgsRef<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let mut exprs = values
            .into_iter()
            .map(|v| self.value_expr(v, span))
            .collect::<R<Vec<_>>>()?
            .into_iter();
        let entry = self.after_and_modify(exprs.next().expect("the entry"), span, out);
        // Made here, `$entry(m, k)`, the key given's to drop where it's
        // there, `or_inserted`'s; of one made before, `$entry`'s, which
        // did (ADR 0467).
        let fused = matches!(&entry.kind, js::ExprKind::Call(callee, parts)
            if parts.len() >= 2 && callee.helper_name() == Some("$entry"));
        let (map, key) = match entry.kind {
            js::ExprKind::Call(callee, parts) if parts.len() >= 2 && callee.helper_name() == Some("$entry") => {
                let mut parts = parts.into_iter();
                (parts.next().expect("a map"), parts.next().expect("a key"))
            }
            kind => {
                let entry = self.spill(
                    "entry",
                    Expr {
                        reads: false,
                        kind,
                        span: entry.span,
                    },
                    out,
                );
                let parts = Expr::member(entry, "_0");
                (
                    Expr::index(parts.clone(), Expr::int(0)),
                    Expr::index(parts, Expr::int(1)),
                )
            }
        };
        let map = if map.reads_same() {
            map
        } else {
            self.spill("map", map, out)
        };
        let key = if key.reads_same() {
            key
        } else {
            self.spill("key", key, out)
        };
        let default = match op {
            MapOp::OrDefault => {
                let value = generic_args
                    .types()
                    .nth(1)
                    .ok_or_else(|| self.unsupported(span, "this entry"))?;
                let value = self.default_value(value, span)?;
                Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(value)).at(js::Span::NONE)])
            }
            _ => exprs.next().expect("the value"),
        };
        let inserted = self.or_inserted(op, vec![map.clone(), key.clone(), default], (generic_args, fused), span)?;
        if !self.is_cell(output) {
            return Ok(Value::Expr(inserted));
        }
        self.flush(state, out)?;
        out.push(StmtKind::Expr(inserted).at(self.js_span(span)));
        Ok(Value::Expr(Expr::call(Expr::helper("$mutGet"), vec![map, key])))
    }

    /// `$orInsert(m, key, value)`, or `$orInsertWith(m, key, f)` of `op`, of
    /// its map, key and value, `list`: of `or_insert` of a value with a
    /// destructor, given its drop, which drops the value given where the
    /// key's there (ADR 0321); and of a key with one, of an entry `fused`
    /// into it, the key's (ADR 0467).
    fn or_inserted(
        &mut self,
        op: MapOp,
        mut list: Vec<Expr>,
        (generic_args, fused): (ty::GenericArgsRef<'tcx>, bool),
        span: Span,
    ) -> R<Expr> {
        let name = match op {
            MapOp::OrInsert => "$orInsert",
            MapOp::OrInsertWith { keyed: true } => "$orInsertWithKey",
            _ => "$orInsertWith",
        };
        // Of `or_insert`, the value's drop; then of each, the key's, which
        // drops the key given where it's there (ADR 0467).
        let value_drop = match (op, generic_args.types().nth(1)) {
            (MapOp::OrInsert, Some(value)) => self.std_drop_function(value, span)?,
            _ => None,
        };
        let key_drop = match generic_args.types().next() {
            Some(key) if fused => self.std_drop_function(key, span)?,
            _ => None,
        };
        // And of `or_insert_with`, its function's, its own type parameter,
        // which drops it where it's left unrun, there (ADR 0469).
        let make_drop = match (op, generic_args.types().last()) {
            (MapOp::OrInsertWith { .. }, Some(make)) if self.drops_unrun(make) => self.std_drop_function(make, span)?,
            _ => None,
        };
        if op == MapOp::OrInsert && (value_drop.is_some() || key_drop.is_some()) {
            list.push(value_drop.unwrap_or_else(Expr::undefined));
        }
        if key_drop.is_some() || make_drop.is_some() {
            list.push(key_drop.unwrap_or_else(Expr::undefined));
        }
        list.extend(make_drop);
        Ok(Expr::call(Expr::helper(name), list))
    }

    /// A call of `def_id`, of its arguments' values, `arg_tys` their types.
    /// Whether `def_id` is a method of a write's result, which never fails
    /// (`never_fails`): `Result`'s own, not `Argument::new_debug(&r)`'s, nor
    /// `?`'s.
    pub(super) fn of_never_failing(&self, def_id: rustc_span::def_id::DefId, arg_tys: &[Ty<'tcx>]) -> bool {
        let tcx = self.tcx;
        arg_tys.first().is_some_and(|ty| self.is_write_result(ty.peel_refs()))
            && tcx.def_kind(def_id) == DefKind::AssocFn
            && tcx
                .impl_of_assoc(def_id)
                .is_some_and(|imp| self.recognition().is_result(tcx.type_of(imp).skip_binder()))
    }

    /// `?` of a write's result that may be an `Err`, what's tried: its error
    /// thrown on, `fmt_check` (ADR 0187), and what's left never breaks. In a
    /// function that gives a value's `Result`, its end catches it and gives
    /// it as its `Err`, converted (ADR 0462).
    pub(super) fn tried_write(
        &mut self,
        state: &mut State<'_, 'tcx>,
        values: Vec<Value<'tcx>>,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let tried = self.value_expr(values.into_iter().next().expect("what's tried"), span)?;
        self.flush(state, out)?;
        let output = state.body.local_decls[mir::RETURN_PLACE].ty;
        if !self.is_write_result(output) && self.is_std_type(output, StdItem::Result) {
            match &state.thrown_writes {
                Some((thrown, _)) if *thrown != ty => {
                    return Err(self.unsupported(span, "`?` of two writes' results of different errors"));
                }
                Some(_) => {}
                None => state.thrown_writes = Some((ty, self.tried_conversion(ty, Some(output), span)?)),
            }
        }
        self.fmt_check(tried, self.js_span(span), out);
        Ok(Value::Branch(Expr::undefined(), ty))
    }

    /// `?`'s residual given back as a write's result, `output`: its error,
    /// thrown, as its `Err` is (ADRs 0187, 0462).
    pub(super) fn thrown_residual(&self, residual: Expr, output: Ty<'tcx>) -> Expr {
        match self.is_fmt_result(output) {
            true => Expr::call(Expr::helper("$fmtError"), Vec::new()),
            false => Expr::call_helper("$ioThrow", vec![Expr::member(residual, "_0")]),
        }
    }

    /// A method of a write's result: an `Ok` `fmt::Result` is nothing in JS
    /// (ADR 0054), and so is a write's `io::Result<()>` (ADR 0132). Its
    /// `unwrap()` is `()`, after what made it ran (ADR 0148), `is_ok()`
    /// `true`, `ok()` `Some(())`: its answer, `fmt_result_values`'.
    pub(super) fn never_failing_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        (values, arg_tys): (Vec<Value<'tcx>>, &[Ty<'tcx>]),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let Some(answer) = crate::lower::recognition::fmt_result_answer(self.tcx, def_id) else {
            return Err(self.unsupported(span, "methods of a `fmt::Result`, from its MIR"));
        };
        let mut values = values.into_iter();
        let result = self.value_expr(values.next().expect("the result"), span)?;
        let message = values.next().map(|v| self.value_expr(v, span)).transpose()?;
        // Only in a crate where a `fmt::Result` may be an `Err` is one tested:
        // what `$writeTry` caught (ADR 0187).
        let may_fail = self.krate.any_failing && self.is_write_result(arg_tys[0].peel_refs());
        if may_fail || result.has_effects() || message.as_ref().is_some_and(Expr::has_effects) {
            self.flush(state, out)?;
        }
        Ok(Value::Expr(
            self.fmt_result_values(answer, result, message, may_fail, span, out),
        ))
    }

    /// `mem::swap`, `mem::replace`, `mem::take`, `Option::take` and
    /// `Option::replace`: of a `&mut` to a place, writing each in turn,
    /// `const t = a; a = b; b = t;`, which nothing else can see while the call
    /// has it; of a `&mut` to an object, the object changed in place,
    /// `$exchange(a, b)`, `$take(a, v)` (ADR 0147).
    pub(super) fn mir_swap_or_replace(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (known, def_id): (Std, rustc_span::def_id::DefId),
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let pointee = arg_tys[0].peel_refs();
        let js_span = self.js_span(span);
        let values = values.into_iter();
        self.flush(state, out)?;
        // Of a `&mut` to a variable, `&mut current`, an object's too: the
        // variable, given its new value; a swap's, of two. One of this body's
        // own, which holds the value itself: a closure's capture may be a name
        // for a `&mut` to it, which assigning would only rebind.
        let borrowed = if known == Std::Swap { 2 } else { 1 };
        let variables = (0..borrowed).all(|i| self.borrows_variable(def_id, span, i));
        let fields = (0..borrowed).all(|i| self.borrows_field(def_id, span, i));
        let own = |e: &Expr| {
            matches!(&e.kind, js::ExprKind::Var(name) if (state.locals.names.iter_enumerated())
                .any(|(local, n)| n == name && !state.body.local_decls[local].ty.is_ref()))
                || (state.captures.iter()).any(|(capture, place)| {
                    *place && matches!((&capture.kind, &e.kind), (js::ExprKind::Var(a), js::ExprKind::Var(b)) if a == b)
                })
        };
        let mut given = Vec::new();
        for (i, value) in values.enumerate() {
            given.push(match value {
                Value::Expr(e) | Value::Place(e) if i < borrowed && variables && own(&e) => Value::Ref(e),
                // Of fields, `&mut self.orig`: each field, written as a place.
                Value::Expr(e) | Value::Place(e)
                    if i < borrowed && fields && matches!(e.kind, js::ExprKind::Member(..)) =>
                {
                    Value::Ref(e)
                }
                // A cell, a generic `&mut V` a store gives: its `value`, the
                // place, of what's held once, as it's read and written.
                Value::Expr(e) if i < borrowed && self.is_cell(arg_tys[i]) => {
                    let cell = match e.reads_same() {
                        true => e,
                        false => self.spill("cell", e, out),
                    };
                    Value::Ref(Expr::held(cell))
                }
                value => value,
            });
        }
        let mut values = given.into_iter();
        let first = values.next().expect("a `&mut`");
        let Value::Ref(a) = first else {
            // Of an object: changed in place.
            if !matches!(known, Std::Swap | Std::Replace | Std::MemTake) || !self.written_in_place(pointee) {
                return Err(self.unsupported(span, "this `&mut` given to `mem::swap` and the like, from its MIR"));
            }
            let a = self.value_expr(first, span)?;
            let b = match known {
                Std::MemTake => self.default_value(pointee, span)?,
                _ => self.value_expr(values.next().expect("a second"), span)?,
            };
            let name = match known {
                Std::Swap => "$exchange",
                _ => "$take",
            };
            let call = Expr::call(Expr::helper(name), vec![a, b]);
            if known == Std::Swap {
                out.push(StmtKind::Expr(call).at(js_span));
                return Ok(Value::Expr(Expr::undefined()));
            }
            return Ok(Value::Expr(call));
        };
        let (b, b_place) = match known {
            Std::Swap => match values.next() {
                Some(Value::Ref(b)) => (b.clone(), Some(b)),
                _ => return Err(self.unsupported(span, "`mem::swap` of these, from its MIR")),
            },
            // `None`, and what's in a `Some`, boxed where a generic one is (ADR 0051).
            Std::OptionTake => (Expr::undefined(), None),
            Std::OptionReplace => {
                let value = self.value_expr(values.next().expect("a value"), span)?;
                let item = self.option_of(pointee).expect("an `Option` has a `T`");
                let value = match self.boxed_payload(item) {
                    true => self.some(value),
                    false => value,
                };
                (value, None)
            }
            Std::MemTake => (self.default_value(pointee, span)?, None),
            _ => (self.value_expr(values.next().expect("a value"), span)?, None),
        };
        let old = self.spill(if b_place.is_some() { "t" } else { "old" }, a.clone(), out);
        out.push(StmtKind::Assign(a, b).at(js_span));
        Ok(Value::Expr(match b_place {
            Some(b) => {
                out.push(StmtKind::Assign(b, old).at(js_span));
                Expr::undefined()
            }
            None => old,
        }))
    }
}
