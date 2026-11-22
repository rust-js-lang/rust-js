//! What a caller asks by naming a callee (`of_call`): a std function's
//! facts, what the impls it gives dictionaries of ask, a library's as its
//! manifest says, and a crate function's, followed once all are read. Its
//! own concept beside the relay of what a callee asks (`relay.rs`), split
//! from type_facts.rs at its cap (#1334).

use super::*;

impl<'tcx> Asking<'tcx> {
    /// What `caller` asks by giving the dictionary of `impl_id` with
    /// `impl_args`, for a callee's bound or as the steps of a JS iterator
    /// it makes of an iterator it holds: what that impl asks of its type
    /// parameters, of a caller's given it, the caller asks.
    fn of_evidence(
        &mut self,
        tcx: TyCtxt<'tcx>,
        foreign: &crate::library::Foreign<'_, 'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        caller: DefId,
        (impl_id, impl_args): (DefId, ty::GenericArgsRef<'tcx>),
    ) {
        // An impl the crate lowers asks of a caller's parameter given
        // it through the call edge below, as of a type built of them.
        for (index, arg) in impl_args.iter().enumerate() {
            if let Some(ty) = arg.as_type()
                && let ty::Param(param) = ty.kind()
                && !crate::traits::is_lowered_impl(foreign, impl_id)
            {
                for (at, fact) in foreign.type_facts(impl_id) {
                    if at as usize == index {
                        self.asked.insert((caller, param.index, fact));
                    }
                }
            }
        }
        // Of one given a type of the caller's slots that isn't one of its
        // type parameters, `impl<T: Iterator> Itertools for T` of an
        // `<I as IntoIterator>::IntoIter`: what the impl asks of that,
        // as of a call's (#794).
        let names_slots = |at: usize| (impl_args.get(at).and_then(|arg| arg.as_type())).filter(|ty| ty.has_param());
        let of_slots = |at: usize| names_slots(at).is_some_and(|ty| !matches!(ty.kind(), ty::Param(_)));
        // And of a library's, its slots past its type parameters, a std
        // associated type of one, `<T as Iterator>::Item` of
        // `impl<T: Iterator> Itertools for T`, given `<I as
        // Iterator>::Item` of the caller's `I`: its caller's, as a call's
        // is (#1401).
        let relayed = |at: usize| of_slots(at) || at >= impl_args.len();
        // Of one given the caller's own type parameters too, the layouts
        // of the types built of them its dictionary asks, `X2<W, u8>`'s
        // of an `impl<W, G> AndNot for X2<W, G>` whose `Output` is
        // `Self`, given `W` (#1430).
        let given_params = (0..impl_args.len()).any(|at| names_slots(at).is_some());
        match crate::traits::is_lowered_impl(foreign, impl_id) {
            true if given_params => self.calls.push((caller, impl_id, impl_args)),
            true => {}
            false => {
                for (at, fact) in foreign.type_facts(impl_id) {
                    if !relayed(at as usize) {
                        continue;
                    }
                    let slot = crate::traits::drop_slot(tcx, (foreign, &Default::default()), impl_id, at);
                    for each in asked_of_caller(tcx, foreign, typing_env, (caller, impl_args), (slot, fact)) {
                        self.ask(caller, each);
                    }
                }
            }
        }
    }

    /// What `caller` asks by calling `callee` with `args` where the call
    /// makes a JS iterator of an iterator it holds, the crate's or a
    /// library's, holding the caller's parameters, `Pool<I>` given where a
    /// generic iterator goes: what that iterator's `Iterator` impl asks,
    /// its `next`'s `I: TrustedLen` of an `extend(it.by_ref().take(1))`, as
    /// a dictionary of it would (`held_iterators_stepped`, #2327).
    pub(super) fn of_held_iterators(
        &mut self,
        (tcx, foreign, fns): (
            TyCtxt<'tcx>,
            &crate::library::Foreign<'_, 'tcx>,
            &IndexMap<DefId, FnInfo>,
        ),
        typing_env: ty::TypingEnv<'tcx>,
        caller: DefId,
        call: (DefId, ty::GenericArgsRef<'tcx>),
    ) {
        let iterator = std_item(tcx, StdItem::Iterator);
        for (held, _) in held_iterators_stepped(tcx, typing_env, fns, call) {
            // Not std's, which is the JS iterator it's made, stepped with
            // no facts of what it holds, as its steps are followed none
            // (`followed_held`, ADR 0561).
            if matches!(held.kind(), ty::Adt(adt, _) if crate::recognition::is_std_crate(tcx, adt.did())) {
                continue;
            }
            let bound = ty::TraitRef::new(tcx, iterator, [held]);
            if let Some(evidence) = evidence_impl(tcx, typing_env, bound) {
                self.of_evidence(tcx, foreign, typing_env, caller, evidence);
            }
        }
    }

    /// What `caller` asks by naming `callee` with `args`: a std function's
    /// facts, what the impls it gives dictionaries of ask, a library's as its
    /// manifest says, and a crate function's, followed once all are read.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn of_call(
        &mut self,
        tcx: TyCtxt<'tcx>,
        foreign: &crate::library::Foreign<'_, 'tcx>,
        given: Given<'_>,
        fns: &IndexMap<DefId, FnInfo>,
        typing_env: ty::TypingEnv<'tcx>,
        caller: DefId,
        callee: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) {
        let facts = type_facts(tcx, callee);
        for &fact in facts {
            // `size_of_val` of an unsized `T` is the value's, not the type's:
            // a size or an alignment is asked of a sized one, and its name
            // of any, a `T: ?Sized`'s too.
            let measured = |of: ty::Ty<'tcx>| {
                of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Room | TypeFact::Align)
            };
            // A downcast's type is its last, after its `Box`'s allocator.
            // A `T: Any`'s `TypeId` is its dictionary's (ADR 0331).
            if let Some(of) = args.types().last()
                && let ty::Param(param) = of.kind()
                && measured(of)
                && !(fact == TypeFact::Id && bound_by_any(tcx, caller, param.index))
            {
                self.asked.insert((caller, param.index, fact));
            }
            // Of a type that holds them, `Cow<'_, T>` or `Option<I::Item>`:
            // its name, each one's, a type parameter's or an associated
            // type's only a caller knows, as `name_of_generic` names them
            // (#1603).
            if fact == TypeFact::Name
                && let Some(of) = args.types().last()
                && !is_unknown_where(tcx, of, typing_env)
            {
                let mut parts = of.walk();
                while let Some(part) = parts.next() {
                    let Some(part) = part.as_type() else { continue };
                    match part.kind() {
                        ty::Param(param) => _ = self.asked.insert((caller, param.index, fact)),
                        ty::Alias(..) if is_unknown_where(tcx, part, typing_env) => {
                            parts.skip_current_subtree();
                            self.ask_of_alias(tcx, foreign, caller, (part, true), fact);
                        }
                        _ => {}
                    }
                }
            }
            if let Some(of) = args.types().last() {
                self.ask_of_alias(tcx, foreign, caller, (of, measured(of)), fact);
            }
            // Of a type built of its slots whose layout rustc can't say
            // here, `size_of::<Self>()` of an `impl<T> for Wrap<T>` over
            // `[T; 2]`: its callers', a built slot, as a collect's (#2287).
            if let Some(of) = args.types().last()
                && measured(of)
                && let Some(each @ Asked::Built(..)) = asked_of(tcx, foreign, typing_env, caller, (of, fact))
            {
                self.ask(caller, each);
            }
            // A flatten's items' constant size, of `I::Item`, a std
            // associated type of its caller's parameter (ADR 0899).
            if fact == TypeFact::ConstSize
                && let Some(of) = args.types().next()
                && !matches!(of.kind(), ty::Param(_))
                && let Some(index) = slot_of(tcx, foreign, caller, of)
            {
                self.asked.insert((caller, index, fact));
            }
        }
        if !facts.is_empty() {
            return;
        }
        // Each impl whose dictionary it's given for a bound, `collect()`'s
        // `B: FromIterator` of an `ArrayVec<T, CAP>`: what that asks of
        // its type parameters, of a caller's given it, the caller asks.
        for bound in bounds(tcx, foreign, given, callee) {
            let bound = ty::EarlyBinder::bind(tcx, bound).instantiate(tcx, args);
            let Ok(bound) = tcx.try_normalize_erasing_regions(typing_env, bound) else {
                continue;
            };
            // A `Debug` of a `PhantomData<K>`, `{:?}`'s, shows `K`'s name.
            for index in debug_named(tcx, bound) {
                self.asked.insert((caller, index, TypeFact::Name));
            }
            let Some((impl_id, impl_args)) = evidence_impl(tcx, typing_env, bound) else {
                // `collect()`'s `B: FromIterator<A>` the caller's own bound
                // gives, its dictionary's `from_iter` of the iterator, which
                // the lowering calls given what its declaration carries
                // (`collected_evidence`, `std_method_facts`, #794).
                let from_iterator = std_item(tcx, StdItem::FromIterator);
                if bound.def_id == from_iterator && Some(callee) == collect_of(tcx) {
                    let from_iter = trait_method(tcx, from_iterator, "from_iter");
                    let from_args = tcx.mk_args(&[bound.args[0], bound.args[1], args[0]]);
                    for (index, fact) in crate::traits::foreign_facts(tcx, foreign, from_iter) {
                        let slot = crate::traits::drop_slot(tcx, (foreign, &Default::default()), from_iter, index);
                        let asked = asked_of_caller(tcx, foreign, typing_env, (caller, from_args), (slot, fact));
                        asked.into_iter().for_each(|each| self.ask(caller, each));
                    }
                }
                continue;
            };
            self.of_evidence(tcx, foreign, typing_env, caller, (impl_id, impl_args));
        }
        // A library's function, which asks what its manifest says: of a
        // caller's type parameter given it, or a std associated type of one,
        // `I::IntoIter`'s, the caller asks it too; a trait's method, of the
        // impl's method it resolves to, `Fuse<I>`'s `next_back` (ADR 0561).
        // Not a std body compiled with this crate, which is followed as the
        // crate's own are.
        if !callee.is_local() && !fns.contains_key(&callee) {
            // Of one whose impl a fact picks, each impl's, compiled here,
            // `FlattenCompat`'s of a `OneShot` or not (ADR 0899).
            if let Some(spec) = fact_specialization(tcx, typing_env, callee, args)
                && fns.contains_key(&spec.selected.0)
            {
                // The method's own arguments after its impl's, as the call
                // gives them after its trait's.
                let impl_args = spec.selected.1;
                let own = &args[tcx.generics_of(callee).parent_count..];
                let args = tcx.mk_args_from_iter(impl_args.iter().chain(own.iter().copied()));
                for method in spec.methods() {
                    self.calls.push((caller, method, args));
                }
                return;
            }
            let mut named = vec![(callee, args)];
            // Not of a struct stepped as the JS iterator it is, `Fuse<I>`'s
            // `next`, whose impl's facts its caller never passes.
            if !crate::recognition::stepped_as_js_iterator(tcx, callee, args)
                && let Ok(Some(instance)) = crate::items::resolve_instance_in(tcx, typing_env, callee, args)
                && instance.def_id() != callee
            {
                // One compiled here, a std body's, `FlattenCompat`'s
                // `size_hint` a `Flatten`'s calls: followed as the crate's
                // own are, not as its manifest says (ADR 0899).
                if fns.contains_key(&instance.def_id()) {
                    self.calls.push((caller, instance.def_id(), instance.args));
                    return;
                }
                named.push((instance.def_id(), instance.args));
            }
            // A std trait's required method resolved to an impl, `Vec`'s
            // `from_iter`, asks what that impl's lowering asks
            // (`collect_asks`); only one called through a dictionary
            // carries what its declaration says (#794).
            let resolved = named.len() > 1;
            for (id, args) in named {
                let through = !(resolved && id == callee);
                let facts = match through {
                    false => foreign.type_facts(id),
                    true => crate::traits::foreign_facts(tcx, foreign, id),
                };
                // Of a trait's method called through a dictionary, what any
                // dictionary of it takes (`facts_through_dictionary`, #2285).
                let facts = match through && tcx.trait_of_assoc(id).is_some() {
                    true => crate::traits::facts_through_dictionary(tcx, foreign, id, facts),
                    false => facts,
                };
                for (index, fact) in facts {
                    let slot = crate::traits::drop_slot(tcx, (foreign, &Default::default()), id, index);
                    for each in asked_of_caller(tcx, foreign, typing_env, (caller, args), (slot, fact)) {
                        self.ask(caller, each);
                    }
                }
            }
            return;
        }
        // A trait's method of the crate's, called through its dictionary
        // or resolved, of its own type parameters (ADR 0466); or one of the
        // crate's functions, a compiled std impl's method among them.
        let trait_method = callee.is_local() && tcx.trait_of_assoc(callee).is_some();
        if !trait_method && !fns.contains_key(&callee) {
            return;
        }
        self.calls.push((caller, callee, args));
    }
}

/// std's `Iterator::collect`.
fn collect_of(tcx: TyCtxt<'_>) -> Option<DefId> {
    Some(trait_method(tcx, std_item(tcx, StdItem::Iterator), "collect"))
}
