//! A chain stepped from its back: `rev` and what it reverses, and what a
//! source keeps of its back.

use super::*;
use crate::lower::recognition::{OwnStep, Reach};
use crate::lower::temporary_names::{Base, Role};

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
            let it = match self.compiled_struct(iter_ty, &receiver) {
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
    pub fn rev_called(
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
            Some(kind) if let Some(step) = kind.step(true) => self.stepped_by(receiver, Expr::helper(step)),
            Some(_) => return Err(unsupported(self)),
            // Of a `&mut`, `by_ref()`'s, a type parameter's given as it is
            // too: what it's to, lent, stepped from its back, as std's `&mut
            // I` passes on of it, its `nth_back` its own, its `rfold` by its
            // `try_rfold` (`$lent`, ADR 0928), its slice's steps its own
            // (`slicing`), and never closed, as a `Rev<&mut I>` drops nothing
            // of its `I` (#755, #2139).
            None if receiver_ty.is_mutable_ptr() => {
                let lends = !std::ptr::eq(crate::lower::iterators::unlent(&receiver), &receiver);
                let back = match self.has_a_back(iter_ty, &receiver) {
                    true => receiver,
                    false => self.as_double_ended(receiver, iter_ty, span)?,
                };
                let lent = if lends {
                    back
                } else {
                    self.lent_iterator(back, receiver_ty)
                };
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
    pub(in crate::lower) fn rev_items(
        &mut self,
        receiver: Expr,
        iter_ty: Ty<'tcx>,
        knowing: bool,
        span: Span,
    ) -> R<Expr> {
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
            // Of a type parameter given as it is, the JS iterator its
            // dictionaries step, its back theirs (`as_double_ended`), which
            // `$rev` steps forward by, `nth_back` its `nth` and `rfold` its
            // `fold`, its own where it writes them, as std's `Rev` asks
            // them, wherever it's reached, a parameter's or what another
            // trait's method gives (#718, #2139, #2147).
            // Closed to drop what it holds, a type parameter's by its drop,
            // as `Rev` drops it (ADR 0516, #650, #971).
            None => {
                let it = self.as_double_ended(receiver, iter_ty, span)?;
                let it = self.closing(it, iter_ty, span)?;
                return Ok(crate::lower::iterators::reversed_by(it));
            }
        };
        let items = crate::lower::iterators::reversed(items);
        Ok(self.stepped_items_of(items, iter_ty))
    }

    /// An adapter cloned apart (ADR 0525), `name`'s, `map`'s, `chain`'s or
    /// `copied`'s, of its `parts`, a source and its function, two sources, or
    /// a source alone: made by a function of them, `make`, which `build`
    /// writes the adapter of, and which its clone, `$cloning`'s, calls again
    /// with a clone of each, as a closure cloned apart is (ADR 0522). Its
    /// sources' positions are all its state, a JS iterator helper's, so a
    /// clone of it is where it is.
    pub fn cloned_apart_adapter(
        &mut self,
        (name, parts, names): (&str, Vec<Expr>, &[Role]),
        (part_tys, output): (Vec<Ty<'tcx>>, Ty<'tcx>),
        lent: bool,
        span: Span,
        made: &mut Vec<Stmt>,
        build: impl FnOnce(&mut Self, Vec<Expr>) -> R<Expr>,
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
        let unbound = part_tys.iter().copied().find(|&ty| {
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
        let named: Vec<rust_js_program::js::Name> = names.iter().map(|&name| self.fresh(name)).collect();
        // `makeMap`, `makeChain`, `makeCopied`.
        let make = self.fresh(Base::Prefixed(Role::Make, &rust_js_program::casing::upper_first(name)));
        let adapter = build(self, named.iter().map(Expr::var).collect())?;
        let mut cloned = Vec::new();
        let mut clones = Vec::new();
        for (part, &ty) in named.iter().zip(&part_tys) {
            clones.push(self.clone_in_place(Expr::var(part), ty, span, &mut cloned)?);
        }
        cloned.push(StmtKind::Return(Some(Expr::call(Expr::var(&make), clones))).at(js::Span::NONE));
        let carried = Expr::call_helper("$cloning", vec![adapter, Expr::arrow(Vec::new(), cloned)]);
        let maker = Expr::arrow(
            named.iter().map(|part| part.clone().into()).collect(),
            vec![StmtKind::Return(Some(carried)).at(js::Span::NONE)],
        );
        made.push(StmtKind::Let(make.clone(), Some(maker)).at(self.js_span(span)));
        Ok(Expr::call(Expr::var(&make), parts))
    }

    /// Whether `next_back()` of an iterator of type `iter_ty` is `$nextBack`'s:
    /// a `$iter`, of an array's iterator or a `Peekable` of one, or a generic
    /// or a `dyn` one that knows its back (ADR 0429).
    /// What keeps `receiver`'s back, of type `iter_ty`, where it knows its
    /// back (ADR 0455): `by_ref()`'s, itself, a `$iter` or a `$iterator`,
    /// which closing doesn't end, lent; else the adapter, a copy's, a map's
    /// `$mapEnds`, an enumeration's, a `skip(n)`'s or a `take(n)`'s, a
    /// filter's `$filterEnds`, of its source made knowing what it's asked
    /// (`back_kept_source`), closed here, once, to drop what it holds, as
    /// the adapter holds it (ADR 0465), where it's owned, not through a
    /// `&mut` (#749): no arm makes a source it leaves unclosed (#2152).
    /// `None` of another.
    pub fn back_kept(
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
        let Some(kept) = self.back_kept_source(method, receiver, (iter_ty, output, held), span)? else {
            return Ok(None);
        };
        let (it, over) = match kept {
            // The one exception: `by_ref()`'s, lent, which its owner drops.
            BackKept::Lent(it) => return Ok(Some(it)),
            BackKept::Held(it, over) => (self.closing(it, held, span)?, over),
        };
        let own = arg_tys.get(1).is_some_and(|ty| self.is_called_as_written(*ty));
        // Its items' drop, which a `skip(n)`, a `take(n)` or a filter drops
        // each it passes over by.
        let dropping = |cx: &mut Self, mut given: Vec<Expr>| -> R<Vec<Expr>> {
            if let Some(item) = cx.iterator_item(output) {
                given.extend(cx.std_drop_function(item, span)?);
            }
            Ok(given)
        };
        Ok(Some(match over {
            Over::Copy => it,
            Over::Map { len } => {
                let f = self.unary_callback(values.next().expect("the function"), own);
                let mapped = Expr::call_helper("$mapEnds", vec![it, f]);
                match len {
                    true => crate::lower::iterators::made_knowing_its_length(mapped),
                    false => mapped,
                }
            }
            Over::Enumerate => crate::lower::iterators::enumerated_knowing(it),
            Over::Hinted { skip } => {
                let given = dropping(self, vec![it, values.next().expect("how many")])?;
                Expr::call_helper(if skip { "$skipping" } else { "$taking" }, given)
            }
            Over::Counted { skip } => {
                let given = dropping(self, vec![it, values.next().expect("how many")])?;
                // Its length follows from its back, of its source made
                // exact-sized, an array's `$iter`: what a variable that holds
                // it asks there, a crate's that reads a `Vec`'s capacity
                // (#1317), `knows_its_length_of` answers of a `take` or a
                // `skip` kept over an array's items made knowing its back.
                let made = Expr::call_helper(if skip { "$skipping" } else { "$taking" }, given);
                crate::lower::iterators::made_knowing_its_back(made)
            }
            Over::Filter => {
                let p = self.unary_callback(values.next().expect("the test"), own);
                let given = dropping(self, vec![it, p])?;
                Expr::call_helper("$filterEnds", given)
            }
        }))
    }

    /// `back_kept`'s source, of `receiver` of type `iter_ty`, given as a
    /// `held`, made knowing what the adapter, `method`'s, giving an
    /// `output`, is asked, and what's made of it: `by_ref()`'s lent, as it
    /// is; each other's to be closed, then made the adapter.
    fn back_kept_source(
        &mut self,
        method: Option<IterMethod>,
        receiver: &Expr,
        (iter_ty, output, held): (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>),
        span: Span,
    ) -> R<Option<BackKept>> {
        Ok(Some(match method {
            // A range's, the range, whose `&mut` it is: what steps it after
            // moves its start, `$rangeNext(it)`, as Rust's does, not a JS
            // iterator of it, which would be stepped as a range again.
            Some(IterMethod::ByRef) if self.range_kind(iter_ty).is_some() => BackKept::Lent(receiver.clone()),
            // Of one that knows its back, lent knowing it, as std's `&mut I`
            // passes on `I`'s back and length, never its ownership (#749).
            Some(IterMethod::ByRef) if self.knows_its_back(iter_ty) => {
                let back = self.as_double_ended(receiver.clone(), iter_ty, span)?;
                BackKept::Lent(self.lent_iterator(back, held))
            }
            // A copy that's its source, of numbers, stepped from its back:
            // its source made knowing its back, a `skip`'s or a `take`'s of a
            // slice's items, which JS's own `drop` and `take` don't (#646).
            Some(IterMethod::Copied | IterMethod::Cloned)
                if self.is_its_source(output)
                    && self.knows_its_back(output)
                    && !self.has_a_back(iter_ty, receiver)
                    && (self.recognition().is_iter_adapter(iter_ty)
                        || crate::lower::recognition::is_compiled_std_iterator(self.tcx, iter_ty)
                        || self.is_user_iterator(iter_ty)) =>
            {
                // Of std's compiled struct, a `fuse()`'s or a
                // `peekable()`'s, given its compiled back where it's made
                // (`knowing_held`), not the `next` alone its dictionary
                // has (#778). Of the crate's own iterator, or a library's,
                // its own `next_back` (#2068).
                BackKept::Held(self.as_double_ended(receiver.clone(), iter_ty, span)?, Over::Copy)
            }
            // A map of one that knows its length, too (ADR 0456).
            // Of a type parameter given as it is, of the JS iterator made of
            // it (ADR 0488).
            Some(IterMethod::Map)
                if self.knows_its_back(output)
                    || self.knows_its_length(output)
                    || self.maps_what_knows_its_length(receiver, iter_ty) =>
            {
                let back = self.knows_its_back(output);
                let len = self.knows_its_length(output) || self.maps_what_knows_its_length(receiver, iter_ty);
                // A source given as it is is given its back where it's asked;
                // one the crate steps from its back without one, a range or
                // another adapter, made knowing it, or refused (ADR 0545).
                let it = match self.given_stepped(receiver.clone(), held, back, len) {
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
                BackKept::Held(it, Over::Map { len })
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
                    // `count` is std's compiled one (#655).
                    || self.recognition().nths_a_slice(output)
                    || self.counts_by_compiled_count(iter_ty)
                    // Or one made knowing its hint alone (#2209).
                    || self.made_knowing_its_hint(output) =>
            {
                // Of the crate's, what it can be made to know.
                // Of a type parameter's, what its bounds give
                // (`given_stepped`), as a `rev()` of it asks its back and its
                // length (#971).
                let of_param = matches!(iter_ty.peel_refs().kind(), ty::Param(_));
                let back = of_param || self.knows_its_back(iter_ty) || self.user_double_ended(iter_ty);
                let len = of_param || self.knows_its_length(iter_ty) || self.user_exact_size(iter_ty);
                let it = match self.given_stepped(receiver.clone(), held, back, len) {
                    Some(it) => it,
                    // The crate's, what it can be made to know.
                    None if self.is_user_iterator(iter_ty) => {
                        self.user_iterator_knowing(receiver.clone(), held, (back, len), span)?
                    }
                    // Of another the crate steps from its back whose JS knows
                    // neither its back nor its length, which its index from
                    // the back is counted by: a `take(n)`'s or a
                    // `step_by(n)`'s, `.take(n)` and `$stepBy` (#651), a
                    // map's, JS's own `.map(f)` (#778), std's compiled struct,
                    // a `Fuse`'s, given its compiled back and length (#383):
                    // made knowing them, or refused (`as_exact_size`), never
                    // stepped by what it lacks.
                    // Its back, which it has, made knowing its length too,
                    // which `Enumerate::next_back` counts by: a map of a
                    // range made for its back alone (#2052).
                    None if self.stepped_from_its_back(output) => {
                        self.as_exact_size(receiver.clone(), iter_ty, true, span)?
                    }
                    None => receiver.clone(),
                };
                BackKept::Held(it, Over::Enumerate)
            }
            // A `skip(n)` or a `take(n)` the crate steps from its back, held
            // where it's given or a chain's side, which MIR holds in a
            // variable before it's stepped (#718): made knowing its back and
            // its length where it's made, `$skipping` or `$taking` of its
            // source made knowing them, given its items' drop, as
            // `as_exact_size` makes one in place. Not one through a `&mut`,
            // its owner's.
            // One generic code asks the hint of, given it (#1236): `$taking`
            // or `$skipping` of its source, each std's hint of its source's,
            // which JS's own `.take(n)` and `.drop(n)` know none of; given
            // its items' drop, so what's left drops as a drop of it panics
            // (#1244).
            Some(IterMethod::Skip | IterMethod::Take) if self.made_knowing_its_hint(output) && !held.is_ref() => {
                // As a JS iterator: generic code's given its dictionary,
                // `$given(it, IIterator)`, not the range it may be (#1240).
                let it = self.as_js_iterator(receiver.clone(), iter_ty, span)?;
                let skip = method == Some(IterMethod::Skip);
                BackKept::Held(it, Over::Hinted { skip })
            }
            // And one whose length is asked, counted, of a map's values say,
            // where a crate that reads a `Vec`'s capacity grows one by it
            // (#1317).
            Some(IterMethod::Skip | IterMethod::Take)
                if (self.made_stepped_from_its_back() || self.made_counted_from_its_back()) && !held.is_ref() =>
            {
                let skip = method == Some(IterMethod::Skip);
                let it = self.as_exact_size(receiver.clone(), iter_ty, true, span)?;
                BackKept::Held(it, Over::Counted { skip })
            }
            // A filter of one (ADR 0526), given its items' drop, of items with
            // a destructor, which drops each it doesn't keep as it passes it
            // (ADR 0098), as a compiled `Rev` steps it from its back (ADR
            // 0871).
            // And one made knowing its hint alone, `(0, upper)` of its
            // source's, its source as its maker made it, never asked its back
            // for it (`made_knowing_its_hint`, #2209).
            Some(IterMethod::Filter) if self.knows_its_back(output) || self.made_knowing_its_hint(output) => {
                let back = self.knows_its_back(output);
                let it = match self.given_stepped(receiver.clone(), held, back, false) {
                    Some(it) => it,
                    None if self.is_user_iterator(iter_ty) => {
                        self.user_iterator_knowing(receiver.clone(), held, (back, false), span)?
                    }
                    None if !back => receiver.clone(),
                    None if !self.has_a_back(iter_ty, receiver) => {
                        self.as_double_ended(receiver.clone(), iter_ty, span)?
                    }
                    None => receiver.clone(),
                };
                BackKept::Held(it, Over::Filter)
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
    /// `value`, written to `local` of type `ty`, made knowing what the
    /// crate's later uses ask of it (`asked_of`). A temporary given to what
    /// steps it from its back, back_stepped's `SteppedLocal::Lent`, made in
    /// place, which MIR holds to lend it or across what an argument runs
    /// before the step: made knowing its back where it's made, `$taking` or
    /// `$mapEnds` say, and marked so for its read (#718, #814, #811). A
    /// variable, or a temporary, whose length a `Vec` grown by what it's
    /// made into asks, made in place: made knowing it as that growth makes
    /// what's made in place (`made_as_growth_asks`, #1795, #1744, #2041).
    /// Each write of a local's value reads it: an assignment's, and a
    /// pending temporary's made a `const` (`flush`). Another as it is.
    pub fn written_as_asked(
        &mut self,
        local: rustc_middle::mir::Local,
        (value, ty): (Expr, Ty<'tcx>),
        span: Span,
    ) -> R<Expr> {
        let asked = self.asked_of(local);
        let value = match asked.back {
            Some(crate::lower::analysis::SteppedLocal::Lent) => self.made_knowing_where_held(value, ty, span)?,
            _ => value,
        };
        // Not one made knowing it already, nor one its maker made to step
        // from its back, a `skip(n)` made `$skipping`, as that step asks.
        let made_unknowing = crate::lower::iterators::made_in_place(&value)
            && asked.back.is_none()
            && !self.knows_its_length_of(&value, ty);
        match asked.len == Some(crate::lower::analysis::LengthAsked::Held) && made_unknowing {
            true => self.made_as_growth_asks(value, ty, span),
            false => Ok(value),
        }
    }

    fn made_knowing_where_held(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
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
    pub(in crate::lower) fn reversed_user(&mut self, receiver: Expr, receiver_ty: Ty<'tcx>, span: Span) -> R<Expr> {
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
    pub fn back_step(
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
        // Of what has a back, the iterator it steps, as `next_back`'s; std's
        // struct, which has none but given it (`has_a_back`), given its
        // compiled back where it's asked, as `rev()`'s is (ADR 0920,
        // `as_double_ended`'s `knowing_held`); another made one, a range's
        // of its bounds, stepped in place.
        // Of a type parameter's given as it is, its caller's own value, a
        // `T::IntoIter` of an `impl Is<T>`'s `T`: the JS iterator its
        // dictionaries step, its back theirs, as `rev()`'s is (#1604).
        let it = match self.iterator_given_as_it_is(iter_ty) && !crate::lower::iterators::is_js_iterator(&it) {
            true => self.given_js_iterator(it.clone(), iter_ty, true, false).unwrap_or(it),
            false => it,
        };
        // Of a sized `Box` of one generic code was given, the box's lend,
        // so its `try_rfold` and `advance_back_by` are the defaults over its
        // `next_back`, never what it holds' own (`lent_as_its_box`).
        let it = self.lent_as_its_box(it, iter_ty);
        let it = match self.range_kind(iter_ty).is_none() && self.has_a_back(iter_ty, &it) {
            true => it,
            false => self.as_double_ended(it, iter_ty, span)?,
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
                false if lent => self.lent_iterator(it, arg_tys[0]),
                false => it,
            };
            // Of a function it moves into its fold, `$consume`'s (#734).
            let values: Vec<Expr> = values.collect();
            if let Some(f) = values.last()
                && let Some(ends) = self.consumed_ends(Some(method), arg_tys, f, span)?
            {
                let mut given = vec![it, Expr::str("rfold")];
                given.extend(values);
                given.push(unwind.unwrap_or_else(Expr::js_undefined));
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
                false => Expr::js_undefined(),
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
        let mut ty = iter_ty;
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
            ty = source;
        }
        true
    }

    /// A map's or a filter's source given as it is, `given_js_iterator`'s:
    /// its back where it's asked and its bounds here give one; of a type
    /// parameter whose bounds give none, forward alone, as the type it's
    /// mapped to is stepped from its back only where they do, which rustc
    /// asks (ADR 0545, amended): the `either` crate's `IterEither::fold`,
    /// whose `Map<R, _>` its `rfold` steps from the back. And a `copied()`
    /// of one, made knowing its length, `method_source`'s (#1983).
    pub(in crate::lower) fn given_stepped(
        &self,
        value: Expr,
        iter_ty: Ty<'tcx>,
        back: bool,
        len: bool,
    ) -> Option<Expr> {
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

    /// Whether an `enumerate()` of an iterator of type `iter_ty` counts by
    /// its source's `count`, std's compiled one, as `Enumerate::count` does:
    /// a held `Peekable`'s keeps what it peeked at till its source's `count`
    /// ends, or unwinds, then drops it (#655). Of one exact-sized and
    /// double-ended too, which a compiled `count` says is std's struct
    /// compiled wherever it's made, whose enumeration `back_stepped` records
    /// where it's stepped from its back (#712); behind an `impl Trait` too,
    /// the type its maker made (#1142).
    fn counts_by_compiled_count(&self, iter_ty: Ty<'tcx>) -> bool {
        self.compiled_step(iter_ty, std_item(self.tcx, StdItem::Iterator), "count")
    }

    /// Whether `value`, of type `iter_ty`, has a back by its form: an
    /// array's; or one whose type knows its back, made so by its maker; and
    /// never one whose form says it has none, JS's own `.map(f)` say, though
    /// its type says it may (`form_lacks_a_back`, #851): what's asked of it
    /// makes it knowing its back or refuses it, never steps a back it hasn't.
    /// Whether it's made so is its value's, not its type's (#851), so std's
    /// compiled struct of a type stepped from its back elsewhere is none
    /// (#856).
    // helper: asked by `mir::iter` alone, the entry it was beneath before #2299 moved it
    pub fn has_a_back(&self, iter_ty: Ty<'tcx>, value: &Expr) -> bool {
        // Of a value made before: whether its type knows its back by its
        // structure, not whether what's being made is to be made so (#851).
        let structural = || {
            let outer = self.asking_made.replace(true);
            let knows = self.knows_its_back(iter_ty);
            self.asking_made.set(outer);
            knows
        };
        // Not std's compiled struct, which has a back only given it,
        // `$knowing`'s (#856), as `knows_its_back_beneath` says.
        // An array's, or a `repeat_n`'s, whose back steps are its front
        // steps (#1139), by its type, as its runtime form has them; and a
        // `str`'s search by a pattern that runs code, `$lazySplitBy` and the
        // like, which has one wherever std's is double-ended (#1501).
        let by_its_type = self.steps_an_array(iter_ty)
            || crate::lower::recognition::steps_back_as_front(self.tcx, iter_ty.peel_refs())
            || self.recognition().search_knows_its_back(iter_ty.peel_refs()) == Some(true);
        (by_its_type || structural() && !crate::lower::iterators::form_lacks_a_back(value))
            && !self.compiled_struct(iter_ty.peel_refs(), value)
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
/// Each `rev()` of `b` read in place, `reversed_in_place`'s, but one a
/// step from its back is given, `$foldBack($rev(c))`, whose chain
/// reversed inward would have no back (#1855).
pub fn reversed_in_body(b: &mut [js::Stmt]) {
    let mut stepped: Vec<*const Expr> = Vec::new();
    js::each_expr_mut(b, &mut |e| {
        if let Some(it) = stepped_from_its_back(e) {
            stepped.push(it as *const Expr);
        }
    });
    js::each_expr_mut(b, &mut |e| {
        if !stepped.contains(&(e as *const Expr)) {
            reversed_in_place(e);
        }
    })
}

/// Of `e`, a step from its back of what it's given, `$foldBack(it, ..)`,
/// `$nthBack`, `$advanceBackBy`, `$findBack`, `$tryRfoldOf`, `$nthBackOf`,
/// or a `Rev`'s own, `$rev(it)`, a `try_rfold`'s `$tryFoldSome($rev(it))`:
/// `it`, where it's a `rev()` read in place, which
/// `reversed_in_place` leaves as it is, as what it would turn it into, the
/// chain over the items reversed, JS's own, has no back to step (#1855).
fn stepped_from_its_back(e: &Expr) -> Option<&Expr> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let backs = [
        "$foldBack",
        "$nthBack",
        "$advanceBackBy",
        "$findBack",
        "$tryRfoldOf",
        "$nthBackOf",
        "$rev",
    ];
    let it = args
        .first()
        .filter(|_| callee.helper_name().is_some_and(|name| backs.contains(&name)))?;
    crate::lower::iterators::reversed_of(it).map(|_| it)
}

fn reversed_in_place(e: &mut Expr) {
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

/// `back_kept_source`'s: what `by_ref()` lends, as it is, or a source to
/// close, then make `back_kept`'s adapter of.
enum BackKept {
    Lent(Expr),
    Held(Expr, Over),
}

/// What `back_kept` makes of a source it's closed: a copy's, itself; a
/// map's, `$mapEnds`, knowing its length or not; an enumeration's; a
/// `skip(n)`'s or a `take(n)`'s, given its hint or counted from its back;
/// a filter's, `$filterEnds`.
enum Over {
    Copy,
    Map { len: bool },
    Enumerate,
    Hinted { skip: bool },
    Counted { skip: bool },
    Filter,
}
