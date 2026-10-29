//! What dropping a value's parts runs, its items', and a body's drop
//! facts, found before it's lowered.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a `ty`, an iterator, is dropped by closing it, `it.return()`:
    /// one closed to drop what it holds (ADR 0422), or an adapter holding a
    /// function with a destructor, made closing to drop it (ADR 0485). The
    /// one answer, of its drop and of what its owner holds (#206).
    pub(in crate::lower) fn dropped_by_closing(&self, ty: Ty<'tcx>) -> bool {
        let recognition = self.recognition();
        recognition.closed_on_drop(ty).is_some()
            || (recognition.held_function(ty)).is_some_and(|function| self.drops(function) != Drops::Nothing)
            // Std's struct wherever it's made, a `Peekable`'s or a `Fuse`'s,
            // never the recognizers' JS chain: closed once, which drops what
            // it holds, not again after std's compiled `fold` took it (ADR
            // 0913).
            || (crate::lower::recognition::is_compiled_std_iterator(self.tcx, self.reveal(ty))
                && !crate::lower::recognition::is_held_compiled(self.tcx, self.reveal(ty)))
    }

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
            ty::Adt(..) if self.dropped_by_closing(ty) => {
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
                            let mut value_dropped = Vec::new();
                            self.drop_in(Expr::var(&key), key_ty, span, made, &mut body)?;
                            self.drop_in(Expr::var(&item), value_ty, span, made, &mut value_dropped)?;
                            body = each_dropped(vec![body, value_dropped], js_span);
                            js::Pattern::names_array([keys.then_some(key), values.then_some(item)])
                        }
                    };
                    out.push(items_dropped(entries, pattern, body, js_span));
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
                // A `RefCell` of an object is the object (ADR 0863).
                let value = match self.cell_dyn_trait(inner).is_some() || self.is_value_cell(ty) {
                    true => value,
                    false => Expr::held(value),
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
                // Of no `drop`, it never throws: a count, one less.
                let mut call = Expr::call(Expr::helper("$rcDrop"), args);
                if let js::ExprKind::Call(_, given) = &call.kind
                    && given.len() == 1
                {
                    call.facts = call.facts.with(js::Known::NeverThrows);
                }
                out.push(StmtKind::Expr(call).at(js_span));
            }
            // A heap's `PeekMut` (ADR 0333).
            ty::Adt(..) if self.recognition().peek_mut_of(ty).is_some() => {
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$peekMutDrop"), vec![value])).at(js_span));
            }
            // A guard: `$unborrow(cell)` (ADR 0328).
            ty::Adt(..) if self.is_guard(ty) => {
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$unborrow"), vec![value])).at(js_span));
            }
            // A count, one less, which never throws.
            ty::Adt(..) if self.counted_weak(ty).is_some() => {
                let mut call = Expr::call(Expr::helper("$weakDrop"), vec![value]);
                call.facts = call.facts.with(js::Known::NeverThrows);
                out.push(StmtKind::Expr(call).at(js_span));
            }
            // One that carries its drop, its own, which its clones don't share
            // (ADRs 0484, 0521, 0522), anywhere but in its own body: an
            // `FnOnce`'s, called, drops what it took as it ends, where the
            // function it is has no name.
            ty::Closure(def_id, _)
                if self.frame.owner != *def_id && self.krate.closure_facts.carries_its_drop(self.tcx, *def_id) =>
            {
                out.push(StmtKind::Expr(Expr::call_helper("$dispose", vec![value])).at(js_span));
            }
            ty::Closure(def_id, _) | ty::CoroutineClosure(def_id, _) => self.drop_taken(*def_id, span, made, out)?,
            // An `async` block's future drops what it took as its body ends,
            // in that body; its body has run, as a promise's does, by the
            // time anything else drops it (ADR 0029): there it's nothing.
            ty::Coroutine(def_id, _) if self.frame.owner == *def_id => self.drop_taken(*def_id, span, made, out)?,
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
                        facts: js::Facts::NONE,
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
                        facts: js::Facts::NONE,
                        kind: js::ExprKind::OptionalCall(Box::new(drop), vec![value]),
                        span: js_span,
                    })
                    .at(js_span),
                );
            }
            ty::Array(item, _) | ty::Slice(item) => self.drop_items(value, *item, span, made, out)?,
            ty::Tuple(items) => {
                let mut parts = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let part = self.project(value.clone(), ty, i);
                    let mut dropped = Vec::new();
                    self.drop_in(part, item, span, made, &mut dropped)?;
                    parts.push(dropped);
                }
                out.extend(each_dropped(parts, js_span));
            }
            ty::Adt(adt, args) => {
                // Its own `drop`, then each field's, each run though one
                // before it panics, as Rust's drop glue goes on (#416).
                let mut parts = Vec::new();
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
                    parts.push(vec![StmtKind::Expr(call).at(js_span)]);
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
                    let test = Expr::bin(Op::LooseNe, value, Expr::null());
                    parts.push(vec![StmtKind::If(test, some, None).at(js_span)]);
                } else if adt.is_struct() {
                    for (i, field) in adt.non_enum_variant().fields.iter().enumerate() {
                        let part = self.project(value.clone(), ty, i);
                        let field = self.field_ty(field, args);
                        // A `dyn` last field: through the `impl` its struct
                        // was given (ADRs 0409, 0436).
                        if let ty::Dynamic(..) = field.kind() {
                            let dictionary = Expr::member(value.clone(), js::DICTIONARY);
                            parts.push(vec![dyn_dropped(dictionary, part, js_span)]);
                            continue;
                        }
                        let mut dropped = Vec::new();
                        self.drop_in(part, field, span, made, &mut dropped)?;
                        parts.push(dropped);
                    }
                } else if adt.is_enum() {
                    // One part, each variant's test: only one of them runs.
                    let mut variants = Vec::new();
                    for variant in adt.variants() {
                        let mut fields = Vec::new();
                        // An untagged enum's variant is its payload (ADR 0214).
                        let untagged = self.untagged(ty).is_some();
                        for (i, field) in variant.fields.iter().enumerate() {
                            let part = match untagged {
                                true => value.clone(),
                                false => Expr::member(value.clone(), variant_field(self.tcx, variant, i)),
                            };
                            let mut dropped = Vec::new();
                            self.drop_in(part, self.field_ty(field, args), span, made, &mut dropped)?;
                            fields.push(dropped);
                        }
                        let fields = each_dropped(fields, js_span);
                        if fields.is_empty() {
                            continue;
                        }
                        let test = match adt.variants().len() {
                            1 => Expr::bool(true),
                            _ => self.variant_test(
                                value.clone(),
                                ty,
                                *adt,
                                adt.variant_index_with_id(variant.def_id),
                                false,
                            ),
                        };
                        variants.push(StmtKind::If(test, fields, None).at(js_span));
                    }
                    parts.push(variants);
                } else if !adt.is_union() {
                    // A union's fields are never dropped, which Rust holds
                    // to what has no destructor, or a `ManuallyDrop` (ADR 0406).
                    return Err(self.unsupported(span, &format!("dropping `{ty}`")));
                }
                out.extend(each_dropped(parts, js_span));
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
        out.push(items_dropped(items, js::Pattern::Name(name), body, js_span));
        Ok(())
    }

    /// Where each of this body's momentary borrows is called, its call's
    /// `fun`'s span: a guard held only while nothing can ask whether its
    /// cell is borrowed (ADR 0328), of the facts found already, which MIR's
    /// lowering asks, found here where they aren't yet.
    pub(in crate::lower) fn momentary_borrows(&self) -> HashSet<Span> {
        let key = std::ptr::from_ref(self.frame.thir) as usize;
        let spans =
            |facts: &Facts| -> HashSet<Span> { facts.momentary.iter().map(|&fun| self.frame.thir[fun].span).collect() };
        match self.drop_state.facts.get(&key) {
            Some(facts) => spans(facts),
            None => spans(&find_facts(
                &self.drop_query(),
                self.krate.cell_borrows,
                self.frame.thir,
                self.frame.owner,
            )),
        }
    }

    /// This body's facts, found the first time they're asked for; what it
    /// does that isn't supported is an error then.
    pub(in crate::lower) fn drop_facts(&mut self) -> R<Rc<Facts>> {
        let key = std::ptr::from_ref(self.frame.thir) as usize;
        if let Some(facts) = self.drop_state.facts.get(&key) {
            return Ok(facts.clone());
        }
        let facts = Rc::new(find_facts(
            &self.drop_query(),
            self.krate.cell_borrows,
            self.frame.thir,
            self.frame.owner,
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
        let facts = find_facts(
            &self.drop_query(),
            self.krate.cell_borrows,
            self.frame.thir,
            self.frame.owner,
        );
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

/// Each of `parts`' drops in turn, as Rust's drop glue runs them: one that
/// may throw, a destructor's panic, the rest after it in its `finally`, so
/// they still run, `try { boom_drop(pair.first); } finally {
/// loudDrop_drop(pair.second); }` (#416). One that can't, as it is.
fn each_dropped(parts: Vec<Vec<Stmt>>, js_span: js::Span) -> Vec<Stmt> {
    let mut rest: Vec<Stmt> = Vec::new();
    for part in parts.into_iter().rev().filter(|part| !part.is_empty()) {
        rest = match rest.is_empty() || !may_throw(&part) {
            true => part.into_iter().chain(rest).collect(),
            false => vec![StmtKind::Try(part, rest).at(js_span)],
        };
    }
    rest
}

/// Each of `items` dropped by `body`, binding it to `pattern`: `for (const
/// item of v) { .. }`, or of a `body` that may throw, `$dropItems(v,
/// loudDrop_drop)`, which drops the rest of them after one throws, as
/// Rust's drop of a slice goes on past a panicking item (#416).
fn items_dropped(items: Expr, pattern: js::Pattern, body: Vec<Stmt>, js_span: js::Span) -> Stmt {
    if !may_throw(&body) {
        return StmtKind::ForOf {
            label: None,
            pattern,
            mutable: false,
            iterable: items,
            body,
            keys: false,
        }
        .at(js_span);
    }
    // `(item) => { dropItem(item); }` is `dropItem`.
    let drop = match (&pattern, body.as_slice()) {
        (
            js::Pattern::Name(name),
            [
                Stmt {
                    kind:
                        StmtKind::Expr(Expr {
                            kind: js::ExprKind::Call(callee, given),
                            ..
                        }),
                    ..
                },
            ],
        ) if matches!(callee.kind, js::ExprKind::Var(_))
            && matches!(given.as_slice(), [Expr { kind: js::ExprKind::Var(n), .. }] if n == name) =>
        {
            (**callee).clone()
        }
        _ => Expr::arrow(vec![pattern], body),
    };
    StmtKind::Expr(Expr::call_helper("$dropItems", vec![items, drop])).at(js_span)
}

/// Whether a drop's `stmts` may throw: a statement that may, or a branch's,
/// a loop's over what's made, or a `try`'s.
pub(super) fn may_throw(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::If(test, then, otherwise) => {
            test.may_throw() || may_throw(then) || otherwise.as_deref().is_some_and(may_throw)
        }
        StmtKind::ForOf { iterable, body, .. } => iterable.may_throw() || may_throw(body),
        StmtKind::Try(body, finally) => may_throw(body) || may_throw(finally),
        _ => !stmt.cannot_throw(),
    })
}
