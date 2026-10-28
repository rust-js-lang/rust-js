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
    /// `ExactSizeIterator`'s, `{ len }`, a range's `$rangeLen` or
    /// `$rangeInclusiveLen`, the crate's
    /// own its impl's, a JS iterator's that knows it its own `len()`; of
    /// std's compiled struct, its compiled one.
    pub(in crate::lower) fn iterator_extension_dictionary(
        &mut self,
        trait_id: DefId,
        ty: ty::Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        // A `&mut` to one is it, whose impls pass on to its own: its
        // `next_back` and `nth_back`, not its `rfold`, which std's `&mut I`
        // folds by `I`'s `try_rfold` (ADR 0928).
        let lent = matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Mut));
        let passed_rfold = |rfold: Option<Expr>| rfold.filter(|_| !lent);
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
            // `a..=b`'s, which rustc gives only 8- and 16-bit numbers, JS's
            // own numbers (ADR 0918).
            (false, Some(RangeKind::Inclusive)) => Some(Expr::helper("$rangeInclusiveLen")),
            (_, Some(_)) => None,
            // Std's struct, which generic code given it as it is is given
            // where it's held, a `Map`'s (ADR 0871): its compiled
            // `next_back` or `len`.
            (_, None) if self.compiled_step(ty, trait_id, name) => Some(self.user_next(ty, trait_id, name, span)?.0),
            // Of one whose `len` is std's default, `Skip`'s (ADR 0918).
            (false, None) if let Some(len) = self.default_len(ty, trait_id) => Some(len),
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
            let rfold = self.compiled_fold(ty, (trait_id, "rfold"), span)?;
            let nth_back = match self.compiled_step(ty, trait_id, "nth_back") {
                true => Some(self.user_method(ty, trait_id, "nth_back", &["k"], span)?.0),
                false => None,
            };
            // And its `try_rfold`, which std's compiled code of it asks, a
            // `Skip`'s of its `Take`: as a JS iterator's `tryRfold` (ADR 0930).
            let try_rfold = self.compiled_try_fold(ty, (trait_id, "try_rfold"), span)?;
            for (method, own) in [
                ("rfold", passed_rfold(rfold)),
                ("nth_back", nth_back),
                ("try_rfold", try_rfold),
            ] {
                if let Some(own) = own {
                    props.push(js::Prop::Field(
                        crate::lower::bindings::trait_method_name(self.tcx, trait_id, method),
                        own,
                    ));
                }
            }
        } else if double_ended {
            // The crate's own `rfold`; of another, none: one a filter of the
            // crate's iterator holds is folded by its own `rfold` without it
            // (#224, #382).
            let rfold = match self.is_user_iterator(ty) {
                true => self.own_methods(ty, false, span)?.rfold,
                false => None,
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
            for (method, own) in [("rfold", passed_rfold(rfold)), ("nth_back", nth_back)] {
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
            // an adapter cloned apart, a `map`'s or a `filter`'s, the JS
            // iterator it is, which its clone makes again (ADR 0525), and
            // std's struct, a `rev()`'s, which its compiled `Clone` clones
            // (ADR 0915).
            None if self.steps_an_array(ty) || self.iterator_cloned_apart(ty) || self.compiled_struct_at(ty, span) => {
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
        let clone = Expr::arrow(vec!["iterator".into()], body);
        let clone = passed_on(&clone, Taker::Made, &[]).unwrap_or(clone);
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
                // Std's struct whose `fold` std compiles folds by that, its
                // meaning, whatever its sources are (ADR 0912).
                let folds = recognition.folds_its_own(ty)
                    || recognition.folds_by_its_own_nth(ty)
                    || (crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty)
                        && self.compiled_fold_of(ty, (iterator, "fold"))?.is_some());
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
        // Of one std reads by index, its reader of an item at `i`, which
        // std's zip of it asks where it's compiled (ADR 0975).
        if crate::lower::recognition::zips_by_index_compiled()
            && crate::lower::recognition::fact_trait(
                self.tcx,
                crate::lower::recognition::TypeFact::RandomAccessNoCoerce,
            )
            .is_some_and(|random_access| self.recognition().implements_trait(ty, random_access))
            && let Some(Some(reader)) = self.index_reader(ty, span)?
        {
            props.push(js::Prop::Field("getUnchecked".into(), reader));
        }
        Ok(Expr::object(props))
    }

    /// What reads the item `i` past where it is of `ty`, an iterator std
    /// reads by index, `__iterator_get_unchecked`'s (ADR 0975): std's
    /// compiled adapter's own reader, its dictionary's `getUnchecked`;
    /// `Some(None)` of an array's `$iter` or a range, which `$getUnchecked`
    /// reads as it is; `None` of another, which std's zip of can't be read
    /// as std reads it.
    pub(in crate::lower) fn index_reader(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Option<Option<Expr>>> {
        let ty = self.reveal(ty);
        // Std's struct first, a `copied()` of a slice's too, which is no
        // array's `$iter` where it's compiled.
        let iterator = std_item(self.tcx, StdItem::Iterator);
        if crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty) {
            return Ok(match self.compiled_step(ty, iterator, "__iterator_get_unchecked") {
                true => Some(Some(
                    self.user_method(ty, iterator, "__iterator_get_unchecked", &["i"], span)?
                        .0,
                )),
                false => None,
            });
        }
        Ok((self.range_kind(ty).is_some() || self.steps_an_array(ty)).then_some(None))
    }

    /// The `Iterator` dictionary of a `&mut I`, of `I`'s, `of`: what std's
    /// impl for it passes on to its `I`, its `next`, `size_hint` and `nth`;
    /// not its `fold`, which is `I`'s `try_fold`, nor its `last`, std's of
    /// `next`, as a JS iterator lent is (`$lent`, ADR 0912).
    pub(in crate::lower) fn mut_ref_iterator_dictionary(&self, of: Expr) -> Expr {
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let passed: Vec<String> = ["next", "size_hint", "nth"]
            .map(|name| crate::lower::bindings::trait_method_name(self.tcx, iterator, name))
            .into();
        entries_passed_on(of, passed)
    }

    /// An extension's dictionary of a `&mut I`, of `I`'s, `of`: what std's
    /// impl for it passes on to its `I`, `next_back`, `nth_back` and
    /// `try_rfold` of a `DoubleEndedIterator`, `len` of an
    /// `ExactSizeIterator`; not `rfold`, which it folds by `I`'s `try_rfold`
    /// (ADRs 0928, 0937).
    pub(in crate::lower) fn mut_ref_extension_dictionary(&self, trait_id: DefId, of: Expr) -> Expr {
        // Its entry, `next_back` or `len`, keyed as the extension's own are,
        // and the methods it passes on, by their names.
        let others: &[&str] = match crate::lower::recognition::is_double_ended(self.tcx, trait_id) {
            true => &["nth_back", "try_rfold"],
            false => &[],
        };
        let entry = crate::lower::recognition::extension_entry(self.tcx, trait_id).to_string();
        let passed: Vec<String> = std::iter::once(entry)
            .chain(
                others
                    .iter()
                    .map(|name| crate::lower::bindings::trait_method_name(self.tcx, trait_id, name)),
            )
            .collect();
        entries_passed_on(of, passed)
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

/// The entries of a dictionary, `of`, a `&mut` to what it's of passes on,
/// `passed`: of one made here, those; of one given, each read from it.
fn entries_passed_on(of: Expr, passed: Vec<String>) -> Expr {
    match of.kind {
        js::ExprKind::Object(props, _) => Expr::object(
            (props.into_iter())
                .filter(|prop| matches!(prop, js::Prop::Field(name, _) if passed.contains(name)))
                .collect(),
        ),
        _ => Expr::object(
            (passed.into_iter())
                .map(|name| js::Prop::Field(name.clone(), Expr::member(of.clone(), &name)))
                .collect(),
        ),
    }
}
