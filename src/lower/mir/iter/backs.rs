//! A chain stepped from its back: `rev` and what it reverses, and what a
//! source keeps of its back.

use super::*;
use crate::lower::recognition::{OwnStep, Reach};

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
        // Of std's compiled struct, which knows its back only given it,
        // `$knowing`'s, made so (#999).
        if self.array_source(iter_ty).is_none() {
            if !self.reversed_in_order(iter_ty) {
                return Ok(None);
            }
            let it = match self.compiled_struct(self.reveal(iter_ty), &receiver) {
                true => self.as_double_ended(receiver, iter_ty, span)?,
                false => receiver,
            };
            return Ok(Some(crate::lower::iterators::reversed_by(it)));
        }
        let it = match self.array_source(iter_ty) {
            Some(_) if matches!(receiver.kind, js::ExprKind::Var(_) | js::ExprKind::Member(..)) => receiver,
            Some(_) => self.std_iterator(receiver, iter_ty),
            None => self.as_js_iterator(receiver, iter_ty, span)?,
        };
        let it = self.closing(it, iter_ty, span)?;
        Ok(Some(crate::lower::iterators::reversed_by(it)))
    }

    /// `rev()` of `receiver`, of type `iter_ty`, called on a `receiver_ty`,
    /// giving `output`: of the crate's iterator, `reversed_user`'s; of a
    /// range, stepped from its end; of items made already, an array's, a
    /// `$iter` of them; not of a lazy one, whose closures would run in
    /// another order (ADR 0164). What it steps is closed with it, which
    /// drops it: a compiled struct, a `fuse()`, by its own `return` (#225).
    /// One stepped from its back made knowing it (`reversed_knowing`). One
    /// made in place, as `rev`, `def_id`, is at `span` (`adapter_in_place`),
    /// is marked so first, which what makes it knowing its back asks of it
    /// (`made_in_place`).
    pub(super) fn rev_called(
        &mut self,
        receiver: Expr,
        (iter_ty, receiver_ty, output): (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>),
        rev: rustc_span::def_id::DefId,
        span: Span,
    ) -> R<Expr> {
        let (name, in_place) = (self.tcx.item_name(rev), self.adapter_in_place(rev, span));
        let unsupported = |cx: &Self| cx.unsupported(span, &format!("`{name}` of a `{iter_ty}`, from its MIR"));
        let reversed = match self.range_kind(iter_ty) {
            _ if self.is_user_iterator(iter_ty) => self.reversed_user(receiver, receiver_ty, span)?,
            // Of a type parameter given as it is that the crate steps from
            // its back (`reversed_made_knowing`), or that drops what it
            // holds: `$rev` of the JS iterator its dictionaries step, its
            // back theirs, which `$rev` steps it forward by, `nth_back` its
            // `nth`, as std's `Rev` does, closed to drop it, the value
            // itself, as `Rev` drops it (`$givenDrop`, #718, #971).
            None if (self.reversed_made_knowing(output) || self.closes_to_drop(receiver_ty))
                && let Some(given) = self.given_js_iterator(receiver.clone(), iter_ty, true, false) =>
            {
                crate::lower::iterators::reversed_by(self.closing(given, receiver_ty, span)?)
            }
            None if let Some(reversed) = self.reversed(&receiver, iter_ty) => reversed,
            Some(kind) if let Some(step) = kind.step(true) => self.stepped_by(receiver, Expr::helper(step)),
            Some(_) => return Err(unsupported(self)),
            // Of a `&mut`, `by_ref()`'s: what it's to, lent, stepped from
            // its back, its slice's steps its own (`slicing`), and never
            // closed, as a `Rev<&mut I>` drops nothing of its `I` (#755).
            None if receiver_ty.is_mutable_ptr() => {
                let lends = !std::ptr::eq(crate::lower::iterators::unlent(&receiver), &receiver);
                let back = match self.has_a_back(iter_ty, &receiver) {
                    true => receiver,
                    false => self.as_double_ended(receiver, iter_ty, span)?,
                };
                let lent = if lends { back } else { self.lent_iterator(back) };
                let lent = self.slicing(lent, iter_ty, true, span)?;
                crate::lower::iterators::reversed_by(lent)
            }
            None => self.rev_items(receiver, iter_ty, self.reversed_made_knowing(output), span)?,
        };
        let reversed = match in_place {
            true => reversed.in_place(),
            false => reversed,
        };
        let reversed = self.reversed_knowing(reversed, output, span)?;
        // Of a `skip(n)` or a `take(n)` that knows its length, made knowing
        // its back where it's made or std's struct given its compiled `len`:
        // marked so, as what's asked of it past its maker, its `len()`, reads
        // the mark, not its type (#814).
        let knows = |source: &Expr| {
            crate::lower::iterators::was_made_knowing_its_back(source)
                || crate::lower::iterators::knows_its_given_length(source)
        };
        Ok(match crate::lower::iterators::reversed_of(&reversed) {
            Some(source) if crate::lower::recognition::counted_source(self.tcx, iter_ty).is_some() && knows(source) => {
                crate::lower::iterators::made_knowing_its_back(reversed)
            }
            _ => reversed,
        })
    }

    /// `rev()` of a std iterator neither a range nor the crate's own: of
    /// items made already, an array's, a `$iter` of them reversed; a lazy
    /// chain made of its items reversed where its closures run in the same
    /// order; else what steps it from its back, reversed (ADR 0535). Of
    /// one made knowing its back (`knowing`, `reversed_made_knowing`), not
    /// a chain reversed inward whose form `reversal` can't read, a range's
    /// map's: what steps it from its back (#718).
    pub(super) fn rev_items(&mut self, receiver: Expr, iter_ty: Ty<'tcx>, knowing: bool, span: Span) -> R<Expr> {
        if let Some(closed) = self.closed_rev(receiver.clone(), iter_ty, span)? {
            return Ok(closed);
        }
        // A `$iter` of an array, `steps_an_array`'s, the one list (ADR 0530).
        let items = match self.steps_an_array(iter_ty).then_some(()) {
            // One a variable or a field holds, a tuple struct's `it[0]` too
            // (#1013), or a function gives, its `$iter` or its array,
            // stepped or not: what's left of it.
            Some(_)
                if matches!(
                    receiver.kind,
                    js::ExprKind::Var(_) | js::ExprKind::Member(..) | js::ExprKind::Index(..)
                ) || given_by_a_call(&receiver) =>
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
            None if let Some(reversed) = reversed_inward(&receiver)
                && !(knowing && self.reversal_of(&reversed, iter_ty).is_none()) =>
            {
                return Ok(reversed);
            }
            // Of another, lazy, a map's say, or the crate's own `next_back`'s
            // chain: what steps it from its back, as Rust's `Rev` does, its
            // closures run as `next_back` runs them (ADR 0535), never its
            // items from the front reversed (#382).
            // Closed to drop what it holds, a type parameter's by its drop,
            // as `Rev` drops it (ADR 0516, #650).
            // What steps it from its back is it, made again: made in place
            // where it was (`made_in_place`).
            None => {
                let in_place = crate::lower::iterators::made_in_place(&receiver);
                let it = self.as_double_ended(receiver, iter_ty, span)?;
                let it = self.closing(it, iter_ty, span)?;
                let it = match in_place {
                    true => it.in_place(),
                    false => it,
                };
                return Ok(crate::lower::iterators::reversed_by(it));
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
            // Of one that knows its back, lent knowing it, as std's `&mut I`
            // passes on `I`'s back and length, never its ownership (#749).
            Some(IterMethod::ByRef) if self.knows_its_back(iter_ty) => {
                let back = self.as_double_ended(receiver.clone(), iter_ty, span)?;
                self.lent_iterator(back)
            }
            // A copy that's its source, of numbers, stepped from its back:
            // its source made knowing its back, a `skip`'s or a `take`'s of a
            // slice's items, which JS's own `drop` and `take` don't (#646).
            Some(IterMethod::Copied | IterMethod::Cloned)
                if self.is_its_source(output)
                    && self.knows_its_back(output)
                    && !self.has_a_back(iter_ty, receiver)
                    && (self.recognition().is_iter_adapter(iter_ty)
                        || crate::lower::recognition::is_compiled_std_iterator(self.tcx, iter_ty)) =>
            {
                // Of std's compiled struct, a `fuse()`'s or a
                // `peekable()`'s, given its compiled back where it's made
                // (`knowing_held`), not the `next` alone its dictionary
                // has (#778).
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
                    // A range's, made knowing its length too where the map is,
                    // `$rangeLen`, a zip's `b` read by index (#775).
                    None if len && self.range_kind(iter_ty).is_some() => {
                        self.as_exact_size(receiver.clone(), iter_ty, back, span)?
                    }
                    None if back && !self.has_a_back(iter_ty, receiver) => {
                        self.as_double_ended(receiver.clone(), iter_ty, span)?
                    }
                    None => receiver.clone(),
                };
                // An owning source of items with a destructor, closed to drop
                // what it hasn't given, as a map's is (ADR 0465); not one
                // through a `&mut`, `by_ref()`'s, its owner's (#749).
                let it = self.closing(it, held, span)?;
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
                    || self.recognition().reaches_its_own(output, OwnStep::Nth, Reach::Sure)
                    // Or one whose `nth` drops what it passes as one slice,
                    // a reversed `Vec`'s, its source's (#277); or whose
                    // `count` is the crate's own, its source's (#505); or
                    // std's compiled one (#655).
                    || self.recognition().nths_a_slice(output)
                    || self.recognition().reaches_its_own(output, OwnStep::Count, Reach::Sure)
                    || self.counts_by_compiled_count(iter_ty, output) =>
            {
                // Of the crate's, what it can be made to know.
                // Of a type parameter's, what its bounds give
                // (`given_stepped`), as a `rev()` of it asks its back and its
                // length (#971).
                let of_param = matches!(iter_ty.peel_refs().kind(), ty::Param(_));
                let back = of_param || self.knows_its_back(iter_ty) || self.user_double_ended(iter_ty);
                let len = of_param || self.knows_its_length(iter_ty) || self.user_exact_size(iter_ty);
                let it = match self.given_stepped(receiver.clone(), iter_ty, back, len) {
                    // Closed to drop it, as a map's is, where it's owned, not
                    // through a `&mut` (#223, #225), a type parameter's
                    // value too (#971).
                    Some(it) => self.closing(it, held, span)?,
                    // The crate's, closed so too.
                    None if self.is_user_iterator(iter_ty) => {
                        let knowing = self.user_iterator_knowing(receiver.clone(), held, (back, len), span)?;
                        self.closing(knowing, held, span)?
                    }
                    // Of another the crate steps from its back whose JS knows
                    // neither its back nor its length, which its index from
                    // the back is counted by: a `take(n)`'s or a
                    // `step_by(n)`'s, `.take(n)` and `$stepBy` (#651), a
                    // map's, JS's own `.map(f)` (#778), std's compiled struct,
                    // a `Fuse`'s, given its compiled back and length (#383):
                    // made knowing them, or refused (`as_exact_size`), never
                    // stepped by what it lacks.
                    None if self.stepped_from_its_back(output) && !self.has_a_back(iter_ty, receiver) => {
                        let it = self.as_exact_size(receiver.clone(), iter_ty, true, span)?;
                        self.closing(it, held, span)?
                    }
                    None => self.closing(receiver.clone(), held, span)?,
                };
                crate::lower::iterators::enumerated_knowing(it)
            }
            // A `skip(n)` or a `take(n)` the crate steps from its back, held
            // where it's given or a chain's side, which MIR holds in a
            // variable before it's stepped (#718): made knowing its back and
            // its length where it's made, `$skipping` or `$taking` of its
            // source made knowing them and closed to drop what it holds, as a
            // map's is, given its items' drop, as `as_exact_size` makes one in
            // place. Not one through a `&mut`, its owner's.
            Some(IterMethod::Skip | IterMethod::Take) if self.made_stepped_from_its_back() && !held.is_ref() => {
                let skip = method == Some(IterMethod::Skip);
                let it = self.as_exact_size(receiver.clone(), iter_ty, true, span)?;
                let it = self.closing(it, held, span)?;
                let n = values.next().expect("how many");
                let mut given = vec![it, n];
                if let Some(item) = self.iterator_item(output) {
                    given.extend(self.std_drop_function(item, span)?);
                }
                crate::lower::iterators::made_knowing_its_back(Expr::call_helper(
                    if skip { "$skipping" } else { "$taking" },
                    given,
                ))
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
                    None if !self.has_a_back(iter_ty, receiver) => {
                        self.as_double_ended(receiver.clone(), iter_ty, span)?
                    }
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

    /// `value` of type `ty`, a local's a back step takes through its `&mut`
    /// (`SteppedLocal::Lent`): an adapter the recognizers made in place
    /// (`made_in_place`), a `skip(n)`'s or a `map(f)`'s, made knowing its
    /// back where it's made, as what
    /// steps it asks (`as_double_ended`), which refuses one of items with a
    /// destructor, and marked so (`made_knowing_its_back`), its mark carried
    /// by its local. Another as it is: one held, `$held` or closed to drop
    /// what it holds, std's struct (#718).
    /// `value`, written to `local` of type `ty`, a temporary given to what
    /// steps it from its back, back_stepped's `SteppedLocal::Lent`, made in
    /// place, which MIR holds to lend it or across what an argument runs
    /// before the step: made knowing its back where it's made, `$taking` or
    /// `$mapEnds` say, and marked so for its read (#718, #814, #811). Another
    /// as it is.
    pub(in crate::lower::mir) fn written_knowing_its_back(
        &mut self,
        local: rustc_middle::mir::Local,
        (value, ty): (Expr, Ty<'tcx>),
        span: Span,
    ) -> R<Expr> {
        match self.local_stepped_from_its_back(local) {
            Some(crate::lower::analysis::SteppedLocal::Lent) => self.made_knowing_where_held(value, ty, span),
            _ => Ok(value),
        }
    }

    fn made_knowing_where_held(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = self.reveal(ty);
        // Held by its owner, which drops it, `$held(it)` (#206): what it
        // holds made so, held again.
        if let Some(held) = crate::lower::iterators::held_of(&value) {
            let made = self.made_knowing_where_held(held.clone(), ty, span)?;
            return Ok(crate::lower::iterators::held_by_its_owner(made));
        }
        // One not made in place, a counted adapter's held as std's struct,
        // as it is: its step asks it as it is (#718).
        if !crate::lower::iterators::made_in_place(&value) {
            return Ok(value);
        }
        // It, made again to step from its back: made in place, as it was.
        let knowing = self.as_double_ended(value, ty, span)?;
        Ok(crate::lower::iterators::made_knowing_its_back(knowing).in_place())
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
        let knowing = match self.compiled_struct(revealed, &it) {
            true => self.knowing_held(it.clone(), revealed, span)?,
            false => None,
        };
        let it = match knowing {
            Some(knowing) => knowing,
            None if self.range_kind(iter_ty).is_none() && self.has_a_back(iter_ty, &it) => it,
            None => self.as_double_ended(it, iter_ty, span)?,
        };
        // `try_rfold`: `try_fold` of it reversed, which gives its items as
        // its own function does (ADR 0531).
        if method == IterMethod::TryRFold {
            let reversed = crate::lower::iterators::reversed_by(it);
            let mut arg = || values.next().expect("rustc checked the arguments");
            let folded = self.try_folded(reversed, method, &mut arg, (false, output), arg_tys, span)?;
            return Ok(Some(folded));
        }
        // `rfold`'s accumulator, which a stage's panic may strand, dropped by
        // what it's given, as `fold`'s is (#423).
        if method == IterMethod::RFold {
            let unwind = match arg_tys.get(1) {
                Some(&acc) => self.stranded_drop(iter_ty, acc, span)?,
                None => None,
            };
            // `rfold(self)` takes it: closed where its consumer's closing
            // drops it, at its end and as a panic unwinds, `consumer_closes`'
            // one answer, as `fold`'s is (#637, #650).
            let lent = arg_tys.first().is_some_and(|ty| ty.is_mutable_ptr());
            let dropping = (self.iterator_item(iter_ty))
                .is_some_and(|item| self.drops(item) != crate::lower::drops::Drops::Nothing);
            // Of a `&mut`, lent: its `rfold` is what it's to's `try_rfold`,
            // as std's `&mut I` gives it, never that one's own `rfold`, which
            // would take it from its owner (#747).
            let it = match self.consumer_closes(Some(method), iter_ty, lent, dropping) {
                true => self.closing(it, iter_ty, span)?,
                false if lent => self.lent_iterator(it),
                false => it,
            };
            // Of a function it moves into its fold, `$consume`'s (#734).
            let values: Vec<Expr> = values.collect();
            if let Some(f) = values.last()
                && let Some(ends) = self.consumed_ends(Some(method), arg_tys, f, span)?
            {
                let mut given = vec![it, Expr::str("rfold")];
                given.extend(values);
                given.push(unwind.unwrap_or_else(Expr::undefined));
                given.push(ends);
                return Ok(Some(Expr::call_helper("$consume", given)));
            }
            let mut given = vec![it];
            given.extend(values);
            given.extend(unwind);
            return Ok(Some(Expr::call_helper("$foldBack", given)));
        }
        // What `nth_back` and `advance_back_by` pass over, a `Vec`'s as one
        // slice, front to back, given that step by `$slicing`, where a
        // `Drain`'s drops each (#234, #704).
        let it = match method {
            IterMethod::NthBack | IterMethod::AdvanceBackBy => self.slicing(it, iter_ty, true, span)?,
            _ => it,
        };
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

    /// Whether an `enumerate()` of an iterator of type `iter_ty`, of type
    /// `enumeration`, counts by its source's `count`, std's compiled one, as
    /// `Enumerate::count` does: a held `Peekable`'s keeps what it peeked at
    /// till its source's `count` ends, or unwinds, then drops it (#655). Of
    /// one exact-sized and double-ended, whose enumeration a `rev()` made in
    /// place may step from its back, only where it's std's struct compiled
    /// wherever it's made (`enumerates_a_compiled_struct`), whose enumeration
    /// `back_stepped` records where it's stepped from its back, made knowing
    /// its back and length where it's made (#712); not another, which
    /// `as_double_ended` makes of its map form.
    fn counts_by_compiled_count(&self, iter_ty: Ty<'tcx>, enumeration: Ty<'tcx>) -> bool {
        let source = self.reveal(iter_ty);
        let traits = [
            self.recognition().double_ended_iterator(),
            self.recognition().exact_size_iterator(),
        ];
        let reversible = traits
            .into_iter()
            .all(|tr| tr.is_some_and(|tr| self.recognition().implements_trait(source.peel_refs(), tr)));
        (!reversible || crate::lower::recognition::enumerates_a_compiled_struct(self.tcx, enumeration))
            && self.compiled_step(source, std_item(self.tcx, StdItem::Iterator), "count")
    }

    /// Whether `value`, of type `iter_ty`, has a back by its form: an
    /// array's; or one whose type knows its back, made so by its maker, but
    /// std's compiled struct, which has one only where it's given its
    /// compiled back, `$knowing`'s, or made knowing it (#856); and never one
    /// whose form says it has none, JS's own `.map(f)` say, though its type
    /// says it may (`form_lacks_a_back`, #851): what's asked of it makes it
    /// knowing its back or refuses it, never steps a back it hasn't. A crate
    /// that steps its type from its back elsewhere made such a value as it
    /// is, a `cloned()`'s (#850), std's struct (#856).
    pub(super) fn has_a_back(&self, iter_ty: Ty<'tcx>, value: &Expr) -> bool {
        // Of a value made before: whether its type knows its back by its
        // structure, not whether what's being made is to be made so (#851).
        let structural = || {
            let outer = self.asking_made.replace(true);
            let knows = self.knows_its_back(iter_ty);
            self.asking_made.set(outer);
            knows
        };
        self.steps_an_array(iter_ty)
            || structural()
                && !crate::lower::iterators::form_lacks_a_back(value)
                && (!self.compiled_struct(self.reveal(iter_ty), value)
                    || crate::lower::iterators::is_knowing(value)
                    || crate::lower::iterators::was_made_knowing_its_back(value))
    }

    /// `rev()` of `it`, an iterator of type `iter_ty` that steps from its
    /// back: of a generic or a `dyn` one, a JS iterator stepped from its
    /// back, its `Some` boxed where it may look like `None` (ADR 0051); of
    /// one that knows its back, `$rev(it)` (ADRs 0429, 0516), of a type
    /// parameter's given as it is, by its `DoubleEndedIterator` dictionary's
    /// `next_back` (ADR 0487). A zip made here is `as_double_ended`'s,
    /// `$zipEnds` (ADR 0493, #740). `None` of another.
    pub(super) fn reversed(&self, it: &Expr, iter_ty: Ty<'tcx>) -> Option<Expr> {
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
    // A `Rev`'s `rev()`, `$rev($rev(it))`: `it` itself, of an array's
    // items or a range, whose steps a `Rev` passes on or that give what
    // std's default does.
    // Not of another, the crate's or a type parameter's, whose own `last`
    // or `count` std's `Rev` doesn't pass on: `rev().rev().last()` asks
    // its `fold` (#621). Nor of an array's items closed to drop them, a
    // `Vec`'s `IntoIter`'s, whose own `last` takes its back, where a
    // `Rev`'s `last` folds, which a drop that panics shows (#722).
    if let Some(inner) = crate::lower::iterators::reversed_of(e)
        && let Some(it) = crate::lower::iterators::reversed_of(inner)
        && (crate::lower::iterators::is_stepped_items(it) || crate::lower::iterators::is_range_stepped(it))
    {
        *e = Expr {
            span: e.span,
            ..it.clone()
        };
        return;
    }
    if let Some(chain) = crate::lower::iterators::reversed_of(e)
        && !matches!(chain.kind, js::ExprKind::Var(_))
        && let Some(reversed) = reversed_inward(chain)
        && !crate::lower::iterators::is_reversed(&reversed)
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
    if crate::lower::iterators::closed(e).is_some_and(|closed| crate::lower::iterators::is_stepped_items(closed.it)) {
        return Some(crate::lower::iterators::reversed_by(e.clone()));
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
        return Some(crate::lower::iterators::stepped_range_by(range.range.clone(), back));
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
