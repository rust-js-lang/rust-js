//! A dictionary for a bound, built where it's given: a std trait's, an
//! impl's, a `dyn`'s, and its associated types' facts.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A std collection's `Extend`, `{ extend(collection, items) }`, what
    /// generic code calls, `c.extend(items)` of a `C: Extend<A>` (ADR 0445):
    /// what a known one's call is, of its items as they're given, any
    /// iterable, and of the box generic code's `&mut` is (ADR 0180). None of
    /// another type, the crate's, whose impl's dictionary is its own.
    fn extend_dictionary(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let (collection, items) = (Expr::member(Expr::var("collection"), js::HELD), Expr::var("items"));
        let mut body = Vec::new();
        // The items' type said as the collection's, which no range is, so
        // what's given is iterated as it is.
        let given = [ty, ty];
        let none = ty::List::empty();
        let value = match self.recognition().std_extend(ty) {
            Some(Std::PushStr) => {
                let text = self.joined_text(items.clone(), None, span, &mut body)?;
                let added = Expr::bin(Op::Add, collection.clone(), text);
                body.push(StmtKind::Assign(collection.clone(), added).at(js::Span::NONE));
                None
            }
            Some(Std::Comb(comb)) => Some(self.comb_values(
                comb,
                (vec![collection.clone(), items.clone()], &given),
                false,
                none,
                span,
                &mut body,
            )?),
            Some(Std::Map(op)) => Some(self.map_values(
                op,
                (vec![collection.clone(), items.clone()], &given),
                none,
                span,
                &mut body,
            )?),
            _ => return Ok(None),
        };
        body.extend(value.map(|value| StmtKind::Expr(value).at(js::Span::NONE)));
        let extend = Expr::arrow(vec!["collection".into(), "items".into()], body);
        Ok(Some(Expr::object(vec![Prop::Field("extend".into(), extend)])))
    }

    pub(in crate::lower) fn dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        // `<Words as Source>::Item: Debug` of a bound instantiated: `String: Debug`;
        // in a copied default, where the impl's bounds and its own hold, in
        // the impl's terms: `K: Hash` of `<Map<T, F> as Iterator>::Item`.
        let tr = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(tr))
            .unwrap_or(tr);
        let tr = match self.impl_typing_env() {
            Some(typing_env) => self
                .tcx
                .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(tr))
                .unwrap_or(tr),
            None => tr,
        };
        // serde's: the function that writes or reads the type (ADR 0081).
        match crate::lower::serde::serde_trait(self.tcx, tr.def_id) {
            Some(true) => return self.json_writer(tr.self_ty(), span),
            Some(false) => return self.json_reader(tr.self_ty(), span),
            None => {}
        }
        if let Some(found) = self.evidence_for(tr) {
            return Ok(found);
        }
        // One of std's errors' `Error`, as its `dyn`'s is.
        if is_std_def(self.tcx, tr.def_id, StdItem::Error)
            && let Some(dictionary) = self.std_error_dictionary(tr.self_ty())
        {
            return Ok(dictionary);
        }
        // A std hasher's `Hasher`, and a std type's `Hash` (ADR 0463).
        if let Some(dictionary) = self.std_hash_dictionary(tr, span)? {
            return Ok(dictionary);
        }
        // An `Iterator`'s, of a type that isn't a parameter given one (ADR
        // 0411). One given as it is, which a JS iterator's `next` can't step,
        // that the analysis found nothing here steps: rust-js's own error.
        if is_std_def(self.tcx, tr.def_id, StdItem::Iterator) {
            // A `&mut I`'s, of what a `&mut` of an iterator is, the iterator:
            // `it.by_ref().take(2)` of a range, or of a type parameter's, by
            // the dictionary it was given (ADR 0912).
            if let ty::Ref(_, inner, ty::Mutability::Mut) = tr.self_ty().kind() {
                let of = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [*inner]), span)?;
                return Ok(self.mut_ref_iterator_dictionary(of));
            }
            if let ty::Param(param) = tr.self_ty().kind()
                && crate::lower::iterators::given_as_it_is(self.tcx, self.tcx.typeck_root_def_id(self.item), *param)
            {
                return Err(self.unsupported(
                    span,
                    &format!("stepping `{}`, which rust-js found nothing here steps", tr.self_ty()),
                ));
            }
            return self.iterator_dictionary(tr.self_ty(), span);
        }
        // Std's `Try` of what its generic code is given (ADR 0913), and its
        // supertrait `FromResidual`'s, the same dictionary.
        if crate::lower::recognition::is_try_trait(self.tcx, tr.def_id) {
            use crate::lower::recognition::TryKind;
            return match crate::lower::recognition::try_kind(self.tcx, tr.self_ty()) {
                Some(TryKind::ControlFlow) => Ok(Expr::helper("$controlFlowTry")),
                Some(TryKind::NeverShortCircuit) => Ok(Expr::helper("$neverShortCircuitTry")),
                None => Err(self.unsupported(
                    span,
                    &format!("the `Try` of a `{}` given to std's generic code", tr.self_ty()),
                )),
            };
        }
        // An iterator's extension's, of one given as it is (ADR 0487); a
        // `&mut` to a type only its bounds know, of the dictionary it was
        // given, as std's `&mut I` passes on to its `I` (ADR 0937).
        if crate::lower::recognition::is_iterator_extension(self.tcx, tr.def_id) {
            if let ty::Ref(_, inner, ty::Mutability::Mut) = tr.self_ty().kind()
                && self.is_unknown(*inner)
            {
                let of = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [*inner]), span)?;
                return Ok(self.mut_ref_extension_dictionary(tr.def_id, of));
            }
            return self.iterator_extension_dictionary(tr.def_id, tr.self_ty(), span);
        }
        // `Any`'s: its type's key, `{ type_id: () => "i32" }` (ADR 0331).
        if any_trait(self.tcx) == Some(tr.def_id) {
            let key = self.type_fact_value(tr.self_ty(), TypeFact::Id, span)?;
            let type_id = Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(key)).at(js::Span::NONE)]);
            let mut props = vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "type_id"),
                type_id,
            )];
            props.extend(self.dictionary_drop(tr.self_ty(), span)?);
            return Ok(Expr::object(props));
        }
        // A marker's of a type a bound gave none for, as none passes one: a
        // type parameter made a `dyn`, `Box::new(value)` of a `T: Marker`.
        // What its impl's would have, its drop and its supertraits', made here.
        if tr.self_ty().has_param() && is_marker(self.tcx, self.krate.foreign, tr.def_id) {
            let mut props = self.supertrait_entries(tr, span)?;
            props.extend(self.dictionary_drop(tr.self_ty(), span)?);
            return Ok(Expr::object(props));
        }
        // Derived and std impls of `Default` and `Clone` (ADR 0052).
        let ty = tr.self_ty();
        // A range's `RangeBounds`, of a range of std's or a pair of `Bound`s:
        // each bound, a `Bound` of what its field holds, `{ TAG: "Included",
        // _0: range.start }`, as any of its values is.
        if is_std_def(self.tcx, tr.def_id, StdItem::RangeBounds)
            && crate::lower::recognition::range_bounds(self.tcx, ty).is_some()
        {
            let start_bound = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "start_bound");
            let bound = self.tcx.instantiate_and_normalize_erasing_regions(
                tr.args,
                self.typing_env,
                self.tcx.fn_sig(start_bound).map_bound(|sig| sig.skip_binder().output()),
            );
            let mut made = Vec::new();
            let values = self.range_bound_values(Expr::var("range"), ty, bound, span, &mut made)?;
            let values = values.expect("a range's bounds, as `range_bounds` found them");
            let props = ["start_bound", "end_bound"]
                .into_iter()
                .zip(values)
                .map(|(name, value)| {
                    let body = vec![StmtKind::Return(Some(value)).at(js::Span::NONE)];
                    let name = bindings::trait_method_name(self.tcx, tr.def_id, name);
                    Prop::Field(name, Expr::arrow(vec!["range".into()], body))
                });
            return Ok(Expr::object(props.collect()));
        }
        // A std collection's `FromIterator`, `{ from_iter(items) }`: what
        // `collect` into it makes, of the items given, any iterable (ADR 0445).
        if is_std_def(self.tcx, tr.def_id, StdItem::FromIterator) && !self.has_user_impl(tr.def_id, ty) {
            let items = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![Expr::var("items")]);
            let made = self.collected_into(items, ty, Some(tr.args.type_at(1)), None, span)?;
            let from_iter = Expr::arrow(
                vec!["items".into()],
                vec![StmtKind::Return(Some(made)).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "from_iter"),
                from_iter,
            )]));
        }
        // std's `Sum` or `Product` of a number, `{ sum(items) }`: what
        // `sum()` of them makes (ADR 0476).
        if let Some(name) = crate::lower::recognition::accumulation(self.tcx, tr.def_id)
            && let Some(num) = Num::of(ty)
            && !self.has_user_impl(tr.def_id, ty)
        {
            let items = self.js_iterator(Expr::var("items"));
            let made = self.accumulated_numbers(items, num, name == "sum");
            let body = vec![StmtKind::Return(Some(made)).at(js::Span::NONE)];
            return Ok(Expr::object(vec![Prop::Field(
                name.into(),
                Expr::arrow(vec!["items".into()], body),
            )]));
        }
        // A std collection's `Extend`, `{ extend(collection, items) }` (ADR 0445).
        if crate::lower::recognition::is_extend(self.tcx, tr.def_id)
            && let Some(dictionary) = self.extend_dictionary(ty, span)?
        {
            return Ok(dictionary);
        }
        // `usize`'s or a range's `SliceIndex` of a slice (ADR 0418): the
        // runtime's, of each kind, `$sliceIndexes.range`.
        if is_std_def(self.tcx, tr.def_id, StdItem::SliceIndex) {
            use crate::lower::recognition::RangeBound::{Field, Unbounded};
            let kind = match ty.is_usize() {
                true => Some("item"),
                false => match crate::lower::recognition::range_bounds(self.tcx, ty) {
                    Some((Field("Included", _), Field("Excluded", _))) => Some("range"),
                    Some((Field("Included", _), Unbounded)) => Some("from"),
                    Some((Unbounded, Field("Excluded", _))) => Some("to"),
                    Some((Field("Included", _), Field("Included", _))) => Some("inclusive"),
                    Some((Unbounded, Field("Included", _))) => Some("toInclusive"),
                    Some((Unbounded, Unbounded)) => Some("full"),
                    _ => None,
                },
            };
            let kind = kind.ok_or_else(|| self.no_evidence(span, tr))?;
            return Ok(Expr::member(Expr::helper("$sliceIndexes"), kind));
        }
        // std's `LowerHex` and the like of an integer: its digits, given the
        // options its caller is, as std's `Debug` of one is (ADR 0185).
        if let Some(num) = Num::of(ty).filter(|n| !n.float())
            && let Some(radix) = self.radix_trait(tr.def_id)
            && !self.has_user_impl(tr.def_id, ty)
        {
            let (params, pretty): (Vec<js::Pattern>, _) = match self.writers_take_options() {
                true => (
                    vec!["value".into(), "options".into()],
                    Pretty::When(Expr::var("options")),
                ),
                false => (vec!["value".into()], Pretty::Plain),
            };
            let text = self.radix_text(Expr::var("value"), num, radix, &pretty);
            let fmt = Expr::arrow(params, vec![StmtKind::Return(Some(text)).at(js::Span::NONE)]);
            return Ok(Expr::object(vec![Prop::Field("fmt".into(), fmt)]));
        }
        let default = is_std_def(self.tcx, tr.def_id, StdItem::Default);
        let clone = self.tcx.is_lang_item(tr.def_id, LangItem::Clone);
        let eq = self.tcx.is_lang_item(tr.def_id, LangItem::PartialEq);
        let display = tr.def_id == self.display_trait();
        let to_string = Some(tr.def_id) == self.to_string_trait();
        let debug = tr.def_id == self.debug_trait();
        let ord = tr.def_id == self.ord_trait();
        let partial_ord = tr.def_id == self.partial_ord_trait();
        // `PartialEq<u32>` or `PartialOrd<u32>` of another type than itself: its
        // impl's, never a comparison of it with itself.
        let of_other = (eq || partial_ord)
            && tr
                .args
                .types()
                .nth(1)
                .is_some_and(|rhs| rhs.peel_refs() != ty.peel_refs());
        // A class's `Display` is its `toString`, which JS's `String` calls (#358).
        let shown = (display || to_string) && self.is_shown_class(ty);
        if (default || clone || eq || display || to_string || debug || ord || partial_ord)
            && !of_other
            && (!self.has_user_impl(tr.def_id, ty) || shown)
        {
            if debug {
                // Given a `Formatter`'s options, where the crate's writers are (ADRs 0058, 0137).
                let (params, pretty): (Vec<js::Pattern>, _) = match self.writers_take_options() {
                    true => (
                        vec!["value".into(), "options".into()],
                        crate::lower::display::Pretty::When(Expr::var("options")),
                    ),
                    false => (vec!["value".into()], crate::lower::display::Pretty::Plain),
                };
                let mut body = Vec::new();
                let shown = self.debug_string_with(Expr::var("value"), ty, span, &pretty)?;
                body.push(StmtKind::Return(Some(shown)).at(js::Span::NONE));
                let fmt = Expr::arrow(params, body);
                return Ok(Expr::object(vec![Prop::Field("fmt".into(), fmt)]));
            }
            if ord || partial_ord {
                let (name, compare) = if ord {
                    ("cmp", self.cmp_fn(ty, false, span)?)
                } else {
                    ("partial_cmp", self.cmp_fn(ty, true, span)?)
                };
                return Ok(Expr::object(vec![Prop::Field(
                    bindings::trait_method_name(self.tcx, tr.def_id, name),
                    compare,
                )]));
            }
            if display {
                let fmt = self.display_fn(ty, span)?;
                return Ok(Expr::object(vec![Prop::Field("fmt".into(), fmt)]));
            }
            // `ToString` is std's of a `Display`: what that shows.
            if to_string {
                let fmt = self.display_fn(ty, span)?;
                return Ok(Expr::object(vec![Prop::Field(
                    bindings::trait_method_name(self.tcx, tr.def_id, "to_string"),
                    fmt,
                )]));
            }
            if eq {
                let eq = self.eq_fn(ty, span)?;
                return Ok(Expr::object(vec![Prop::Field("eq".into(), eq)]));
            }
            if default {
                let value = self.default_value(ty, span)?;
                return Ok(Expr::object(vec![Prop::Field(
                    "default".into(),
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(value)).at(js::Span::NONE)]),
                )]));
            }
            if clone {
                let clone = self.clone_fn("value", ty, span)?;
                return Ok(Expr::object(vec![Prop::Field("clone".into(), clone)]));
            }
        }
        // std's `AsRef` of text as a `str`, or of a `Vec`, an array or a slice
        // as a slice: each is the same JS value (ADR 0162).
        // A `String`'s, or a `Formatter`'s, given in a box of its text: what's
        // written is added to it (ADR 0180).
        if is_std_def(self.tcx, tr.def_id, StdItem::FmtWrite) {
            let text =
                |ty: Ty<'tcx>| self.is_lang_adt(ty, LangItem::String) || self.recognition().is_formatter_type(ty);
            if text(ty) {
                return Ok(Expr::helper("$stringWriter"));
            }
        }
        // A `&mut` to a writer taken by value, `W = &mut s`, given in a box
        // (ADR 0454), a generic one's too: its own dictionary, through the
        // box, of a `fmt::Write` and an `io::Write` alike.
        if (is_std_def(self.tcx, tr.def_id, StdItem::FmtWrite) || is_std_def(self.tcx, tr.def_id, StdItem::IoWrite))
            && let ty::Ref(_, inner, Mutability::Mut) = *ty.kind()
        {
            let writer = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [inner]), span)?;
            return Ok(Expr::call_helper("$mutWriter", vec![writer]));
        }
        // std's `Deref` of a `P: Deref` in generic code: what std's `deref`
        // of the type is, as a function, `(x) => x` of a reference, a `Box`
        // or text, which are their values (ADR 0394). The crate's own is its
        // impl's.
        if self.tcx.is_lang_item(tr.def_id, LangItem::Deref) && !self.has_user_impl(tr.def_id, ty) {
            // A `&mut`'s, which generic code is given in a box: what's in it
            // (ADR 0454).
            if let ty::Ref(_, _, Mutability::Mut) = ty.kind() {
                let value = Expr::held(Expr::var("x"));
                let f = Expr::arrow(vec!["x".into()], vec![StmtKind::Return(Some(value)).at(js::Span::NONE)]);
                return Ok(Expr::object(vec![Prop::Field("deref".into(), f)]));
            }
            let deref = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "deref");
            let function = Ty::new_fn_def(self.tcx, deref, ty::Binder::dummy(tr.args));
            if let Some(known) = self.recognition().classify(deref, tr.args)
                && let Some(f) = self.std_fn_value(known, function, span)?
            {
                return Ok(Expr::object(vec![Prop::Field("deref".into(), f)]));
            }
        }
        // std's `DerefMut` of a `P: DerefMut` in generic code, given the cell
        // its `&mut self` is and giving the one its `&mut Self::Target` is
        // (ADR 0454): of a pointer whose target is the same JS value, a
        // `Box`, text, a `Vec` or a JS object, that cell; of a `&mut` or a
        // guard, which holds a cell of its target, what's in it (ADR 0394).
        if self.tcx.is_lang_item(tr.def_id, LangItem::DerefMut) && !self.has_user_impl(tr.def_id, ty) {
            let deref_mut = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "deref_mut");
            let target = match self.recognition().classify(deref_mut, tr.args) {
                _ if matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut)) => Some(Expr::held(Expr::var("cell"))),
                Some(Std::GuardValue { .. }) => Some(Expr::held(Expr::var("cell"))),
                Some(Std::Same) => Some(Expr::var("cell")),
                _ if ty.is_box() => Some(Expr::var("cell")),
                _ => None,
            };
            if let Some(target) = target {
                let f = Expr::arrow(
                    vec!["cell".into()],
                    vec![StmtKind::Return(Some(target)).at(js::Span::NONE)],
                );
                let mut props = self.supertrait_entries(tr, span)?;
                props.push(Prop::Field(bindings::fn_name(self.tcx, deref_mut), f));
                return Ok(Expr::object(props));
            }
        }
        // std's `IntoIterator` of a `&C` or a `&mut C` a bound gives generic
        // code (ADR 0457): the JS iterator `into_iter` of it is where its type
        // is known, given what generic code has, a `&mut` to an object in a
        // box (ADR 0454): a handle on each item of a `&mut`, as `for x in &mut
        // v` makes, or std's own of what it's given.
        if is_std_def(self.tcx, tr.def_id, StdItem::IntoIterator)
            && let ty::Ref(_, pointee, mutability) = *ty.kind()
            && !self.has_user_impl(tr.def_id, ty)
        {
            let into_iter = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "into_iter");
            // A `&mut`'s, the cell generic code has, a box of an object's.
            let name = match mutability {
                Mutability::Mut => "cell",
                Mutability::Not => self.parameter_name(pointee),
            };
            let given = match self.held_in_box(ty) {
                true => Expr::held(Expr::var(name)),
                false => Expr::var(name),
            };
            let mut body = Vec::new();
            let items = (self.ref_into_iter(given, ty, span, &mut body)?).ok_or_else(|| self.no_evidence(span, tr))?;
            body.push(StmtKind::Return(Some(items)).at(js::Span::NONE));
            let f = Expr::arrow(vec![name.into()], body);
            let mut props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];
            props.extend(self.made_steps(tr, span)?);
            return Ok(Expr::object(props));
        }
        // std's `IntoIterator` of a type known here, given where what it
        // makes is held as it is (ADR 0552): `into_iter` of it as concrete
        // code has it, an iterator's, a range's too, itself, an array's
        // `$iter`; and the `Iterator` of what it makes, which steps that as
        // it is, `IntoIterIterator`, as the trait declares its `IntoIter`.
        // Of one only bounds know, an iterator's, std's for every iterator:
        // itself, stepped by the dictionary its bounds give.
        if is_std_def(self.tcx, tr.def_id, StdItem::IntoIterator)
            && !matches!(ty.kind(), ty::Ref(..))
            && (!crate::lower::recognition::known_by_bounds(ty) || self.recognition().iterates_as_itself(ty))
            && !self.has_user_impl(tr.def_id, ty)
        {
            let itself = Expr::arrow(
                vec!["items".into()],
                vec![StmtKind::Return(Some(Expr::var("items"))).at(js::Span::NONE)],
            );
            let steps = self.made_steps(tr, span)?;
            if self.recognition().iterates_as_itself(ty) {
                let mut props = vec![Prop::Field(
                    bindings::trait_method_name(self.tcx, tr.def_id, "into_iter"),
                    itself,
                )];
                props.extend(steps);
                return Ok(Expr::object(props));
            }
            let into_iter = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "into_iter");
            let known = self
                .recognition()
                .classify(into_iter, tr.args)
                .ok_or_else(|| self.no_evidence(span, tr))?;
            let into_iter_ty = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "IntoIter");
            let output = Ty::new_projection(self.tcx, ty::IsRigid::No, into_iter_ty, tr.args);
            let output = (self.tcx)
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
                .unwrap_or(output);
            let mut body = Vec::new();
            let items = self.std_by_values(
                known,
                into_iter,
                tr.args,
                (&[ty], Default::default()),
                vec![Expr::var("items")],
                output,
                span,
                &mut body,
            )?;
            let made = self.std_iterator(items, output);
            body.push(StmtKind::Return(Some(made)).at(js::Span::NONE));
            let f = Expr::arrow(vec!["items".into()], body);
            let mut props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];
            props.extend(steps);
            return Ok(Expr::object(props));
        }
        // An `io::Write`'s bytes, of a `Vec<u8>`, added to it, or of a
        // standard stream, written to it (ADR 0392).
        if is_std_def(self.tcx, tr.def_id, StdItem::IoWrite)
            && let Some(writer) = self.recognition().io_writer(ty)
        {
            return Ok(Expr::helper(writer));
        }
        // std's `AsMut` of text as a `str`, of a sequence as a slice, or of a
        // `Box` as what it holds (`as_mut_is_itself`), given
        // the cell generic code's `&mut self` is, and giving the one its `&mut
        // T` is (ADR 0454): that cell, as a `&mut str` is its text's (ADR
        // 0334) and a `&mut [T]`'s holds its array. Of a `&mut` to one, std's
        // `impl AsMut<U> for &mut T`, that one's, through the cell the `&mut`
        // is: one `value` further for each.
        if is_std_def(self.tcx, tr.def_id, StdItem::AsMut) && !self.has_user_impl(tr.def_id, ty) {
            let mut cell = Expr::var("cell");
            let mut at = ty;
            while let ty::Ref(_, inner, Mutability::Mut) = *at.kind() {
                cell = Expr::held(cell);
                at = inner;
            }
            if self.recognition().as_mut_is_itself(ty, tr.args.type_at(1)) {
                let f = Expr::arrow(
                    vec!["cell".into()],
                    vec![StmtKind::Return(Some(cell)).at(js::Span::NONE)],
                );
                return Ok(Expr::object(vec![Prop::Field(
                    bindings::trait_method_name(self.tcx, tr.def_id, "as_mut"),
                    f,
                )]));
            }
        }
        if is_std_def(self.tcx, tr.def_id, StdItem::AsRef)
            && !self.has_user_impl(tr.def_id, ty)
            && self.recognition().as_ref_is_itself(ty, tr.args.type_at(1))
        {
            let same = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(Expr::var("value"))).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "as_ref"),
                same,
            )]));
        }
        // A path's `Component`'s, as a `Path` or an `OsStr`: its text (ADR 0716).
        if is_std_def(self.tcx, tr.def_id, StdItem::AsRef)
            && self.recognition().as_ref_is_component_text(ty, tr.args.type_at(1))
        {
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "as_ref"),
                Expr::helper("$componentText"),
            )]));
        }
        // std's `AsRef<[u8]>` of text: its UTF-8 bytes, as `as_bytes()` gives
        // them (ADR 0162).
        if is_std_def(self.tcx, tr.def_id, StdItem::AsRef)
            && !self.has_user_impl(tr.def_id, ty)
            && self.is_string_like(ty)
            && matches!(tr.args.type_at(1).kind(), ty::Slice(item) if *item == self.tcx.types.u8)
        {
            let bytes = crate::lower::std_types::text::utf8_bytes(Expr::var("value"));
            let as_ref = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(bytes)).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "as_ref"),
                as_ref,
            )]));
        }
        // std's `Borrow` where it's the same JS value: a value as itself, text
        // as a `str`, a `Vec` as a slice (ADR 0167).
        if is_std_def(self.tcx, tr.def_id, StdItem::Borrow)
            && !self.has_user_impl(tr.def_id, ty)
            && self.recognition().borrows_as_itself(ty, tr.args.type_at(1))
        {
            let same = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(Expr::var("value"))).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field("borrow".into(), same)]));
        }
        // And its `BorrowMut` of what's an object, a `Vec` as a slice, which a
        // `&mut` to is it: the same array, and its `Borrow`'s entry (ADR 0483).
        if is_std_def(self.tcx, tr.def_id, StdItem::BorrowMut)
            && !self.has_user_impl(tr.def_id, ty)
            && self.recognition().borrows_as_itself(ty, tr.args.type_at(1))
            && self.is_object(tr.args.type_at(1))
        {
            let same = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(Expr::var("value"))).at(js::Span::NONE)],
            );
            let mut props = vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "borrow_mut"),
                same,
            )];
            props.extend(self.supertrait_entries(tr, span)?);
            return Ok(Expr::object(props));
        }
        // std's `FromStr` of a number, a `bool`, a `char` or a `String`: what
        // `s.parse()` of it is (ADR 0161).
        let parsed_by_std = crate::lower::representation::Num::of(ty).is_some()
            || ty.is_bool()
            || ty.is_char()
            || self.is_lang_adt(ty, LangItem::String);
        if crate::lower::recognition::is_from_str(self.tcx, tr.def_id) && parsed_by_std {
            let mut body = Vec::new();
            let parsed = self.parse_as(Expr::var("s"), ty, span, &mut body)?;
            body.push(StmtKind::Return(Some(parsed)).at(js::Span::NONE));
            let from_str = Expr::arrow(vec!["s".into()], body);
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "from_str"),
                from_str,
            )]));
        }
        // `std::num::Wrapping`'s, its number's in its `[x]` (ADR 0175).
        if let Some(op) = crate::lower::recognition::value_operator(self.tcx, tr.def_id)
            && let Some(inner) = self.recognition().wrapping_of(ty.peel_refs())
        {
            let a = Expr::index(Expr::var("a"), Expr::int(0));
            let (params, value) = match op {
                Ok(op) => (
                    vec!["a".into(), "b".into()],
                    self.wrapping_result(op, a, Expr::var("b"), inner, span)?,
                ),
                Err(op) => (vec!["a".into()], self.unary(op, a, inner, span)?),
            };
            let body = vec![StmtKind::Return(Some(Expr::array(vec![value]))).at(js::Span::NONE)];
            return Ok(Expr::object(vec![Prop::Field(
                self.operator_entry(tr.def_id),
                Expr::arrow(params, body),
            )]));
        }
        // A number's `+` or `-`, as `a + b` of one is (ADR 0108), of a
        // number on each side: `impl Add<Meters> for f64` is the crate's.
        if let Some(op) = crate::lower::recognition::value_operator(self.tcx, tr.def_id)
            && self.primitive_operands(tr)
        {
            let params: Vec<js::Pattern> = match op {
                Ok(_) => vec!["a".into(), "b".into()],
                Err(_) => vec!["value".into()],
            };
            let (a, b) = match op {
                Ok(_) => (Expr::var("a"), Some(Expr::var("b"))),
                Err(_) => (Expr::var("value"), None),
            };
            let value = self.number_operator(op, a, b, tr, span)?;
            return Ok(Expr::object(vec![Prop::Field(
                self.operator_entry(tr.def_id),
                Expr::arrow(params, vec![StmtKind::Return(Some(value)).at(js::Span::NONE)]),
            )]));
        }
        // A number's `+=`, of a number on each side, and a `String`'s of
        // text, through the box a generic `&mut` is given in (ADR 0180): its
        // `value` assigned `a + b` (ADR 0396).
        let text = |t: Ty<'tcx>| self.is_lang_adt(t, LangItem::String) || t.peel_refs().is_str();
        if let Some(op) = crate::lower::recognition::assign_operator(self.tcx, tr.def_id)
            && (self.primitive_operands(tr) || (op == BinOp::Add && tr.args.types().all(text)))
        {
            let place = Expr::member(Expr::var("cell"), js::HELD);
            let value = match self.primitive_operands(tr) {
                true => self.number_operator(Ok(op), place.clone(), Some(Expr::var("b")), tr, span)?,
                false => Expr::bin(Op::Add, place.clone(), Expr::var("b")),
            };
            return Ok(Expr::object(vec![Prop::Field(
                self.operator_entry(tr.def_id),
                Expr::arrow(
                    vec!["cell".into(), "b".into()],
                    vec![StmtKind::Assign(place, value).at(js::Span::NONE)],
                ),
            )]));
        }
        // `x.into()` of a `T: Into<U>` (ADR 0108), and `W::from(u)` of a `W:
        // From<U>` (ADR 0477): std's conversion, of a `T` to itself too (ADR
        // 0387), or the crate's `From`.
        let into = is_std_def(self.tcx, tr.def_id, StdItem::Into);
        if into || is_std_def(self.tcx, tr.def_id, StdItem::From) {
            let method = self.tcx.associated_item_def_ids(tr.def_id)[0];
            let (target, source) = match into {
                true => (tr.args.type_at(1), ty),
                false => (ty, tr.args.type_at(1)),
            };
            let from = std_item(self.tcx, StdItem::From);
            let from = self.tcx.associated_item_def_ids(from)[0];
            let from_args = self.tcx.mk_args(&[target.into(), source.into()]);
            let value = if let Some(known) = self.recognition().classify(method, tr.args)
                && let Some(f) = self.std_fn_value(
                    known,
                    Ty::new_fn_def(self.tcx, method, ty::Binder::dummy(tr.args)),
                    span,
                )? {
                f
            } else if let Some(instance) = self.resolve_instance(from, from_args)?
                && self.is_rust_fn(instance.def_id())
                && self.tcx.trait_of_assoc(instance.def_id()).is_none()
            {
                let callee = self.fn_ref(instance.def_id());
                let mut values = vec![Expr::var("value")];
                values.extend(self.evidence_args(instance.def_id(), instance.args, span)?);
                match values.len() {
                    1 => callee,
                    _ => Expr::arrow(
                        vec!["value".into()],
                        vec![StmtKind::Return(Some(Expr::call(callee, values))).at(js::Span::NONE)],
                    ),
                }
            } else {
                return Err(self.no_evidence(span, tr));
            };
            let name = if into { "into" } else { "from" };
            return Ok(Expr::object(vec![Prop::Field(name.into(), value)]));
        }
        if self.tcx.is_lang_item(tr.def_id, LangItem::Copy) {
            let ty = tr.self_ty();
            if self.is_unknown(ty) {
                return Err(self.unsupported(span, "Copy without representation evidence"));
            }
            // What a `Copy` bound lets be copied, as each call of a `Copy`
            // `FnOnce` is (ADR 0246).
            if self.copies_own_captures(ty) {
                return Err(self.unsupported(span, "copying a closure that changes what it captured"));
            }
            let value = Expr::var("value");
            let copy = self.copy(value.clone(), ty);
            // A value that's its own copy is given none (ADR 0289, amended).
            if js::same_path(&copy, &value) {
                return Ok(Expr::undefined());
            }
            return Ok(Expr::object(vec![Prop::Field(
                "copy".into(),
                Expr::arrow(
                    vec!["value".into()],
                    vec![StmtKind::Return(Some(copy)).at(js::Span::NONE)],
                ),
            )]));
        }
        // The crate's own impl's accessor, or one a library exports (ADR 0100),
        // or one of a library's its JS doesn't, which the crate compiles
        // (ADR 0914, amended): in a copied default, of the impl's terms, where
        // its bounds and the default's hold, `Map<T, F>: Counting` of a
        // blanket impl's.
        let typing_env = self.impl_typing_env().unwrap_or(self.typing_env);
        if let Some((impl_id, impl_args)) = evidence_impl(self.tcx, typing_env, tr)
            && (self.krate.trait_impls.contains(&impl_id)
                || self.krate.foreign.item(impl_id).is_some()
                || self.krate.foreign.compiles(impl_id))
        {
            let callee = self.fn_ref(impl_id);
            let args = self.evidence_args(impl_id, impl_args, span)?;
            // One that takes nothing, its module's `const`, `circleShape` (ADR 0592).
            return Ok(match self.takes_evidence(impl_id) {
                true => Expr::call(callee, args),
                false => callee,
            });
        }
        // A trait object's: Rust's built-in `impl Trait for dyn Trait`.
        let selected = self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr));
        if let Ok(ImplSource::Builtin(BuiltinImplSource::Object(_), _)) = selected {
            return self.object_dictionary(tr, span);
        }
        // A reference's, by std's impl that passes it on to what it refers
        // to, `impl<T: AsRef<U>> AsRef<U> for &T`: that one's, as a reference
        // is the value it refers to in JS.
        if let Some(referent) = self.forwarded_to_referent(tr, typing_env) {
            return self.dictionary(referent, span);
        }
        Err(self.no_evidence(span, tr))
    }

    /// `tr` of what `tr`'s type refers to, where its impl is std's that
    /// passes it on: of a type `&T` or `&mut T` of its own parameter `T`, and
    /// asking `T` the same trait. `None` of another.
    fn forwarded_to_referent(
        &self,
        tr: ty::TraitRef<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
    ) -> Option<ty::TraitRef<'tcx>> {
        let ty::Ref(_, referent, _) = tr.self_ty().kind() else {
            return None;
        };
        let (impl_id, _) = evidence_impl(self.tcx, typing_env, tr)?;
        if !crate::lower::recognition::is_std_crate(self.tcx, impl_id) {
            return None;
        }
        let ty::Ref(_, param, _) = self
            .tcx
            .type_of(impl_id)
            .instantiate_identity()
            .skip_normalization()
            .kind()
        else {
            return None;
        };
        let ty::Param(param) = param.kind() else { return None };
        let clauses = self.tcx.clauses_of(impl_id).instantiate_identity(self.tcx);
        let passes_on = clauses.into_iter().any(|(clause, _)| {
            clause.skip_normalization().as_trait_clause().is_some_and(|t| {
                t.def_id() == tr.def_id && matches!(t.self_ty().skip_binder().kind(), ty::Param(p) if p == param)
            })
        });
        passes_on.then(|| {
            let mut args: Vec<ty::GenericArg<'tcx>> = tr.args.iter().collect();
            args[0] = (*referent).into();
            ty::TraitRef::new(self.tcx, tr.def_id, args)
        })
    }

    /// An `IntoIterator` dictionary's `Iterator` of what it makes,
    /// `IntoIterIterator`, which steps that as it is, as the trait declares
    /// its `IntoIter` (ADRs 0552, 0566): every one has it, a reference's too,
    /// as generic code that holds what it makes steps it (#278).
    fn made_steps(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
        let mut steps = Vec::new();
        for (name, bound) in item_bounds(self.tcx, tr.def_id, tr.args) {
            if is_std_def(self.tcx, bound.def_id, StdItem::Iterator) {
                let of = self.tcx.erase_and_anonymize_regions(bound.self_ty());
                let of = (self.tcx)
                    .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(of))
                    .unwrap_or(of);
                // A type parameter's given as it is, its own, as what an
                // `I: Iterator` makes is itself (ADR 0920).
                let dictionary = match self.iterator_evidence(of, bound.def_id) {
                    Some(given) => given,
                    None => self.iterator_dictionary(of, span)?,
                };
                steps.push(Prop::Field(
                    name,
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
                ));
            }
        }
        Ok(steps)
    }

    /// A dictionary's facts of its associated types, what generic code asks
    /// of `<A as Array>::Item` (ADR 0145): `ItemSize`, its type's for `tr`.
    pub(super) fn item_facts(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
        let mut props = Vec::new();
        for (item, item_ty) in self.item_types(tr) {
            for &(_, fact) in self.krate.type_facts.get(&item).into_iter().flatten() {
                let value = self.type_fact_value(item_ty, fact, span)?;
                props.push(Prop::Field(item_fact_key(self.tcx, item, fact), value));
            }
        }
        Ok(props)
    }

    /// Each associated type of `tr`'s trait, plain, and what it is for `tr`.
    pub(super) fn item_types(&self, tr: ty::TraitRef<'tcx>) -> Vec<(DefId, Ty<'tcx>)> {
        let items = self.tcx.associated_items(tr.def_id).in_definition_order();
        items
            .filter(|item| {
                self.tcx.def_kind(item.def_id) == DefKind::AssocTy
                    && !item.is_impl_trait_in_trait()
                    && self.tcx.generics_of(item.def_id).own_params.is_empty()
            })
            .filter_map(|item| {
                let projection = Ty::new_projection(self.tcx, ty::IsRigid::No, item.def_id, tr.args);
                let normalized = self
                    .tcx
                    .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection));
                Some((item.def_id, normalized.ok()?))
            })
            .collect()
    }

    fn object_dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        let object = Expr::var("object");
        let principal = self
            .dyn_trait_ref(tr.self_ty(), tr.self_ty())
            .ok_or_else(|| self.no_evidence(span, tr))?;
        let mut props = self.supertrait_entries(tr, span)?;
        props.extend(self.item_bound_entries(tr, span)?);
        for item in self.tcx.associated_items(tr.def_id).in_definition_order() {
            // What a `dyn` can't be called with, and what a std trait's
            // dictionary hasn't got, as `lower_dictionary` leaves out.
            if !item.is_fn()
                || self.tcx.generics_require_sized_self(item.def_id)
                || (!self.is_rust_trait(tr.def_id)
                    && self.tcx.defaultness(item.def_id).has_value()
                    && !in_std_dictionary(self.tcx, item.def_id))
            {
                continue;
            }
            let inputs = self
                .tcx
                .fn_sig(item.def_id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs();
            // Without a `Formatter`, which isn't a JS parameter (ADR 0054), and
            // with a writer's options, where the crate's take them (ADRs 0058, 0137).
            let writer = self.formatter_param(item.def_id).is_some();
            let count = inputs.len() - usize::from(writer) + usize::from(self.writers_take_options() && writer);
            // A `&mut self` method is given a box of the pair, as generic code
            // has a `&mut T` (ADR 0099), and gives the pair, as a call on the
            // `dyn` does: `object.value.impl.scale(object.value, k)`.
            let mutable = matches!(inputs[0].kind(), ty::Ref(_, _, Mutability::Mut));
            let pair = match mutable {
                true => Expr::held(object.clone()),
                false => object.clone(),
            };
            let dictionary = self
                .super_evidence(principal, tr, Expr::member(pair.clone(), js::DICTIONARY))
                .ok_or_else(|| self.no_evidence(span, tr))?;
            let this = match mutable {
                true => pair,
                false => Expr::held(pair),
            };
            let mut params: Vec<js::Pattern> = vec!["object".into()];
            let mut values = vec![this];
            for i in 1..count {
                params.push(format!("arg{i}").into());
                values.push(Expr::var(&format!("arg{i}")));
            }
            let name = bindings::fn_name(self.tcx, item.def_id);
            let call = Expr::call(Expr::member(dictionary.clone(), name.clone()), values);
            props.push(Prop::Field(
                name,
                Expr::arrow(params, vec![StmtKind::Return(Some(call)).at(js::Span::NONE)]),
            ));
        }
        props.extend(self.item_drops(tr, span)?);
        Ok(Expr::object(props))
    }
}
