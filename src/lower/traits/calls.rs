//! A call of a trait's method: the dictionaries it's given, an operator's,
//! and a generic impl's constant.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A type whose `Borrow` is the crate's that std's function would take
    /// as the value itself, for what it borrows as (ADR 0167): not the
    /// crate's own `borrow()` or `borrow_mut()`, which is the crate's.
    pub(in crate::lower) fn borrowed_by_user(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> R<Option<Ty<'tcx>>> {
        let crates = self
            .resolve_instance(def_id, generic_args)?
            .is_some_and(|instance| self.is_rust_fn(instance.def_id()));
        Ok(match self.is_rust_fn(def_id) || crates {
            true => None,
            false => self.recognition().borrowed_by_user(def_id, generic_args),
        })
    }

    /// A collection of the crate's that a call takes as any `IntoIterator`:
    /// generic code, std's or the crate's, iterates what it's given as JS
    /// does, which never calls the collection's own `into_iter` (ADR 0160).
    /// Not `into_iter` itself, which is the collection's.
    pub(in crate::lower) fn collection_as_iterable(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<Ty<'tcx>> {
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        if self.tcx.trait_of_assoc(def_id) == Some(into_iterator) {
            return None;
        }
        let declared = self.tcx.clauses_of(def_id).instantiate_identity(self.tcx);
        let clauses = self.tcx.clauses_of(def_id).instantiate(self.tcx, generic_args);
        clauses
            .clauses
            .iter()
            .zip(declared.clauses.iter())
            .find_map(|(clause, declared)| {
                // One on a reference to a type parameter is given its dictionary,
                // the collection's own `into_iter` (ADR 0457).
                let declared = declared.skip_normalization().as_trait_clause()?.skip_binder().trait_ref;
                if into_iter_by_dictionary(self.tcx, declared.def_id, declared.self_ty()) {
                    return None;
                }
                // A bound of any lifetime, `for<'a> &'a T: IntoIterator`, is of an
                // erased one: a type rustc can select an impl for.
                let bound = self
                    .tcx
                    .instantiate_bound_regions_with_erased(clause.skip_normalization().as_trait_clause()?)
                    .trait_ref;
                (bound.def_id == into_iterator)
                    .then(|| bound.self_ty())
                    .filter(|&ty| self.user_into_iter(ty).is_some())
            })
    }

    /// What's refused of a call of `def_id`: a collection of the crate's
    /// taken as any `IntoIterator`, a type taken as what its own `Borrow`
    /// gives (ADRs 0160, 0167), and a `&mut &mut` of an object given to
    /// generic code (ADR 0454).
    pub(in crate::lower) fn check_call_takes(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
    ) -> R<()> {
        if let Some(collection) = self.collection_as_iterable(def_id, generic_args) {
            let what = format!("a `{collection}`, whose `IntoIterator` is the crate's, where any `IntoIterator` goes");
            return Err(self.unsupported(span, &what));
        }
        if let Some(ty) = self.borrowed_by_user(def_id, generic_args)? {
            let path = self.tcx.def_path_str(def_id);
            return Err(self.unsupported(span, &format!("`{path}` of a `{ty}` as what its own `Borrow` gives")));
        }
        // A `&mut` of a `&mut` of an object for a type parameter of the
        // crate's: generic code's each `&mut` is a box (ADR 0454), where
        // the caller's `&mut` of an object is the object, adapted one level
        // only, `given_box`'s, so it would be a box short (#167).
        let crate_call =
            self.is_rust_fn(def_id) || self.tcx.trait_of_assoc(def_id).is_some_and(|tr| self.is_rust_trait(tr));
        if crate_call
            && let Some(ty) = generic_args.types().find(|&ty| {
                matches!(ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut)
                    if matches!(inner.kind(), ty::Ref(_, object, ty::Mutability::Mut) if self.is_object(*object)))
            })
        {
            return Err(self.unsupported(
                span,
                &format!("a `{ty}`, a `&mut` of a `&mut` of an object, given to generic code"),
            ));
        }
        Ok(())
    }

    pub(in crate::lower) fn evidence_args(
        &mut self,
        id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        span: Span,
    ) -> R<Vec<Expr>> {
        let location = self.caller_location(span);
        self.evidence_args_with(id, args, location, span)
    }

    /// `evidence_args`, `location` the one it's given where it takes one: its
    /// caller's, or, called by a shim of it, a function pointer or a
    /// dictionary's entry, its definition's, as rustc's shim gives it (ADR
    /// 0427).
    pub(in crate::lower) fn evidence_args_with(
        &mut self,
        id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        location: Expr,
        span: Span,
    ) -> R<Vec<Expr>> {
        self.evidence_args_given(id, args, location, &[], span)
    }

    /// `evidence_args_with`, of the bounds `given` names, the dictionary it
    /// gives for each: one made where it's called, as what a JS iterator's
    /// `tryFold` folds into is `$tryOf(broke)` (ADR 0913).
    pub(in crate::lower) fn evidence_args_given(
        &mut self,
        id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        location: Expr,
        given: &[(ty::TraitRef<'tcx>, Expr)],
        span: Span,
    ) -> R<Vec<Expr>> {
        self.check_drops_given(id, args, span)?;
        if let Some(held) = self.dyn_in_generic_cell(id, args) {
            let what = format!("a cell of a `{held}` given to generic code as a cell of a type parameter");
            return Err(self.unsupported(span, &what));
        }
        // Each of `evidence_slots`', of what this call gives for it: its
        // location; a const's value; a bound's dictionary; a fact of the
        // type given, a library's as its manifest says (ADR 0145); a drop
        // function, a type's with nothing to drop none, left out at the end
        // (ADR 0098), or, of one that takes only the drops it uses, each an
        // argument the pipeline keeps or leaves out (ADR 0300).
        let takes_used = self.krate.takes_used_drops.contains(id);
        let (mut values, mut drops) = (Vec::new(), Vec::new());
        let mut location = Some(location);
        for slot in self.evidence_slots(id, Scope::Whole) {
            match slot {
                Slot::Location => values.extend(location.take()),
                Slot::Const(index) => values.push(self.const_arg(args.const_at(index as usize), span)?),
                Slot::Bound(bound) => {
                    let bound = ty::EarlyBinder::bind(self.tcx, bound)
                        .instantiate(self.tcx, args)
                        .skip_normalization();
                    let erased = self.tcx.erase_and_anonymize_regions(bound);
                    match given
                        .iter()
                        .find(|(of, _)| self.tcx.erase_and_anonymize_regions(*of) == erased)
                    {
                        Some((_, dictionary)) => values.push(dictionary.clone()),
                        None => values.push(self.dictionary(bound, span)?),
                    }
                }
                Slot::Fact(index, fact) => {
                    let of = self.drop_slot_of(id, index, args);
                    // Of a `dyn`, generic code over a `T: ?Sized` resolves a
                    // blanket impl's method where rustc calls the `dyn`'s
                    // own, its value's type's (ADR 0145): refused, as its
                    // answer would be another's.
                    if matches!(of.kind(), ty::Dynamic(..)) {
                        return Err(self.unsupported(span, &format!("a type's fact of `{of}` given to generic code")));
                    }
                    values.push(self.type_fact_value(of, fact, span)?);
                }
                Slot::Drop(index) => {
                    let ty = self.drop_slot_of(id, index, args);
                    let iterated = self.iterated_slot(id, index, args, ty);
                    drops.push(match takes_used {
                        true => Some(self.drop_argument(id, index, (ty, iterated), span)?),
                        false => self.held_drop_function(ty, iterated, span)?,
                    });
                }
            }
        }
        Ok(Expr::optional_args(values, drops))
    }

    /// What `id` asks of its type parameters, `TSize` and `KMap`, by their
    /// index: the crate's, or a library's as its manifest says (ADR 0145).
    /// The type parameters `id` takes a drop function of, by index: a
    /// library's as its manifest says, the crate's own as found (ADR 0098).
    pub(in crate::lower) fn drops_taken(&self, id: DefId) -> Vec<u32> {
        match self.krate.foreign.item(id) {
            Some(item) if !self.compiled_here(id) => item.calling.drop_params(),
            _ => self.krate.drop_params.get(&id).cloned().unwrap_or_default(),
        }
    }

    /// Whether `id` is compiled with this crate: its own, a std body
    /// compiled with it (ADR 0561), or a library's it compiles itself (ADR
    /// 0914), whose facts are this compile's.
    fn compiled_here(&self, id: DefId) -> bool {
        id.is_local()
            || self.krate.foreign.is_instance(id)
            || crate::lower::recognition::is_compiled_std(self.tcx, id) && self.krate.fns.contains_key(&id)
    }

    pub(in crate::lower) fn facts_asked(&self, id: DefId) -> Vec<(u32, TypeFact)> {
        // A std body compiled with this crate asks as the crate's own do
        // (ADR 0561).
        match self.compiled_here(id) {
            true => self.krate.type_facts.get(&id).cloned().unwrap_or_default(),
            false => self.krate.foreign.type_facts(id),
        }
    }

    /// Select user code before std intrinsics, so custom implementations win.
    /// A trait method call, or `None` if it isn't one rust-js dispatches.
    /// `out` gets what must run first, like a receiver computed once.
    /// Are an operator's types, `tr`'s, each a number or a `bool`: JS's own
    /// operator's (ADR 0108)?
    pub(super) fn primitive_operands(&self, tr: ty::TraitRef<'tcx>) -> bool {
        tr.args
            .types()
            .all(|t| crate::lower::representation::Num::of(t.peel_refs()).is_some() || t.peel_refs().is_bool())
    }

    /// `a + b`, or `-a` where `b` is `None`, of `tr`'s numbers, as a number's
    /// `+` is (ADR 0108): a shift's amount of its own type, `Shl<u64>` of a `u32`.
    pub(super) fn number_operator(
        &mut self,
        op: Result<BinOp, UnOp>,
        a: Expr,
        b: Option<Expr>,
        tr: ty::TraitRef<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let ty = tr.self_ty().peel_refs();
        match op {
            Ok(op) => {
                let b = crate::lower::std_types::number::shift_amount_of(
                    op,
                    b.expect("two operands"),
                    ty,
                    tr.args.type_at(1),
                );
                self.binary(op, a, b, None, ty, span)
            }
            Err(op) => self.unary(op, a, ty, span),
        }
    }

    /// A generic impl's constant of its parameters (ADR 0176): its initializer,
    /// `() => [TConstZero.ZERO]`, lowered in its dictionary, which has the
    /// impl's evidence.
    /// The impl's own, `id`, or the trait's default.
    pub(super) fn impl_const_getter(&mut self, id: DefId, span: Span) -> R<Expr> {
        let body = (self.krate.closures.get(&id).copied())
            .ok_or_else(|| self.unsupported(span, "a generic impl's constant of its parameters"))?;
        let nested = crate::lower::Nested::Closure {
            names: self.names.clone(),
        };
        // From its MIR (ADR 0364), its value given back.
        let mut stmts = Vec::new();
        let (_, hoisted) = self.lowered_in(body, id, nested, |cx| match &body.mir {
            Some(mir) => cx.mir_signature(body, mir, &mut stmts),
            None => Err(cx.unsupported(span, "this constant, without its MIR")),
        })?;
        stmts.splice(0..0, hoisted);
        Ok(Expr::arrow(Vec::new(), stmts))
    }

    /// An operator trait's dictionary entry: its method, `add`, after its
    /// `Output`.
    pub(super) fn operator_entry(&self, trait_id: DefId) -> String {
        let method = self
            .tcx
            .associated_items(trait_id)
            .in_definition_order()
            .find(|item| item.is_fn())
            .expect("an operator trait has a method");
        bindings::fn_name(self.tcx, method.def_id)
    }

    pub(in crate::lower) fn trait_call(
        &mut self,
        id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        values: Vec<Expr>,
        span: Span,
        out: &mut Vec<js::Stmt>,
    ) -> R<Option<Expr>> {
        let Some(trait_id) = self.tcx.trait_of_assoc(id) else {
            return Ok(None);
        };
        if self.tcx.fn_trait_kind_from_def_id(trait_id).is_some() {
            return Ok(None);
        }
        // `x.to_string()` is std's, as its `Display` shows it, or a generic
        // `T: ToString`'s dictionary's, which the string call decides: of a
        // `dyn ToString` too, which has no pair to call through.
        if Some(trait_id) == self.to_string_trait() {
            return Ok(None);
        }
        // Whether the call's types are known where it's written, as MIR's
        // recognition reads them: not a copied default's `Self::Item`.
        let known_here = !generic_args.has_non_region_param();
        // In a copied default, `Self` is the impl's type: a call on it
        // resolves to the impl's method, called directly.
        let generic_args = self.in_impl_terms(generic_args);
        let tr = ty::TraitRef::from_assoc(self.tcx, trait_id, generic_args);
        // Rest arguments as a value read, `args.iter()` through the js
        // crate's `Deref` of them: the array itself (ADR 0851).
        if bindings::reads_rest_value(self.tcx, trait_id, tr.self_ty()) {
            return Ok(values.into_iter().next());
        }
        if matches!(tr.self_ty().kind(), ty::Dynamic(..)) && operational(self.tcx, self.krate.foreign, trait_id) {
            // A std trait's dictionary has only what it's given: `Error`'s, its
            // `source` (ADR 0141), and none of the methods std provides.
            if !self.is_rust_trait(trait_id) && self.tcx.defaultness(id).has_value() && !in_std_dictionary(self.tcx, id)
            {
                let what = format!("`{}` of a `{}`", self.tcx.item_name(id), tr.self_ty());
                return Err(self.unsupported(span, &what));
            }
            let mut values = values;
            let receiver = values.remove(0);
            // A `&mut self` method its dictionary takes a box for is given the
            // pair, whose `value` a box's is: a number's impl writes the place
            // it reads (ADR 0099). One of a trait whose `Self` is an iterator,
            // `trait Summed: Iterator`'s, takes the value, as generic code holds
            // it (ADRs 0061, 0511).
            let by_mut = self.param_is_box(id, 0);
            // A pair made here of parts read the same, a struct's last field's
            // `{ value: x.ptr, impl: x.impl }` (ADR 0409), is its parts.
            let parts = match &receiver.kind {
                js::ExprKind::Object(props, ..) if !by_mut => match props.as_slice() {
                    [Prop::Field(held, value), Prop::Field(dictionary, of)]
                        if held == js::HELD
                            && dictionary == js::DICTIONARY
                            && value.reads_same()
                            && of.reads_same() =>
                    {
                        Some((value.clone(), of.clone()))
                    }
                    _ => None,
                },
                _ => None,
            };
            let (this, impl_of) = match parts {
                Some(parts) => parts,
                None => {
                    // The pair is read twice, so one that doesn't read the same,
                    // made here or with effects, goes in a `const` first. It's
                    // still first: `operands` put anything before it that needed
                    // statements in `const`s of its own.
                    let pair = if !receiver.reads_same() {
                        self.spill("receiver", receiver, out)
                    } else {
                        receiver
                    };
                    let dictionary = Expr::member(pair.clone(), js::DICTIONARY);
                    (if by_mut { pair } else { Expr::held(pair) }, dictionary)
                }
            };
            let principal = self.dyn_trait_ref(tr.self_ty(), tr.self_ty()).unwrap();
            let dictionary = self
                .super_evidence(principal, tr, impl_of)
                .ok_or_else(|| self.unsupported(span, "this trait object supertrait"))?;
            values.insert(0, this);
            // Where it's called, where its declaration tracks its caller; where
            // only an impl does, none, which its entry has as its definition's,
            // as rustc's shim gives it, and nothing comes after: a `dyn`'s
            // method has no type parameters of its own (ADR 0427).
            let location = tracks_caller(self.tcx, id).then(|| self.caller_location(span));
            values.extend(self.own_evidence(id, generic_args, location, span)?);
            let called = Expr::call(Expr::member(dictionary, bindings::fn_name(self.tcx, id)), values);
            return Ok(Some(self.fmt_result_value(id, generic_args, called)));
        }
        // A std trait's provided method compiled into `@rust-js/std`,
        // `Iterator::peekable`'s, called on a type parameter: the library's
        // function over `Self`, given its dictionary and drops (ADR 0185,
        // ADR 0561).
        if crate::lower::recognition::is_compiled_std(self.tcx, id)
            && self.resolve_self_instance(id, generic_args)?.is_none()
        {
            let mut values = values;
            let given = self
                .tcx
                .fn_sig(id)
                .instantiate(self.tcx, generic_args)
                .skip_normalization()
                .skip_binder()
                .inputs()
                .to_vec();
            self.given_iterators(id, &mut values, &given, span, out)?;
            values.extend(self.evidence_args(id, generic_args, span)?);
            let called = Expr::call(self.fn_ref(id), values);
            return Ok(Some(self.fmt_result_value(id, generic_args, called)));
        }
        // A std trait's method whose impl rustc leaves to the caller's
        // type, `FuseImpl::next(self)` of a `Fuse<I>`: the impl its type
        // parameter's being a `FusedIterator` picks, by the caller's fact of
        // it, `IFused`, or a `OneShot`, `UOneShot`, and where rustc knows the
        // type whole, that one alone (ADRs 0473, 0543, 0561, 0899).
        if self.resolve_self_instance(id, generic_args)?.is_none()
            && let Some(spec) = crate::lower::recognition::fact_specialization(
                self.tcx,
                self.impl_typing_env().unwrap_or(self.typing_env),
                id,
                generic_args,
            )
        {
            let fact = self.type_fact_value(spec.of, spec.fact, span)?;
            let (selected, impl_args) = spec.selected;
            // The method's own arguments after its impl's, as the call gives
            // them after its trait's.
            let own = &generic_args[self.tcx.generics_of(id).parent_count..];
            let args = self.tcx.mk_args_from_iter(impl_args.iter().chain(own.iter().copied()));
            let mut called = Vec::new();
            for method in [spec.special, selected] {
                let mut values = values.clone();
                values.extend(self.evidence_args(method, args, span)?);
                called.push(Expr::call(self.fn_ref(method), values));
            }
            let selected_call = called.pop().expect("the selected impl's");
            let special = called.pop().expect("the specializing impl's");
            let called = Expr::cond(fact, special, selected_call);
            return Ok(Some(self.fmt_result_value(selected, args, called)));
        }
        // `a == b` of a type equal only to itself is `===`, as of references
        // to one, not a call of its `eq` (ADR 0864, amended).
        let identity = matches!(
            self.recognition().trait_call(id, trait_id),
            Some(TraitCall::Equality { .. })
        ) && tr.args.type_at(1) == tr.self_ty()
            && self.eq_by_address(tr.self_ty());
        // A library's function the call resolves to: an impl's method, or
        // a std trait's provided method compiled into `@rust-js/std`, which
        // is a function over `Self` (ADR 0185, 0561).
        if !identity
            && let Some(instance) = self.resolve_self_instance(id, generic_args)?
            && self.is_rust_fn(instance.def_id())
            && (self.tcx.trait_of_assoc(instance.def_id()).is_none()
                || self.krate.foreign.item(instance.def_id()).is_some()
                || self.krate.foreign.compiles(instance.def_id()))
        {
            let mut values = values;
            let given = self
                .tcx
                .fn_sig(instance.def_id())
                .instantiate(self.tcx, instance.args)
                .skip_normalization()
                .skip_binder()
                .inputs()
                .to_vec();
            self.given_iterators(instance.def_id(), &mut values, &given, span, out)?;
            // Its arguments in the impl's terms, a copied default's, asked of
            // the impl's drops (ADR 0495).
            values.extend(self.with_impl_drops(|cx| cx.evidence_args(instance.def_id(), instance.args, span))?);
            let called = Expr::call(self.fn_ref(instance.def_id()), values);
            return Ok(Some(self.fmt_result_value(instance.def_id(), instance.args, called)));
        }
        // What rust-js writes itself, in place: `c.clone()` of a struct is a
        // copy of it, not a dictionary's `clone` (ADR 0052).
        let known = self.recognition().trait_call(id, trait_id);
        if matches!(known, Some(TraitCall::Clone)) {
            let mut values = values;
            return Ok(Some(self.clone_value(values.remove(0), tr.self_ty(), span, out)?));
        }
        if matches!(known, Some(TraitCall::Default)) {
            return Ok(Some(self.default_value(tr.self_ty(), span)?));
        }
        // `a != b` is `!(a == b)`, as Rust requires them to agree (ADR 0053).
        if let Some(TraitCall::Equality { negate }) = known {
            let [a, b]: [Expr; 2] = values.try_into().map_err(|_| self.unsupported(span, "this `==`"))?;
            // A hand-written `PartialEq<Rhs>` is its own `eq`, whatever `Rhs`
            // is; but not one of a type equal only to itself.
            let eq = if self.is_user_impl(tr) && !identity {
                let eq = self.tcx.associated_item_def_ids(trait_id)[0];
                self.impl_call(eq, tr.args, vec![a, b], span)?
            } else {
                // A `Cow` beside what it borrows, `cow == "a"`: what it borrows (ADR 0400).
                let (lhs, rhs) = (tr.self_ty(), tr.args.type_at(1));
                let (a, lhs) = match lhs == rhs {
                    true => (a, lhs),
                    false => self.cow_side(a, lhs),
                };
                let (b, rhs) = match tr.self_ty() == rhs {
                    true => (b, rhs),
                    false => self.cow_side(b, rhs),
                };
                // An `OsStr` beside a path compares as a path (ADR 0401).
                match self.recognition().is_path(rhs) {
                    true => self.eq_value(a, b, rhs, span, out)?,
                    false => self.eq_values(a, b, (lhs, rhs), span, out)?,
                }
            };
            return Ok(Some(if negate { Expr::unary(js::UnaryOp::Not, eq) } else { eq }));
        }
        // `a < b`, `a.cmp(&b)`, `a.max(b)` (ADR 0057). Of numbers, they're
        // std's operators and `Math.max`, as before.
        let ordering = matches!(known, Some(TraitCall::Ordering));
        if let Some(call) = self.ordering_call(id, tr, values.clone(), span, out)? {
            return Ok(Some(call));
        }
        if ordering && crate::lower::representation::Num::of(tr.self_ty().peel_refs()).is_some() {
            return Ok(None);
        }
        // Where the types are known, `"paren".into()` and `a.add(b)` are std's,
        // written in place, as ever: a dictionary is for generic code (ADR 0108).
        // A conversion no bound gives, `VecDeque::from(v)` of a `Vec<T>`, is
        // std's impl wherever it's called, written in place too (ADR 0477).
        let conversion = is_std_def(self.tcx, trait_id, StdItem::Into) || is_std_def(self.tcx, trait_id, StdItem::From);
        if (crate::lower::recognition::value_operator(self.tcx, trait_id).is_some()
            || crate::lower::recognition::assign_operator(self.tcx, trait_id).is_some()
            || conversion)
            && (!tr.args.has_non_region_param() || (conversion && self.evidence_for(tr).is_none()))
        {
            // In a copied default, the call is the trait's, of `Self`, which std's
            // path doesn't see as the number it is here: `self - Self::one()`.
            if self.given.self_args.is_some()
                && let Some(op) = crate::lower::recognition::value_operator(self.tcx, trait_id)
                && self.primitive_operands(tr)
            {
                let mut values = values.into_iter();
                let a = values.next().expect("an operand");
                return Ok(Some(self.number_operator(op, a, values.next(), tr, span)?));
            }
            return Ok(None);
        }
        // Not an `IntoIterator` whose `IntoIter` is held as it is, its
        // dictionary's (ADR 0552).
        if std_written_in_place(self.tcx, trait_id, tr.self_ty(), known_here) && !self.holds_into_iter(tr.self_ty()) {
            return Ok(None);
        }
        // A std trait's dictionary has only its required methods, and those
        // provided ones it's given.
        if operational(self.tcx, self.krate.foreign, trait_id)
            && !self.is_rust_trait(trait_id)
            && self.tcx.defaultness(id).has_value()
            && !in_std_dictionary(self.tcx, id)
        {
            return Err(crate::lower::refusal::refuse(
                self.tcx,
                span,
                crate::lower::refusal::Refused::Call(id),
            ));
        }
        if operational(self.tcx, self.krate.foreign, trait_id) {
            // Of the impl's terms, in a copied default (ADR 0495).
            let dictionary = self.with_impl_drops(|cx| cx.dictionary(tr, span))?;
            let mut values = values;
            // A range or an iterator of the crate's, or one given as it is,
            // where a method's own generic `IntoIterator` goes, a JS iterator,
            // as a function's (ADRs 0445, 0551).
            let given = (self.tcx.fn_sig(id).instantiate(self.tcx, generic_args))
                .skip_normalization()
                .skip_binder()
                .inputs()
                .to_vec();
            self.given_iterators(id, &mut values, &given, span, out)?;
            let location = self.caller_location(span);
            values.extend(self.with_impl_drops(|cx| cx.own_evidence(id, generic_args, Some(location), span))?);
            return Ok(Some(Expr::call(
                Expr::member(dictionary, bindings::fn_name(self.tcx, id)),
                values,
            )));
        }
        Ok(None)
    }

    /// What a trait's generic method is given where it's called through a
    /// dictionary, after its arguments: its own bounds' evidence, for this call
    /// (ADR 0106). None for one that isn't generic.
    fn own_evidence(
        &mut self,
        id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        location: Option<Expr>,
        span: Span,
    ) -> R<Vec<Expr>> {
        // Each of its own `evidence_slots`', of the types given for them:
        // where it's called, unless it's given none (ADR 0427); its own
        // consts', `repeat::<3>`'s `3` (ADR 0135), bounds' and type
        // parameters' facts (ADR 0466); and a drop for each of its own type
        // parameters the trait declares one for, none, left out at the end,
        // for one with nothing to drop (ADR 0163).
        let (mut values, mut drops) = (Vec::new(), Vec::new());
        let mut location = location;
        for slot in self.evidence_slots(id, Scope::Own) {
            match slot {
                Slot::Location => values.extend(location.take()),
                Slot::Const(index) => values.push(self.const_arg(generic_args.const_at(index as usize), span)?),
                Slot::Bound(bound) => values.push(
                    self.dictionary(
                        ty::EarlyBinder::bind(self.tcx, bound)
                            .instantiate(self.tcx, generic_args)
                            .skip_normalization(),
                        span,
                    )?,
                ),
                Slot::Fact(index, fact) => {
                    let of = self.drop_slot_of(id, index, generic_args);
                    values.push(self.type_fact_value(of, fact, span)?);
                }
                Slot::Drop(index) => {
                    let ty = self.drop_slot_of(id, index, generic_args);
                    drops.push(self.drop_function(ty, span)?);
                }
            }
        }
        Ok(Expr::optional_args(values, drops))
    }

    /// `evidence_args` of an impl's generic method, for its dictionary's entry:
    /// its own bounds' evidence is the entry's caller's, `names`, given for the
    /// trait's, `declared`, each passed on as the impl's bound it is, which may
    /// be in another order; the impl's own are this dictionary's (ADR 0106).
    pub(super) fn method_evidence(
        &mut self,
        method: DefId,
        args: ty::GenericArgsRef<'tcx>,
        declared: &[ty::TraitRef<'tcx>],
        names: &[String],
        location: Expr,
        span: Span,
    ) -> R<Vec<Expr>> {
        let own = own_bounds(self.tcx, self.krate.foreign, self.krate.given, method);
        // Each of its `evidence_slots`': where it's called, as
        // `evidence_args` gives it (ADR 0427); the impl's const parameters'
        // values and its own (ADR 0135); a bound of its own, its caller's,
        // passed on; what it asks of the impl's type parameters and its own
        // (ADRs 0145, 0466); its drops, the impl's type parameters' (ADR
        // 0098): its own, a caller through a dictionary gives none of, are
        // refused (`validate`).
        let (mut values, mut drops) = (Vec::new(), Vec::new());
        let mut location = Some(location);
        for slot in self.evidence_slots(method, Scope::Whole) {
            match slot {
                Slot::Location => values.extend(location.take()),
                Slot::Const(index) => values.push(self.const_arg(args.const_at(index as usize), span)?),
                Slot::Bound(bound) => {
                    let here = ty::EarlyBinder::bind(self.tcx, bound)
                        .instantiate(self.tcx, args)
                        .skip_normalization();
                    let here = self.tcx.erase_and_anonymize_regions(here);
                    if own.contains(&bound) {
                        let at = declared
                            .iter()
                            .position(|&d| self.tcx.erase_and_anonymize_regions(d) == here)
                            .ok_or_else(|| {
                                self.unsupported(span, "a generic method whose bound isn't one its trait declares")
                            })?;
                        values.push(Expr::var(&names[at]));
                    } else {
                        values.push(self.dictionary(here, span)?);
                    }
                }
                Slot::Fact(index, fact) => {
                    let of = self.drop_slot_of(method, index, args);
                    values.push(self.type_fact_value(of, fact, span)?);
                }
                Slot::Drop(index) => {
                    let ty = self.drop_slot_of(method, index, args);
                    drops.push(self.drop_function(ty, span)?);
                }
            }
        }
        Ok(Expr::optional_args(values, drops))
    }
}
