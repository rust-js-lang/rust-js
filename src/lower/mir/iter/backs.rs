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
        if item_drop.is_none() {
            return Ok(None);
        }
        // A `Rev`, a `filter` or a `map` of one, closed already: stepped
        // from its back as it is, never its items taken out; tidy's
        // `reversed_in_place` turns it inward (#205). Not an `enumerate`'s,
        // whose index from the back is its length's, which `$closing`
        // doesn't know: refused as before.
        if self.array_source(iter_ty).is_none() {
            return Ok(self
                .reversed_in_order(iter_ty)
                .then(|| Expr::call_helper("$rev", vec![receiver])));
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
            // Of such an array's skipped and taken from, the array
            // sliced: no closure runs over it.
            None if counted_chain(&receiver) => sliced(&receiver),
            // A chain over an array's items of callbacks that read
            // no index: the chain over them reversed, each called
            // from the back, as Rust's `next_back` calls them.
            None if let Some(reversed) = reversed_inward(&receiver) => {
                return Ok(reversed);
            }
            // Of another, lazy, a map's say, or the crate's own `next_back`'s
            // chain: what steps it from its back, as Rust's `Rev` does, its
            // closures run as `next_back` runs them (ADR 0535), never its
            // items from the front reversed (#382).
            None => {
                return Ok(Expr::call_helper(
                    "$rev",
                    vec![self.as_double_ended(receiver, iter_ty, span)?],
                ));
            }
        };
        let items = crate::lower::iterators::reversed(items);
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
            self.clone_in_place(Expr::var(&a), part_tys[0], span, &mut cloned)?,
            self.clone_in_place(Expr::var(&b), part_tys[1], span, &mut cloned)?,
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
        // What it's given, a `&mut` of what it steps or not: what's closed to
        // drop it is what's owned.
        let held = arg_tys.first().copied().unwrap_or(iter_ty);
        Ok(Some(match method {
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
                // A source given as it is is given its back where it's asked;
                // one the crate steps from its back without one, a range or
                // another adapter, made knowing it, or refused (ADR 0545).
                let it = match self.given_stepped(receiver.clone(), iter_ty, back, len) {
                    Some(it) => it,
                    None if self.is_user_iterator(iter_ty) => {
                        self.user_iterator_knowing(receiver.clone(), held, (back, len), span)?
                    }
                    None if back && !self.has_a_back(iter_ty) => {
                        self.as_double_ended(receiver.clone(), iter_ty, span)?
                    }
                    None => receiver.clone(),
                };
                // An owning source of items with a destructor, closed to drop
                // what it hasn't given, as a map's is (ADR 0465).
                let it = self.closing(it, iter_ty, span)?;
                let own = arg_tys.get(1).is_some_and(|ty| self.is_called_as_written(*ty));
                let f = self.unary_callback(values.next().expect("the function"), own);
                Expr::call_helper("$mapEnds", vec![it, f])
            }
            // An enumeration of one, its index from the back where it is
            // (ADR 0528), made of a source as capable as its type is, a
            // range's knowing its back, a generic one's given it where it's
            // asked. One a struct holds is std's struct (ADR 0871).
            Some(IterMethod::Enumerate)
                if self.knows_its_back(output)
                    || self.knows_its_length(output)
                    || self.recognition().nths_its_own(output)
                    // Or one whose `nth` drops what it passes as one slice,
                    // a reversed `Vec`'s, its source's (#277).
                    || self.recognition().nths_a_slice(output) =>
            {
                // Of the crate's, what it can be made to know.
                let back = self.knows_its_back(iter_ty) || self.user_double_ended(iter_ty);
                let len = self.knows_its_length(iter_ty) || self.user_exact_size(iter_ty);
                let it = match self.given_js_iterator(receiver.clone(), iter_ty, back, len) {
                    Some(it) => it,
                    // Closed to drop it, as a map's is, where it's owned, not
                    // through a `&mut` (#223, #225).
                    None if self.is_user_iterator(iter_ty) => {
                        let knowing = self.user_iterator_knowing(receiver.clone(), held, (back, len), span)?;
                        self.closing(knowing, held, span)?
                    }
                    // Of one the crate steps from its back, std's compiled
                    // struct, a `Fuse`'s, given its compiled back and length,
                    // as `as_exact_size` makes it (#383).
                    None if self.stepped_from_its_back(output)
                        && let Some(knowing) = self.knowing_held(receiver.clone(), iter_ty, span)? =>
                    {
                        knowing
                    }
                    None => self.closing(receiver.clone(), held, span)?,
                };
                Expr::call_helper("$enumerate", vec![it])
            }
            // A filter of one (ADR 0526), given its items' drop, of items with
            // a destructor, which drops each it doesn't keep as it passes it
            // (ADR 0098), as a compiled `Rev` steps it from its back (ADR
            // 0871).
            Some(IterMethod::Filter) if self.knows_its_back(output) => {
                let it = match self.given_stepped(receiver.clone(), iter_ty, true, false) {
                    Some(it) => it,
                    None if self.is_user_iterator(iter_ty) => {
                        self.user_iterator_knowing(receiver.clone(), held, (true, false), span)?
                    }
                    None if !self.has_a_back(iter_ty) => self.as_double_ended(receiver.clone(), iter_ty, span)?,
                    None => receiver.clone(),
                };
                // Closed to drop what it holds, as a map's is, the crate's
                // iterator or a compiled `fuse()` of it (#225).
                let it = self.closing(it, held, span)?;
                let own = arg_tys.get(1).is_some_and(|ty| self.is_called_as_written(*ty));
                let p = self.unary_callback(values.next().expect("the test"), own);
                let mut given = vec![it, p];
                if let Some(item) = self.iterator_item(output) {
                    given.extend(self.std_drop_function(item, span)?);
                }
                Expr::call_helper("$filterEnds", given)
            }
            _ => return Ok(None),
        }))
    }

    /// `rev()` of the crate's iterator, `receiver` of type `receiver_ty`:
    /// its own `next_back` and back methods, through a `&mut` as std passes
    /// them on (#217), closed to drop it where it's owned (#223); refused of
    /// one with no `next_back` of its own.
    pub(super) fn reversed_user(&mut self, receiver: Expr, receiver_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let iter_ty = receiver_ty.peel_refs();
        let Some(back) = self.own_next_back(iter_ty)? else {
            return Err(self.unsupported(span, &format!("`rev` of a `{iter_ty}`, from its MIR")));
        };
        let reversed = self.user_iterator(receiver, receiver_ty, back, "next_back", span)?;
        self.closing(reversed, receiver_ty, span)
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
        (iter_ty, output, arg_tys): (Ty<'tcx>, Ty<'tcx>, &[Ty<'tcx>]),
        span: Span,
    ) -> R<Option<Expr>> {
        let drop = match self.iterator_item(iter_ty) {
            Some(item) if method != IterMethod::RFold => self.std_drop_function(item, span)?,
            _ => None,
        };
        let boxed = self.option_of(output).is_some_and(|item| self.boxed_payload(item));
        let mut values = values.into_iter();
        let it = values.next().expect("the receiver");
        // Of what has a back, the iterator it steps, as `next_back`'s, std's
        // struct given its compiled back where it's asked, as `rev()`'s is
        // (ADR 0920); another made one, a range's of its bounds, stepped in
        // place.
        let revealed = self.reveal(iter_ty.peel_refs());
        let knowing = match self.compiled_struct_at(revealed, span) {
            true => self.knowing_held(it.clone(), revealed, span)?,
            false => None,
        };
        let it = match knowing {
            Some(knowing) => knowing,
            None if self.range_kind(iter_ty).is_none() && self.has_a_back(iter_ty) => it,
            None => self.as_double_ended(it, iter_ty, span)?,
        };
        // `try_rfold`: `try_fold` of it reversed, which gives its items as
        // its own function does (ADR 0531).
        if method == IterMethod::TryRFold {
            let reversed = Expr::call_helper("$rev", vec![it]);
            let mut arg = || values.next().expect("rustc checked the arguments");
            let folded = self.try_folded(reversed, method, &mut arg, (false, output), arg_tys, span)?;
            return Ok(Some(folded));
        }
        // `rfold`'s accumulator, which a stage's panic may strand, dropped by
        // what it's given, as `fold`'s is (#423).
        if method == IterMethod::RFold {
            let unwind = match arg_tys.get(1) {
                Some(&acc) => self.stranded_drop(&it, iter_ty, acc, span)?,
                None => None,
            };
            let mut given = vec![it];
            given.extend(values);
            given.extend(unwind);
            return Ok(Some(Expr::call_helper("$foldBack", given)));
        }
        let mut given = vec![it];
        given.extend(values);
        if boxed || drop.is_some() {
            given.push(match boxed {
                true => Expr::helper("$some"),
                false => Expr::undefined(),
            });
        }
        // Of one that owns an array's items and drops each it passes
        // over, not as one slice as a `Vec`'s does, `false` (#234).
        let each = matches!(method, IterMethod::NthBack | IterMethod::AdvanceBackBy)
            && drop.is_some()
            && self.passes_each_owned(iter_ty);
        given.extend(drop);
        if each {
            given.push(Expr::bool(false));
        }
        let helper = match method {
            IterMethod::NthBack => "$nthBack",
            IterMethod::AdvanceBackBy => "$advanceBackBy",
            _ => "$findBack",
        };
        Ok(Some(Expr::call_helper(helper, given)))
    }

    /// Whether `iter_ty` is an array's items under `Rev`s, `Filter`s and
    /// `Map`s alone: what `reversed_inward` steps from the back, each
    /// closure called on the same items in the same order (#205).
    fn reversed_in_order(&self, iter_ty: Ty<'tcx>) -> bool {
        use crate::lower::recognition::{Parted, cloned_parts, filtered_source, reversed_source};
        let mut ty = self.reveal(iter_ty);
        while self.array_source(ty).is_none() {
            let mapped = || match cloned_parts(self.tcx, ty) {
                Some((Parted::Map, source, _)) => Some(source),
                _ => None,
            };
            let Some(source) = reversed_source(self.tcx, ty)
                .or_else(|| filtered_source(self.tcx, ty))
                .or_else(mapped)
            else {
                return false;
            };
            ty = self.reveal(source);
        }
        true
    }

    /// A map's or a filter's source given as it is, `given_js_iterator`'s:
    /// its back where it's asked and its bounds here give one; of a type
    /// parameter whose bounds give none, forward alone, as the type it's
    /// mapped to is stepped from its back only where they do, which rustc
    /// asks (ADR 0545, amended): the `either` crate's `IterEither::fold`,
    /// whose `Map<R, _>` its `rfold` steps from the back.
    fn given_stepped(&self, value: Expr, iter_ty: Ty<'tcx>, back: bool, len: bool) -> Option<Expr> {
        let parameter = matches!(iter_ty.peel_refs().kind(), ty::Param(_));
        (self.given_js_iterator(value.clone(), iter_ty, back, len))
            .or_else(|| {
                parameter
                    .then(|| self.given_js_iterator(value.clone(), iter_ty, false, len))
                    .flatten()
            })
            .or_else(|| {
                parameter
                    .then(|| self.given_js_iterator(value, iter_ty, false, false))
                    .flatten()
            })
    }

    pub(super) fn has_a_back(&self, iter_ty: Ty<'tcx>) -> bool {
        self.steps_an_array(iter_ty) || self.knows_its_back(iter_ty)
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
/// `$rev(chain)` of a chain read in place after it was made, a
/// temporary's: `reversed_inward`'s of it, where it says (#205).
pub(in crate::lower::mir) fn reversed_in_place(e: &mut Expr) {
    // A `Rev`'s `rev()`, `$rev($rev(it))`: `it` itself.
    if let js::ExprKind::Call(callee, args) = &e.kind
        && callee.helper_name() == Some("$rev")
        && let [inner] = args.as_slice()
        && let js::ExprKind::Call(inner_callee, inner_args) = &inner.kind
        && inner_callee.helper_name() == Some("$rev")
        && let [it] = inner_args.as_slice()
    {
        *e = Expr {
            span: e.span,
            ..it.clone()
        };
        return;
    }
    if let js::ExprKind::Call(callee, args) = &e.kind
        && callee.helper_name() == Some("$rev")
        && let [chain] = args.as_slice()
        && !matches!(chain.kind, js::ExprKind::Var(_))
        && let Some(reversed) = reversed_inward(chain)
        && !matches!(&reversed.kind, js::ExprKind::Call(c, _) if c.helper_name() == Some("$rev"))
    {
        *e = Expr {
            span: e.span,
            ..reversed
        };
    }
}

fn reversed_inward(e: &Expr) -> Option<Expr> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    // An array's items closed to drop what's left, `$closing($iter(v),
    // drop)`: stepped from its back, `$rev(..)`, which drops what's left
    // front to back, as `closed_rev` makes it (ADR 0516, #205).
    if callee.helper_name() == Some("$closing") && args.first().is_some_and(crate::lower::iterators::is_stepped_items) {
        return Some(Expr::call_helper("$rev", vec![e.clone()]));
    }
    // `$iter(v)` of an array just made iterable: `$iter(v.toReversed())`.
    if crate::lower::iterators::is_stepped_items(e) {
        let items = crate::lower::iterators::iterated(e.clone());
        return Some(Expr::call(
            (**callee).clone(),
            vec![crate::lower::iterators::reversed(items)],
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
                vec![crate::lower::iterators::reversed(items)],
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
