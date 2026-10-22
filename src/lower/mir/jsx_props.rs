//! A JSX binding's call: the props a component is given, a struct
//! update's base, and THIR's call where MIR's is.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The locals each struct JSX gives a component as its props is made in,
    /// `<*>`'s second, and those of their flattened fields, made there.
    pub(super) fn mir_jsx_props(&self, body: &mir::Body<'tcx>) -> std::collections::HashSet<Local> {
        let decls = &body.local_decls;
        let mut made: Vec<Local> = (body.basic_blocks.iter())
            .filter_map(|data| match &data.terminator().kind {
                TerminatorKind::Call { func, args, .. } => {
                    let (def_id, _) = fn_def(func.ty(decls, self.tcx))?;
                    let component = bindings::is_binding(self.tcx, def_id)
                        && matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::Jsx(tag) if tag == "*");
                    match (component, &args[..]) {
                        (true, [_, given]) => given.node.place().filter(|p| p.projection.is_empty()).map(|p| p.local),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect();
        let mut props = std::collections::HashSet::new();
        while let Some(local) = made.pop() {
            if !props.insert(local) {
                continue;
            }
            for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
                if let StatementKind::Assign(assign) = &statement.kind
                    && assign.0 == Place::from(local)
                    && let Rvalue::Aggregate(kind, operands) = &assign.1
                    && let AggregateKind::Adt(..) = **kind
                {
                    let ty = assign.0.ty(decls, self.tcx).ty;
                    for (i, operand) in operands.iter_enumerated() {
                        if bindings::is_flatten_field(self.tcx, ty, i.as_usize())
                            && let Some(field) = operand.place().filter(|p| p.projection.is_empty())
                        {
                            made.push(field.local);
                        }
                    }
                }
            }
        }
        props
    }

    /// The base of a struct update, THIR's at `span`, where it's spread whole,
    /// `..*item` of a shared reference or `..hit` of a variable (ADR 0250): its
    /// place, the variable's by its debug info. MIR reads a field of it at most,
    /// `Props { title, ..props }`, or none, every field given,
    /// `Hit { url, ..*item }`, which MIR alone can't tell from `level: a.level`.
    pub(super) fn written_base(
        &self,
        state: &State<'_, 'tcx>,
        ty: Ty<'tcx>,
        operands: &IndexVec<FieldIdx, Operand<'tcx>>,
        span: Span,
    ) -> Option<Place<'tcx>> {
        let strip = |e| crate::lower::body_queries::strip(self.thir, e);
        let base = (self.thir.exprs.iter()).find_map(|e| match &e.kind {
            thir::ExprKind::Adt(adt) if e.span == span => match &adt.base {
                thir::AdtExprBase::Base(fru) => Some(strip(fru.base)),
                _ => None,
            },
            _ => None,
        })?;
        let named = |var: thir::ExprId| match self.thir[var].kind {
            thir::ExprKind::VarRef { id } => Some(id.0),
            thir::ExprKind::UpvarRef { var_hir_id, .. } => Some(var_hir_id.0),
            _ => None,
        };
        let (var, deref) = match self.thir[base].kind {
            thir::ExprKind::Deref { arg }
                if matches!(self.thir[arg].ty.kind(), ty::Ref(_, _, ty::Mutability::Not))
                    && named(strip(arg)).is_some() =>
            {
                (Some(strip(arg)), true)
            }
            thir::ExprKind::VarRef { .. } | thir::ExprKind::UpvarRef { .. }
                if !bindings::has_flatten(self.tcx, ty) && (!self.is_copy(ty) || !self.contains_mutated(ty)) =>
            {
                (Some(base), false)
            }
            thir::ExprKind::VarRef { .. } | thir::ExprKind::UpvarRef { .. } => return None,
            // Of a place no variable names, `..*unsafe { posts.get_unchecked(0) }`
            // or `..posts[0]`, as react.dev's home page gives each BlogCard a
            // post: the place MIR reads each of its fields from, `(*t).title`.
            thir::ExprKind::Deref { arg }
                if matches!(self.thir[arg].ty.kind(), ty::Ref(_, _, ty::Mutability::Not))
                    && !bindings::has_flatten(self.tcx, ty) =>
            {
                (None, false)
            }
            thir::ExprKind::Index { .. } if !bindings::has_flatten(self.tcx, ty) => (None, false),
            _ => return None,
        };
        let base = match var {
            Some(var) => {
                let binding = self.tcx.hir_span(named(var)?);
                let place = (state.body.var_debug_info.iter()).find_map(|info| match info.value {
                    mir::VarDebugInfoContents::Place(place) if info.source_info.span == binding => Some(place),
                    _ => None,
                })?;
                match deref {
                    true => place.project_deeper(&[PlaceElem::Deref], self.tcx),
                    false => place,
                }
            }
            None => {
                let mut read = None;
                for (i, operand) in operands.iter_enumerated() {
                    let Some(place) = operand.place() else { continue };
                    let Some((PlaceElem::Field(f, _), rest)) = place.projection.split_last() else {
                        continue;
                    };
                    let of = Place {
                        local: place.local,
                        projection: self.tcx.mk_place_elems(rest),
                    };
                    match read {
                        _ if *f != i => continue,
                        None => read = Some(of),
                        Some(seen) if seen == of => {}
                        Some(_) => return None,
                    }
                }
                read?
            }
        };
        // Each field read of it, the spread's, its own: one that needs no copy
        // of its own.
        let Shape::Object(fields) = self.shape(ty) else {
            return None;
        };
        for (i, operand) in operands.iter_enumerated() {
            let Some(read) = operand.place().filter(|p| p.local == base.local) else {
                continue;
            };
            let field = matches!(read.projection.split_last(), Some((PlaceElem::Field(f, _), rest))
                if *f == i && rest == base.projection.as_slice());
            let t = fields[i.as_usize()].1;
            if !field || self.contains_mutated(t) && self.is_copy(t) {
                return None;
            }
        }
        Some(base)
    }

    /// A JSX binding's arguments, each with what THIR has of it, where THIR
    /// has the call too, for its shape. `component` says a component's
    /// tag, where it's one, in place of its value.
    pub(super) fn jsx_args(
        &mut self,
        def_id: rustc_span::def_id::DefId,
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
        component: impl Fn(&Self, usize, Ty<'tcx>) -> Option<Expr>,
    ) -> R<Vec<JsxArg<'tcx>>> {
        let exprs = self.thir_call_args(def_id, span);
        let mut given = Vec::new();
        for (i, (value, &ty)) in values.into_iter().zip(arg_tys).enumerate() {
            let value = match component(self, i, ty) {
                Some(tag) => tag,
                None => self.value_expr(value, span)?,
            };
            let expr = exprs.as_ref().and_then(|exprs| exprs.get(i).copied());
            given.push(JsxArg {
                value,
                ty,
                expr,
                span: expr.map_or(span, |e| self.thir[e].span),
            });
        }
        Ok(given)
    }

    /// The arguments of THIR's call of `def_id` at `fn_span`, MIR's call
    /// there, where it's the one.
    fn thir_call_args(&self, def_id: rustc_span::def_id::DefId, fn_span: Span) -> Option<Vec<thir::ExprId>> {
        self.thir_call(def_id, fn_span).map(|(_, args)| args)
    }

    /// Whether THIR's call of `def_id` at `span`, MIR's there, gives a
    /// `&mut` to a variable as its `i`th argument, `&mut current`, which a
    /// closure may have captured: a place JS assigns.
    pub(super) fn borrows_variable(&self, def_id: rustc_span::def_id::DefId, span: Span, i: usize) -> bool {
        self.borrowed_place(def_id, span, i)
            .is_some_and(|place| matches!(place, thir::ExprKind::VarRef { .. } | thir::ExprKind::UpvarRef { .. }))
    }

    /// Whether the call's `i`th argument borrows a field, `&mut self.orig`,
    /// whose JS is the field itself, a place (ADR 0553).
    pub(super) fn borrows_field(&self, def_id: rustc_span::def_id::DefId, span: Span, i: usize) -> bool {
        self.borrowed_place(def_id, span, i)
            .is_some_and(|place| matches!(place, thir::ExprKind::Field { .. }))
    }

    /// What the call's `i`th argument borrows by `&mut`, as THIR has it.
    fn borrowed_place(&self, def_id: rustc_span::def_id::DefId, span: Span, i: usize) -> Option<&thir::ExprKind<'tcx>> {
        let place = self
            .thir_call_args(def_id, span)
            .and_then(|args| self.mut_borrowed(*args.get(i)?))?;
        Some(&self.thir[self.strip(place)].kind)
    }

    /// THIR's update whose base is the call at `span`, MIR's there, of a
    /// derived `Default`, `P { a, ..Default::default() }`: the base's fields,
    /// each default the update reads. `None` of another, or where a default the
    /// update replaces is made still, for its effects.
    pub(super) fn default_base_at(&mut self, span: Span) -> R<Option<Vec<(String, Expr)>>> {
        let strip = |e| crate::lower::body_queries::strip(self.thir, e);
        let found = (self.thir.exprs.iter()).find_map(|e| match &e.kind {
            thir::ExprKind::Adt(adt) => match &adt.base {
                thir::AdtExprBase::Base(fru)
                    if matches!(self.thir[strip(fru.base)].kind,
                        thir::ExprKind::Call { fn_span, .. } if fn_span.source_equal(span)) =>
                {
                    Some((
                        strip(fru.base),
                        e.ty,
                        adt.fields.iter().map(|f| f.name.as_usize()).collect::<Vec<_>>(),
                    ))
                }
                _ => None,
            },
            _ => None,
        });
        let Some((base, ty, given)) = found else {
            return Ok(None);
        };
        let Some(fields) = self.derived_default_fields(base, ty) else {
            return Ok(None);
        };
        let (props, kept) = self.update_default(fields, &given, span)?;
        Ok((!kept).then_some(props))
    }

    /// Whether THIR's cast at `span`, MIR's there, is of a float it knows whole
    /// and in the target's range: the value as it is (`whole_casts`).
    pub(super) fn whole_cast_at(&self, span: Span) -> bool {
        let mut casts = (self.thir.exprs.iter_enumerated())
            .filter(|(_, e)| matches!(e.kind, thir::ExprKind::Cast { .. }) && e.span.source_equal(span));
        matches!((casts.next(), casts.next()), (Some((id, _)), None) if self.body_facts.whole_casts.contains(&id))
    }

    /// THIR's call of `def_id` at `fn_span`, MIR's call there, where it's
    /// the one: its function and its arguments.
    pub(super) fn thir_call(
        &self,
        def_id: rustc_span::def_id::DefId,
        fn_span: Span,
    ) -> Option<(thir::ExprId, Vec<thir::ExprId>)> {
        let mut calls = self.thir.exprs.iter().filter_map(|e| match &e.kind {
            thir::ExprKind::Call {
                fun, args, fn_span: at, ..
            } if *at == fn_span && matches!(*self.thir[*fun].ty.kind(), ty::FnDef(id, _) if id == def_id) => {
                Some((*fun, args.to_vec()))
            }
            _ => None,
        });
        let call = calls.next()?;
        calls.next().is_none().then_some(call)
    }

    /// A future whose `Future` is the crate's own made a JS promise: by a
    /// call that asks a `Future` or an `IntoFuture` of it, `.await`'s
    /// `into_future` or a generic function's `F: Future`, or as a `dyn
    /// Future`. No promise is one (ADR 0425).
    pub(super) fn unlowered_future_awaited(&self, future: Ty<'tcx>, span: Span) -> rustc_span::ErrorGuaranteed {
        let what = format!("`{future}` as a promise");
        let why = "its `Future` is the crate's own, which rust-js never compiles (ADR 0425)";
        refuse(self.tcx, span, Refused::Because(&what, why))
    }
}
