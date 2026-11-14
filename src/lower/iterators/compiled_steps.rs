//! The steps a compiled iterator's struct takes, the crate's own or std's:
//! its `next`, `fold`, `try_fold`, `size_hint` and `len`, as its impl
//! writes them or std's defaults, and the dictionaries that give them to
//! generic code. What an iterator knows of its back and its length is
//! `knowing.rs`'.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
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
        let own = self.own_methods(ty, method == "next_back", span)?;
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
        // One made a JS iterator already, closed to drop it,
        // `$closing($iterator(value, next), drop)`: made knowing them inside,
        // still closed to drop it, not stepped as the crate's itself (#223).
        if let Some((it, drop)) = crate::lower::iterators::closed_parts(&value) {
            let it = stepped_value(it).unwrap_or_else(|| it.clone());
            let drop = drop.clone();
            let knowing = self.user_iterator_knowing(it, ty, (back, len), span)?;
            return Ok(self.closed_by(knowing, drop, ty));
        }
        // One lent as a `&mut`, `$lent($iterator(value, next))`: what it
        // lends made knowing them, given its own methods as std's `&mut I`
        // passes them on, not its folds, which would take it; nothing
        // closes it (#225).
        if is_lent(&value) {
            let lent = crate::lower::iterators::unlent(&value);
            let it = stepped_value(lent).unwrap_or_else(|| lent.clone());
            let borrowed = ty::Ty::new_mut_ref(self.tcx, self.tcx.lifetimes.re_erased, ty.peel_refs());
            return self.user_iterator_knowing(it, borrowed, (back, len), span);
        }
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let (next, boxed) = self.user_next(ty, iterator, "next", span)?;
        let hint = self.own_size_hint(ty, span)?;
        let back = match self.own_next_back(ty)? {
            Some(double_ended) if back => Some(self.user_next(ty, double_ended, "next_back", span)?.0),
            _ => None,
        };
        let len = (len && self.user_exact_size(ty)).then(|| exact_len_by(&hint));
        let own = self.own_methods(ty, false, span)?;
        Ok(stepped_ends(value, next, boxed, back, len, hint, own))
    }

    /// `rev()` of the crate's iterator of type `ty` made in place,
    /// `reversed_user`'s `$iterator(it, nextBack)`, made again knowing what a
    /// step past the call that made it asks (#257): its back, the
    /// iterator's own `next`, and its length, the iterator's, where it's
    /// exact-sized; its folds and its `nth`s each the other end's, as std's
    /// `Rev` passes them on. Closed to drop it still, and lent as it was, as
    /// `user_iterator_knowing`'s is.
    pub(in crate::lower) fn reversed_user_knowing(
        &mut self,
        value: Expr,
        ty: ty::Ty<'tcx>,
        (back, len): (bool, bool),
        span: Span,
    ) -> R<Expr> {
        let closed = crate::lower::iterators::closed_parts(&value).map(|(it, drop)| (it.clone(), drop.clone()));
        if let Some((it, drop)) = closed {
            let knowing = self.reversed_user_knowing(it, ty, (back, len), span)?;
            return Ok(self.closed_by(knowing, drop, ty));
        }
        let lent = is_lent(&value).then(|| crate::lower::iterators::unlent(&value).clone());
        if let Some(lent) = lent {
            let borrowed = ty::Ty::new_mut_ref(self.tcx, self.tcx.lifetimes.re_erased, ty.peel_refs());
            return self.reversed_user_knowing(lent, borrowed, (back, len), span);
        }
        // Of the form it's made in, `$iterator(it, nextBack)`: not one a
        // variable holds, whose form isn't there to be made again.
        let Some(value) = crate::lower::iterators::steps(&value).cloned() else {
            return Err(self.unsupported(
                span,
                &format!("a `rev()` of a `{ty}` a variable holds, stepped from its back past what's made of it"),
            ));
        };
        let Some(double_ended) = self.own_next_back(ty)? else {
            return Err(self.unsupported(span, &format!("`rev` of a `{ty}`, from its MIR")));
        };
        let (next, boxed) = self.user_next(ty, double_ended, "next_back", span)?;
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let back = match back {
            true => Some(self.user_next(ty, iterator, "next", span)?.0),
            false => None,
        };
        let hint = self.own_size_hint(ty, span)?;
        let len = (len && self.user_exact_size(ty)).then(|| exact_len_by(&hint));
        let forward = self.own_methods(ty, false, span)?;
        let own = crate::lower::iterators::OwnMethods {
            fold: forward.rfold,
            nth: forward.nth_back,
            last: None,
            rfold: forward.fold,
            nth_back: forward.nth,
            count: None,
        };
        Ok(stepped_ends(value, next, boxed, back, len, hint, own))
    }

    /// The `size_hint` of the crate's iterator `ty`, where its impl has one
    /// of its own: what `size_hint()` gives of it made a JS iterator, as of
    /// it as it is. `None` of one that has std's, `(0, None)`, which
    /// `$sizeHint` gives too.
    pub(super) fn own_size_hint(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        if !self.recognition().writes_size_hint(ty) {
            return Ok(None);
        }
        let iterator = std_item(self.tcx, StdItem::Iterator);
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
            false if self.hinted_by_its_struct(ty) => {
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
        // Its `nth`, the compiled impl's, which passes it on as std's does
        // (#182).
        if self.compiled_step(ty, iterator, "nth") {
            let (nth, _) = self.user_method(ty, iterator, "nth", &["n"], span)?;
            props.push(crate::js::Prop::Field("nth".into(), nth));
        }
        // Its `advance_by`, the compiled impl's, a `Take`'s, which a chain's
        // `nth` asks (ADR 0913).
        if self.compiled_step(ty, iterator, "advance_by") {
            let (advance, _) = self.user_method(ty, iterator, "advance_by", &["n"], span)?;
            props.push(crate::js::Prop::Field("advance".into(), advance));
        }
        // What of these std's compiled ones take whole, as Rust's move it.
        let mut takes = Vec::new();
        // And its `last`, its source's, which takes it whole and drops what
        // it doesn't give, its source's own `last` or `fold` that (#522).
        if self.compiled_step(ty, iterator, "last") {
            let (last, _) = self.user_next(ty, iterator, "last", span)?;
            props.push(crate::js::Prop::Field("last".into(), last));
            takes.push("last");
        }
        // Its fold, std's own compiled one where it has one, `mapIterator_fold`
        // of a map, whatever its source's own fold is among what it does
        // (#264, #297, ADR 0912).
        let compiled_fold = self.compiled_fold(ty, (iterator, "fold"), span)?;
        if let Some(fold) = compiled_fold {
            props.push(crate::js::Prop::Field("fold".into(), fold));
            takes.push("fold");
        }
        // And its `try_fold`, std's compiled one, in the JS iterator's
        // terms (ADR 0913).
        if let Some(try_fold) = self.compiled_try_fold(ty, (iterator, "try_fold"), span)? {
            props.push(crate::js::Prop::Field("tryFold".into(), try_fold));
        }
        // Its `count`, std's compiled one, a peekable's what it peeked at
        // then its source's own, a `Take`'s advancing its source (ADR 0913).
        if self.compiled_step(ty, iterator, "count") {
            let (count, _) = self.user_next(ty, iterator, "count", span)?;
            props.push(crate::js::Prop::Field("count".into(), count));
            takes.push("count");
        }
        if !takes.is_empty() {
            let takes = takes.into_iter().map(Expr::str).collect();
            props.push(crate::js::Prop::Field("takes".into(), Expr::array(takes)));
        }
        // Its `advance_by`, std's: a `Rev`'s its source's `advance_back_by`,
        // a `Vec`'s, which drops what it passes as one slice (#286).
        let advance = self.recognition().reversed(ty).map(|source| {
            let dictionary = self
                .iterator_evidence(source, iterator)
                .unwrap_or_else(Expr::js_undefined);
            let back = (self.recognition().double_ended_iterator())
                .and_then(|back| self.iterator_evidence(source, back))
                .unwrap_or_else(Expr::js_undefined);
            ("$revAdvanceBy", vec![dictionary, back])
        });
        // Not where std's is compiled, `steps.advance` (ADR 0913).
        let advance = advance.filter(|_| !self.compiled_step(ty, iterator, "advance_by"));
        if let Some((helper, given)) = advance {
            let mut args = vec![Expr::var("it"), Expr::var("m"), Expr::var("drop")];
            args.extend(given);
            // Its trailing `undefined`s left off, as a call writes none.
            while args.len() > 3
                && matches!(
                    args.last().map(|a| &a.kind),
                    Some(js::ExprKind::Undefined | js::ExprKind::Bool(false))
                )
            {
                args.pop();
            }
            let advance = Expr::arrow(
                vec!["it".into(), "m".into(), "drop".into()],
                vec![StmtKind::Return(Some(Expr::call_helper(helper, args))).at(js::Span::NONE)],
            );
            props.push(crate::js::Prop::Field("advanceBy".into(), advance));
        }
        // Of a kind dropped by closing, `Rev`'s, which an adapter over it
        // closes as it's closed: closing it drops what it holds, each field
        // by its drop, as Rust drops the struct (ADR 0871), the rest still
        // dropped after one whose drop panics, as its drop glue goes on
        // (#677).
        if self.dropped_by_closing(ty)
            && let ty::Adt(adt, args) = ty.peel_refs().kind()
        {
            let fields = (adt.all_fields())
                .map(|field| {
                    let field_ty = self
                        .tcx
                        .erase_and_anonymize_regions(field.ty(self.tcx, args).skip_normalization());
                    let value = Expr::member(
                        Expr::var("iterator"),
                        crate::lower::bindings::field_key(self.tcx, field).text(),
                    );
                    (value, field_ty)
                })
                .collect();
            let body = self.drop_each(fields, span)?;
            if !body.is_empty() {
                props.push(crate::js::Prop::Field(
                    "close".into(),
                    Expr::arrow(vec!["iterator".into()], body),
                ));
            }
        }
        props.extend(self.source_iter_steps(ty, span)?);
        Ok(Expr::object(props))
    }

    /// Whether std's struct `ty` is given its `size_hint` where it's made
    /// (`std_iterator_steps`): the compiled impl's, or a `Peekable`'s
    /// rewrite, `$peekableHint`, of one whose body reaches memory. The one
    /// answer, which its default `len` asks too (#706).
    fn hinted_by_its_struct(&self, ty: ty::Ty<'tcx>) -> bool {
        self.compiled_step(ty, std_item(self.tcx, StdItem::Iterator), "size_hint") || self.is_peekable(ty)
    }

    /// `ExactSizeIterator::len` of std's struct `ty` whose `len` is std's
    /// default, `Skip`'s or `Peekable`'s: its exact `size_hint`, its
    /// struct's (`hinted_by_its_struct`), `(iterator) =>
    /// $exactLen($sizeHint(iterator))` (ADR 0918). `None` of one that's no
    /// `ExactSizeIterator` here, `exact_size`, or whose struct has no hint:
    /// a `Peekable`'s was refused that, so a `skip()` of one stepped from
    /// its back read a length it didn't have (#706).
    pub(in crate::lower) fn default_len(&self, ty: ty::Ty<'tcx>, exact_size: DefId) -> Option<Expr> {
        let tr = (self.tcx).erase_and_anonymize_regions(ty::TraitRef::new(self.tcx, exact_size, [ty]));
        let exact = crate::lower::traits::select(self.tcx, self.typing_env, tr).is_ok();
        (exact && self.hinted_by_its_struct(ty)).then(|| {
            let hint = Expr::call_helper("$sizeHint", vec![Expr::var("iterator")]);
            Expr::arrow(
                vec!["iterator".into()],
                vec![StmtKind::Return(Some(Expr::call_helper("$exactLen", vec![hint]))).at(js::Span::NONE)],
            )
        })
    }

    /// Whether `ty`, a compiled std iterator, has `trait_id`'s `method`
    /// compiled, `Fuse`'s `next_back`, and is a `trait_id` here, so a
    /// use site can give what steps it by that (ADR 0561).
    pub(in crate::lower) fn compiled_step(&self, ty: ty::Ty<'tcx>, trait_id: DefId, method: &str) -> bool {
        debug_assert!(
            trait_id != std_item(self.tcx, StdItem::Iterator)
                || crate::lower::recognition::STRUCT_STEPS.contains(&method),
            "`{method}` is a step `type_facts` reads too (`STRUCT_STEPS`)"
        );
        let ty = ty.peel_refs();
        if !crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty) {
            return false;
        }
        let tr = self
            .tcx
            .erase_and_anonymize_regions(ty::TraitRef::new(self.tcx, trait_id, [ty]));
        crate::lower::traits::select(self.tcx, self.typing_env, tr).is_ok() && {
            // The trait's arguments, and a method's own parameters,
            // `rfold`'s `B` and `F`, themselves: they pick no impl.
            let method = trait_method(self.tcx, trait_id, method);
            let of = self.args_of(trait_id, ty);
            let args = ty::GenericArgs::for_item(self.tcx, method, |param, _| match of.get(param.index as usize) {
                Some(&arg) => arg,
                None => self.tcx.mk_param_from_def(param),
            });
            self.resolve_instance(method, args).is_ok_and(|instance| {
                instance.is_some_and(|i| crate::lower::recognition::compiled_instance(self.tcx, i.def_id(), i.args))
            })
        }
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
        self.user_method(ty, trait_id, method, &[], span)
    }

    /// `user_next` of a method that takes more than the iterator, `nth`'s
    /// `n`, `params`: a function of the iterator and them, in order.
    pub(in crate::lower) fn user_method(
        &mut self,
        ty: ty::Ty<'tcx>,
        trait_id: DefId,
        method: &str,
        params: &[&str],
        span: Span,
    ) -> R<(Expr, bool)> {
        let next = trait_method(self.tcx, trait_id, method);
        let args = self.args_of(trait_id, crate::lower::recognition::iterator_held(ty));
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
        let given: Vec<Expr> = std::iter::once("iterator")
            .chain(params.iter().copied())
            .map(Expr::var)
            .collect();
        let call = self.impl_call(next, args, given, span)?;
        let next = Expr::arrow(
            std::iter::once("iterator")
                .chain(params.iter().copied())
                .map(Into::into)
                .collect(),
            vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
        );
        let next = passed_on(&next, Taker::Made, &[]).unwrap_or(next);
        Ok((next, boxed))
    }

    /// The compiled `fold` or `rfold`, `method`, of std's struct `ty`, a
    /// held `Map`'s (ADRs 0871, 0912), as a dictionary's entry,
    /// `(iterator, init, f, unwind, ends) => ..`, given `unwind`, the
    /// runtime's fold's, `$foldDropping`'s or `$foldBack`'s, as what drops
    /// its accumulator, its `B`, where a stage's panic strands it inside its
    /// closure, before what's left (#423, #424), and `ends`, what drops with
    /// `f`, as its `F`'s drop, which its body drops where std's does, before
    /// what's left of what it folds (#710).
    /// `None` where it isn't compiled.
    pub(in crate::lower) fn compiled_fold(
        &mut self,
        ty: ty::Ty<'tcx>,
        (trait_id, method): (DefId, &str),
        span: Span,
    ) -> R<Option<Expr>> {
        match self.compiled_fold_of(ty, (trait_id, method))? {
            Some(found) => self.fold_entry(found, span).map(Some),
            None => Ok(None),
        }
    }

    /// `compiled_fold`'s entry, of the `fold` or `rfold` `fold` of `args`
    /// resolves to: std's compiled one, or the crate's own, given its
    /// impl's drops as its type's arguments have them (#423).
    pub(in crate::lower) fn fold_entry(
        &mut self,
        (fold, args): (DefId, ty::GenericArgsRef<'tcx>),
        span: Span,
    ) -> R<Expr> {
        let instance =
            (self.resolve_instance(fold, args)?).ok_or_else(|| self.unsupported(span, "this implementation"))?;
        // The method's own first type parameter, its `B`.
        let acc = self.tcx.generics_of(instance.def_id()).parent_count as u32;
        // Its parameters, named of the item's names, each its own (#1681's
        // change 3, M2), as preparation spells them (ADR 0352).
        let params =
            ["iterator", "init", "f", "unwind", "ends"].map(|name| self.fresh(crate::lower::Base::Named(name)));
        let [_, _, _, unwind, ends] = &params;
        let dropped = vec![(acc, Expr::var(unwind)), (acc + 1, Expr::var(ends))];
        let mut values = params[..3].iter().map(|name| Expr::var(name)).collect::<Vec<_>>();
        let location = self.caller_location(span);
        values.extend(self.evidence_args_given(
            instance.def_id(),
            instance.args,
            location,
            crate::lower::traits::GivenEvidence {
                bounds: &[],
                drops: &dropped,
            },
            span,
        )?);
        let call = Expr::call(self.fn_ref(instance.def_id()), values);
        Ok(Expr::arrow(
            params.into_iter().map(Into::into).collect(),
            vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
        ))
    }

    /// Std's compiled `try_fold` of std's struct `ty`, as its steps'
    /// `tryFold`, `(iterator, init, f, broke, unwind, ends) => ..`: of any
    /// `R`, whose `Try` is `$tryOf(broke)`, as a JS iterator's `tryFold`
    /// folds (ADR 0913), given `unwind` as `compiled_fold`'s is (#423), and
    /// `ends`, what drops with `f`, as its `F`'s drop, which its body drops
    /// where std's does, a `SkipWhile`'s before the accumulator (#768).
    /// Given no drops of what it folds, as `fold` is, so its `B` is `()`,
    /// its `F` a function of it and its `R` a `ControlFlow<(), ()>`, which
    /// nothing drops. `None` where std's isn't compiled. Of `method`, a
    /// trait's: `Iterator`'s `try_fold` or `DoubleEndedIterator`'s
    /// `try_rfold`, `tryRfold` (ADR 0930).
    pub(in crate::lower) fn compiled_try_fold(
        &mut self,
        ty: ty::Ty<'tcx>,
        (trait_id, method): (DefId, &str),
        span: Span,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        let Some(item) = self.iterator_item(ty) else {
            return Ok(None);
        };
        let control_flow = std_item(tcx, StdItem::ControlFlow);
        let unit = tcx.types.unit;
        let tried = ty::Ty::new_adt(tcx, tcx.adt_def(control_flow), tcx.mk_args(&[unit.into(), unit.into()]));
        let by = ty::Ty::new_fn_ptr(
            tcx,
            ty::Binder::dummy(tcx.mk_fn_sig(
                [unit, item],
                tried,
                ty::FnSigKind::default().set_safety(rustc_hir::Safety::Safe),
            )),
        );
        let try_fold = trait_method(tcx, trait_id, method);
        let args = tcx.mk_args(&[ty.into(), unit.into(), by.into(), tried.into()]);
        let Some(instance) = (self.resolve_instance(try_fold, args)?)
            .filter(|instance| crate::lower::recognition::compiled_instance(tcx, instance.def_id(), instance.args))
        else {
            return Ok(None);
        };
        let try_trait = tcx.require_lang_item(rustc_hir::attrs::lang_items::LangItem::Try, span);
        // Its parameters, named of the item's names, as `fold_entry`'s.
        let params = ["iterator", "init", "f", "broke", "unwind", "ends"]
            .map(|name| self.fresh(crate::lower::Base::Named(name)));
        let [iterator, init, f, broke, unwind, ends] = &params;
        let given = [(
            ty::TraitRef::new(tcx, try_trait, [tried]),
            Expr::call_helper("$tryOf", vec![Expr::var(broke)]),
        )];
        // Given `unwind`, as its `B`'s drop, and `ends`, as its `F`'s, as
        // `compiled_fold` is.
        let acc = tcx.generics_of(instance.def_id()).parent_count as u32;
        let mut values = [iterator, init, f].map(|name| Expr::var(name)).to_vec();
        let location = self.caller_location(span);
        values.extend(self.evidence_args_given(
            instance.def_id(),
            instance.args,
            location,
            crate::lower::traits::GivenEvidence {
                bounds: &given,
                drops: &[(acc, Expr::var(unwind)), (acc + 1, Expr::var(ends))],
            },
            span,
        )?);
        let call = Expr::call(self.fn_ref(instance.def_id()), values);
        Ok(Some(Expr::arrow(
            params.iter().map(|name| js::Pattern::Name(name.clone())).collect(),
            vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
        )))
    }

    /// Std's compiled `fold` or `rfold`, `method`, of std's struct `ty` and
    /// what it's called with: given no drops of what it folds into and by,
    /// as the crate's own is (#162), the caller's fold dropping them, so its
    /// own `B` and `F` are `()` and a function of it, which nothing drops.
    /// `None` where std's isn't compiled.
    pub(in crate::lower) fn compiled_fold_of(
        &self,
        ty: ty::Ty<'tcx>,
        (trait_id, method): (DefId, &str),
    ) -> R<Option<(DefId, ty::GenericArgsRef<'tcx>)>> {
        let Some(found) = self.fold_of(ty, (trait_id, method)) else {
            return Ok(None);
        };
        let compiled = self.resolve_instance(found.0, found.1)?.is_some_and(|instance| {
            crate::lower::recognition::compiled_instance(self.tcx, instance.def_id(), instance.args)
        });
        Ok(compiled.then_some(found))
    }

    /// The trait's `fold` or `rfold`, `method`, of `ty` and what it's
    /// called with as a dictionary's entry: given no drops of what it folds
    /// into and by, so its own `B` and `F` are `()` and a function of it.
    pub(in crate::lower) fn fold_of(
        &self,
        ty: ty::Ty<'tcx>,
        (trait_id, method): (DefId, &str),
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
        let tcx = self.tcx;
        let item = self.iterator_item(ty)?;
        let unit = tcx.types.unit;
        let by = ty::Ty::new_fn_ptr(
            tcx,
            ty::Binder::dummy(tcx.mk_fn_sig(
                [unit, item],
                unit,
                ty::FnSigKind::default().set_safety(rustc_hir::Safety::Safe),
            )),
        );
        let fold = trait_method(tcx, trait_id, method);
        Some((fold, tcx.mk_args(&[ty.into(), unit.into(), by.into()])))
    }

    /// The dictionary this function was given for `ty`, a type parameter
    /// given as it is, of its bound `trait_id`: `Iterator`'s (ADR 0411), or
    /// an extension's, `DoubleEndedIterator`'s or `ExactSizeIterator`'s
    /// (ADR 0487).
    pub(in crate::lower) fn iterator_evidence(&self, ty: ty::Ty<'tcx>, trait_id: DefId) -> Option<Expr> {
        // Of a copied default, its MIR's, in its impl's terms already (#803):
        // `T` of its trait's `Self` in `impl<T: Iterator> Trait for T` (ADR 0492).
        let ty = ty.peel_refs();
        // Only of one given as it is: another's is a JS iterator (ADR 0061),
        // whatever bound names `Iterator`, a supertrait's too. Or of what an
        // `IntoIterator` held as it is makes, through its dictionary (ADR
        // 0552).
        if !self.iterator_given_as_it_is(ty) {
            return None;
        }
        // A `Box<I>`'s, `I`'s, of what std's `Box<I>` passes on (#1611).
        if let Some(inner) = ty.boxed_ty() {
            return Some(self.boxed_iterator_dictionary(trait_id, self.iterator_evidence(inner, trait_id)?));
        }
        self.evidence_for(ty::TraitRef::new(self.tcx, trait_id, [ty]))
    }
}

/// The length of the crate's iterator by its hint, `hint` its own
/// `size_hint` or std's, which std's `len()` checks is exact (`$exactLen`,
/// ADR 0456): `(it) => $exactLen(hint(it))`.
fn exact_len_by(hint: &Option<Expr>) -> Expr {
    let hint_of = match hint {
        Some(hint) => Expr::call(hint.clone(), vec![Expr::var("it")]),
        None => Expr::array(vec![Expr::int(0), Expr::js_undefined()]),
    };
    let exact = Expr::call_helper("$exactLen", vec![hint_of]);
    Expr::arrow(
        vec!["it".into()],
        vec![StmtKind::Return(Some(exact)).at(js::Span::NONE)],
    )
}
