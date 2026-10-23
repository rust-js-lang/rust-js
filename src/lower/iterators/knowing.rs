//! What an iterator knows: its back, its length, its exact size, and the
//! steps a std or a user iterator's struct takes.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `ty`, a map's or a filter's, is one the crate steps from its
    /// back (ADR 0545), made knowing it where it's made: the one answer, of
    /// what's made and what's stepped.
    pub(in crate::lower) fn stepped_from_its_back(&self, ty: ty::Ty<'tcx>) -> bool {
        (self.krate.back_stepped).contains(&self.tcx.erase_and_anonymize_regions(ty.peel_refs()))
    }

    /// Whether an adapter of type `ty` is explicit (ADR 0542): of a kind the
    /// crate's types hold, which keeps its source and its state wherever the
    /// crate makes one. The one answer, of what's made and what's asked of it.
    pub(in crate::lower) fn explicit_adapter(&self, ty: ty::Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adapter, _) if self.krate.explicit_adapters.contains(&adapter.did()))
    }

    /// Whether `$sizeHint` finds the size of an iterator of type `ty` held
    /// by a struct, a compiled std iterator's, `Fuse`'s or `Peekable`'s,
    /// whose own hint is of it: a `$iter`'s, a range's, the crate's own
    /// iterator's or a generic one's, by its dictionary, an explicit
    /// adapter's, or another such struct's (ADRs 0170, 0542, 0561). Not a
    /// std adapter's made here, `map`'s or `filter`'s, a JS iterator's,
    /// which knows nothing of its length. The one answer, of `size_hint()`
    /// and `len()`.
    pub(in crate::lower) fn held_hint_known(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        self.array_source(ty).is_some()
            || self.is_unknown(ty)
            || self.explicit_adapter(ty)
            || self.range_kind(ty).is_some()
            || self.is_user_iterator(ty)
            || self.struct_hint_known(ty)
    }

    /// Whether a compiled std iterator of type `ty` knows its hint, its
    /// struct's own, of what it holds: where each iterator it holds is one
    /// whose size `held_hint_known` finds.
    pub(in crate::lower) fn struct_hint_known(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty::Adt(_, args) = ty.peel_refs().kind() else {
            return false;
        };
        crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty)
            && (args.types())
                .filter(|&held| self.implements_iterator(held))
                .all(|held| self.held_hint_known(held))
    }

    /// Whether `ty`'s JS iterator knows its back, which `$nextBack` steps
    /// (ADR 0429): a `dyn DoubleEndedIterator`'s, or a type parameter's
    /// bound by one, given a JS iterator (ADR 0061), not as it is (ADR 0411).
    /// The one answer, of what generic code steps from its back and of what
    /// its callers give it.
    pub(in crate::lower) fn knows_its_back(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        // A map of one, made one that knows its back too, `$mapEnds` (ADR
        // 0455), a filter or a `filter_map`, `$filterEnds` (ADR 0526); a
        // copy of one, which it is (ADR 0480); of a type parameter given as
        // it is, of the JS iterator made of it (ADR 0488).
        // And a map or a filter the crate steps from its back, made knowing
        // it (ADR 0545).
        if let Some(source) = crate::lower::recognition::back_stepped_source(self.tcx, ty) {
            return self.knows_its_back(source) || self.stepped_from_its_back(ty);
        }
        // A flatten the crate steps from its back, made knowing it,
        // `$flattenEnds`.
        if crate::lower::recognition::flattened_source(self.tcx, ty).is_some() {
            return self.stepped_from_its_back(ty);
        }
        // An enumeration of one that knows its length too, `$enumerate`,
        // whose index from the back is where it is (ADR 0528).
        // Of the crate's, made knowing both (`user_iterator_knowing`).
        if let Some(source) = crate::lower::recognition::enumerated_source(self.tcx, ty) {
            return (self.knows_its_back(source) || self.user_double_ended(source))
                && (self.knows_its_length(source) || self.user_exact_size(source));
        }
        if let Some(id) = crate::lower::recognition::dyn_iter_trait(self.tcx, ty) {
            return crate::lower::recognition::is_double_ended(self.tcx, id);
        }
        // memchr's, `$memchrIter`'s (ADR 0529).
        if self.recognition().is_memchr_iter(ty) {
            return true;
        }
        self.double_ended_param(self.tcx.typeck_root_def_id(self.item), ty)
    }

    /// Whether `ty` is `id`'s type parameter bound by `DoubleEndedIterator`,
    /// given a JS iterator that knows its back (`knows_its_back`).
    pub(super) fn double_ended_param(&self, id: DefId, ty: ty::Ty<'tcx>) -> bool {
        self.extension_param(id, ty, crate::lower::recognition::is_double_ended)
    }

    /// Whether `ty` is `id`'s type parameter bound by `ExactSizeIterator`,
    /// given a JS iterator that knows its length (`knows_its_length`).
    pub(super) fn exact_size_param(&self, id: DefId, ty: ty::Ty<'tcx>) -> bool {
        self.extension_param(id, ty, crate::lower::recognition::is_exact_size)
    }

    /// Whether `ty` is `id`'s type parameter bound by an iterator's
    /// extension trait, `is`'s, given a JS iterator (ADR 0061), not as it is
    /// (ADR 0411).
    fn extension_param(&self, id: DefId, ty: ty::Ty<'tcx>, is: fn(ty::TyCtxt<'tcx>, DefId) -> bool) -> bool {
        // A type parameter's, or an associated type's, `I::IntoIter: DoubleEndedIterator`,
        // a JS iterator as its caller has it (ADR 0480).
        let js_iterator = match ty.kind() {
            ty::Param(param) => !given_as_it_is(self.tcx, id, *param),
            ty::Alias(..) => true,
            _ => false,
        };
        let env = ty::TypingEnv::post_analysis(self.tcx, id);
        js_iterator && (self.declared_bounds(ty, env.param_env).into_iter()).any(|bound| is(self.tcx, bound))
    }

    /// Whether this function holds what `ty` makes as an `IntoIterator` as
    /// it is, `into_iter_held_as_it_is` of it (ADR 0552).
    pub(in crate::lower) fn holds_into_iter(&self, ty: Ty<'tcx>) -> bool {
        into_iter_held_as_it_is(self.tcx, self.tcx.typeck_root_def_id(self.item), ty)
    }

    /// Whether `ty` is a type parameter of the function being lowered given
    /// as it is (ADR 0411): its caller's own value, an object, the crate's
    /// iterator or a `$iter`, which `*it = other` writes in place, `$assign`
    /// (ADR 0481).
    pub(in crate::lower) fn iterator_given_as_it_is(&self, ty: ty::Ty<'tcx>) -> bool {
        let owner = self.tcx.typeck_root_def_id(self.item);
        matches!(ty.kind(), ty::Param(param) if given_as_it_is(self.tcx, owner, *param))
            || into_iter_as_it_is(self.tcx, owner, ty)
    }

    /// Whether an adapter of `source`, a type parameter given as it is, is
    /// one of a JS iterator that knows what `extension` asks, its back or its
    /// length: made of it by `given_js_iterator`, asked what it's given, a
    /// map's by `back_kept` (ADR 0488). Not `source`'s own value, its
    /// caller's.
    fn made_knowing(&self, source: ty::Ty<'tcx>, extension: Option<DefId>) -> bool {
        let source = source.peel_refs();
        matches!(source.kind(), ty::Param(_))
            && extension.is_some_and(|extension| self.iterator_evidence(source, extension).is_some())
    }

    /// Whether a value of `ty` is a JS iterator that knows its length, its
    /// `len()` (ADR 0456): a type parameter's bound by `ExactSizeIterator`,
    /// a `dyn`'s, or a map of one. The one answer, of what generic code asks
    /// the length of and what its callers give it.
    pub(in crate::lower) fn knows_its_length(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        if let Some(source) = crate::lower::recognition::length_kept_source(self.tcx, ty) {
            return self.knows_its_length(source)
                || self.made_knowing(source, self.recognition().exact_size_iterator());
        }
        // An explicit `Take` or `Skip`, which counts from its source, made as
        // capable as its type is, an array's iterator or an exclusive range's
        // knowing its length (ADR 0542).
        if self.explicit_adapter(ty)
            && let Some(source) = crate::lower::recognition::held_steps_source(self.tcx, ty)
        {
            return self.knows_its_length(source)
                || self.steps_an_array(source)
                || self.range_kind(source) == Some(crate::lower::std_types::range::RangeKind::Exclusive);
        }
        if let Some(id) = crate::lower::recognition::dyn_iter_trait(self.tcx, ty) {
            return crate::lower::recognition::is_exact_size(self.tcx, id);
        }
        self.exact_size_param(self.tcx.typeck_root_def_id(self.item), ty)
    }

    /// `value`, an iterator of type `ty`, given where generic code asks its
    /// length, an `ExactSizeIterator`'s, and its back, `back` (ADR 0456): a
    /// JS iterator that knows them, an array's, its `$iter`; a range's of
    /// numbers, `$iterator(range, $rangeNext, false, $rangeNextBack,
    /// $rangeLen)`; one already, itself.
    pub(in crate::lower) fn as_exact_size(&mut self, value: Expr, ty: ty::Ty<'tcx>, back: bool, span: Span) -> R<Expr> {
        let ty = self.reveal(ty);
        // One a struct holds, as `as_double_ended` says (ADR 0549).
        if let Some(knowing) = self.knowing_held(value.clone(), ty, span)? {
            return Ok(knowing);
        }
        // The crate's, made knowing its length, and its back where asked.
        if self.is_user_iterator(ty) {
            let value = stepped_value(&value).unwrap_or(value);
            return self.user_iterator_knowing(value, ty, (back, true), span);
        }
        if self.knows_its_length(ty) && (!back || self.knows_its_back(ty)) {
            return Ok(value);
        }
        if let Some(it) = self.given_js_iterator(value.clone(), ty, back, true) {
            return Ok(it);
        }
        // A copy of one, which it is (ADR 0480).
        if let Some(source) = crate::lower::recognition::copied_source(self.tcx, ty) {
            return self.as_exact_size(value, source, back, span);
        }
        // A range's, `$rangeLen` of `a..b`, `$rangeInclusiveLen` of `a..=b`,
        // which std counts of small numbers alone.
        let len = match self.range_kind(ty) {
            Some(crate::lower::std_types::range::RangeKind::Exclusive) => Some("$rangeLen"),
            Some(crate::lower::std_types::range::RangeKind::Inclusive) => Some("$rangeInclusiveLen"),
            _ => None,
        };
        if let (Some(kind), Some(len)) = (self.range_kind(ty), len)
            && let (Some(next), Some(from_back)) = (kind.step(false), kind.step(true))
        {
            let from_back = back.then(|| Expr::helper(from_back));
            return Ok(stepped_ends(
                value,
                Expr::helper(next),
                false,
                from_back,
                Some(Expr::helper(len)),
                None,
                (None, None),
            ));
        }
        if self.steps_an_array(ty) {
            return Ok(self.std_iterator(value, ty));
        }
        // `skip(n)` or `take(n)` made here, `src.drop(n)`: `$skipping` or
        // `$taking` of its source, made knowing its back and its length,
        // which they step from the back by, as std's `Skip` and `Take` of an
        // `ExactSizeIterator` do, and know their length (ADR 0542). Of items with a destructor,
        // which std drops as a `Vec`'s tail slice, refused below.
        if let Some((source, skip)) = crate::lower::recognition::counted_source(self.tcx, ty)
            && let Some((source_value, n)) = counted_parts(&value, skip)
            && (self.iterator_item(ty)).is_some_and(|item| self.drops(item) == crate::lower::drops::Drops::Nothing)
        {
            let source_value = stepped_value(&source_value).unwrap_or(source_value);
            let source_value = self.as_exact_size(source_value, source.peel_refs(), true, span)?;
            let name = if skip { "$skipping" } else { "$taking" };
            return Ok(Expr::call_helper(name, vec![source_value, n]));
        }
        // A map made here, `src.map(f)`: its source's, each `f`'s, as
        // `as_double_ended` makes it (ADR 0455).
        if let Some(source) = crate::lower::recognition::mapped_source(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && let js::ExprKind::Member(source_value, method) = &callee.kind
            && method == "map"
            && let [f] = args.as_slice()
        {
            let source_value = stepped_value(source_value).unwrap_or_else(|| (**source_value).clone());
            let source_value = self.as_exact_size(source_value, source.peel_refs(), back, span)?;
            return Ok(Expr::call_helper("$mapEnds", vec![source_value, f.clone()]));
        }
        Err(self.unsupported(
            span,
            &format!("a `{ty}` as an `ExactSizeIterator`, which rust-js can't count"),
        ))
    }

    /// `value`, a double-ended iterator of type `ty`, as a JS iterator that
    /// knows its back (ADR 0429): an array's, its `$iter`; a range's,
    /// `$iterator(range, $rangeNext, false, $rangeNextBack)`; the crate's
    /// own, stepped by its impls' `next` and `next_back`; one already, a
    /// `dyn`'s or a type parameter's, itself.
    pub(in crate::lower) fn as_double_ended(&mut self, value: Expr, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = self.reveal(ty);
        let value = stepped_value(&value).unwrap_or(value);
        // One a struct holds, which keeps its source (ADR 0542): of one given
        // as it is, its back and length this use's dictionaries', wherever it
        // was made; of one made knowing them, an array's or a range's, itself.
        if let Some(knowing) = self.knowing_held(value.clone(), ty, span)? {
            return Ok(knowing);
        }
        // A flatten made here, `src.flatMap(each)`, of items that are arrays,
        // which know their backs: `$flattenEnds` of its source made knowing
        // its back, as std's `Flatten` steps from either end.
        if let Some((source, made)) = crate::lower::recognition::flattened_source(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && let js::ExprKind::Member(source_value, method) = &callee.kind
            && method == "flatMap"
            && let [each] = args.as_slice()
            && let Some(inner) = made.or_else(|| self.iterator_item(source.peel_refs()))
            && {
                let inner = inner.peel_refs();
                inner.is_array() || inner.is_slice() || self.is_vec_like(inner)
            }
        {
            let source_value = self.as_double_ended((**source_value).clone(), source.peel_refs(), span)?;
            return Ok(Expr::call_helper("$flattenEnds", vec![source_value, each.clone()]));
        }
        // The crate's, made knowing its back.
        if self.is_user_iterator(ty) {
            return self.user_iterator_knowing(value, ty, (true, false), span);
        }
        if self.knows_its_back(ty) || self.held_knows_its_back(ty) {
            return Ok(value);
        }
        if let Some(it) = self.given_js_iterator(value.clone(), ty, true, false) {
            return Ok(it);
        }
        // A copy of one, which it is (ADR 0480).
        if let Some(source) = crate::lower::recognition::copied_source(self.tcx, ty) {
            return self.as_double_ended(value, source, span);
        }
        if let Some(kind) = self.range_kind(ty)
            && let (Some(next), Some(back)) = (kind.step(false), kind.step(true))
        {
            return Ok(stepped(value, Expr::helper(next), false, Some(Expr::helper(back))));
        }
        // `skip(n)` or `take(n)` made here, exact-sized as std's `Skip` and
        // `Take` must be to step from the back: `as_exact_size`'s.
        if let Some((_, skip)) = crate::lower::recognition::counted_source(self.tcx, ty)
            && counted_parts(&value, skip).is_some()
        {
            return self.as_exact_size(value, ty, true, span);
        }
        if self.steps_an_array(ty) {
            return Ok(self.std_iterator(value, ty));
        }
        // A map made here of one that knows its back, `src.map(f)`: its
        // source's items, each `f`'s, from either end, as Rust's `Map` steps
        // them (ADR 0455).
        if let Some(source) = crate::lower::recognition::mapped_source(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && let js::ExprKind::Member(source_value, method) = &callee.kind
            && method == "map"
            && let [f] = args.as_slice()
        {
            let source_value = self.as_double_ended((**source_value).clone(), source.peel_refs(), span)?;
            return Ok(Expr::call_helper("$mapEnds", vec![source_value, f.clone()]));
        }
        // A `step_by(n)` made here, `src.filter((_, i) => i % n === 0)`:
        // `$stepByEnds` of its source made knowing its back and its length,
        // which it finds its last by, as std's `StepBy` does.
        if let Some(source) = crate::lower::recognition::stepped_by_source(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && let js::ExprKind::Member(source_value, method) = &callee.kind
            && method == "filter"
            && let [f] = args.as_slice()
            && let Some(step) = crate::lower::std_types::range::every_nth(f)
        {
            let source_value = stepped_value(source_value).unwrap_or_else(|| (**source_value).clone());
            let source_value = self.as_exact_size(source_value, source.peel_refs(), true, span)?;
            return Ok(Expr::call_helper("$stepByEnds", vec![source_value, step]));
        }
        // A chain made here, `$lazyChain(a, b)`: each made knowing its back,
        // `$chainEnds` of them, which steps `b`'s then `a`'s from the back, as
        // std's `Chain` does.
        if let Some((first, second)) = crate::lower::recognition::chained_sources(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && callee.helper_name() == Some("$lazyChain")
            && let [a, b] = args.as_slice()
        {
            let a = stepped_value(a).unwrap_or_else(|| a.clone());
            let b = stepped_value(b).unwrap_or_else(|| b.clone());
            let a = self.as_double_ended(a, first.peel_refs(), span)?;
            let b = self.as_double_ended(b, second, span)?;
            return Ok(Expr::call_helper("$chainEnds", vec![a, b]));
        }
        // A filter made here, `src.filter(p)`: its source's items `p` keeps,
        // from either end, as Rust's `Filter` steps them (ADR 0526).
        if let Some(source) = crate::lower::recognition::filtered_source(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && let js::ExprKind::Member(source_value, method) = &callee.kind
            && method == "filter"
            && let [p] = args.as_slice()
        {
            let source_value = self.as_double_ended((**source_value).clone(), source.peel_refs(), span)?;
            return Ok(Expr::call_helper("$filterEnds", vec![source_value, p.clone()]));
        }
        if let Some(double_ended) = self.own_next_back(ty)? {
            let iterator = std_item(self.tcx, StdItem::Iterator);
            let (next, boxed) = self.user_next(ty, iterator, "next", span)?;
            let (back, _) = self.user_next(ty, double_ended, "next_back", span)?;
            return Ok(stepped(value, next, boxed, Some(back)));
        }
        Err(self.unsupported(
            span,
            &format!("a `{ty}` as a `DoubleEndedIterator`, which rust-js can't step from its back"),
        ))
    }

    /// `exact_len`'s, of the iterator's value and its type `ty`.
    pub(in crate::lower) fn exact_len_of(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        // A compiled std iterator's, `Peekable`'s, whose `len` isn't: its
        // struct's hint, which it knows as it knows its `next` (ADR 0561),
        // of what it holds; else refused.
        if crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty) {
            if !self.struct_hint_known(ty) {
                return Err(self.unsupported(span, &format!("`len()` of a `{ty}` of a lazy adapter")));
            }
            return Ok(Expr::call(
                Expr::helper("$exactLen"),
                vec![Expr::call_helper("$sizeHint", vec![value])],
            ));
        }
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let size_hint = trait_method(self.tcx, iterator, "size_hint");
        let args = self.args_of(iterator, ty);
        let hint = match self.trait_call(size_hint, args, vec![value.clone()], span, out)? {
            Some(hint) => hint,
            None => {
                if value.has_effects() {
                    out.push(StmtKind::Expr(value).at(self.js_span(span)));
                }
                Expr::array(vec![Expr::int(0), Expr::undefined()])
            }
        };
        Ok(Expr::call(Expr::helper("$exactLen"), vec![hint]))
    }

    /// An iterator of the crate's as a JS one, stepped by its trait's
    /// `method`: `Iterator`'s `next`, or `DoubleEndedIterator`'s `next_back`.
    pub(in crate::lower) fn user_iterator(
        &mut self,
        value: Expr,
        ty: ty::Ty<'tcx>,
        trait_id: DefId,
        method: &str,
        span: Span,
    ) -> R<Expr> {
        let (next, boxed) = self.user_next(ty, trait_id, method, span)?;
        let hint = self.own_size_hint(ty, span)?;
        let own = self.own_fold_and_nth(ty, span)?;
        Ok(stepped_ends(value, next, boxed, None, None, hint, own))
    }

    /// An iterator of the crate's as a JS one stepped from its front that
    /// knows what's asked of it: its back, `back`, its `next_back`, where
    /// it's double-ended, and its length, `len`, its hint's, which std's
    /// `len()` checks is exact (`$exactLen`, ADR 0456), where it's exact-sized.
    pub(in crate::lower) fn user_iterator_knowing(
        &mut self,
        value: Expr,
        ty: ty::Ty<'tcx>,
        (back, len): (bool, bool),
        span: Span,
    ) -> R<Expr> {
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let (next, boxed) = self.user_next(ty, iterator, "next", span)?;
        let hint = self.own_size_hint(ty, span)?;
        let back = match self.own_next_back(ty)? {
            Some(double_ended) if back => Some(self.user_next(ty, double_ended, "next_back", span)?.0),
            _ => None,
        };
        let len = (len && self.user_exact_size(ty)).then(|| {
            let hint_of = match &hint {
                Some(hint) => Expr::call(hint.clone(), vec![Expr::var("it")]),
                None => Expr::array(vec![Expr::int(0), Expr::undefined()]),
            };
            let exact = Expr::call_helper("$exactLen", vec![hint_of]);
            Expr::arrow(
                vec!["it".into()],
                vec![StmtKind::Return(Some(exact)).at(js::Span::NONE)],
            )
        });
        let own = self.own_fold_and_nth(ty, span)?;
        Ok(stepped_ends(value, next, boxed, back, len, hint, own))
    }

    /// The `size_hint` of the crate's iterator `ty`, where its impl has one
    /// of its own: what `size_hint()` gives of it made a JS iterator, as of
    /// it as it is. `None` of one that has std's, `(0, None)`, which
    /// `$sizeHint` gives too.
    pub(super) fn own_size_hint(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let size_hint = trait_method(self.tcx, iterator, "size_hint");
        let args = self.args_of(iterator, ty.peel_refs());
        let own = self
            .resolve_instance(size_hint, args)?
            .is_some_and(|instance| instance.def_id() != size_hint && self.is_rust_fn(instance.def_id()));
        if !own {
            return Ok(None);
        }
        Ok(Some(self.user_next(ty, iterator, "size_hint", span)?.0))
    }

    /// What steps a compiled std iterator's struct of type `ty` where it's
    /// made, `$stdIterator`'s `steps` (ADR 0561): its `next`, the compiled
    /// impl's, and its `hint`, its `size_hint`, the compiled impl's, or the
    /// rewrite of one whose body reaches memory, `$peekableHint`.
    pub(in crate::lower) fn std_iterator_steps(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let (next, _) = self.user_next(ty, iterator, "next", span)?;
        let mut props = vec![crate::js::Prop::Field("next".into(), next)];
        let hint = match self.compiled_step(ty, iterator, "size_hint") {
            true => Some(self.user_next(ty, iterator, "size_hint", span)?.0),
            // A peekable's, of what it steps by its dictionary where it's
            // given one, which has that impl's own `size_hint`.
            false if self.is_peekable(ty) => {
                let source = match ty.peel_refs().kind() {
                    ty::Adt(_, args) => args.type_at(0),
                    _ => unreachable!("a `Peekable`"),
                };
                Some(match self.iterator_evidence(source, iterator) {
                    Some(dictionary) => Expr::arrow(
                        vec!["peekable".into()],
                        vec![
                            StmtKind::Return(Some(Expr::call_helper(
                                "$peekableHint",
                                vec![Expr::var("peekable"), dictionary],
                            )))
                            .at(js::Span::NONE),
                        ],
                    ),
                    None => Expr::helper("$peekableHint"),
                })
            }
            false => None,
        };
        props.extend(hint.map(|hint| crate::js::Prop::Field("hint".into(), hint)));
        Ok(Expr::object(props))
    }

    /// Whether `ty`, a compiled std iterator, has `trait_id`'s `method`
    /// compiled, `Fuse`'s `next_back`, and is a `trait_id` here, so a
    /// use site can give what steps it by that (ADR 0561).
    pub(in crate::lower) fn compiled_step(&self, ty: ty::Ty<'tcx>, trait_id: DefId, method: &str) -> bool {
        let ty = ty.peel_refs();
        if !crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty) {
            return false;
        }
        let tr = self
            .tcx
            .erase_and_anonymize_regions(ty::TraitRef::new(self.tcx, trait_id, [ty]));
        self.tcx
            .codegen_select_candidate(self.typing_env.as_query_input(tr))
            .is_ok()
            && self
                .resolve_instance(trait_method(self.tcx, trait_id, method), self.args_of(trait_id, ty))
                .is_ok_and(|instance| {
                    instance.is_some_and(|i| crate::lower::recognition::is_compiled_std(self.tcx, i.def_id()))
                })
    }

    /// The function that steps the crate's iterator of type `ty`, its impl's
    /// `method` of `trait_id`, `countdownIterator_next`, and whether its
    /// `Some` is boxed, as a generic `next`'s is (ADR 0051).
    pub(in crate::lower) fn user_next(
        &mut self,
        ty: ty::Ty<'tcx>,
        trait_id: DefId,
        method: &str,
        span: Span,
    ) -> R<(Expr, bool)> {
        let next = trait_method(self.tcx, trait_id, method);
        let args = self.args_of(trait_id, ty.peel_refs());
        // A generic `next` boxes a `Some` that looks like `None` (ADR 0051).
        let boxed = self.resolve_instance(next, args)?.is_some_and(|instance| {
            let id = instance.def_id();
            let output = self
                .tcx
                .fn_sig(id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .output();
            let output = self
                .tcx
                .try_normalize_erasing_regions(
                    ty::TypingEnv::post_analysis(self.tcx, id),
                    ty::Unnormalized::new_wip(output),
                )
                .unwrap_or(output);
            self.option_of(output).is_some_and(|item| self.boxed_payload(item))
        });
        let call = self.impl_call(next, args, vec![Expr::var("iterator")], span)?;
        let next = Expr::arrow(
            vec!["iterator".into()],
            vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
        );
        Ok((next.called_alone(), boxed))
    }

    /// The dictionary this function was given for `ty`, a type parameter
    /// given as it is, of its bound `trait_id`: `Iterator`'s (ADR 0411), or
    /// an extension's, `DoubleEndedIterator`'s or `ExactSizeIterator`'s
    /// (ADR 0487).
    pub(in crate::lower) fn iterator_evidence(&self, ty: ty::Ty<'tcx>, trait_id: DefId) -> Option<Expr> {
        // Of a copied default, in its impl's terms: `T` of its trait's `Self`
        // in `impl<T: Iterator> Trait for T` (ADR 0492).
        let ty = self.impl_type(ty.peel_refs());
        // Only of one given as it is: another's is a JS iterator (ADR 0061),
        // whatever bound names `Iterator`, a supertrait's too. Or of what an
        // `IntoIterator` held as it is makes, through its dictionary (ADR
        // 0552).
        if !self.iterator_given_as_it_is(ty) {
            return None;
        }
        self.evidence_for(ty::TraitRef::new(self.tcx, trait_id, [ty]))
    }

    /// Whether `ty` is an adapter a struct holds (ADR 0542) made knowing its
    /// back where it's made, of an array's iterator or a range, which it
    /// steps as it is: the one answer, of what steps it from its back.
    pub(in crate::lower) fn held_knows_its_back(&self, ty: Ty<'tcx>) -> bool {
        self.explicit_adapter(ty)
            && crate::lower::recognition::held_steps_source(self.tcx, ty.peel_refs())
                .is_some_and(|source| self.steps_an_array(source) || self.range_kind(source).is_some())
    }

    /// `it`, an adapter of type `ty` a struct holds (ADR 0542), of a source
    /// given as it is: it, stepped from its back and counted by this use's
    /// dictionaries of what it steps, `$knowing(it,
    /// IDoubleEndedIterator.next_back, IExactSizeIterator.len)`, as it may
    /// have been made where they weren't given (ADR 0549). Or a compiled
    /// std iterator's struct, `Fuse`'s, given its compiled `next_back` and
    /// `len` with this use's dictionaries, where it's each here (ADR 0561).
    /// `None` of another, or where this use has neither.
    pub(in crate::lower) fn knowing_held(&mut self, it: Expr, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let ty = ty.peel_refs();
        if crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty) {
            let traits = [
                (self.recognition().double_ended_iterator(), "next_back"),
                (self.recognition().exact_size_iterator(), "len"),
            ];
            let mut steps = Vec::new();
            for (trait_id, method) in traits {
                steps.push(match trait_id {
                    Some(trait_id) if self.compiled_step(ty, trait_id, method) => {
                        Some(self.user_next(ty, trait_id, method, span)?.0)
                    }
                    _ => None,
                });
            }
            let len = steps.pop().expect("len");
            let back = steps.pop().expect("back");
            if back.is_none() && len.is_none() {
                return Ok(None);
            }
            let [back, len] = [back, len].map(|entry| entry.unwrap_or_else(Expr::undefined));
            return Ok(Some(Expr::call_helper("$knowing", vec![it, back, len])));
        }
        if !self.explicit_adapter(ty) {
            return Ok(None);
        }
        let Some(source) = crate::lower::recognition::held_steps_source(self.tcx, ty) else {
            return Ok(None);
        };
        let entry = |trait_id: Option<DefId>| -> Option<Expr> {
            let trait_id = trait_id?;
            let name = crate::lower::recognition::extension_entry(self.tcx, trait_id);
            Some(Expr::member(self.iterator_evidence(source, trait_id)?, name))
        };
        let back = entry(self.recognition().double_ended_iterator());
        let len = entry(self.recognition().exact_size_iterator());
        if back.is_none() && len.is_none() {
            return Ok(None);
        }
        let [back, len] = [back, len].map(|entry| entry.unwrap_or_else(Expr::undefined));
        Ok(Some(Expr::call_helper("$knowing", vec![it, back, len])))
    }
}

/// What a `skip(n)` or a `take(n)` made here steps, and `n`: of
/// `src.drop(n)` or `src.take(n)`.
fn counted_parts(value: &Expr, skip: bool) -> Option<(Expr, Expr)> {
    let js::ExprKind::Call(callee, args) = &value.kind else {
        return None;
    };
    let method = if skip { "drop" } else { "take" };
    match (&callee.kind, args.as_slice()) {
        (js::ExprKind::Member(source, name), [n]) if name == method => Some(((**source).clone(), n.clone())),
        _ => None,
    }
}
