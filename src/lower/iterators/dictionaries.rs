//! An iterator given as a trait's dictionary: a JS iterator's, an
//! extension's, a cycled one's.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value`, of `ty`, a type parameter given as it is, as a JS iterator
    /// stepped by its dictionaries (ADRs 0411, 0487): `$iterator(it,
    /// IIterator.next, boxed)`, given its back, `IDoubleEndedIterator.next_back`,
    /// and its length, `IExactSizeIterator.len`, where `back` and `len` ask
    /// them. `None` of another, or of one not given what's asked.
    pub(in crate::lower) fn given_js_iterator(&self, value: Expr, ty: Ty<'tcx>, back: bool, len: bool) -> Option<Expr> {
        let dictionary = self.iterator_evidence(ty, std_item(self.tcx, StdItem::Iterator))?;
        let entry = |asked: bool, trait_id: Option<DefId>| -> Option<Option<Expr>> {
            let trait_id = match asked {
                true => trait_id?,
                false => return Some(None),
            };
            let name = crate::lower::recognition::extension_entry(self.tcx, trait_id);
            Some(Some(Expr::member(self.iterator_evidence(ty, trait_id)?, name)))
        };
        let back = entry(back, self.recognition().double_ended_iterator())?;
        let len = entry(len, self.recognition().exact_size_iterator())?;
        let boxed = self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));
        Some(stepped_ends(
            value,
            Expr::member(dictionary, "next"),
            boxed,
            back,
            len,
            None,
        ))
    }

    /// `ty`'s dictionary of an iterator's extension trait, `trait_id`, given
    /// generic code that steps it as it is (ADR 0487): `DoubleEndedIterator`'s,
    /// `{ next_back }`, a range's `$rangeNextBack`, the crate's own its
    /// impl's, a JS iterator's that knows its back `$nextBack`; and
    /// `ExactSizeIterator`'s, `{ len }`, a range's `$rangeLen`, the crate's
    /// own its impl's, a JS iterator's that knows it its own `len()`.
    pub(in crate::lower) fn iterator_extension_dictionary(
        &mut self,
        trait_id: DefId,
        ty: ty::Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let ty = self.reveal(ty);
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
        match entry {
            Some(entry) => Ok(Expr::object(vec![js::Prop::Field(name.into(), entry)])),
            None => Err(self.unsupported(span, &format!("the `{}` of a `{ty}`", self.tcx.item_name(trait_id)))),
        }
    }

    /// `it.cycle()` of `it`, of type `ty`, an iterator not std's over an
    /// array (ADR 0181): `$cycleOf(it, clone, next, boxed)`, which keeps a
    /// clone of it and steps a fresh clone of that each time what it steps
    /// ends, as Rust's `Cycle` does (ADR 0491): a range's, the crate's own,
    /// or a type parameter's given as it is, by its dictionaries; given its
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
            // An array's, a struct holds, its `$iter` stepped (ADR 0544).
            None if self.steps_an_array(ty) => {
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
        let clone = self.clone_fn("iterator", ty, span)?.called_alone();
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
            props.push(js::Prop::Field("size_hint".into(), hint));
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
