//! An iterator given as a trait's dictionary: a JS iterator's, an
//! extension's, a cycled one's.

use super::*;
use crate::recognition::{OwnStep, Reach};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value`, of `ty`, a type parameter given as it is, as a JS iterator
    /// stepped by its dictionaries (ADRs 0411, 0487): `$given(it, IIterator,
    /// boxed)`, its `next`, and its own `fold`, `nth` and `size_hint` where
    /// its impl writes them (#162), given its back, `IDoubleEndedIterator.next_back`,
    /// and its length, `IExactSizeIterator.len`, where `back` and `len` ask
    /// them. `None` of another, or of one not given what's asked.
    pub(crate) fn given_js_iterator(&self, value: Expr, ty: Ty<'tcx>, back: bool, len: bool) -> Option<Expr> {
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
            if crate::recognition::is_double_ended(self.tcx, trait_id) {
                return Some(Some(dictionary));
            }
            let name = crate::recognition::extension_entry(self.tcx, trait_id);
            Some(Some(Expr::member(dictionary, name)))
        };
        let back = entry(back, self.recognition().double_ended_iterator())?;
        let len = entry(len, self.recognition().exact_size_iterator())?;
        let boxed = self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));
        // Each given, and each before it, `undefined` where it isn't.
        let given = [boxed.then(|| Expr::bool(true)), back, len];
        let last = given.iter().rposition(Option::is_some).map_or(0, |at| at + 1);
        let given = (given.into_iter().enumerate().take(last))
            .map(|(at, arg)| {
                arg.unwrap_or_else(|| match at {
                    0 => Expr::bool(false),
                    _ => Expr::js_undefined(),
                })
            })
            .collect();
        Some(
            IterValue::of(Form::Given {
                value,
                dictionary,
                given,
            })
            .emit(),
        )
    }

    /// The dictionary of an iterator's extension trait, `trait_id`, of the
    /// crate's iterator `ty` made a JS iterator that knows its back and its
    /// length, as a call gives it for a `T::IntoIter` generic code holds so
    /// (ADR 0480, #1604): the JS iterator's own, `{ next_back: $nextBack }`
    /// and `$foldBack`, which reaches the iterator's own `rfold` through the
    /// value and says whether it took it (`$took`, #649), so std's compiled
    /// `Rev` that folds it doesn't drop it again; or `{ len }`, its `len()`.
    /// Not its impl's, which steps it as it is (`iterator_extension_dictionary`).
    pub(crate) fn made_iterator_extension_dictionary(&self, trait_id: DefId, ty: ty::Ty<'tcx>) -> Expr {
        let name = crate::recognition::extension_entry(self.tcx, trait_id);
        let double_ended = crate::recognition::is_double_ended(self.tcx, trait_id);
        let mut props = vec![js::Prop::Field(name.into(), self.knowing_entry(double_ended, ty))];
        if double_ended {
            let rfold = crate::bindings::trait_method_name(self.tcx, trait_id, "rfold");
            props.push(js::Prop::Field(rfold, Expr::helper("$foldBack")));
        }
        Expr::object(props)
    }

    /// The entry of a JS iterator of items of `ty` that knows its back,
    /// `double_ended`, or its length: its `next_back`, `$nextBack`, or
    /// `$nextBackSome` where its items are boxed, or its `len()`.
    fn knowing_entry(&self, double_ended: bool, ty: ty::Ty<'tcx>) -> Expr {
        let made_iterator = self.unspelled("iterator");
        if double_ended {
            let boxed = self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));
            return Expr::helper(if boxed { "$nextBackSome" } else { "$nextBack" });
        }
        let len = Expr::call(Expr::member(Expr::var(&made_iterator), "len"), Vec::new());
        Expr::arrow(
            vec![made_iterator.clone().into()],
            vec![StmtKind::Return(Some(len)).at(js::Span::NONE)],
        )
    }

    /// `ty`'s dictionary of an iterator's extension trait, `trait_id`, given
    /// generic code that steps it as it is (ADR 0487): `DoubleEndedIterator`'s,
    /// `{ next_back }`, a range's `$rangeNextBack`, the crate's own its
    /// impl's, a JS iterator's that knows its back `$nextBack`; and
    /// `ExactSizeIterator`'s, `{ len }`, a range's `$rangeLen` or
    /// `$rangeInclusiveLen`, the crate's
    /// own its impl's, a JS iterator's that knows it its own `len()`; of
    /// std's compiled struct, its compiled one.
    pub(crate) fn iterator_extension_dictionary(&mut self, trait_id: DefId, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        // A `&mut` to one is it, whose impls pass on to its own: its
        // `next_back` and `nth_back`, not its `rfold`, which std's `&mut I`
        // folds by `I`'s `try_rfold` (ADR 0928).
        // Of a `&mut I`, its `rfold` as std's `&mut I` passes it on
        // (`Wrapper::passes`): of a sized `I`, by `I`'s `try_rfold`, never
        // stopped (`DoubleEndedIteratorRefSpec`), where std's compiled `Rev`
        // of it folded by `next_back`, a `take()`'s skipped items 4 then 5,
        // std's 5 then 4 (#2002); of an unsized one, none, std's default.
        let lent = crate::recognition::Wrapper::of(self.tcx, self.typing_env, ty).filter(|w| !w.owns());
        let passed_rfold = |rfold: Option<Expr>, try_rfold: Option<&Expr>| {
            use crate::recognition::Passed;
            match lent.map(|w| w.passes("rfold")) {
                None | Some(Passed::Own) => rfold,
                Some(Passed::ByTry) => try_rfold.cloned().map(folded_by_try),
                Some(Passed::Default) => None,
            }
        };
        let ty = ty.peel_refs();
        let name = crate::recognition::extension_entry(self.tcx, trait_id);
        // A `Box` of std's struct, compiled: what std's `Box<I>` passes on of
        // it, its `next_back`, `nth_back` and `len`; its folds the traits'
        // defaults, none of its own (#1995).
        let boxed = ty
            .boxed_ty()
            .filter(|&inner| crate::recognition::is_compiled_std_iterator(self.tcx, inner));
        let double_ended = crate::recognition::is_double_ended(self.tcx, trait_id);
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
            (_, None)
                if let Some(inner) = boxed
                    && self.compiled_step(inner, trait_id, name) =>
            {
                Some(self.user_next(inner, trait_id, name, span)?.0)
            }
            // Of one whose `len` is std's default, `Skip`'s (ADR 0918).
            (false, None) if let Some(len) = self.default_len(ty, trait_id) => Some(len),
            (true, None) if self.is_user_iterator(ty) => Some(self.user_next(ty, trait_id, name, span)?.0),
            // The crate's, its own or its hint's (`user_len`, #2073).
            (false, None) if self.is_user_iterator(ty) => {
                let hint = self.own_size_hint(ty, span)?;
                Some(self.user_len(ty, &hint, span)?)
            }
            // And a `repeat_n`'s or a `take(n)` of a `repeat`'s, whose runtime
            // form steps its back as its front (#1139).
            (true, None)
                if self.steps_an_array(ty)
                    || self.knows_its_back(ty)
                    || crate::recognition::steps_back_as_front(self.tcx, ty) =>
            {
                Some(self.knowing_entry(true, ty))
            }
            (false, None)
                if self.steps_an_array(ty)
                    || self.knows_its_length(ty)
                    || crate::recognition::steps_back_as_front(self.tcx, ty) =>
            {
                Some(self.knowing_entry(false, ty))
            }
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
        if let Some(inner) = boxed {
            if double_ended && self.compiled_step(inner, trait_id, "nth_back") {
                let nth_back = self.user_method(inner, trait_id, "nth_back", &["k"], span)?.0;
                props.push(js::Prop::Field(
                    crate::bindings::trait_method_name(self.tcx, trait_id, "nth_back"),
                    nth_back,
                ));
            }
        } else if double_ended && crate::recognition::is_compiled_std_iterator(self.tcx, ty) {
            // Each of its back's steps std writes (`back_steps`), compiled:
            // its `try_rfold`, which std's compiled code of it asks, a
            // `Skip`'s of its `Take`, as a JS iterator's `tryRfold` (ADR
            // 0930); its `advance_back_by`, which drops what it passes as
            // its source's does, a `Vec`'s as one slice (#2125).
            let try_rfold = self.compiled_try_fold(ty, (trait_id, "try_rfold"), span)?;
            for method in crate::recognition::back_steps() {
                let own = match method {
                    "rfold" => passed_rfold(self.compiled_fold(ty, (trait_id, "rfold"), span)?, try_rfold.as_ref()),
                    "try_rfold" => try_rfold.clone(),
                    _ if self.compiled_step(ty, trait_id, method) => {
                        Some(self.user_method(ty, trait_id, method, &[counted_by(method)], span)?.0)
                    }
                    _ => None,
                };
                if let Some(own) = own {
                    props.push(js::Prop::Field(
                        crate::bindings::trait_method_name(self.tcx, trait_id, method),
                        own,
                    ));
                }
            }
        } else if double_ended {
            // The crate's own `rfold`; of another that may reach it, a type
            // parameter's or std's adapter's over one, `$foldBack`, which asks
            // the value, so what it gives this to asks the value whether its
            // own took it (`$took`, #649); of another, none: one a filter of
            // the crate's iterator holds is folded by its own `rfold` without
            // it (#224, #382).
            let rfold = match self.is_user_iterator(ty) {
                true => self.own_methods(ty, false, span)?.rfold,
                false => (self.recognition())
                    .reaches_its_own(ty, OwnStep::Rfold, Reach::May)
                    .then(|| Expr::helper("$foldBack")),
            };
            // A range's of numbers, of its bounds, std's own (#163), so what
            // steps it from its back by `nth_back`, a held `Rev`'s `nth`
            // (ADR 0871), never steps each number.
            let numbers = self.range_index(ty).and_then(Num::of).is_some();
            // And of one that may reach the crate's own, a type parameter's
            // or std's adapter's over one, `$nthBack`, which asks the value,
            // as std's `nth_back` of it asks its own (#2134); `$nthBackSome`
            // of items that may look like `None` (ADR 0051).
            let reaches_own = || (self.recognition()).reaches_its_own(ty, OwnStep::NthBack, Reach::Sure);
            let boxed_items = || self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));
            let nth_back = match (self.is_user_iterator(ty), self.range_kind(ty)) {
                (true, _) => self.own_methods(ty, false, span)?.nth_back,
                (false, Some(RangeKind::Exclusive)) if numbers => Some(Expr::helper("$rangeNthBack")),
                (false, Some(RangeKind::Inclusive)) if numbers => Some(Expr::helper("$rangeInclusiveNthBack")),
                (false, _) if reaches_own() => Some(Expr::helper(match boxed_items() {
                    true => "$nthBackSome",
                    false => "$nthBack",
                })),
                (false, _) => None,
            };
            // Of a sized `&mut` to the crate's iterator, its `try_rfold` is the
            // trait's default, over `next_back`, as it can't be written on
            // stable Rust: so no `rfold` passed on, as before.
            let rfold = passed_rfold(rfold, None);
            for (method, own) in [("rfold", rfold), ("nth_back", nth_back)] {
                if let Some(own) = own {
                    props.push(js::Prop::Field(
                        crate::bindings::trait_method_name(self.tcx, trait_id, method),
                        own,
                    ));
                }
            }
        }
        debug_assert!(
            props.iter().skip(1).all(|prop| matches!(prop, js::Prop::Field(entry, _)
                if (crate::recognition::back_steps().iter()).any(|&method| crate::bindings::trait_method_name(self.tcx, trait_id, method) == *entry))),
            "an extension's dictionary carries its entry and `back_steps` alone"
        );
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
    pub fn cycled(&mut self, it: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let made_iterator = self.unspelled("iterator");
        let (next, boxed) = match self.range_kind(ty) {
            Some(_) => match self.range_stepper(ty, false) {
                Some(step) => (step, false),
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
            // (ADR 0915). And a `repeat(x)`'s, which carries its clone
            // (#1139).
            None if self.steps_an_array(ty)
                || self.iterator_cloned_apart(ty)
                || self.compiled_struct(ty, &it)
                || crate::recognition::is_repeat(self.tcx, ty) =>
            {
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
        let cloned = match self.compiled_struct(ty, &it) {
            true => self.clone_value(Expr::var(&made_iterator), ty, span, &mut body)?,
            false => self.clone_in_place(Expr::var(&made_iterator), ty, span, &mut body)?,
        };
        body.push(StmtKind::Return(Some(cloned)).at(js::Span::NONE));
        let clone = Expr::arrow(vec![made_iterator.clone().into()], body);
        let clone = passed_on(&clone, Taker::Made, &[]).unwrap_or(clone);
        let drop = self.drop_function(ty, span)?;
        let mut args = vec![it, clone, next];
        if boxed || drop.is_some() {
            args.push(Expr::bool(boxed));
        }
        args.extend(drop);
        Ok(Expr::call_helper("$cycleOf", args))
    }

    /// `ty`'s `Iterator` dictionary: of one given as it is, or a `Box<I>`
    /// of one, which is its value, the dictionary this function was given
    /// (`iterator_evidence`, #1611, #2223); else the one it is
    /// (`iterator_dictionary`). The one order, of a bound's dictionary and of
    /// what an `IntoIterator`'s makes. An extension's of a `Box<I>` asks it
    /// itself (`iterator_extension_dictionary`).
    pub(crate) fn iterator_dictionary_of(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        match self.iterator_evidence(ty, std_item(self.tcx, StdItem::Iterator)) {
            Some(given) => Ok(given),
            None => self.iterator_dictionary(ty, span),
        }
    }

    /// `ty`'s `Iterator` dictionary, `{ next }` (ADR 0411): its `next`
    /// steps a `ty` as it is, a range's `$rangeNext`, the crate's own its
    /// impl's, a JS iterator's `$next`, each giving the `Option` generic code
    /// reads, a `Some` that looks like `None` boxed (ADR 0051).
    pub(crate) fn iterator_dictionary(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let next = match self.range_kind(ty) {
            Some(_) => match self.range_stepper(ty, false) {
                Some(step) => step,
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
            let size_hint = crate::bindings::trait_method_name(self.tcx, iterator, "size_hint");
            props.push(js::Prop::Field(size_hint, hint));
        }
        // And its own `fold`, `nth` and `last`, which std's consumers of it
        // call, `$foldBy`'s (#162, #182).
        let crate::iterators::OwnMethods {
            fold, nth, last, count, ..
        } = self.own_methods(ty, false, span)?;
        let iterator = std_item(self.tcx, StdItem::Iterator);
        // Of std's adapter over the crate's own, the JS iterator's, which
        // passes them on as std's does (#214).
        let (fold, nth, last, count) = match self.is_user_iterator(ty) {
            true => (fold, nth, last, count),
            false => {
                let recognition = self.recognition();
                let boxed = self.iterator_item(ty).is_some_and(|item| self.boxed_payload(item));
                // Std's struct whose `fold` std compiles folds by that, its
                // meaning, whatever its sources are (ADR 0912). Of a type
                // parameter's, its value's own, which `$foldOf` asks at run
                // time, `May`'s: generic code's fold of a `map()` or a
                // `fuse()` of one it was given folds by the crate's own
                // where its caller's is (#621, #623, #637).
                let folds = recognition.reaches_its_own(ty, OwnStep::Fold, Reach::May)
                    || recognition.folds_by_its_own_nth(ty)
                    || (crate::recognition::is_compiled_std_iterator(self.tcx, ty)
                        && self.compiled_fold_of(ty, (iterator, "fold"))?.is_some());
                (
                    folds.then(|| Expr::helper("$foldOf")),
                    recognition
                        .reaches_its_own(ty, OwnStep::Nth, Reach::Sure)
                        .then(|| Expr::helper(if boxed { "$nthSome" } else { "$nth" })),
                    recognition
                        .reaches_its_own(ty, OwnStep::Last, Reach::Sure)
                        .then(|| Expr::helper(if boxed { "$lastSome" } else { "$last" })),
                    recognition
                        .reaches_its_own(ty, OwnStep::Count, Reach::Sure)
                        .then(|| Expr::helper("$count")),
                )
            }
        };
        for (name, own) in [("fold", fold), ("nth", nth), ("last", last), ("count", count)] {
            if let Some(own) = own {
                props.push(js::Prop::Field(
                    crate::bindings::trait_method_name(self.tcx, iterator, name),
                    own,
                ));
            }
        }
        // Of one std reads by index, its reader of an item at `i`, which
        // std's zip of it asks where it's compiled (ADR 0975): present
        // exactly where its `TrustedRandomAccessNoCoerce` fact, by which
        // std's zip takes the index path, may be `true`, here or as its
        // caller says, `Cloned<I>`'s of a caller's `IRandomAccess`
        // (`trait_fact_may_hold`, #1982).
        if crate::recognition::zips_by_index_compiled(self.tcx)
            && self.trait_fact_may_hold(ty, crate::recognition::TypeFact::RandomAccessNoCoerce)
            && let Some(Some(reader)) = self.index_reader(ty, span)?
        {
            props.push(js::Prop::Field("getUnchecked".into(), reader));
        }
        // A `Vec`'s, whose `advance_by` and `advance_back_by` drop what they
        // pass as one slice, `$advanceSlice`, which `$given` gives the value
        // (`$slicing`), where generic code given it as it is can't tell it
        // from a `Drain`'s, whose JS iterator is an array's too (#703, #704).
        if self.slice_seen(ty, true).is_some() {
            let advance_by = crate::bindings::trait_method_name(self.tcx, iterator, "advance_by");
            props.push(js::Prop::Field(advance_by, Expr::helper("$advanceSlice")));
        }
        props.extend(self.lent_entry(ty));
        // Of a JS iterator whose every entry is the runtime's step that asks
        // what it has of its own, the one named dictionary of it,
        // `$jsIterator`, or of items that look like `None`, `$jsIteratorSome`
        // (#1681): any other entry, the crate's own or one left out, keeps
        // its own object.
        let named = |next: &'static str, nth: &'static str, last: &'static str| {
            let entry = |name: &str, step: &'static str| {
                js::Prop::Field(
                    crate::bindings::trait_method_name(self.tcx, iterator, name),
                    Expr::helper(step),
                )
            };
            vec![
                js::Prop::Field("next".into(), Expr::helper(next)),
                entry("fold", "$foldOf"),
                entry("nth", nth),
                entry("last", last),
                entry("count", "$count"),
            ]
        };
        if props == named("$next", "$nth", "$last") {
            return Ok(Expr::helper("$jsIterator"));
        }
        if props == named("$nextSome", "$nthSome", "$lastSome") {
            return Ok(Expr::helper("$jsIteratorSome"));
        }
        Ok(Expr::object(props))
    }

    /// What reads the item `i` past where it is of `ty`, an iterator std
    /// reads by index, `__iterator_get_unchecked`'s (ADR 0975): std's
    /// compiled adapter's own reader, its dictionary's `getUnchecked`;
    /// `Some(None)` of an array's `$iter` or a range, which `$getUnchecked`
    /// reads as it is; `None` of another, which std's zip of can't be read
    /// as std reads it.
    pub(crate) fn index_reader(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Option<Option<Expr>>> {
        // Std's struct first, a `copied()` of a slice's too, which is no
        // array's `$iter` where it's compiled.
        let iterator = std_item(self.tcx, StdItem::Iterator);
        if crate::recognition::is_compiled_std_iterator(self.tcx, ty) {
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
    /// impl for it passes on to its `I`, its `next`, `size_hint`, `nth` and
    /// `advance_by`, a `Vec`'s its slice's (#703);
    /// not its `fold`, which is `I`'s `try_fold`, nor its `last`, std's of
    /// `next`, as a JS iterator lent is (`$lent`, ADR 0912).
    /// Of type `ty`, its `lent` too (`lent_entry`), as its value is `I`'s.
    pub(crate) fn mut_ref_iterator_dictionary(&self, ty: ty::Ty<'tcx>, of: Expr) -> Expr {
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let passed: Vec<String> = ["next", "size_hint", "nth", "advance_by"]
            .map(|name| crate::bindings::trait_method_name(self.tcx, iterator, name))
            .into();
        let mut dictionary = entries_passed_on(of, passed);
        if let js::ExprKind::Object(props, _) = &mut dictionary.kind {
            props.extend(self.lent_entry(ty));
        }
        dictionary
    }

    /// An `Iterator` dictionary's `lent` of a value of type `ty` that's the
    /// JS iterator a wrapper holds, a `Box`'s or a `&mut`'s: `$lent`'s kinds
    /// of each wrapper std steps it through, innermost first, which
    /// `$given` lends it as before it steps it, `lent: ["boxed"]`, so
    /// generic code given it as it is steps it as std's wrapper does, never
    /// by its own `fold` (`wrapper_lends`, #2185, #2187). None where nothing
    /// lends it.
    fn lent_entry(&self, ty: ty::Ty<'tcx>) -> Option<js::Prop> {
        let lends = self.wrapper_lends(ty);
        (!lends.is_empty()).then(|| {
            let kinds = (lends.iter()).map(|how| Expr::str(how.lend_kind())).collect();
            js::Prop::Field("lent".into(), Expr::array(kinds))
        })
    }

    /// A dictionary of `trait_id`, `Iterator` or one of its extensions, of a
    /// `Box<I>`, of `I`'s, `of`: what std's impl for a `Box<I>` passes on
    /// to `I`'s, `Wrapper::passes`', its other steps the trait's defaults,
    /// by `next`, never the crate's own of them a caller's dictionary may
    /// carry, its `fold` or `count` (#1611).
    pub(crate) fn boxed_iterator_dictionary(&self, trait_id: DefId, of: Expr) -> Expr {
        let entry = (!crate::recognition::is_std_def(self.tcx, trait_id, StdItem::Iterator))
            .then(|| crate::recognition::extension_entry(self.tcx, trait_id));
        let declares = |name: &str| {
            (self.tcx.associated_items(trait_id).in_definition_order())
                .any(|item| item.is_fn() && item.name().as_str() == name)
        };
        let passed: Vec<String> = (crate::recognition::Wrapper::BoxSized.own_methods())
            .filter(|&name| declares(name))
            .map(|name| match entry {
                Some(entry) if declares(entry) && name == entry => entry.to_string(),
                _ => crate::bindings::trait_method_name(self.tcx, trait_id, name),
            })
            .collect();
        entries_passed_on(of, passed)
    }

    /// An extension's dictionary of a `&mut I` of type `ty`, of `I`'s, `of`: its entry, `next_back` or `len`,
    /// and of `back_steps` those std's impl for it passes on to its `I`'s
    /// own (`Wrapper::passes`, #2000): of a sized `I`, `nth_back` and `try_rfold`,
    /// not `rfold`, which it folds by `I`'s `try_rfold` (ADRs 0928, 0937); of
    /// an unsized one, `nth_back` alone.
    pub(crate) fn mut_ref_extension_dictionary(&self, trait_id: DefId, ty: ty::Ty<'tcx>, of: Expr) -> Expr {
        use crate::recognition::{Passed, Wrapper};
        let wrapper = Wrapper::of(self.tcx, self.typing_env, ty).expect("a `&mut`");
        let others: Vec<&str> = match crate::recognition::is_double_ended(self.tcx, trait_id) {
            true => crate::recognition::back_steps(),
            false => Vec::new(),
        };
        let entry = crate::recognition::extension_entry(self.tcx, trait_id).to_string();
        let passed: Vec<String> = std::iter::once(entry)
            .chain(
                (others.iter())
                    .filter(|&&method| wrapper.passes(method) == Passed::Own)
                    .map(|method| crate::bindings::trait_method_name(self.tcx, trait_id, method)),
            )
            .collect();
        entries_passed_on(of, passed)
    }

    /// What an `IntoIterator` of type `ty` gives, its `Item`.
    pub(crate) fn iterable_item(&self, ty: ty::Ty<'tcx>) -> ty::Ty<'tcx> {
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        let item = trait_method(self.tcx, into_iterator, "Item");
        let projection = ty::Ty::new_projection(self.tcx, ty::IsRigid::No, item, [ty]);
        (self.tcx)
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection))
            .unwrap_or(projection)
    }
}

/// A dictionary's `rfold`, `(iterator, init, f, unwind, ends) =>
/// tried(iterator, init, f, () => false, unwind, ends)`: its `try_rfold`,
/// `tried`, never stopped, as std folds by `try_fold` (`impl_fold_via_try_fold`).
fn folded_by_try(tried: Expr) -> Expr {
    let names: Vec<js::Name> = ["iterator", "init", "f", "unwind", "ends"]
        .map(|word| js::Name::declared(word.to_string()))
        .to_vec();
    let never = Expr::arrow(
        Vec::new(),
        vec![StmtKind::Return(Some(Expr::bool(false))).at(js::Span::NONE)],
    );
    let mut given: Vec<Expr> = names[..3].iter().map(Expr::var).collect();
    given.push(never);
    given.extend(names[3..].iter().map(Expr::var));
    let call = Expr::call(tried, given);
    Expr::arrow(
        names.into_iter().map(Into::into).collect(),
        vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
    )
}

/// The name of what a step of its back counts, `nth_back(k)`'s or
/// `advance_back_by(n)`'s.
pub(super) fn counted_by(method: &str) -> &'static str {
    match method {
        "nth_back" => "k",
        _ => "n",
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
