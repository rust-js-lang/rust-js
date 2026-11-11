//! What an iterator knows: its back, its length, its exact size, and the
//! makers that make it knowing them. The steps a compiled iterator's
//! struct takes are `compiled_steps.rs`'.

use super::*;
use crate::lower::recognition::{OwnStep, Reach};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value`'s reversal, a `rev()` made in place of `source` (ADR 0987).
    pub(in crate::lower) fn reversal_of<'e>(&self, value: &'e Expr, source: ty::Ty<'tcx>) -> Option<Reversal<'e>> {
        let range_back = self.range_kind(source.peel_refs()).and_then(|kind| kind.step(true));
        crate::lower::iterators::reversal(value, range_back)
    }

    /// Whether a `rev()` of type `ty` is made knowing its back where it's
    /// made (`reversed_knowing`): one the crate steps from its back, its own
    /// `rfold`, `nth_back` or `rfind` called (ADR 0545), of a source whose
    /// `rev()` is made here, the crate's iterator, a range or a std
    /// adapter, or a type parameter, `$rev` of the JS iterator it's given
    /// made knowing its back (`rev_items`, #718), or of one given as it is,
    /// of the JS iterator its dictionaries step (`rev_called`, #718); a
    /// zip's, `$rev` of its
    /// `$zipEnds` (#740). So one made in place that MIR
    /// holds in a variable
    /// before it's stepped, past a call or a cleanup that drops it, or lends
    /// to `nth_back`, steps from its back as it is, no form of it left to
    /// read (#702). The one answer, of what's made and what's stepped.
    pub(in crate::lower) fn reversed_made_knowing(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        crate::lower::recognition::reversed_source(self.tcx, ty).is_some_and(|source| {
            let source = source.peel_refs();
            !self.is_unknown(source) || matches!(source.kind(), ty::Param(_))
        }) && self.stepped_from_its_back(ty)
    }

    /// `value`, a `rev()` of type `ty` just made, made knowing its back
    /// where `reversed_made_knowing` says: of the crate's iterator, by its
    /// own `next` (`reversed_user_knowing`); of another, by its form
    /// (`reversal_knowing`), or refused. Else itself.
    pub(in crate::lower) fn reversed_knowing(&mut self, value: Expr, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = ty.peel_refs();
        if !self.reversed_made_knowing(ty) {
            return Ok(value);
        }
        let source = crate::lower::recognition::reversed_source(self.tcx, ty).expect("a `rev()`");
        if self.is_user_iterator(source) {
            return self
                .reversed_user_knowing(value, source, (true, true), span)
                .map(made_knowing_its_back);
        }
        match self.reversal_knowing(&value, source, span)? {
            Some(knowing) => Ok(made_knowing_its_back(knowing)),
            None => Err(self.unsupported(
                span,
                &format!("a `{ty}` stepped from its back, made in a form rust-js can't step from its back"),
            )),
        }
    }

    /// `value`, a `rev()` of `source` made here, made knowing its back by
    /// its form (`reversal`, ADR 0987), before its range is read out of it:
    /// `$rev(it)` as it is, `it` made knowing its back where it was made;
    /// `$rev` of a range stepped from its end made knowing its back; of an
    /// array's items reversed, as its source's of them is. `None` of
    /// another form.
    fn reversal_knowing(&mut self, value: &Expr, source: ty::Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let Some(reversal) = self.reversal_of(value, source) else {
            return Ok(None);
        };
        Ok(Some(match reversal {
            Reversal::Stepped(_) | Reversal::Items => value.clone(),
            Reversal::Range(range) => {
                let range = self.as_double_ended(range.clone(), source, span)?;
                crate::lower::iterators::reversed_by(range)
            }
            Reversal::Chain => self.as_double_ended(value.clone(), source, span)?,
        }))
    }

    /// Whether an adapter of type `ty` is explicit (ADR 0542): of a kind the
    /// crate's types hold, which keeps its source and its state wherever the
    /// crate makes one. The one answer, of what's made and what's asked of it.
    /// Never one compiled where it's held, `Enumerate`'s, which code asking
    /// this relies on (ADR 0871): checked in a debug build.
    pub(in crate::lower) fn explicit_adapter(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty::Adt(adapter, _) = ty.peel_refs().kind() else {
            return false;
        };
        let explicit = self.krate.explicit_adapters.contains(&adapter.did());
        debug_assert!(
            !explicit
                || !crate::lower::recognition::is_held_compiled(
                    self.tcx,
                    self.tcx
                        .type_of(adapter.did())
                        .instantiate_identity()
                        .skip_normalization()
                ),
            "an explicit adapter compiled where it's held, {ty}",
        );
        explicit
    }

    /// Whether `$sizeHint` finds the size of an iterator of type `ty` held
    /// by a struct, a compiled std iterator's, `Fuse`'s or `Peekable`'s,
    /// whose own hint is of it: a `$iter`'s, of an array's items or of what
    /// was made one where it was made, a slice's chunks' say (#1100), a
    /// range's, the crate's own iterator's or a generic one's, by its
    /// dictionary, an explicit adapter's, or another such struct's (ADRs
    /// 0170, 0542, 0561), or a std source's whose form gives std's
    /// (`source_hint_kept`, #1357). Not a
    /// std adapter's made here, `map`'s or `filter`'s, a JS iterator's,
    /// which knows nothing of its length. The one answer, of `size_hint()`
    /// and `len()`.
    pub(in crate::lower) fn held_hint_known(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        self.array_source(ty).is_some()
            || self.recognition().iterates_items(ty)
            || self.is_unknown(ty)
            || self.explicit_adapter(ty)
            || self.range_kind(ty).is_some()
            || self.is_user_iterator(ty)
            || self.struct_hint_known(ty)
            || crate::lower::recognition::source_hint_kept(self.tcx, ty)
    }

    /// Whether `size_hint()` of an iterator of type `ty` is std's where the
    /// recognizers make it in place: what `held_hint_known` finds, of no
    /// array's items, whose adapters are JS's own, which know no hint; or a
    /// std adapter of such iterators, each a `$Steps`, whose `sizeHint` is
    /// std's (ADR 1092's table), not of a type parameter's, whose JS
    /// adapters may be JS's own (#733). A flatten asked its hint is std's
    /// struct, compiled, whose hint of `[T; N]` is `N` times its source's.
    pub(in crate::lower) fn adapted_hint_known(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        if self.array_source(ty).is_some() {
            return false;
        }
        if self.held_hint_known(ty) {
            return true;
        }
        // Not of a type parameter's, which its caller may give as JS's own
        // iterator, an array's, whose JS adapters know no hint.
        // `char::decode_utf16`'s and `cycle()`'s, always a `$Steps` that
        // asks its source's hint, an array's `$iter`'s among them (#1151,
        // #1179).
        let decodes = crate::lower::recognition::asks_its_sources_hint(self.tcx, ty);
        crate::lower::recognition::adapted_iterators(self.tcx, self.typing_env, ty).is_some_and(|held| {
            (held.into_iter()).all(|held| {
                !self.is_unknown(held.peel_refs())
                    && (self.adapted_hint_known(held) || decodes && self.held_hint_known(held))
            })
        })
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
        // What an `impl` hides, as it's made.
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
        // `$flattenEnds`, and a chain a function gives behind an `impl`,
        // `$lazyChain` of sides made knowing theirs (#739).
        // And a zip the crate steps from its back, `$zipEnds` (#740).
        if crate::lower::recognition::flattened_source(self.tcx, ty).is_some()
            || crate::lower::recognition::chained_sources(self.tcx, ty).is_some() && self.stepped_from_its_back(ty)
            || crate::lower::recognition::zipped_sources(self.tcx, ty).is_some()
        {
            return self.stepped_from_its_back(ty);
        }
        // An enumeration of one that knows its length too, `$enumerate`,
        // whose index from the back is where it is (ADR 0528).
        // Of the crate's, made knowing both (`user_iterator_knowing`).
        if let Some(source) = crate::lower::recognition::enumerated_source(self.tcx, ty) {
            return (self.knows_its_back(source) || self.user_double_ended(source))
                && (self.knows_its_length(source) || self.user_exact_size(source))
                || self.stepped_from_its_back(ty);
        }
        if let Some(id) = crate::lower::recognition::dyn_iter_trait(self.tcx, ty) {
            return crate::lower::recognition::is_double_ended(self.tcx, id);
        }
        // memchr's, `$memchrIter`'s (ADR 0529), and a `Vec`'s `Splice`,
        // `$splice`'s over what it took out (#1149).
        if self.recognition().is_memchr_iter(ty) || self.recognition().is_vec_splice(ty) {
            return true;
        }
        if let Some(knows) = self.text_knows_its_back(ty) {
            return knows;
        }
        self.double_ended_param(self.generics_fn(), ty)
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
        into_iter_held_as_it_is(self.tcx, self.generics_fn(), ty)
    }

    /// Whether `ty` is a type parameter of the function being lowered given
    /// as it is (ADR 0411): its caller's own value, an object, the crate's
    /// iterator or a `$iter`, which `*it = other` writes in place, `$assign`
    /// (ADR 0481).
    pub(in crate::lower) fn iterator_given_as_it_is(&self, ty: ty::Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Param(param) if self.param_given_as_it_is(*param))
            || into_iter_as_it_is(self.tcx, self.generics_fn(), ty)
    }

    /// Whether `param`, a type parameter of the function being lowered, is
    /// given as it is, as its callers ask it of that function: one of its
    /// trait's, `Self` of a default, as the trait's impl takes it (ADR
    /// 0551); its own, as `given_as_it_is` says (ADR 0411), of a default
    /// lowered into an impl's dictionary too, `J` of `fn chain_all<J:
    /// IntoIterator>(self, other: J) -> Both<Self, J::IntoIter>`, whose
    /// callers give it as it is with its dictionary (itertools' laziness tests).
    pub(in crate::lower) fn param_given_as_it_is(&self, param: ty::ParamTy) -> bool {
        let owner = self.generics_fn();
        crate::lower::iterators::given_by_its_trait(self.tcx, owner, param.index)
            || given_as_it_is(self.tcx, owner, param)
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
        // A `skip(n)` or a `take(n)` made `$skipping` or `$taking` where it's
        // made, of its source made `$iter`, as `hint_asked.rs` asks of a
        // chain's or a zip's parts a `Vec` grows by (#1216).
        if crate::lower::recognition::counted_source(self.tcx, ty).is_some() && self.made_knowing_its_hint(ty) {
            return true;
        }
        // Of an array's items made knowing its back, `$mapEnds` of their
        // `$iter`, which knows its length too.
        if let Some(source) = crate::lower::recognition::length_kept_source(self.tcx, ty) {
            // And of std's compiled struct that knows it, a `Fuse`'s, of a
            // map the crate steps from its back, made knowing it,
            // `$knowing(fuse, back, len)` (#385).
            let compiled_exact = || {
                crate::lower::recognition::is_compiled_std_iterator(self.tcx, source.peel_refs())
                    && (self.recognition().exact_size_iterator())
                        .is_some_and(|exact| self.recognition().implements_trait(source.peel_refs(), exact))
            };
            return self.knows_its_length(source)
                || self.made_knowing(source, self.recognition().exact_size_iterator())
                || self.steps_an_array(source) && self.knows_its_back(ty)
                // And a map of a range's whose closure has a destructor, held
                // in a variable to drop it, a zip's `b` read by index say, or
                // that a variable holds for an enumeration counted from its
                // back (#1354): made knowing its length where it's made,
                // `$rangeLen` (#775).
                || self.range_kind(source.peel_refs()).is_some()
                    && self.stepped_from_its_back(ty)
                    && crate::lower::recognition::mapped_source(self.tcx, ty).is_some()
                    && (matches!(ty.kind(), ty::Adt(_, args) if self.drops(args.type_at(1)) != crate::lower::drops::Drops::Nothing)
                        || self.made_counted_from_its_back())
                || crate::lower::recognition::mapped_source(self.tcx, ty).is_some()
                    && compiled_exact()
                    && self.stepped_from_its_back(ty);
        }
        // A zip the crate steps from its back, made knowing its length too,
        // `$zipEnds` (#740): by its value's fact, as its back is (#851).
        if crate::lower::recognition::zipped_sources(self.tcx, ty).is_some() && self.knows_its_back(ty) {
            return true;
        }
        if let Some(id) = crate::lower::recognition::dyn_iter_trait(self.tcx, ty) {
            return crate::lower::recognition::is_exact_size(self.tcx, id);
        }
        // A `Vec`'s `Splice`, `$splice`'s iterator over what it took out
        // (#1149).
        if self.recognition().is_vec_splice(ty) {
            return true;
        }
        self.exact_size_param(self.generics_fn(), ty)
    }

    /// Whether `value`, an iterator of type `ty`, knows its length, its
    /// `len()`: where its type says so (`knows_its_length`), or where it's
    /// an adapter that keeps its length, a map, a copy, an enumeration or a
    /// `rev()`, of an array's items, made knowing its back, as its value's
    /// form or mark says (`knows_its_back_of`): each form of one that knows
    /// its back, `$mapEnds`, `$rev`, `$enumerate`, of a `$iter`, knows its
    /// length too; or one its maker marked made knowing it, a map of what
    /// knows it (`made_knowing_its_length`, #851).
    pub(in crate::lower) fn knows_its_length_of(&self, value: &Expr, ty: ty::Ty<'tcx>) -> bool {
        self.knows_its_length(ty)
            || crate::lower::iterators::was_made_knowing_its_length(value)
            // An array's `$iter`, a hash set's `$iter(Array.from(set))` (#1200).
            || super::fresh_iter_source(value).is_some()
            || self.kept_over_an_array(ty) && self.knows_its_back_of(value, ty)
    }

    /// Whether a map of `source`, an iterator of type `ty`, is made knowing
    /// its length as its source does (`knows_its_length_of`): where what it
    /// reads down to, through each `rev()` and adapter made knowing its back,
    /// is a variable its maker made knowing its back or length, as its mark
    /// says, `$rev(mapped)` of a `mapped` stepped from its back elsewhere,
    /// whose length a use may ask, as std's `Map` passes its source's on.
    /// Not one made knowing it in place, which reads its form, nor an
    /// array's own, which `len_in_place` counts as it is (#851).
    pub(in crate::lower) fn maps_what_knows_its_length(&self, source: &Expr, ty: ty::Ty<'tcx>) -> bool {
        fn held_knowing(e: &Expr) -> bool {
            match &e.kind {
                js::ExprKind::Var(_) => {
                    crate::lower::iterators::was_made_knowing_its_back(e)
                        || crate::lower::iterators::was_made_knowing_its_length(e)
                }
                _ => (crate::lower::iterators::reversed_of(e))
                    .or_else(|| crate::lower::iterators::back_source(e).map(|(source, _)| source))
                    .is_some_and(held_knowing),
            }
        }
        !self.steps_an_array(ty) && held_knowing(source) && self.knows_its_length_of(source, ty)
    }

    /// Whether `ty` is an adapter that keeps its source's length
    /// (`length_kept_source`), down to an array's items.
    fn kept_over_an_array(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        match crate::lower::recognition::length_kept_source(self.tcx, ty) {
            Some(source) => self.steps_an_array(source.peel_refs()) || self.kept_over_an_array(source),
            None => false,
        }
    }

    /// A zip made here of `ty`, `$lazyZip(a, b, ..)` or one std reads by
    /// index, `$trustedZip(a, b)`: `$zipEnds` or `$trustedZipEnds` of its
    /// sides, each made knowing its length, and its back where `back` asks,
    /// as std's `Zip` is exact-sized and double-ended where both its sides
    /// are, what its back trims of each dropped by its items' drop (#740).
    /// `None` of another value.
    fn zip_knowing(&mut self, value: &Expr, ty: ty::Ty<'tcx>, back: bool, span: Span) -> R<Option<Expr>> {
        let (Some((a_ty, b_ty)), Some(zip)) = (
            crate::lower::recognition::zipped_sources(self.tcx, ty),
            value::zip_parts(value),
        ) else {
            return Ok(None);
        };
        let a = self.as_exact_size(zip.a.clone(), a_ty.peel_refs(), back, span)?;
        let b = self.as_exact_size(zip.b.clone(), b_ty.peel_refs(), back, span)?;
        let item_drop = |cx: &mut Self, side: ty::Ty<'tcx>| match cx.iterator_item(side.peel_refs()) {
            Some(item) if back => cx.std_drop_function(item, span),
            _ => Ok(None),
        };
        // What std's back trims of `a` dropped too where it reads by index,
        // whose own `next` drops none.
        let drop_a = match (zip.drop_a, zip.by_index) {
            (Some(drop_a), _) => Some(drop_a.clone()),
            (None, true) => item_drop(self, a_ty)?,
            (None, false) => None,
        };
        let drop_b = item_drop(self, b_ty)?;
        // One std folds to the shorter's count of pairs, where its `next`
        // steps `a` once more, `SpecFold`'s `TrustedLen` impl (ADR 0997).
        let to_shorter = crate::lower::recognition::zip_folds_to_the_shorter(self.tcx, ty) == Some(false);
        Ok(Some(value::zip_ends(zip.by_index, a, b, (drop_a, drop_b), to_shorter)))
    }

    /// `value`, an iterator of type `ty`, given where generic code asks its
    /// length, an `ExactSizeIterator`'s, and its back, `back` (ADR 0456): a
    /// JS iterator that knows them, an array's, its `$iter`; a range's of
    /// numbers, `$iterator(range, $rangeNext, false, $rangeNextBack,
    /// $rangeLen)`; one already, itself.
    pub(in crate::lower) fn as_exact_size(&mut self, value: Expr, ty: ty::Ty<'tcx>, back: bool, span: Span) -> R<Expr> {
        // One a struct holds, as `as_double_ended` says (ADR 0549).
        if let Some(knowing) = self.knowing_held(value.clone(), ty, span)? {
            return Ok(knowing);
        }
        // The crate's, made knowing its length, and its back where asked.
        if self.is_user_iterator(ty) {
            let value = stepped_value(&value).unwrap_or(value);
            return self.user_iterator_knowing(value, ty, (back, true), span);
        }
        // A `rev()` of it made in place, as `reversed_user_knowing` makes it.
        if let Some(source) = crate::lower::recognition::reversed_source(self.tcx, ty)
            && made_in_place(&value)
            && self.is_user_iterator(source)
        {
            return self.reversed_user_knowing(value, source, (back, true), span);
        }
        // Of one whose form, or what it holds down to its type, knows its
        // back, its back too (#851).
        if self.knows_its_length_of(&value, ty)
            && (!back || self.knows_its_back(ty) || self.knows_its_back_of(&value, ty))
        {
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
            // Of one stepped from its start already, an `enumerate()`'s
            // source's, the range it steps (#385).
            let value = stepped_value(&value).unwrap_or(value);
            let from_back = back.then(|| Expr::helper(from_back));
            return Ok(stepped_ends(
                value,
                Expr::helper(next),
                false,
                from_back,
                Some(Expr::helper(len)),
                None,
                crate::lower::iterators::OwnMethods::default(),
            ));
        }
        if self.steps_an_array(ty) {
            return Ok(self.std_iterator(value, ty));
        }
        if let Some(zipped) = self.zip_knowing(&value, ty, back, span)? {
            return Ok(zipped);
        }
        // `skip(n)` or `take(n)` made here, `src.drop(n)`: `$skipping` or
        // `$taking` of its source, made knowing its back and its length,
        // which they step from the back by, as std's `Skip` and `Take` of an
        // `ExactSizeIterator` do, and know their length (ADR 0542).
        // One made so already, `$skipping(source, n)` of `as_exact_size`'s
        // own, a `rev()`'s source (#385); not `closing`'s, given a drop.
        if let Some((_, skip)) = crate::lower::recognition::counted_source(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && args.len() == 2
            && callee.helper_name() == Some(if skip { "$skipping" } else { "$taking" })
        {
            return Ok(value);
        }
        // Of items with a destructor (#718): a skip made so, `$skipping(source,
        // n, drop)`, which drops each it skips as it's skipped (ADR 0465), its
        // source made knowing what's asked, its `len` and back what it steps
        // by (#778); a take, `$taking(source, n, drop)`, whose back passes
        // what it won't give to its source's `nth_back`, a `Vec`'s as one
        // slice, else each dropped by `drop`, as std's `Take::next_back` and
        // `rfold` pass it.
        if let Some((source, skip)) = crate::lower::recognition::counted_source(self.tcx, ty)
            && let Some((source_value, n, drop)) = counted_parts(&value, skip)
        {
            let back = back || drop.is_none();
            let drop = match drop {
                Some(drop) => Some(drop),
                None => match self.iterator_item(ty) {
                    Some(item) => self.std_drop_function(item, span)?,
                    None => None,
                },
            };
            let source_value = stepped_value(&source_value).unwrap_or(source_value);
            let source_value = self.as_exact_size(source_value, source.peel_refs(), back, span)?;
            let name = if skip { "$skipping" } else { "$taking" };
            return Ok(Expr::call_helper(
                name,
                [vec![source_value, n], drop.into_iter().collect()].concat(),
            ));
        }
        // An enumeration made here of an array's items, `$enumerate` of their
        // `$iter`, as `as_double_ended` makes it; one made so, itself (#764).
        // Of another's, its source made knowing its length (#1215).
        if let Some(source) = crate::lower::recognition::enumerated_source(self.tcx, ty)
            && let Some(enumerated) = crate::lower::iterators::enumerated_of(&value)
        {
            if enumerated.form == crate::lower::iterators::EnumeratedForm::Made {
                return Ok(value.clone());
            }
            let (source, value) = (source.peel_refs(), enumerated.source.clone());
            let items = match self.steps_an_array(source) {
                true => self.std_iterator(value, source),
                false => self.as_exact_size(stepped_value(&value).unwrap_or(value), source, back, span)?,
            };
            return Ok(crate::lower::iterators::enumerated_knowing(items));
        }
        // A `step_by(n)` made here, as `as_double_ended` makes it (#718).
        if let Some(stepped) = self.step_by_knowing(&value, ty, span)? {
            return Ok(stepped);
        }
        // A map whose function gives each as it is, its source's value: its
        // source made so (`mapped_as_it_is`, #1342).
        if let Some(source) = crate::lower::recognition::mapped_source(self.tcx, ty)
            && let Some(source_value) = crate::lower::iterators::source_mapped_as_it_is(&value)
        {
            return self.as_exact_size(source_value, source.peel_refs(), back, span);
        }
        // A map made here, `src.map(f)`: its source's, each `f`'s, as
        // `as_double_ended` makes it (ADR 0455). And an `inspect(f)`, made
        // so, and a `cloned()` of what isn't a number, `src.map(clone)` (#778).
        if let Some(source) = crate::lower::recognition::mapped_source(self.tcx, ty)
            .or_else(|| crate::lower::recognition::inspected_source(self.tcx, ty))
            .or_else(|| crate::lower::recognition::cloned_source(self.tcx, ty))
            && let js::ExprKind::Call(callee, args) = &value.kind
            && let js::ExprKind::Member(source_value, method) = &callee.kind
            && method == "map"
            && let [f] = args.as_slice()
        {
            let source_value = stepped_value(source_value).unwrap_or_else(|| (**source_value).clone());
            let source_value = self.as_exact_size(source_value, source.peel_refs(), back, span)?;
            return Ok(Expr::call_helper("$mapEnds", vec![source_value, f.clone()]));
        }
        // A `rev()` made here, by its form (`reversal`, ADRs 0986, 0987):
        // `$rev` of its source, or of a range stepped from its end, made
        // knowing its back and length; of an array's items reversed, as its
        // source's of them is. Not one a dictionary steps from its back,
        // `$iterator(it, nextBack)`, which knows neither.
        if let Some(source) = crate::lower::recognition::reversed_source(self.tcx, ty)
            && let Some(reversal) = self.reversal_of(&value, source)
        {
            return match reversal {
                Reversal::Stepped(it) | Reversal::Range(it) => {
                    let it = self.as_exact_size(it.clone(), source, true, span)?;
                    Ok(crate::lower::iterators::reversed_by(it))
                }
                Reversal::Items => Ok(value.clone()),
                Reversal::Chain => self.as_exact_size(value.clone(), source, back, span),
            };
        }
        Err(self.unsupported(
            span,
            &format!("a `{ty}` as an `ExactSizeIterator`, which rust-js can't count"),
        ))
    }

    pub(in crate::lower) fn as_double_ended(&mut self, value: Expr, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        // A `rev()` made in place, knowing its back where it's made, as its
        // maker marked it (`made_knowing_its_back`): itself, wherever MIR
        // holds it, the mark carried by its local. Its maker makes none a
        // variable or a field holds, std's struct (ADR 0871), so that has
        // no mark, and `knowing_held` gives its back below.
        if was_made_knowing_its_back(&value) {
            return Ok(value);
        }
        // One whose form knows its back already, `$mapEnds` of one that
        // does, its maker's, through what its form holds down to what its
        // type says, an array's iterator a variable holds: itself (#851).
        if self.knows_its_back_of(&value, ty) {
            return Ok(value);
        }
        // A `rev()` of the crate's iterator made in place, of its form, as
        // `reversed_user_knowing` makes it again.
        if let Some(source) = crate::lower::recognition::reversed_source(self.tcx, ty)
            && made_in_place(&value)
            && self.is_user_iterator(source)
        {
            return self.reversed_user_knowing(value, source, (true, false), span);
        }
        if let Some(source) = crate::lower::recognition::reversed_source(self.tcx, ty)
            && let Some(knowing) = self.reversal_knowing(&value, source, span)?
        {
            return Ok(knowing);
        }
        let value = stepped_value(&value).unwrap_or(value);
        // One a struct holds, which keeps its source (ADR 0542): of one given
        // as it is, its back and length this use's dictionaries', wherever it
        // was made; of one made knowing them, an array's or a range's, itself.
        if let Some(knowing) = self.knowing_held(value.clone(), ty, span)? {
            return Ok(knowing);
        }
        // A flatten made here, `src.flatMap(each)`: `$flattenEnds` of its
        // source made knowing its back, as std's `Flatten` steps from either
        // end. Of items that are arrays, which know their backs, as they
        // are; of items that are iterators, an `Option`'s slice's iterator
        // say, each made knowing its back as it's taken (#260).
        // Or `$flatMap(src, each)`, the one closed as std's is (#644).
        if let Some((source, made)) = crate::lower::recognition::flattened_source(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && let Some((source_value, each)) = match (&callee.kind, args.as_slice()) {
                (js::ExprKind::Member(source_value, method), [each]) if method == "flatMap" => {
                    Some((&**source_value, each))
                }
                (_, [source_value, each]) if callee.helper_name() == Some("$flatMap") => Some((source_value, each)),
                _ => None,
            }
            && let Some(inner) = made.or_else(|| self.iterator_item(source.peel_refs()))
        {
            let inner = inner.peel_refs();
            let each = match inner.is_array() || inner.is_slice() || self.is_vec_like(inner) {
                true => Some(each.clone()),
                // What `each` gives of its item, `(item) => $iterator(item,
                // next)`, made knowing its back where it's given (#716); of
                // a function given by its name, `flat_map(pairs)`, what its
                // call gives, `(item) => pairs(item)`, std's struct of an
                // adapter it gives among them (#1235).
                // Of an `IntoIterator`'s items, by what it gives, its
                // `IntoIter`, an iterator's itself, `<S::Pairs as
                // IntoIterator>::IntoIter`, its back its dictionary's where
                // generic code has it (#1355); refused where none says it
                // has one, never stepped as if.
                false
                    if let Some(into_iter) = self.recognition().flattened_inner(ty)
                        && let Some((item, mut body, taken)) = self.each_returned(each) =>
                {
                    let knowing = self.as_double_ended(taken, into_iter, span)?;
                    body.push(StmtKind::Return(Some(knowing)).at(js::Span::NONE));
                    Some(Expr::arrow(vec![item.as_str().into()], body))
                }
                false => None,
            };
            if let Some(each) = each {
                let source_value = self.as_double_ended(source_value.clone(), source.peel_refs(), span)?;
                return Ok(Expr::call_helper("$flattenEnds", vec![source_value, each]));
            }
        }
        // A chain made here, `$lazyChain(a, b)`: each made knowing its back,
        // and the one runtime `Chain` of them again, which then knows its
        // own and steps `b`'s then `a`'s from the back, as std's `Chain`
        // does (#739).
        if let Some((first, second)) = crate::lower::recognition::chained_sources(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && callee.helper_name() == Some("$lazyChain")
            && let [a, b] = args.as_slice()
        {
            let a = stepped_value(a).unwrap_or_else(|| a.clone());
            let b = stepped_value(b).unwrap_or_else(|| b.clone());
            let a = self.as_double_ended(a, first.peel_refs(), span)?;
            let b = self.as_double_ended(b, second, span)?;
            return Ok(Expr::call_helper("$lazyChain", vec![a, b]));
        }
        if let Some(zipped) = self.zip_knowing(&value, ty, true, span)? {
            return Ok(zipped);
        }
        // A map whose function gives each as it is, its source's value: its
        // source made so (`mapped_as_it_is`, #1342).
        if let Some(source) = crate::lower::recognition::mapped_source(self.tcx, ty)
            && let Some(source_value) = crate::lower::iterators::source_mapped_as_it_is(&value)
        {
            return self.as_double_ended(source_value, source.peel_refs(), span);
        }
        // A map made here of one that knows its back, `src.map(f)`: its
        // source's items, each `f`'s, from either end, as Rust's `Map` steps
        // them (ADR 0455). And an `inspect(f)`, made as a map giving each
        // item back, `f` called on each as Rust's `Inspect` gives it.
        if let Some(source) = crate::lower::recognition::mapped_source(self.tcx, ty)
            .or_else(|| crate::lower::recognition::inspected_source(self.tcx, ty))
            && let js::ExprKind::Call(callee, args) = &value.kind
            && let js::ExprKind::Member(source_value, method) = &callee.kind
            && method == "map"
            && let [f] = args.as_slice()
        {
            let source_value = self.as_double_ended((**source_value).clone(), source.peel_refs(), span)?;
            return Ok(Expr::call_helper("$mapEnds", vec![source_value, f.clone()]));
        }
        // A `filter_map` made here, `src.map(f).filter(present)`, each kept
        // `Some` taken out of its box after, `.map($someValue)`, where it
        // looks like `None`: its source's items `f` gives a value of, from
        // either end, as Rust's `FilterMap` steps them, as one made of a
        // source that knows its back is (`iter_comb`, ADR 0526): made so
        // where it's made, of one the crate steps from its back, itertools'
        // `self.iter.filter_map(..).rfold(..)` of a source given as it is
        // (#473). And a flatten of `Option`s, `src.filter(present)`, or a
        // `flat_map` to them, made so as a `filter_map` is
        // (`flattened_options`), which std's `Flatten` steps from either end
        // as its `Some`s (#1326).
        let somes = crate::lower::recognition::filter_mapped_source(self.tcx, ty)
            .map(|source| (source, true))
            .or_else(|| (self.recognition().flattened_options(ty)).map(|(source, _, mapped)| (source, mapped)));
        if let Some((source, mapped)) = somes {
            let (kept, unboxed) = match &value.kind {
                js::ExprKind::Call(callee, args)
                    if let js::ExprKind::Member(kept, method) = &callee.kind
                        && method == "map"
                        && let [unbox] = args.as_slice()
                        && unbox.helper_name() == Some("$someValue") =>
                {
                    (&**kept, Some(unbox))
                }
                _ => (&value, None),
            };
            // Its source's, each `f`'s where a function makes the `Option`s.
            fn made(mapped: bool, source_value: &Expr) -> Option<(&Expr, Option<&Expr>)> {
                match (mapped, &source_value.kind) {
                    (false, _) => Some((source_value, None)),
                    (true, js::ExprKind::Call(map_callee, map_args))
                        if let js::ExprKind::Member(source_value, map) = &map_callee.kind
                            && map == "map"
                            && let [f] = map_args.as_slice() =>
                    {
                        Some((&**source_value, Some(f)))
                    }
                    _ => None,
                }
            }
            if let js::ExprKind::Call(callee, args) = &kept.kind
                && let js::ExprKind::Member(filtered, method) = &callee.kind
                && method == "filter"
                && let [present] = args.as_slice()
                && let Some((source_value, f)) = made(mapped, filtered)
            {
                let source_value = self.as_double_ended(source_value.clone(), source.peel_refs(), span)?;
                let mapped = match f {
                    Some(f) => Expr::call_helper("$mapEnds", vec![source_value, f.clone()]),
                    None => source_value,
                };
                let some = Expr::call_helper("$filterEnds", vec![mapped, present.clone()]);
                return Ok(match unboxed {
                    Some(unbox) => Expr::call_helper("$mapEnds", vec![some, unbox.clone()]),
                    None => some,
                });
            }
        }
        // The crate's, made knowing its back.
        if self.is_user_iterator(ty) {
            return self.user_iterator_knowing(value, ty, (true, false), span);
        }
        if self.knows_its_back(ty) {
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
        // An enumeration made here, `src.map((x, i) => [i, x])`: `$enumerate`
        // of its source made knowing its back and its length, whose index
        // from the back is where it is (ADR 0528): an array's items' `$iter`,
        // or another's, a `fuse()`'s of what drops (#383).
        // One made so already, itself (#764).
        if let Some(source) = crate::lower::recognition::enumerated_source(self.tcx, ty)
            && let Some(enumerated) = crate::lower::iterators::enumerated_of(&value)
        {
            if enumerated.form == crate::lower::iterators::EnumeratedForm::Made {
                return Ok(value.clone());
            }
            let source_value = enumerated.source;
            let items = match self.steps_an_array(source.peel_refs()) {
                true => self.std_iterator(source_value.clone(), source.peel_refs()),
                false => self.as_exact_size(source_value.clone(), source.peel_refs(), true, span)?,
            };
            return Ok(crate::lower::iterators::enumerated_knowing(items));
        }
        if let Some(stepped) = self.step_by_knowing(&value, ty, span)? {
            return Ok(stepped);
        }
        // A filter made here, `src.filter(p)`, or of items with a
        // destructor `$filterDropping(src, p, drop)` (#651): its source's
        // items `p` keeps, from either end, as Rust's `Filter` steps them
        // (ADR 0526), each it doesn't dropped by `drop`.
        if let Some(source) = crate::lower::recognition::filtered_source(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
        {
            let filtered = match (&callee.kind, args.as_slice()) {
                (js::ExprKind::Member(source_value, method), [p]) if method == "filter" => {
                    Some(((**source_value).clone(), vec![p.clone()]))
                }
                (_, [source_value, p, drop]) if callee.helper_name() == Some("$filterDropping") => {
                    Some((source_value.clone(), vec![p.clone(), drop.clone()]))
                }
                _ => None,
            };
            if let Some((source_value, given)) = filtered {
                let source_value = self.as_double_ended(source_value, source.peel_refs(), span)?;
                return Ok(Expr::call_helper("$filterEnds", [vec![source_value], given].concat()));
            }
        }
        // A type of generic code's whose own bounds don't make it
        // double-ended, `I` or `Take<I>` of an `I: Iterator`, whatever its
        // callers give: nothing steps it from its back, here or where it's
        // given, so it's as it is, a map of it made `$mapEnds` of it for its
        // hint (#1240).
        if self.recognition().never_double_ended(ty) {
            return Ok(value);
        }
        let why = self.back_refused_because(ty);
        Err(self.unsupported(
            span,
            &format!("a `{ty}` as a `DoubleEndedIterator`, which rust-js can't step from its back{why}"),
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
        if self.compiled_struct(ty, &it) {
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
                    Some(trait_id) if method == "len" => self.default_len(ty, trait_id),
                    _ => None,
                });
            }
            let len = steps.pop().expect("len");
            let back = steps.pop().expect("back");
            if back.is_none() && len.is_none() {
                return Ok(None);
            }
            // And its `rfold` and `nth_back`, std's compiled ones: its `rfold`
            // where it may reach the `rfold` of the crate's own, which std
            // passes on, as a `Fuse` does (#224), a type parameter's too; or
            // where it holds what drops, which std's drops where it does, a
            // `Peekable`'s source before what it peeked at (#649); of
            // another, the same as by `next_back`.
            let double_ended = self.recognition().double_ended_iterator();
            let std_order = self.drops(ty) != crate::lower::drops::Drops::Nothing;
            let rfold = match double_ended {
                Some(back) if std_order || self.recognition().reaches_its_own(ty, OwnStep::Rfold, Reach::May) => {
                    self.compiled_fold(ty, (back, "rfold"), span)?
                }
                _ => None,
            };
            let nth_back = match double_ended {
                Some(back) if self.compiled_step(ty, back, "nth_back") => {
                    Some(self.user_method(ty, back, "nth_back", &["k"], span)?.0)
                }
                _ => None,
            };
            // And its `try_rfold`, compiled, which `rfind` and what folds it
            // from its back by `try_rfold` ask (ADR 0930).
            let try_rfold = match double_ended {
                Some(back) => self.compiled_try_fold(ty, (back, "try_rfold"), span)?,
                None => None,
            };
            let own = [rfold, nth_back, try_rfold];
            let mut given = vec![
                back.unwrap_or_else(Expr::undefined),
                len.unwrap_or_else(Expr::undefined),
            ];
            let last = own.iter().rposition(Option::is_some).map_or(0, |at| at + 1);
            given.extend(
                own.into_iter()
                    .take(last)
                    .map(|entry| entry.unwrap_or_else(Expr::undefined)),
            );
            return Ok(Some(IterValue::of(Form::Knowing { it, given }).emit()));
        }
        Ok(None)
    }
}

/// What a `skip(n)` or a `take(n)` made here steps, `n`, and the drop it
/// drops what it skips by: of `src.drop(n)` or `src.take(n)`, none; of a
/// skip of items with a destructor, `$skipping(src, n, drop)` (`skipped`),
/// its `drop`.
fn counted_parts(value: &Expr, skip: bool) -> Option<(Expr, Expr, Option<Expr>)> {
    let js::ExprKind::Call(callee, args) = &value.kind else {
        return None;
    };
    let method = if skip { "drop" } else { "take" };
    match (&callee.kind, args.as_slice()) {
        (js::ExprKind::Member(source, name), [n]) if name == method => Some(((**source).clone(), n.clone(), None)),
        (_, [source, n, drop]) if skip && callee.helper_name() == Some("$skipping") => {
            Some((source.clone(), n.clone(), Some(drop.clone())))
        }
        _ => None,
    }
}

/// Of `f`, a function of one item that gives one value, `(item) =>
/// value`: its item's name and what it gives.
impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a flatten's `each` gives of its item: its item's name, what its
    /// body does first and what it returns, of an arrow that ends in its
    /// one `return`; or of a function given by its name, `pairs` or
    /// `Doubled.of`, which reads nothing that changes, its call of a fresh
    /// item (#1235).
    fn each_returned(&mut self, each: &Expr) -> Option<(String, Vec<Stmt>, Expr)> {
        if let Some((item, body, taken)) = returned_of(each) {
            return Some((item.clone(), body.to_vec(), taken.clone()));
        }
        let named = match &each.kind {
            js::ExprKind::Var(_) => true,
            js::ExprKind::Member(object, _) => matches!(object.kind, js::ExprKind::Var(_)),
            _ => false,
        };
        if !named {
            return None;
        }
        let item = self.fresh("item");
        let taken = Expr::call(each.clone(), vec![Expr::var(&item)]);
        Some((item, Vec::new(), taken))
    }
}

fn returned_of(f: &Expr) -> Option<(&String, &[Stmt], &Expr)> {
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    let ([js::Pattern::Name(param)], [before @ .., last]) = (params.as_slice(), body.as_slice()) else {
        return None;
    };
    // What it does first returns from nowhere: no statement that holds
    // others, which might.
    let plain = before.iter().all(|stmt| {
        matches!(
            stmt.kind,
            StmtKind::Const(..)
                | StmtKind::Let(..)
                | StmtKind::Destructure { .. }
                | StmtKind::Assign(..)
                | StmtKind::Compound(..)
                | StmtKind::Expr(_)
        )
    });
    match &last.kind {
        StmtKind::Return(Some(value)) if plain => Some((param, before, value)),
        _ => None,
    }
}
