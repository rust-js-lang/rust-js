//! What dropping a value's parts runs, its items', and which `RefCell`s
//! are plain, found before a body's lowered.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `ty`'s drop, written in place: its own `drop`, then its parts'.
    pub(super) fn drop_parts(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        span: Span,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let js_span = self.js_span(span);
        match ty.kind() {
            // A `Pin` is its pointer (ADR 0329).
            ty::Adt(_, args) if ty.is_box() || self.recognition().pinned(ty).is_some() => {
                self.drop_in(value, args.type_at(0), span, made, out)?
            }
            ty::Adt(_, args) if self.is_vec_like(ty) => self.drop_items(value, args.type_at(0), span, made, out)?,
            // An iterator owning an array's items: each it hasn't given yet.
            ty::Adt(..) if let Some(item) = self.recognition().owned_items(ty) => {
                self.drop_items(value, item, span, made, out)?
            }
            // Closed, `it.return()`, which drops what it holds (ADR 0422), and
            // an adapter that holds a function with a destructor, made closing
            // to drop it, `map(f)`'s (ADR 0485).
            ty::Adt(..)
                if self.recognition().closed_on_drop(ty).is_some()
                    || (self.recognition().held_function(ty))
                        .is_some_and(|function| self.drops(function) != Drops::Nothing) =>
            {
                out.push(StmtKind::Expr(Expr::call(Expr::member(value, "return"), Vec::new())).at(js_span));
            }
            // A channel's end (ADR 0142): a receiver's drops what's still in
            // its queue, as Rust's does.
            ty::Adt(_, args) if let Some(end) = self.recognition().channel_end(ty) => {
                let call = match end {
                    ChannelEnd::Sender => Expr::call_helper("$dropSender", vec![value]),
                    ChannelEnd::Receiver => {
                        let mut values = vec![value];
                        values.extend(self.drop_function(args.type_at(0), span)?);
                        Expr::call_helper("$dropReceiver", values)
                    }
                };
                out.push(StmtKind::Expr(call).at(js_span));
            }
            // A map's entries, each its key, then its value, as Rust drops a
            // pair, and a set's items, a B-tree's in its keys' order (ADRs
            // 0321, 0467).
            ty::Adt(_, args) if self.is_map(ty) => {
                let set = self.is_set(ty);
                let key_ty = args.type_at(0);
                let value_ty = (!set).then(|| args.type_at(1));
                let drops = |this: &Self, ty: Option<Ty<'tcx>>| ty.is_some_and(|ty| this.drops(ty) != Drops::Nothing);
                let (keys, values) = (drops(self, Some(key_ty)), drops(self, value_ty));
                if !self.is_sorted(ty) && !(keys && values) {
                    let (items, item) = match (set, keys) {
                        (true, _) => (value, key_ty),
                        (false, true) => (Expr::call(Expr::member(value, "keys"), Vec::new()), key_ty),
                        (false, false) => (
                            Expr::call(Expr::member(value, "values"), Vec::new()),
                            value_ty.expect("a map's values"),
                        ),
                    };
                    self.drop_items(items, item, span, made, out)?;
                } else {
                    let entries = match self.is_sorted(ty) {
                        true => self.in_order_of(value, ty, span)?,
                        false => value,
                    };
                    let (key, item) = (self.fresh("key"), self.fresh("value"));
                    let mut body = Vec::new();
                    let pattern = match value_ty {
                        None => {
                            self.drop_in(Expr::var(&key), key_ty, span, made, &mut body)?;
                            js::Pattern::Name(key)
                        }
                        Some(value_ty) => {
                            self.drop_in(Expr::var(&key), key_ty, span, made, &mut body)?;
                            self.drop_in(Expr::var(&item), value_ty, span, made, &mut body)?;
                            js::Pattern::names_array([keys.then_some(key), values.then_some(item)])
                        }
                    };
                    out.push(
                        StmtKind::ForOf {
                            label: None,
                            pattern,
                            mutable: false,
                            iterable: entries,
                            body,
                        }
                        .at(js_span),
                    );
                }
            }
            // A `RefCell`'s value (ADR 0320), and a `Mutex`'s or an
            // `RwLock`'s, each `{ value }` as it is (ADR 0144), and a `Cell`'s
            // (ADR 0519); of a `RefCell<dyn T>`, the cell is the pair, `{
            // value, impl }` (ADR 0397).
            ty::Adt(_, args)
                if [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock, StdItem::Cell]
                    .into_iter()
                    .any(|cell| self.is_std_type(ty, cell)) =>
            {
                let inner = args.type_at(0);
                let value = match self.cell_dyn_trait(inner) {
                    Some(_) => value,
                    None => Expr::held(value),
                };
                self.drop_in(value, inner, span, made, out)?
            }
            // A `OnceCell`'s `Option`, `{ value }`, `undefined` where it's
            // not set (ADR 0519).
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::OnceCell) => {
                let held = Ty::new_option(self.tcx, args.type_at(0));
                self.drop_in(Expr::held(value), held, span, made, out)?
            }
            // A counted `Rc`: `$rcDrop(rc, drop)`, `drop` what the last drops of
            // what it points at, if that has a destructor, of a `dyn` its
            // pair, which it is (ADR 0520); a `Weak`: `$weakDrop` (ADR 0320).
            ty::Adt(..) if let Some(pointee) = self.counted_rc(ty) => {
                let mut args = vec![value];
                if self.drops(pointee) == Drops::Runs {
                    let name = self.fresh("value");
                    let mut body = Vec::new();
                    self.drop_in(Expr::var(&name), pointee, span, made, &mut body)?;
                    // `(value) => { dropNode(value); }` is `dropNode`.
                    let drop = match body.as_slice() {
                        [
                            Stmt {
                                kind:
                                    StmtKind::Expr(Expr {
                                        kind: js::ExprKind::Call(callee, given),
                                        ..
                                    }),
                                ..
                            },
                        ] if matches!(callee.kind, js::ExprKind::Var(_))
                            && matches!(given.as_slice(), [Expr { kind: js::ExprKind::Var(n), .. }] if *n == name) =>
                        {
                            (**callee).clone()
                        }
                        _ => Expr::arrow(vec![name.into()], body),
                    };
                    args.push(drop);
                }
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$rcDrop"), args)).at(js_span));
            }
            // A heap's `PeekMut` (ADR 0333).
            ty::Adt(..) if self.recognition().peek_mut_of(ty).is_some() => {
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$peekMutDrop"), vec![value])).at(js_span));
            }
            // A guard: `$unborrow(cell)` (ADR 0328).
            ty::Adt(..) if self.is_guard(ty) => {
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$unborrow"), vec![value])).at(js_span));
            }
            ty::Adt(..) if self.weak_of(ty).is_some() => {
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$weakDrop"), vec![value])).at(js_span));
            }
            // One that carries its drop, its own, which its clones don't share
            // (ADRs 0484, 0521, 0522), anywhere.
            ty::Closure(def_id, _) if self.krate.closure_facts.carries_its_drop(self.tcx, *def_id) => {
                out.push(StmtKind::Expr(Expr::call_helper("$dispose", vec![value])).at(js_span));
            }
            ty::Closure(def_id, _) | ty::CoroutineClosure(def_id, _) => self.drop_taken(*def_id, span, made, out)?,
            // An `async` block's future drops what it took as its body ends,
            // in that body; its body has run, as a promise's does, by the
            // time anything else drops it (ADR 0029): there it's nothing.
            ty::Coroutine(def_id, _) if self.body_owner == *def_id => self.drop_taken(*def_id, span, made, out)?,
            ty::Coroutine(..) => {}
            // A function's, the drop it carries, if any (ADR 0521).
            ty::Dynamic(predicates, ..) if crate::lower::recognition::is_fn_dyn(self.tcx, predicates) => {
                out.push(StmtKind::Expr(Expr::call_helper("$dispose", vec![value])).at(js_span));
            }
            // A `dyn`'s: its dictionary's `$drop`, if what it holds has one.
            ty::Dynamic(..) => {
                let dictionary = Expr::member(value.clone(), js::DICTIONARY);
                out.push(dyn_dropped(dictionary, Expr::held(value), js_span));
            }
            // `dropT?.(value)`: the caller's drop, if its `T` has one; of a
            // std trait's associated type of one, `dropIIntoIter` (ADR 0421).
            _ if let Some(index) = self.given_drop(ty) => {
                self.drop_state.used_drops.insert(index);
                let drop = Expr::var(&self.drop_state.param_drops[&index]);
                let js_span = self.js_span(span);
                out.push(
                    StmtKind::Expr(Expr {
                        reads: false,
                        kind: js::ExprKind::OptionalCall(Box::new(drop), vec![value]),
                        span: js_span,
                    })
                    .at(js_span),
                );
            }
            // `ZZone.$dropOffset?.(value)`: its impl's, if its type has one (ADR 0178).
            ty::Alias(..) if let Some(drop) = self.item_drop(ty) => {
                out.push(
                    StmtKind::Expr(Expr {
                        reads: false,
                        kind: js::ExprKind::OptionalCall(Box::new(drop), vec![value]),
                        span: js_span,
                    })
                    .at(js_span),
                );
            }
            ty::Array(item, _) | ty::Slice(item) => self.drop_items(value, *item, span, made, out)?,
            ty::Tuple(items) => {
                for (i, item) in items.iter().enumerate() {
                    let part = self.project(value.clone(), ty, i);
                    self.drop_in(part, item, span, made, out)?;
                }
            }
            ty::Adt(adt, args) => {
                if let Some(d) = self.tcx.adt_destructor(adt.did())
                    && self.runs_drop(d.did)
                {
                    // `drop(&mut self)`: a `&mut` to what isn't an object is a
                    // box of it (ADR 0072).
                    let this = match self.is_boxable(ty) {
                        true => Expr::object(vec![js::Prop::Field(js::HELD.into(), value.clone())]),
                        false => value.clone(),
                    };
                    let drop = self
                        .tcx
                        .lang_items()
                        .drop_trait()
                        .map(|t| self.tcx.associated_item_def_ids(t)[0])
                        .expect("`Drop` has `drop`");
                    let call = self
                        .trait_call(drop, self.tcx.mk_args(&[ty.into()]), vec![this], span, out)?
                        .ok_or_else(|| self.unsupported(span, &format!("calling `{ty}`'s `drop`")))?;
                    out.push(StmtKind::Expr(call).at(js_span));
                }
                if let Some(inner) = self.option_of(ty) {
                    // A `Some` of what may look like `None`, a generic `T`
                    // say, is boxed (ADR 0051): its value, unboxed.
                    let payload = match self.boxed_payload(inner) {
                        true => self.some_value(value.clone()),
                        false => value.clone(),
                    };
                    let mut some = Vec::new();
                    self.drop_in(payload, inner, span, made, &mut some)?;
                    out.push(StmtKind::If(Expr::bin(Op::LooseNe, value, Expr::null()), some, None).at(js_span));
                } else if adt.is_struct() {
                    for (i, field) in adt.non_enum_variant().fields.iter().enumerate() {
                        let part = self.project(value.clone(), ty, i);
                        let field = self.field_ty(field, args);
                        // A `dyn` last field: through the `impl` its struct
                        // was given (ADRs 0409, 0436).
                        if let ty::Dynamic(..) = field.kind() {
                            let dictionary = Expr::member(value.clone(), js::DICTIONARY);
                            out.push(dyn_dropped(dictionary, part, js_span));
                            continue;
                        }
                        self.drop_in(part, field, span, made, out)?;
                    }
                } else if adt.is_enum() {
                    for variant in adt.variants() {
                        let mut fields = Vec::new();
                        // An untagged enum's variant is its payload (ADR 0214).
                        let untagged = self.untagged(ty).is_some();
                        for (i, field) in variant.fields.iter().enumerate() {
                            let part = match untagged {
                                true => value.clone(),
                                false => Expr::member(value.clone(), variant_field(self.tcx, variant, i)),
                            };
                            self.drop_in(part, self.field_ty(field, args), span, made, &mut fields)?;
                        }
                        if fields.is_empty() {
                            continue;
                        }
                        let test = match adt.variants().len() {
                            1 => Expr::bool(true),
                            _ => self.variant_test(value.clone(), ty, *adt, adt.variant_index_with_id(variant.def_id)),
                        };
                        out.push(StmtKind::If(test, fields, None).at(js_span));
                    }
                } else if !adt.is_union() {
                    // A union's fields are never dropped, which Rust holds
                    // to what has no destructor, or a `ManuallyDrop` (ADR 0406).
                    return Err(self.unsupported(span, &format!("dropping `{ty}`")));
                }
            }
            _ => return Err(self.unsupported(span, &format!("dropping `{ty}`"))),
        }
        Ok(())
    }

    /// Each item of an array, in order: `for (const item of v) { .. }`.
    fn drop_items(
        &mut self,
        items: Expr,
        item: Ty<'tcx>,
        span: Span,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let name = self.fresh("item");
        let mut body = Vec::new();
        self.drop_in(Expr::var(&name), item, span, made, &mut body)?;
        let js_span = self.js_span(span);
        out.push(
            StmtKind::ForOf {
                label: None,
                pattern: js::Pattern::Name(name),
                mutable: false,
                iterable: items,
                body,
            }
            .at(js_span),
        );
        Ok(())
    }

    /// This body's facts, found the first time they're asked for; what it
    /// does that isn't supported is an error then.
    /// A `RefCell` type a field holds as its value (ADR 0362): each field of
    /// one in the crate is only ever borrowed for a moment, where nothing can
    /// ask, so no borrow of it is counted, its count stays 0, and its checks
    /// can't fail. A cell lent, bound, moved, replaced or cloned, of a derive,
    /// or of a type with a destructor, or a library's, is a cell.
    pub(in crate::lower) fn plain_ref_cell(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::RefCell)
            && is_plain_ref_cell(
                (self.krate.plain_ref_cells).get_or_init(|| self.find_plain_ref_cells()),
                self.tcx,
                ty,
            )
    }

    fn find_plain_ref_cells(&self) -> HashSet<Ty<'tcx>> {
        let tcx = self.tcx;
        // Of THIR's borrows, as MIR's are, whose guards of a field's handle
        // tidy reads as the field (ADR 0364).
        if self.krate.library {
            return HashSet::new();
        }
        let mut plain = HashSet::new();
        let mut cells = HashSet::new();
        let bodies = (self.krate.bodies.values().copied()).chain(self.krate.closures.values().copied());
        for body in bodies {
            // Each body's own facts, asked where it is, of its own type
            // parameters, with no dictionaries: a type parameter's drop is
            // then a maybe, and its borrow counted.
            let owner = tcx.typeck_root_def_id(body.def_id);
            let recognition = crate::lower::recognition::Recognition {
                tcx,
                typing_env: ty::TypingEnv::post_analysis(tcx, owner),
                trait_impls: self.krate.trait_impls,
                foreign: self.krate.foreign,
            };
            let state = TypeDrops::default();
            let query = without_dictionaries(
                recognition,
                (false, self.krate.closure_facts),
                self.krate.counted,
                &state,
            );
            let thir = &body.thir;
            let facts = find_facts(&query, self.krate.cell_borrows, thir, body.def_id);
            // `x.f` lent to a borrow held for a moment: `x.f.borrow_mut().push(..)`,
            // or to a write that holds it only while it swaps the value,
            // `x.f.replace(v)`, whose old value drops after; `x.f.take()`
            // runs `T::default()` too, which std's runs nothing of.
            let writes_for_a_moment = |fun: rustc_middle::thir::ExprId| {
                crate::lower::fn_def(thir[fun].ty).is_some_and(|(id, args)| match recognition.classify(id, args) {
                    Some(crate::lower::Std::CellReplace) => true,
                    Some(crate::lower::Std::CellTake) => {
                        args.types().next().is_some_and(|held| recognition.std_default(held))
                    }
                    _ => false,
                })
            };
            let mut for_a_moment = HashSet::new();
            for expr in thir.exprs.iter() {
                if let ExprKind::Call { fun, ref args, .. } = expr.kind
                    && (facts.momentary.contains(&fun) || writes_for_a_moment(fun))
                    && let Some(&lent) = args.first()
                    && let ExprKind::Borrow { arg, .. } = thir[crate::lower::body_queries::strip(thir, lent)].kind
                {
                    for_a_moment.insert(crate::lower::body_queries::strip(thir, arg));
                }
            }
            for (id, expr) in thir.exprs.iter_enumerated() {
                if let ExprKind::Field { .. } = expr.kind
                    && self.is_std_type(expr.ty, StdItem::RefCell)
                {
                    match for_a_moment.contains(&id) {
                        true => plain.insert(expr.ty),
                        false => cells.insert(expr.ty),
                    };
                }
            }
            // One bound by a pattern is a value of its own, a cell.
            let mut bound = BoundRefCells {
                recognition: self.recognition(),
                thir,
                found: &mut cells,
            };
            for expr in thir.exprs.iter() {
                if let ExprKind::Let { ref pat, .. } = expr.kind {
                    rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pat);
                }
            }
            for stmt in thir.stmts.iter() {
                if let ThirStmt::Let { ref pattern, .. } = stmt.kind {
                    rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pattern);
                }
            }
            for arm in thir.arms.iter() {
                rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, &arm.pattern);
            }
            for param in thir.params.iter() {
                if let Some(pat) = &param.pat {
                    rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pat);
                }
            }
        }
        // A derive's, which reads its fields itself (ADR 0049).
        for imp in tcx.hir_crate_items(()).definitions() {
            if matches!(tcx.def_kind(imp), DefKind::Impl { of_trait: true })
                && crate::lower::recognition::known_derive(tcx, imp.to_def_id())
                && let ty::Adt(adt, args) = tcx.type_of(imp).instantiate_identity().skip_normalization().kind()
            {
                for field in adt.all_fields() {
                    let field_ty = field.ty(tcx, args).skip_normalization();
                    if self.is_std_type(field_ty, StdItem::RefCell) {
                        cells.insert(field_ty);
                    }
                }
            }
        }
        // A generic one may be any of them.
        if cells.iter().any(|ty| ty.has_param()) {
            return HashSet::new();
        }
        // Of no type parameter: asked anywhere.
        let recognition = crate::lower::recognition::Recognition {
            tcx,
            typing_env: ty::TypingEnv::fully_monomorphized(),
            trait_impls: self.krate.trait_impls,
            foreign: self.krate.foreign,
        };
        let state = TypeDrops::default();
        let query = without_dictionaries(
            recognition,
            (false, self.krate.closure_facts),
            self.krate.counted,
            &state,
        );
        // Each of its regions erased, as `is_plain_ref_cell` asks.
        let erased = |set: HashSet<Ty<'tcx>>| -> HashSet<Ty<'tcx>> {
            set.into_iter().map(|ty| tcx.erase_and_anonymize_regions(ty)).collect()
        };
        let cells = erased(cells);
        let mut plain = erased(plain);
        plain.retain(|ty| !cells.contains(ty) && !ty.has_param() && !query.has_drops(*ty));
        plain
    }

    pub(in crate::lower) fn drop_facts(&mut self) -> R<Rc<Facts>> {
        let key = std::ptr::from_ref(self.thir) as usize;
        if let Some(facts) = self.drop_state.facts.get(&key) {
            return Ok(facts.clone());
        }
        let facts = Rc::new(find_facts(
            &self.drop_query(),
            self.krate.cell_borrows,
            self.thir,
            self.body_owner,
        ));
        self.drop_state.facts.insert(key, facts.clone());
        let mut failed = None;
        for (span, what, _) in &facts.problems {
            failed = Some(self.unsupported(*span, what));
        }
        match failed {
            Some(guar) => Err(guar),
            None => Ok(facts),
        }
    }

    /// What THIR's facts refuse of the closures of the body being lowered
    /// that hold a value with a destructor: one made where JS can't see what
    /// it took, given away uncalled, or assigned.
    pub(in crate::lower) fn check_closure_drops(&self) -> R<()> {
        let facts = find_facts(&self.drop_query(), self.krate.cell_borrows, self.thir, self.body_owner);
        let mut failed = None;
        for (span, what, _) in facts.problems.iter().filter(|(.., closure)| *closure) {
            failed = Some(self.unsupported(*span, what));
        }
        match failed {
            Some(guar) => Err(guar),
            None => Ok(()),
        }
    }
}

/// Whether `ty` is one of `plain`, the crate's `RefCell`s held as their
/// values (ADR 0362), of its regions erased: a field's own type says `dyn
/// FnMut() + 'static`, a body's `'{erased}`.
pub(in crate::lower) fn is_plain_ref_cell<'tcx>(plain: &HashSet<Ty<'tcx>>, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    plain.contains(&tcx.erase_and_anonymize_regions(ty))
}
