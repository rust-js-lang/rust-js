//! A `copied()` or a `cloned()` from MIR: whether one is its source
//! itself in JS (`is_its_source`), and what each is made of, its source,
//! a map of each item's copy, or, of one the crate clones, made apart
//! (ADR 0480, ADR 0547, ADR 0525).

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether an adapter of type `ty` is its source itself in JS: a
    /// `copied()`, or a `cloned()` of what needs no clone, each item as it
    /// is (ADR 0547), where it asks the crate's own steps as its source
    /// does (`asks_as_its_source`). The one answer, of what's made and
    /// what's dropped.
    pub(in crate::lower) fn is_its_source(&self, ty: Ty<'tcx>) -> bool {
        (crate::lower::recognition::copied_source(self.tcx, ty))
            .or_else(|| crate::lower::recognition::cloned_source(self.tcx, ty))
            .is_some_and(|source| self.copies_its_items_as_they_are(ty) && self.asks_as_its_source(ty, source))
    }

    /// Whether a `copied()` or a `cloned()` of type `ty` gives each item
    /// as it is: a copy of a `Copy` item is a clone of it too, a struct's
    /// an object of its own where something changes one in place.
    fn copies_its_items_as_they_are(&self, ty: Ty<'tcx>) -> bool {
        self.iterator_item(ty)
            .is_some_and(|item| !self.needs_clone(item.peel_refs()))
    }

    /// Whether a copy's `source` is what generic code was given, whose
    /// steps its caller's value says (`is_unknown`).
    fn given_source(&self, source: Ty<'tcx>) -> bool {
        self.is_unknown(source.peel_refs())
    }

    /// Whether a `copied()` of type `ty` is of what generic code was given,
    /// of items whose copy is the item, and isn't its source in JS, as the
    /// value its caller gives may write `nth_back`, which std's `Copied`
    /// doesn't pass on: `$copiedGiven(it)`, `it` itself where it has no
    /// own `nth_back`, else `$copied` of it (#2130). Cloned as what it's
    /// made of is, by `$givenClone`, never apart.
    pub(in crate::lower) fn copied_given(&self, ty: Ty<'tcx>) -> bool {
        crate::lower::recognition::copied_source(self.tcx, ty).is_some_and(|source| {
            self.given_source(source) && self.copies_its_items_as_they_are(ty) && !self.asks_as_its_source(ty, source)
        })
    }

    /// Whether a `copied()` or a `cloned()` of type `ty` of a `source` asks
    /// the crate's own steps as its source does, so its JS may be its
    /// source's: as std's catalog says (`reaches_as_its_source`), not a
    /// `cloned()` of one that writes `count`, which std's `Cloned` leaves
    /// to its `fold` (#2122), nor a `copied()` of one that writes
    /// `nth_back`, which std's `Copied` leaves to its default, its source's
    /// `advance_back_by` (#2130); or std's compiled struct, made of its
    /// source where what steps it asks it (`knowing_held`), which asks what
    /// std's does.
    fn asks_as_its_source(&self, ty: Ty<'tcx>, source: Ty<'tcx>) -> bool {
        crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty)
            || self.recognition().reaches_as_its_source(ty, source)
    }

    /// `it.copied()` or `it.cloned()`, `method`, of `item`s, of one the
    /// crate clones (`iterator_cloned_apart`): made by a function of its
    /// source, `it`, the value it's given, as its maker took it, which its
    /// clone calls again with a clone of it, `makeCopied`, each item copied
    /// as `copied_items` copies it (ADR 0525, #1181). Never of the JS
    /// iterator made of it, which its clone would take for the value
    /// (#2130).
    pub(super) fn copied_cloned_apart(
        &mut self,
        (it, method, item): (Expr, IterMethod, Ty<'tcx>),
        (iter_ty, receiver_ty, output): (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>),
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let name = if method == IterMethod::Copied {
            "copied"
        } else {
            "cloned"
        };
        let lent = matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut));
        // Made knowing its back wherever it may be stepped from it, as no
        // form shows what its maker makes, so nothing past it can make it
        // so (#2130).
        let back = self.knows_its_back(output) || self.may_step_from_its_back(output, iter_ty);
        let hint = !back && self.made_knowing_its_hint(output);
        let copied = self.cloned_apart_adapter(
            (name, vec![it], &[Role::It]),
            (vec![iter_ty], output),
            lent,
            span,
            made,
            |cx, parts| {
                let source = parts.into_iter().next().expect("its source");
                let source = cx.as_js_iterator(source, iter_ty, span)?;
                let source = cx.closing(source, iter_ty, span)?;
                cx.copied_items_knowing(source, item, output, (back, hint), span)
            },
        )?;
        // Made knowing its back where it's to know it, as `copied_items`
        // makes what it holds: said so of its maker's call, which
        // `as_double_ended` reads, as no form shows it (#2130).
        Ok(match back {
            true => crate::lower::iterators::made_knowing_its_back(copied),
            false => copied,
        })
    }

    /// Whether a copy of type `ty` of a `source`, the crate's own iterator
    /// that steps from its back, is a `DoubleEndedIterator`, as rustc's
    /// trait solver says here, so may be stepped from its back past what
    /// shows its form.
    fn may_step_from_its_back(&self, ty: Ty<'tcx>, source: Ty<'tcx>) -> bool {
        self.user_double_ended(source)
            && (self.recognition().double_ended_iterator())
                .is_some_and(|double_ended| self.recognition().implements_trait(ty, double_ended))
    }

    /// `copied()` or `cloned()` of an iterator of `item`s, references: a
    /// copy of what each points at, once. Of an item that's a reference
    /// to a reference, `&&Fiber`, the reference, which a copy is itself,
    /// not what it points at, so the items as they are. Of one that knows
    /// its back or its length, `output`, a map that knows them too,
    /// `$mapEnds`, as a map is (#387): a chain stepped from its back knows
    /// no length, and `.map` has no back to step (#850).
    /// Its source made knowing its back where it's to know its own, as a
    /// map's is: a `step_by(n)`'s, `$stepByEnds` (#851); or its hint alone,
    /// where it's made knowing that, never asked its back for it (#2209).
    pub(super) fn copied_items(&mut self, it: Expr, item: Ty<'tcx>, output: Ty<'tcx>, span: Span) -> R<Expr> {
        let back = self.knows_its_back(output);
        let hint = !back && self.made_knowing_its_hint(output);
        self.copied_items_knowing(it, item, output, (back, hint), span)
    }

    /// `it`, what generic code was given of type `source`, made knowing its
    /// back where `back` says it's to know it and its back is given, its
    /// `DoubleEndedIterator`'s evidence, as a source a copy is in JS is
    /// (`given_stepped`): a `copied()`'s and a `cloned()`'s alike, so a
    /// clone of either stepped from its back steps what it's made of
    /// (#2130, #2137).
    fn given_knowing_its_back(&mut self, it: Expr, source: Ty<'tcx>, back: bool, span: Span) -> R<Expr> {
        let double_ended = ty::TraitRef::new(
            self.tcx,
            self.recognition().double_ended_iterator().expect("std's"),
            [source.peel_refs()],
        );
        match back && self.evidence_for(double_ended).is_some() {
            true => self.as_double_ended(it, source.peel_refs(), span),
            false => Ok(it),
        }
    }

    /// `copied_items`, made knowing its back where `back` says, or its hint
    /// alone where `hint` does.
    fn copied_items_knowing(
        &mut self,
        it: Expr,
        item: Ty<'tcx>,
        output: Ty<'tcx>,
        (back, hint): (bool, bool),
        span: Span,
    ) -> R<Expr> {
        // Of one that knows its length and not its back, the map that
        // copies its items would lose it; none reaches here: a copy that
        // knows its length is of what knows its back too (#1979).
        let ends = back || hint;
        let source = crate::lower::recognition::copied_source(self.tcx, output)
            .or_else(|| crate::lower::recognition::cloned_source(self.tcx, output));
        // Of what generic code was given, whose steps its caller's value
        // says, a `cloned()` of items whose clone is the item, `$cloned`,
        // which is it where it has none of its own that std's `Cloned`
        // hides, as it's made (#2122).
        // Of its source made knowing its back where it's to know it, as a
        // `copied()`'s is (#2137).
        if let Some(source) = crate::lower::recognition::cloned_source(self.tcx, output)
            && self.given_source(source)
            && !self.needs_clone(item.peel_refs())
            && !self.asks_as_its_source(output, source)
        {
            let it = self.given_knowing_its_back(it, source, back, span)?;
            return Ok(Expr::call_helper("$cloned", vec![it]));
        }
        // A `copied()`'s, `$copiedGiven` (`copied_given`, #2130), of its
        // source made knowing its back where it's to know it.
        if let Some(source) = source.filter(|_| self.copied_given(output)) {
            let it = self.given_knowing_its_back(it, source, back, span)?;
            return Ok(Expr::call_helper("$copiedGiven", vec![it]));
        }
        let it = match source {
            Some(source) if back => self.as_double_ended(it, source.peel_refs(), span)?,
            _ => it,
        };
        let x = self.fresh(Role::Item);
        let mut body = Vec::new();
        let pointee = match *item.kind() {
            ty::Ref(_, pointee, _) => pointee,
            _ => item,
        };
        // A `copied()`'s item is its bits, `Copy`'s, never a hand-written
        // `Clone` of a `Copy` type, which std's `Copied` doesn't call; a
        // `cloned()`'s is its `Clone`.
        let copy = match crate::lower::recognition::copied_source(self.tcx, output) {
            Some(_) => self.copy_if_needed(Expr::var(&x), pointee),
            None => self.clone_value(Expr::var(&x), pointee, span, &mut body)?,
        };
        // Its source itself, of items whose copy is the item, where it asks
        // the crate's own steps as its source does (`asks_as_its_source`):
        // a `copied()`'s, which passes each on, not a `cloned()`'s of one
        // that writes `count` or `last`, which std's `Cloned` leaves to its
        // `fold` (#2122).
        let as_its_source = source.is_none_or(|source| self.asks_as_its_source(output, source));
        let recognition = self.recognition();
        if body.is_empty() && matches!(&copy.kind, js::ExprKind::Var(v) if *v == x) && as_its_source {
            return Ok(it);
        }
        // A `copied()` that reaches the crate's own `nth`, `last` or `count`,
        // which std's `Copied` asks of its source, `$copied`, which does
        // too; else a map, as std's `Cloned` steps as its `Map` does.
        let passes_on = [OwnStep::Nth, OwnStep::Last, OwnStep::Count]
            .into_iter()
            .any(|step| recognition.reaches_its_own(output, step, Reach::Sure));
        let copied = crate::lower::recognition::copied_source(self.tcx, output).is_some() && passes_on;
        body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
        let copy = Expr::arrow(vec![x.into()], body);
        Ok(match (copied, ends) {
            (true, _) => Expr::call_helper("$copied", vec![it, copy]),
            (false, true) => Expr::call_helper("$mapEnds", vec![it, copy]),
            (false, false) => Expr::builtin(it, "map", vec![copy]),
        })
    }
}
