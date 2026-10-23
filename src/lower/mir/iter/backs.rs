//! A chain stepped from its back: `rev` and what it reverses, and what a
//! source keeps of its back.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `rev()` of what holds what has a destructor, closed to drop it:
    /// `$rev(it)` of an array's items, stepped from its back, closed to drop
    /// what's left front to back, as Rust's `Rev` drops what it holds (ADR
    /// 0516). `None` of another.
    fn closed_rev(&mut self, receiver: Expr, iter_ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let item = self.iterator_item(iter_ty).expect("an iterator's item");
        let item_drop = self.std_drop_function(item, span)?;
        if item_drop.is_none() || self.array_source(iter_ty).is_none() {
            return Ok(None);
        }
        let it = match self.array_source(iter_ty) {
            Some(_) if matches!(receiver.kind, js::ExprKind::Var(_) | js::ExprKind::Member(..)) => receiver,
            Some(_) => self.std_iterator(receiver, iter_ty),
            None => self.as_js_iterator(receiver, iter_ty, span)?,
        };
        let it = self.closing(it, iter_ty, span)?;
        Ok(Some(Expr::call_helper("$rev", vec![it])))
    }

    /// `rev()` of a std iterator neither a range nor the crate's own: of
    /// items made already, an array's, a `$iter` of them reversed; a lazy
    /// chain made of its items reversed where its closures run in the same
    /// order; else what steps it from its back, reversed (ADR 0535).
    pub(super) fn rev_items(&mut self, receiver: Expr, iter_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        if let Some(closed) = self.closed_rev(receiver.clone(), iter_ty, span)? {
            return Ok(closed);
        }
        // A `$iter` of an array, `steps_an_array`'s, the one list (ADR 0530).
        let items = match self.steps_an_array(iter_ty).then_some(()) {
            // One a variable or a field holds, or a function gives, its
            // `$iter` or its array, stepped or not: what's left of it.
            Some(_)
                if matches!(receiver.kind, js::ExprKind::Var(_) | js::ExprKind::Member(..))
                    || given_by_a_call(&receiver) =>
            {
                Expr::call_helper("$rest", vec![receiver])
            }
            Some(_) => match self.std_iterator(receiver, iter_ty) {
                // Of one just made, the array, which `toReversed`
                // copies anyway.
                it if crate::lower::iterators::is_stepped_items(&it) => crate::lower::iterators::iterated(it),
                it => Expr::call_helper("$rest", vec![it]),
            },
            // A `$iter` of an array, just made: the array.
            None if crate::lower::iterators::is_stepped_items(&receiver) => crate::lower::iterators::iterated(receiver),
            // Of such an array's skipped and taken from, the array
            // sliced: no closure runs over it.
            None if counted_chain(&receiver) => sliced(&receiver),
            // A chain over an array's items of callbacks that read
            // no index: the chain over them reversed, each called
            // from the back, as Rust's `next_back` calls them.
            None if let Some(reversed) = reversed_inward(&receiver) => {
                return Ok(reversed);
            }
            None => match from_iterator(receiver) {
                Ok(items) => Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items]),
                // Of a lazy one, `rev()`, which rustc found steps from its
                // back, a map's say: what steps it from there, as Rust's `Rev`
                // does, its closures run as `next_back` runs them (ADR 0535).
                Err(lazy) => {
                    return Ok(Expr::call_helper(
                        "$rev",
                        vec![self.as_double_ended(lazy, iter_ty, span)?],
                    ));
                }
            },
        };
        let items = Expr::call(Expr::member(items, "toReversed"), Vec::new());
        Ok(self.stepped_items(items))
    }

    /// An adapter cloned apart (ADR 0525), `name`'s, `map`'s or `chain`'s,
    /// of its two `parts`, a source and its function or two sources: made by
    /// a function of them, `make`, which `build` writes the adapter of, and
    /// which its clone, `$cloning`'s, calls again with a clone of each, as a
    /// closure cloned apart is (ADR 0522). Its sources' positions are all its
    /// state, a JS iterator helper's, so a clone of it is where it is.
    pub(super) fn cloned_apart_adapter(
        &mut self,
        (name, parts, names): (&str, [Expr; 2], [&str; 2]),
        (part_tys, output): ([Ty<'tcx>; 2], Ty<'tcx>),
        lent: bool,
        span: Span,
        made: &mut Vec<Stmt>,
        build: impl FnOnce(&mut Self, Expr, Expr) -> R<Expr>,
    ) -> R<Expr> {
        let dropping = self
            .iterator_item(output)
            .is_some_and(|item| self.drops(item) != crate::lower::drops::Drops::Nothing);
        if lent || part_tys[0].is_ref() || dropping {
            return Err(self.unsupported(
                span,
                &format!("cloning a `{output}` that's lent, or of items with a destructor"),
            ));
        }
        // Of generic code's source or function, cloned by its `Clone`'s
        // dictionary, which it's given only where it's bound by one.
        let unbound = part_tys.into_iter().find(|&ty| {
            self.is_unknown(ty)
                && self
                    .evidence_for(ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]))
                    .is_none()
        });
        if let Some(ty) = unbound {
            return Err(self.unsupported(
                span,
                &format!("cloning a `{output}` made where `{ty}` isn't bound by `Clone`"),
            ));
        }
        let (a, b) = (self.fresh(names[0]), self.fresh(names[1]));
        // `makeMap`, `makeChain`.
        let make = self.fresh(&format!("make{}{}", name[..1].to_uppercase(), &name[1..]));
        let adapter = build(self, Expr::var(&a), Expr::var(&b))?;
        let mut cloned = Vec::new();
        let clones = vec![
            self.clone_value(Expr::var(&a), part_tys[0], span, &mut cloned)?,
            self.clone_value(Expr::var(&b), part_tys[1], span, &mut cloned)?,
        ];
        cloned.push(StmtKind::Return(Some(Expr::call(Expr::var(&make), clones))).at(js::Span::NONE));
        let carried = Expr::call_helper("$cloning", vec![adapter, Expr::arrow(Vec::new(), cloned)]);
        let maker = Expr::arrow(
            vec![a.as_str().into(), b.as_str().into()],
            vec![StmtKind::Return(Some(carried)).at(js::Span::NONE)],
        );
        made.push(StmtKind::Let(make.clone(), Some(maker)).at(self.js_span(span)));
        let [first, second] = parts;
        Ok(Expr::call(Expr::var(&make), vec![first, second]))
    }

    /// Whether `next_back()` of an iterator of type `iter_ty` is `$nextBack`'s:
    /// a `$iter`, of an array's iterator or a `Peekable` of one, or a generic
    /// or a `dyn` one that knows its back (ADR 0429).
    /// What keeps `receiver`'s back, of type `iter_ty`, where it knows its
    /// back (ADR 0455): `by_ref()`'s, itself, a `$iter` or a `$iterator`,
    /// which closing doesn't end, not lent; a map's, `$mapEnds`, which steps
    /// it from either end and doesn't close it. `None` of another.
    pub(super) fn back_kept(
        &mut self,
        method: Option<IterMethod>,
        receiver: &Expr,
        (iter_ty, output): (Ty<'tcx>, Ty<'tcx>),
        (values, arg_tys): (&mut std::vec::IntoIter<Expr>, &[Ty<'tcx>]),
        span: Span,
    ) -> R<Option<Expr>> {
        Ok(Some(match method {
            // One a struct holds, itself, which closing doesn't end (ADR 0542).
            Some(IterMethod::ByRef) if self.explicit_adapter(iter_ty) => receiver.clone(),
            // A range's, the range, whose `&mut` it is: what steps it after
            // moves its start, `$rangeNext(it)`, as Rust's does, not a JS
            // iterator of it, which would be stepped as a range again.
            Some(IterMethod::ByRef) if self.range_kind(iter_ty).is_some() => receiver.clone(),
            Some(IterMethod::ByRef) if self.knows_its_back(iter_ty) => {
                self.as_double_ended(receiver.clone(), iter_ty, span)?
            }
            // A map of one that knows its length, too (ADR 0456).
            // Of a type parameter given as it is, of the JS iterator made of
            // it (ADR 0488).
            Some(IterMethod::Map) if self.knows_its_back(output) || self.knows_its_length(output) => {
                let (back, len) = (self.knows_its_back(output), self.knows_its_length(output));
                // One the crate steps from its back, of a range made knowing
                // it; a source given as it is is given it where it's asked
                // (ADR 0545).
                let it = match self.given_js_iterator(receiver.clone(), iter_ty, back, len) {
                    Some(it) => it,
                    None if self.stepped_from_its_back(output) && self.range_kind(iter_ty).is_some() => {
                        self.as_double_ended(receiver.clone(), iter_ty, span)?
                    }
                    None if self.is_user_iterator(iter_ty) => {
                        self.user_iterator_knowing(receiver.clone(), iter_ty, (back, len), span)?
                    }
                    None => receiver.clone(),
                };
                // An owning source of items with a destructor, closed to drop
                // what it hasn't given, as a map's is (ADR 0465).
                let it = self.closing(it, iter_ty, span)?;
                let own = arg_tys.get(1).is_some_and(|ty| self.is_crate_fn_item(*ty));
                let f = self.unary_callback(values.next().expect("the function"), own);
                Expr::call_helper("$mapEnds", vec![it, f])
            }
            // An enumeration of one, its index from the back where it is
            // (ADR 0528); and one a struct holds, which keeps its source and
            // count, made of a source as capable as its type is, a range's
            // knowing its back, a generic one's given it where it's asked
            // (ADR 0542).
            Some(IterMethod::Enumerate)
                if self.knows_its_back(output) || self.knows_its_length(output) || self.explicit_adapter(output) =>
            {
                // Of the crate's, what it can be made to know.
                let back = self.knows_its_back(iter_ty) || self.user_double_ended(iter_ty);
                let len = self.knows_its_length(iter_ty) || self.user_exact_size(iter_ty);
                let it = match self.given_js_iterator(receiver.clone(), iter_ty, back, len) {
                    Some(it) => it,
                    None if self.explicit_adapter(output) => self.held_source(receiver.clone(), iter_ty, span)?,
                    None if self.is_user_iterator(iter_ty) => {
                        self.user_iterator_knowing(receiver.clone(), iter_ty, (back, len), span)?
                    }
                    None => receiver.clone(),
                };
                Expr::call_helper("$enumerate", vec![it])
            }
            // A take or a skip of a kind explicit, which keeps its source and
            // how many it has left, `n`, `$taking` and `$skipping` (ADR 0542),
            // given its items' drop, of what it skips past.
            Some(method @ (IterMethod::Take | IterMethod::Skip)) if self.explicit_adapter(output) => {
                let (back, len) = (self.knows_its_back(iter_ty), self.knows_its_length(iter_ty));
                let it = match self.given_js_iterator(receiver.clone(), iter_ty, back, len) {
                    Some(it) => it,
                    None => self.held_source(receiver.clone(), iter_ty, span)?,
                };
                let mut args = vec![it, values.next().expect("how many")];
                if let Some(item) = self.iterator_item(output) {
                    args.extend(self.std_drop_function(item, span)?);
                }
                let helper = match method {
                    IterMethod::Take => "$taking",
                    _ => "$skipping",
                };
                Expr::call_helper(helper, args)
            }
            // A filter of one, of items with nothing to drop as it passes
            // them (ADR 0526).
            Some(IterMethod::Filter)
                if self.knows_its_back(output)
                    && self
                        .iterator_item(output)
                        .is_none_or(|item| self.drops(item) == crate::lower::drops::Drops::Nothing) =>
            {
                let it = match self.given_js_iterator(receiver.clone(), iter_ty, true, false) {
                    Some(it) => it,
                    None if self.stepped_from_its_back(output) && self.range_kind(iter_ty).is_some() => {
                        self.as_double_ended(receiver.clone(), iter_ty, span)?
                    }
                    None if self.is_user_iterator(iter_ty) => {
                        self.user_iterator_knowing(receiver.clone(), iter_ty, (true, false), span)?
                    }
                    None => receiver.clone(),
                };
                let own = arg_tys.get(1).is_some_and(|ty| self.is_crate_fn_item(*ty));
                let p = self.unary_callback(values.next().expect("the test"), own);
                Expr::call_helper("$filterEnds", vec![it, p])
            }
            _ => return Ok(None),
        }))
    }

    /// `it.nth_back(n)`, `it.rfold(init, f)` or `it.rfind(p)`, of its
    /// values, the iterator first, of type `iter_ty`, giving `output`: its
    /// helper, stepping `it` from its back (ADR 0455). Of items with a
    /// destructor, what `nth_back` and `rfind` pass over is dropped, as
    /// std's are; `rfold` gives each to its function. Of items that could
    /// look like `None`, what's found is their `Some` (ADR 0051).
    pub(super) fn back_step(
        &mut self,
        method: IterMethod,
        values: Vec<Expr>,
        (iter_ty, output): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
    ) -> R<Expr> {
        let drop = match self.iterator_item(iter_ty) {
            Some(item) if method != IterMethod::RFold => self.std_drop_function(item, span)?,
            _ => None,
        };
        let boxed = self.option_of(output).is_some_and(|item| self.boxed_payload(item));
        let mut values = values.into_iter();
        let it = values.next().expect("the receiver");
        // Of what has a back, the iterator it steps, as `next_back`'s;
        // another made one, a range's of its bounds, stepped in place.
        let it = match self.range_kind(iter_ty).is_none() && self.has_a_back(iter_ty) {
            true => it,
            false => self.as_double_ended(it, iter_ty, span)?,
        };
        // `try_rfold`: `try_fold` of it reversed, which gives its items as
        // its own function does (ADR 0531).
        if method == IterMethod::TryRFold {
            let reversed = Expr::call_helper("$rev", vec![it]);
            let mut arg = || values.next().expect("rustc checked the arguments");
            return self.try_folded(reversed, method, &mut arg, (false, output), span);
        }
        let mut given = vec![it];
        given.extend(values);
        if boxed || drop.is_some() {
            given.push(match boxed {
                true => Expr::helper("$some"),
                false => Expr::undefined(),
            });
        }
        given.extend(drop);
        let helper = match method {
            IterMethod::NthBack => "$nthBack",
            IterMethod::RFold => "$foldBack",
            _ => "$findBack",
        };
        Ok(Expr::call_helper(helper, given))
    }

    /// The source of an adapter a struct holds (ADR 0542), `receiver` of type
    /// `iter_ty`, as capable as its type is: a range's knowing its back, and
    /// an exclusive one's its length; another's, its JS iterator.
    pub(in crate::lower) fn held_source(&mut self, receiver: Expr, iter_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        match self.range_kind(iter_ty) {
            Some(crate::lower::std_types::range::RangeKind::Exclusive) => {
                self.as_exact_size(receiver, iter_ty, true, span)
            }
            Some(_) => self.as_double_ended(receiver, iter_ty, span),
            None => self.as_js_iterator(receiver, iter_ty, span),
        }
    }

    pub(super) fn has_a_back(&self, iter_ty: Ty<'tcx>) -> bool {
        self.steps_an_array(iter_ty) || self.knows_its_back(iter_ty) || self.held_knows_its_back(iter_ty)
    }

    /// `rev()` of `it`, an iterator of type `iter_ty` that steps from its
    /// back: of a generic or a `dyn` one, a JS iterator stepped from its
    /// back, its `Some` boxed where it may look like `None` (ADR 0051); of
    /// one that knows its back, `$rev(it)` (ADRs 0429, 0516), of a type
    /// parameter's given as it is, by its `DoubleEndedIterator` dictionary's
    /// `next_back` (ADR 0487); and of a zip of two arrays' iterators, made
    /// here, `$zipBack` of them, of items with nothing to drop (ADR 0493).
    /// `None` of another.
    pub(super) fn reversed(&self, it: &Expr, iter_ty: Ty<'tcx>) -> Option<Expr> {
        if let Some((a, b)) = crate::lower::recognition::zipped_sources(self.tcx, iter_ty) {
            let js::ExprKind::Call(zip, sides) = &it.kind else {
                return None;
            };
            let leaves_none = |side: Ty<'tcx>| {
                self.steps_an_array(side)
                    && self
                        .iterator_item(side)
                        .is_some_and(|item| self.drops(item) == crate::lower::drops::Drops::Nothing)
            };
            return (zip.helper_name() == Some("$lazyZip") && sides.len() == 2 && leaves_none(a) && leaves_none(b))
                .then(|| Expr::call_helper("$zipBack", sides.clone()));
        }
        let boxed = self.iterator_item(iter_ty).is_none_or(|item| self.boxed_payload(item));
        let double_ended = self.recognition().double_ended_iterator()?;
        let next_back = crate::lower::recognition::extension_entry(self.tcx, double_ended);
        let back = Expr::member(self.iterator_evidence(iter_ty, double_ended)?, next_back);
        Some(crate::lower::iterators::stepped(it.clone(), back, boxed, None))
    }
}

/// `$iter(v).map(f).filter(p)`, a lazy chain over an array's items whose
/// callbacks read no index, made of the items reversed:
/// `$iter(v.toReversed()).map(f).filter(p)`. Rust's `Map` and `Filter`
/// call theirs from the back as they're stepped from it, so each is called
/// on the same items, in the same order (ADR 0164). An index, an
/// `enumerate`'s `(x, i) => [i, x]`, would count from the other end, so
/// it's made of the array's items first, where nothing's called before.
fn reversed_inward(e: &Expr) -> Option<Expr> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    // `$iter(v)` of an array just made iterable: `$iter(v.toReversed())`.
    if crate::lower::iterators::is_stepped_items(e) {
        let items = crate::lower::iterators::iterated(e.clone());
        return Some(Expr::call(
            (**callee).clone(),
            vec![Expr::call(Expr::member(items, "toReversed"), Vec::new())],
        ));
    }
    // A range made here, stepped from its start, `$iterator({ start, end },
    // $rangeNext)`: stepped from its end, `$rangeNextBack`, as Rust's range
    // is by `next_back`.
    if let Some(range) = crate::lower::iterators::stepped_range(e)
        && !range.back
    {
        let kind = match range.inclusive {
            true => crate::lower::std_types::range::RangeKind::Inclusive,
            false => crate::lower::std_types::range::RangeKind::Exclusive,
        };
        let back = Expr::helper(kind.step(true)?);
        return Some(Expr::call((**callee).clone(), vec![args[0].clone(), back]));
    }
    let js::ExprKind::Member(base, method) = &callee.kind else {
        return None;
    };
    // A function given as it is, a Rust one of the item alone, reads no index.
    let no_index = |f: &Expr| match &f.kind {
        js::ExprKind::Arrow(params, _) => params.len() == 1,
        js::ExprKind::Var(_) | js::ExprKind::Member(..) => true,
        _ => false,
    };
    match (method.as_str(), args.as_slice()) {
        // An index read of an array's items, nothing before it to call: the
        // array mapped, then reversed, `$iter(v.map((x, i) => [i, x])
        // .toReversed())`, each index its item's.
        ("map", [f]) if !no_index(f) && crate::lower::iterators::is_stepped_items(base) => {
            let js::ExprKind::Call(stepper, _) = &base.kind else {
                return None;
            };
            let items = Expr::call(
                Expr::member(crate::lower::iterators::iterated((**base).clone()), "map"),
                args.clone(),
            );
            Some(Expr::call(
                (**stepper).clone(),
                vec![Expr::call(Expr::member(items, "toReversed"), Vec::new())],
            ))
        }
        ("map" | "filter", [f]) if no_index(f) => {
            Some(Expr::call(Expr::member(reversed_inward(base)?, method), args.clone()))
        }
        _ => None,
    }
}

/// Whether `e` is what a function gives, the crate's own or a library's,
/// not a helper's or an array made here: of a `slice::Iter`'s type, its
/// `$iter` or an array, as it made it.
fn given_by_a_call(e: &Expr) -> bool {
    matches!(&e.kind, js::ExprKind::Call(callee, _) if callee.helper_name().is_none())
        && !crate::lower::iterators::made_array(e)
        && !crate::lower::iterators::is_js_iterator(e)
}
