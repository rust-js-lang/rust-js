//! An iterator given as a trait's dictionary: a JS iterator's, an
//! extension's, a cycled one's.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value`, of `ty`, a type parameter given as it is, as a JS iterator
    /// stepped by its dictionaries (ADRs 0411, 0487): `$given(it, IIterator,
    /// boxed)`, its `next`, and its own `fold`, `nth` and `size_hint` where
    /// its impl writes them (#162), given its back, `IDoubleEndedIterator.next_back`,
    /// and its length, `IExactSizeIterator.len`, where `back` and `len` ask
    /// them. `None` of another, or of one not given what's asked.
    pub(in crate::lower) fn given_js_iterator(&self, value: Expr, ty: Ty<'tcx>, back: bool, len: bool) -> Option<Expr> {
        let dictionary = self.iterator_evidence(ty, std_item(self.tcx, StdItem::Iterator))?;
        let entry = |asked: bool, trait_id: Option<DefId>| -> Option<Option<Expr>> {
            let trait_id = match asked {
                true => trait_id?,
                false => return Some(None),
            };
            // Its back, the dictionary itself, whose `next_back`, and own
            // `rfold` and `nth_back`, `$given` reads (#224); its length,
            // its `len`.
            let dictionary = self.iterator_evidence(ty, trait_id)?;
            if crate::lower::recognition::is_double_ended(self.tcx, trait_id) {
                return Some(Some(dictionary));
            }
            let name = crate::lower::recognition::extension_entry(self.tcx, trait_id);
            Some(Some(Expr::member(dictionary, name)))
        };
        let back = entry(back, self.recognition().double_ended_iterator())?;
        let len = entry(len, self.recognition().exact_size_iterator())?;
        let boxed = self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));
        // Each given, and each before it, `undefined` where it isn't.
        let given = [boxed.then(|| Expr::bool(true)), back, len];
        let last = given.iter().rposition(Option::is_some).map_or(0, |at| at + 1);
        let mut list = vec![value, dictionary];
        for (at, arg) in given.into_iter().enumerate().take(last) {
            list.push(arg.unwrap_or_else(|| match at {
                0 => Expr::bool(false),
                _ => Expr::undefined(),
            }));
        }
        Some(Expr::call_helper("$given", list))
    }

    /// `ty`'s dictionary of an iterator's extension trait, `trait_id`, given
    /// generic code that steps it as it is (ADR 0487): `DoubleEndedIterator`'s,
    /// `{ next_back }`, a range's `$rangeNextBack`, the crate's own its
    /// impl's, a JS iterator's that knows its back `$nextBack`; and
    /// `ExactSizeIterator`'s, `{ len }`, a range's `$rangeLen`, the crate's
    /// own its impl's, a JS iterator's that knows it its own `len()`; of
    /// std's compiled struct, its compiled one.
    pub(in crate::lower) fn iterator_extension_dictionary(
        &mut self,
        trait_id: DefId,
        ty: ty::Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        // A `&mut` to one is it, whose impls pass on to its own.
        let ty = self.reveal(ty.peel_refs());
        let name = crate::lower::recognition::extension_entry(self.tcx, trait_id);
        let double_ended = crate::lower::recognition::is_double_ended(self.tcx, trait_id);
        let returned = |value: Expr| {
            Expr::arrow(
                vec!["iterator".into()],
                vec![StmtKind::Return(Some(value)).at(js::Span::NONE)],
            )
        };
        let entry = match (double_ended, self.range_kind(ty)) {
            (true, Some(kind)) => kind.step(true).map(Expr::helper),
            (false, Some(RangeKind::Exclusive)) => Some(Expr::helper("$rangeLen")),
            (_, Some(_)) => None,
            // Std's struct, which generic code given it as it is is given
            // where it's held, a `Map`'s (ADR 0871): its compiled
            // `next_back` or `len`.
            (_, None) if self.compiled_step(ty, trait_id, name) => Some(self.user_next(ty, trait_id, name, span)?.0),
            (true, None) if self.is_user_iterator(ty) => Some(self.user_next(ty, trait_id, name, span)?.0),
            (false, None) if self.is_user_iterator(ty) => {
                let mut made = Vec::new();
                let len = self.exact_len_of(Expr::var("iterator"), ty, span, &mut made)?;
                made.is_empty().then(|| returned(len))
            }
            (true, None) if self.steps_an_array(ty) || self.knows_its_back(ty) => {
                let boxed = self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));
                Some(Expr::helper(if boxed { "$nextBackSome" } else { "$nextBack" }))
            }
            (false, None) if self.steps_an_array(ty) || self.knows_its_length(ty) => Some(returned(Expr::call(
                Expr::member(Expr::var("iterator"), "len"),
                Vec::new(),
            ))),
            (_, None) => None,
        };
        let Some(entry) = entry else {
            return Err(self.unsupported(span, &format!("the `{}` of a `{ty}`", self.tcx.item_name(trait_id))));
        };
        let mut props = vec![js::Prop::Field(name.into(), entry)];
        // The crate's own `rfold` and `nth_back`, where its impl writes them,
        // which std's compiled code of it, a `fuse()`'s, asks (#224).
        // Of std's adapter over the crate's own, the JS iterator's, which
        // passes it on as std's does, `$foldBack`.
        // Of std's struct, a held `Map`'s, its compiled ones (ADR 0871).
        if double_ended && crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty) {
            let rfold = self.compiled_rfold(ty, trait_id, span)?;
            let nth_back = match self.compiled_step(ty, trait_id, "nth_back") {
                true => Some(self.user_method(ty, trait_id, "nth_back", &["k"], span)?.0),
                false => None,
            };
            for (method, own) in [("rfold", rfold), ("nth_back", nth_back)] {
                if let Some(own) = own {
                    props.push(js::Prop::Field(
                        crate::lower::bindings::trait_method_name(self.tcx, trait_id, method),
                        own,
                    ));
                }
            }
        } else if double_ended {
            let rfold = match self.is_user_iterator(ty) {
                true => self.own_methods(ty, false, span)?.rfold,
                false => self.recognition().rfolds_its_own(ty).then(|| Expr::helper("$foldBack")),
            };
            // A range's of numbers, of its bounds, std's own (#163), so what
            // steps it from its back by `nth_back`, a held `Rev`'s `nth`
            // (ADR 0871), never steps each number.
            let numbers = self.range_index(ty).and_then(Num::of).is_some();
            let nth_back = match (self.is_user_iterator(ty), self.range_kind(ty)) {
                (true, _) => self.own_methods(ty, false, span)?.nth_back,
                (false, Some(RangeKind::Exclusive)) if numbers => Some(Expr::helper("$rangeNthBack")),
                (false, Some(RangeKind::Inclusive)) if numbers => Some(Expr::helper("$rangeInclusiveNthBack")),
                (false, _) => None,
            };
            for (method, own) in [("rfold", rfold), ("nth_back", nth_back)] {
                if let Some(own) = own {
                    props.push(js::Prop::Field(
                        crate::lower::bindings::trait_method_name(self.tcx, trait_id, method),
                        own,
                    ));
                }
            }
        }
        Ok(Expr::object(props))
    }

    /// `it.cycle()` of `it`, of type `ty`, an iterator not std's over an
    /// array (ADR 0181): `$cycleOf(it, clone, next, boxed)`, which keeps a
    /// clone of it and steps a fresh clone of that each time what it steps
    /// ends, as Rust's `Cycle` does (ADR 0491): a range's, the crate's own,
    /// an adapter cloned apart, a `map`'s or a `filter`'s (ADR 0525), or a
    /// type parameter's given as it is, by its dictionaries; given its
    /// drop, of one with a destructor, which drops each it replaces, and
    /// both it holds as it's closed.
    pub(in crate::lower) fn cycled(&mut self, it: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = self.reveal(ty);
        let (next, boxed) = match self.range_kind(ty) {
            Some(kind) => match kind.step(false) {
                Some(step) => (Expr::helper(step), false),
                None => return Err(self.unsupported(span, &format!("`cycle()` of a `{ty}`"))),
            },
            None if self.is_user_iterator(ty) => {
                let iterator = std_item(self.tcx, StdItem::Iterator);
                self.user_next(ty, iterator, "next", span)?
            }
            // An array's, a struct holds, its `$iter` stepped (ADR 0544),
            // and an adapter cloned apart, a `map`'s or a `filter`'s, the
            // JS iterator it is, which its clone makes again (ADR 0525).
            None if self.steps_an_array(ty) || self.iterator_cloned_apart(ty) => {
                let boxed = self.iterator_item(ty).is_some_and(|item| self.boxed_payload(item));
                (Expr::helper(if boxed { "$nextSome" } else { "$next" }), boxed)
            }
            None => match self.iterator_evidence(ty, std_item(self.tcx, StdItem::Iterator)) {
                Some(dictionary) => {
                    let boxed = self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));
                    (Expr::member(dictionary, "next"), boxed)
                }
                None => return Err(self.unsupported(span, &format!("`cycle()` of a `{ty}`"))),
            },
        };
        // Std's struct, a `Map` held (ADR 0871), cloned by its compiled
        // `Clone`; another, the JS iterator it is.
        let mut body = Vec::new();
        let cloned = match self.compiled_struct_at(ty, span) {
            true => self.clone_value(Expr::var("iterator"), ty, span, &mut body)?,
            false => self.clone_in_place(Expr::var("iterator"), ty, span, &mut body)?,
        };
        body.push(StmtKind::Return(Some(cloned)).at(js::Span::NONE));
        let clone = Expr::arrow(vec!["iterator".into()], body).called_alone();
        let drop = self.drop_function(ty, span)?;
        let mut args = vec![it, clone, next];
        if boxed || drop.is_some() {
            args.push(Expr::bool(boxed));
        }
        args.extend(drop);
        Ok(Expr::call_helper("$cycleOf", args))
    }

    /// `ty`'s `Iterator` dictionary, `{ next }` (ADR 0411): its `next`
    /// steps a `ty` as it is, a range's `$rangeNext`, the crate's own its
    /// impl's, a JS iterator's `$next`, each giving the `Option` generic code
    /// reads, a `Some` that looks like `None` boxed (ADR 0051).
    pub(in crate::lower) fn iterator_dictionary(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = self.reveal(ty);
        let next = match self.range_kind(ty) {
            Some(kind) => match kind.step(false) {
                Some(step) => Expr::helper(step),
                None => return Err(self.unsupported(span, &format!("the `Iterator` of a `{ty}`"))),
            },
            None if self.is_user_iterator(ty) => {
                let trait_id = std_item(self.tcx, StdItem::Iterator);
                self.user_next(ty, trait_id, "next", span)?.0
            }
            None => match self.iterator_item(ty) {
                Some(item) if !self.boxed_payload(item) => Expr::helper("$next"),
                _ => Expr::helper("$nextSome"),
            },
        };
        let mut props = vec![js::Prop::Field("next".into(), next)];
        // The crate's own `size_hint`, where its impl has one (ADR 0170),
        // which `$sizeHint` asks of what it's given.
        if self.is_user_iterator(ty)
            && let Some(hint) = self.own_size_hint(ty, span)?
        {
            let iterator = std_item(self.tcx, StdItem::Iterator);
            let size_hint = crate::lower::bindings::trait_method_name(self.tcx, iterator, "size_hint");
            props.push(js::Prop::Field(size_hint, hint));
        }
        // And its own `fold`, `nth` and `last`, which std's consumers of it
        // call, `$foldBy`'s (#162, #182).
        let crate::lower::iterators::OwnMethods { fold, nth, last, .. } = self.own_methods(ty, false, span)?;
        let iterator = std_item(self.tcx, StdItem::Iterator);
        // Of std's adapter over the crate's own, the JS iterator's, which
        // passes them on as std's does (#214).
        let (fold, nth, last) = match self.is_user_iterator(ty) {
            true => (fold, nth, last),
            false => {
                let recognition = self.recognition();
                let boxed = self.iterator_item(ty).is_some_and(|item| self.boxed_payload(item));
                let folds = recognition.folds_its_own(ty) || recognition.folds_by_its_own_nth(ty);
                (
                    folds.then(|| Expr::helper("$foldOf")),
                    recognition
                        .nths_its_own(ty)
                        .then(|| Expr::helper(if boxed { "$nthSome" } else { "$nth" })),
                    recognition
                        .lasts_its_own(ty)
                        .then(|| Expr::helper(if boxed { "$lastSome" } else { "$last" })),
                )
            }
        };
        for (name, own) in [("fold", fold), ("nth", nth), ("last", last)] {
            if let Some(own) = own {
                props.push(js::Prop::Field(
                    crate::lower::bindings::trait_method_name(self.tcx, iterator, name),
                    own,
                ));
            }
        }
        Ok(Expr::object(props))
    }

    /// What an `IntoIterator` of type `ty` gives, its `Item`.
    pub(in crate::lower) fn iterable_item(&self, ty: ty::Ty<'tcx>) -> ty::Ty<'tcx> {
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        let item = trait_method(self.tcx, into_iterator, "Item");
        let projection = ty::Ty::new_projection(self.tcx, ty::IsRigid::No, item, [ty]);
        (self.tcx)
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection))
            .unwrap_or(projection)
    }
}
