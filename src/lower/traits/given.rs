//! What a function was given: a trait's default body copied into an impl,
//! in the impl's terms, and its dictionaries, facts and location.

use super::*;

/// What a copied default body replaced of the given of the item it's
/// lowered in, its evidence, arguments, const parameters and caller's
/// location, and how many facts it had, to put back when it's done.
pub(in crate::lower) struct GivenScope<'tcx> {
    evidence: Vec<(ty::TraitRef<'tcx>, Expr)>,
    self_args: Option<ty::GenericArgsRef<'tcx>>,
    const_params: Vec<(u32, Expr)>,
    caller_location: Option<(DefId, Expr)>,
    facts: usize,
}

/// What a copied default body is given (ADRs 0049, 0135, 0427, 0466): the
/// impl's evidence and arguments for its trait's parameters, its const
/// parameters, the location its caller gives, and the facts of its own
/// type parameters, after the item's.
pub(in crate::lower) struct DefaultGiven<'tcx> {
    pub(in crate::lower) evidence: Vec<(ty::TraitRef<'tcx>, Expr)>,
    pub(in crate::lower) self_args: ty::GenericArgsRef<'tcx>,
    pub(in crate::lower) const_params: Vec<(u32, Expr)>,
    pub(in crate::lower) caller_location: Option<(DefId, Expr)>,
    pub(in crate::lower) type_facts: Vec<(Ty<'tcx>, TypeFact, Expr)>,
}

/// The arguments a trait's default, `default`, is copied into the impl
/// `imp` with (ADR 0049), of `args`, its trait's parameters the impl's
/// arguments for them: its own parameters, each the parameter itself,
/// numbered after the impl's, as in the impl's terms none of them is one of
/// the impl's numbered as it is, `U` of `fn counted_with<U>` and the impl's
/// `V` of `Paired<I, V>` (#804). The one mapping of a default into an
/// impl's terms: its body's, its slots' (`slot_bounds`), its drops'.
pub(in crate::lower) fn default_args<'tcx>(
    tcx: TyCtxt<'tcx>,
    imp: DefId,
    default: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> ty::GenericArgsRef<'tcx> {
    let generics = tcx.generics_of(default);
    let (parent, first) = (generics.parent_count, tcx.generics_of(imp).count());
    tcx.mk_args_from_iter(args.iter().enumerate().map(|(at, arg)| {
        if at < parent {
            return arg;
        }
        let param = generics.param_at(at, tcx);
        let index = (first + at - parent) as u32;
        match param.kind {
            ty::GenericParamDefKind::Lifetime => arg,
            ty::GenericParamDefKind::Type { .. } => Ty::new_param(tcx, index, param.name).into(),
            ty::GenericParamDefKind::Const { .. } => {
                ty::Const::new_param(tcx, ty::ParamConst::new(index, param.name)).into()
            }
        }
    }))
}

/// Where a trait's default, `default`, copied into the impl `imp` with
/// `args` (`default_args`), is lowered, in the impl's terms, as its body is
/// (#803), and its types there are known (ADR 0107): the impl's bounds, and
/// the default's own where clause in those terms, `T::Item: IntoIterator`
/// of its `Self::Item: IntoIterator`, its `Self: Sized` the impl's `T:
/// Sized` (ADR 0541), and its own parameters', numbered after the impl's
/// (#804). Not its trait's own `Self: Trait`, which would be chosen over the
/// impl that meets it. The one environment of a copied default: its
/// lowering's, and the analysis's of what it asks (`type_facts`).
pub(in crate::lower) fn default_typing_env<'tcx>(
    tcx: TyCtxt<'tcx>,
    imp: DefId,
    default: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> ty::TypingEnv<'tcx> {
    let own = ty::TypingEnv::post_analysis(tcx, default);
    let impl_env = ty::TypingEnv::post_analysis(tcx, imp);
    // Each as the impl's bounds say it is, `I::Item` of a `Wrapper<I>`'s
    // `Self::Item`.
    let in_impl = (tcx.clauses_of(default).clauses.iter()).map(|&(clause, _)| {
        let clause = ty::EarlyBinder::bind(tcx, clause)
            .instantiate(tcx, args)
            .skip_normalization();
        (tcx.try_normalize_erasing_regions(impl_env, ty::Unnormalized::new_wip(clause))).unwrap_or(clause)
    });
    let in_impl = ty::elaborate::elaborate(tcx, in_impl);
    let mut seen = HashSet::new();
    let bounds = (impl_env.param_env.caller_bounds().iter())
        .chain(in_impl)
        .filter(|&clause| seen.insert(clause));
    let param_env = ty::ParamEnv::new(tcx.mk_clauses_from_iter(bounds));
    ty::TypingEnv::new(param_env, own.typing_mode())
}

/// `mir`, a trait's default's or a closure's in it, in the terms of the
/// impl it's copied into, `args` of them (`default_args`), each type
/// normalized where `typing_env` says what it is,
/// `<Enumerated<P> as Pull>::Out` its impl's `(usize, P::Out)` (#803).
pub(in crate::lower) fn default_mir<'tcx>(
    tcx: TyCtxt<'tcx>,
    mir: &crate::lower::mir::Mir<'tcx>,
    args: ty::GenericArgsRef<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
) -> crate::lower::mir::Mir<'tcx> {
    use rustc_middle::ty::TypeFoldable;
    let mut normal = ty::BottomUpFolder {
        tcx,
        ty_op: |t: Ty<'tcx>| match t.has_aliases() && !t.has_escaping_bound_vars() {
            true => (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(t))).unwrap_or(t),
            false => t,
        },
        lt_op: |r| r,
        ct_op: |c| c,
    };
    let mut made = mir.instantiated(tcx, args);
    made.body = made.body.fold_with(&mut normal);
    made.promoted = made
        .promoted
        .into_iter()
        .map(|body| body.fold_with(&mut normal))
        .collect();
    made
}

/// The arguments `body`, a copied default or a closure made in it, is read
/// in the impl's terms with: the default's (`default_args`), and a
/// closure's own generics past them, its kind, signature and captures, as
/// they are, which its MIR and THIR may name, `minmax_by`'s `|_| ()` in
/// itertools (#828).
pub(in crate::lower) fn default_body_args<'tcx>(
    tcx: TyCtxt<'tcx>,
    args: ty::GenericArgsRef<'tcx>,
    body: DefId,
) -> ty::GenericArgsRef<'tcx> {
    let own = ty::GenericArgs::identity_for_item(tcx, body);
    tcx.mk_args_from_iter(args.iter().chain(own.iter().skip(args.len())))
}

/// Whether `clause`, a trait's bound or an associated type's equality,
/// names only type and const parameters `named` says of, by index: of a
/// default's, its trait's alone, the impl's in its terms, or its own alone
/// (`default_args`). Any other clause, none.
pub(in crate::lower) fn names_only<'tcx>(clause: ty::Clause<'tcx>, named: impl Fn(u32) -> bool) -> bool {
    let parts: Vec<ty::GenericArg<'tcx>> = match clause.kind().skip_binder() {
        ty::ClauseKind::Trait(bound) => bound.trait_ref.args.to_vec(),
        ty::ClauseKind::Projection(equal) => (equal.projection_term.args.iter())
            .chain(equal.term.as_type().map(Into::into))
            .collect(),
        _ => return false,
    };
    (parts.iter().flat_map(|part| part.walk())).all(|part| match part.kind() {
        ty::GenericArgKind::Type(t) => !matches!(t.kind(), ty::Param(p) if !named(p.index)),
        ty::GenericArgKind::Const(c) => !matches!(c.kind(), ty::ConstKind::Param(p) if !named(p.index)),
        ty::GenericArgKind::Lifetime(_) => true,
    })
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Enter a trait's default body copied into an impl (ADR 0049): it's
    /// given `evidence`, and the impl's arguments for its trait's
    /// parameters, `self_args`, which resolve in the typing environment
    /// of the item it's lowered in, the impl's.
    pub(in crate::lower) fn enter_default(&mut self, given: DefaultGiven<'tcx>) -> GivenScope<'tcx> {
        let DefaultGiven {
            evidence,
            self_args,
            const_params,
            caller_location,
            type_facts,
        } = given;
        let facts = self.given.type_facts.len();
        self.given.type_facts.extend(type_facts);
        GivenScope {
            evidence: std::mem::replace(&mut self.given.evidence, evidence),
            self_args: self.given.self_args.replace(self_args),
            const_params: std::mem::replace(&mut self.given.const_params, const_params),
            caller_location: std::mem::replace(&mut self.given.caller_location, caller_location),
            facts,
        }
    }

    /// Leave the copied default `enter_default` entered.
    pub(in crate::lower) fn leave_default(&mut self, scope: GivenScope<'tcx>) {
        self.given.evidence = scope.evidence;
        self.given.self_args = scope.self_args;
        self.given.const_params = scope.const_params;
        self.given.caller_location = scope.caller_location;
        self.given.type_facts.truncate(scope.facts);
    }

    /// Lower what's given nothing of this item's, a recursive type's own
    /// function of its module (`recursive_fns`): what it's given, set
    /// aside, which `restore_given` gives back.
    pub(in crate::lower) fn set_given_aside(&mut self) -> super::super::Given<'tcx> {
        std::mem::take(&mut self.given)
    }

    /// What `set_given_aside` set aside, given again.
    pub(in crate::lower) fn restore_given(&mut self, given: super::super::Given<'tcx>) {
        self.given = given;
    }

    /// `resolve_instance` of a call whose arguments a copied default's
    /// `Self` was replaced in (ADR 0049): they're the impl's, which resolve in
    /// the impl's typing environment, as its own body's types don't.
    pub(in crate::lower) fn resolve_self_instance(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Result<Option<ty::Instance<'tcx>>, rustc_span::ErrorGuaranteed> {
        // The impl's, with the default's own bounds, which its own type
        // parameters need, `[B]: Index<PatternID>` (ADR 0560).
        let typing_env = self.typing_env;
        crate::lower::items::resolve_instance_in(self.tcx, typing_env, def_id, args)
    }

    /// Is this a trait's default body copied into an impl (ADR 0049), whose
    /// type parameters are its trait's?
    /// The const parameters' values this function was given, by index: of
    /// an impl's dictionary, the impl's, which a default copied into it, in
    /// its terms, names (ADR 0135).
    pub(in crate::lower) fn impl_const_params(&self) -> Vec<(u32, Expr)> {
        self.given.const_params.clone()
    }

    /// In a trait's default copied into an impl, the arguments it's copied
    /// with (`default_args`), which its body and each closure in it are in
    /// the terms of.
    pub(in crate::lower) fn copied_default_args(&self) -> Option<ty::GenericArgsRef<'tcx>> {
        self.given.self_args
    }

    pub(in crate::lower) fn in_copied_default(&self) -> bool {
        self.given.self_args.is_some()
    }

    /// The first dictionary this function was given whose bound `which`
    /// says is the one: `T: Copy`'s, or a codec's (ADRs 0049, 0081).
    pub(in crate::lower) fn given_evidence(&self, which: impl Fn(ty::TraitRef<'tcx>) -> bool) -> Option<Expr> {
        self.given
            .evidence
            .iter()
            .find(|&&(tr, _)| which(tr))
            .map(|(_, value)| value.clone())
    }

    /// The names what this function was given reads, its parameters after
    /// its own or, in a copied default, its impl's, which its body's top
    /// can read: its dictionaries, `TTouch`, and what one made of them reads,
    /// a copied default's `holderTouch(TTouch)`; its const parameters'
    /// values, facts, caller's location and drops (ADR 0979, amended).
    pub(in crate::lower) fn given_names(&self) -> HashSet<String> {
        let given = (self.given.evidence.iter().map(|(_, value)| value))
            .chain(self.given.const_params.iter().map(|(_, value)| value))
            .chain(self.given.type_facts.iter().map(|(_, _, value)| value))
            .chain(self.given.caller_location.iter().map(|(_, value)| value));
        let mut names: HashSet<String> = self.given_drops().into_iter().collect();
        for value in given {
            value.clone().each_mut(&mut |part| {
                if let js::ExprKind::Var(name) = &part.kind {
                    names.insert(name.clone());
                }
            });
        }
        names
    }

    /// The fact of its type parameter `index` this function was given,
    /// `TSize` (ADR 0145).
    /// Whether type parameter `index` is a `FusedIterator`, as its caller
    /// says, `IFused` (ADR 0543): `None` where none says, which is `false`,
    /// a generic iterator's `Fuse` cleared as it ends, as most are (ADR
    /// 0473). The one answer, of a `fuse()` and of a callee asking it.
    pub(in crate::lower) fn fused_fact(&self, index: u32) -> Option<Expr> {
        self.given_type_fact(index, TypeFact::Fused)
    }

    pub(in crate::lower) fn given_type_fact(&self, index: u32, fact: TypeFact) -> Option<Expr> {
        (self.given.type_facts.iter())
            .find(|&&(of, given, _)| matches!(of.kind(), ty::Param(p) if p.index == index) && given == fact)
            .map(|(_, _, value)| value.clone())
    }

    /// The fact of `of`, a std associated type of a type parameter, this
    /// function was given, `IItemMap` (ADR 0466): each given one and `of`
    /// compared as the bounds here normalize them, so a copied default's
    /// `<Self as Iterator>::Item`, the impl's slot, is the `<T as
    /// TupleCollect>::Item` its where clause, `Self: Iterator<Item =
    /// T::Item>`, says it is, itertools' `collect_tuple` (#828).
    pub(in crate::lower) fn given_fact_of(&self, of: Ty<'tcx>, fact: TypeFact) -> Option<Expr> {
        let normal = |ty: Ty<'tcx>| {
            let ty = self.tcx.erase_and_anonymize_regions(ty);
            (self.tcx)
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
                .unwrap_or(ty)
        };
        let of = normal(of);
        (self.given.type_facts.iter())
            .find(|&&(at, given, _)| given == fact && normal(at) == of)
            .map(|(_, _, value)| value.clone())
    }

    /// The caller's location this body was given, of a function that takes
    /// one, in its own body, not a closure in it (ADR 0427).
    pub(in crate::lower) fn given_location(&self) -> Option<Expr> {
        let (owner, location) = self.given.caller_location.as_ref()?;
        (*owner == self.frame.owner).then(|| location.clone())
    }

    /// Whether an impl's dictionary takes any of `evidence_params`': a const
    /// parameter's value, a bound's dictionary, a type's fact or a drop; not
    /// a caller's location, which only a `#[track_caller]` function takes. One
    /// that takes none is its module's `const` (ADR 0592).
    pub(in crate::lower) fn takes_evidence(&self, id: DefId) -> bool {
        self.takes_more_than_drops(id) || !self.drops_taken(id).is_empty()
    }

    /// Whether `id` takes evidence besides its drops: a const parameter's
    /// value, a bound's dictionary or a type's fact, without which a call
    /// of it can't be made; a drop it's not given it doesn't do.
    pub(in crate::lower) fn takes_more_than_drops(&self, id: DefId) -> bool {
        !const_params(self.tcx, id).is_empty()
            || !bounds(self.tcx, self.krate.foreign, self.krate.given, id).is_empty()
            || !self.facts_asked(id).is_empty()
    }

    /// Whether what `id`, an impl's method, takes besides its drops is of
    /// its own parameters, not only its impl's: a const's value, a bound's
    /// dictionary or a fact of one, `B: Clone` of a fold's `B`. What it takes
    /// of its impl's a caller gives of the type it knows, the sides'
    /// dictionaries of an `Either<L, R>`'s `rfold` (#475); of its own, a
    /// caller giving it no type of them, the runtime's fold, can't.
    pub(in crate::lower) fn takes_of_its_own(&self, id: DefId) -> bool {
        let impls = self.tcx.generics_of(id).parent_count as u32;
        let of_its_own = |ty: Ty<'tcx>| {
            ty.walk()
                .filter_map(|arg| arg.as_type())
                .any(|part| matches!(part.kind(), ty::Param(param) if param.index >= impls))
        };
        const_params(self.tcx, id).iter().any(|param| param.index >= impls)
            || bounds(self.tcx, self.krate.foreign, self.krate.given, id)
                .iter()
                .any(|tr| tr.args.types().any(of_its_own))
            || self.facts_asked(id).iter().any(|&(index, _)| index >= impls)
    }

    /// What a body takes after its own parameters, `evidence_slots`'
    /// order, each named and given to what lowers it: its caller's
    /// location, `location` (ADR 0427); a const parameter's value, `N` (ADR
    /// 0107); a bound's dictionary, `IIterator`, `writeT` and `readT` of a
    /// generic codec's (ADR 0081), `XConvertF64` of two impls of one trait;
    /// a fact of a type parameter, `TSize` (ADR 0145); and a drop function,
    /// `dropT` (ADR 0098), `dropIIntoIter` of a std trait's associated type
    /// (ADR 0421).
    pub(in crate::lower) fn evidence_params(&mut self, id: DefId) -> Vec<js::Pattern> {
        self.give_drop_slots(drop_items(self.tcx, self.krate.foreign, id));
        let mut params: Vec<js::Pattern> = Vec::new();
        for slot in self.evidence_slots(id, Scope::Whole) {
            let name = match slot {
                Slot::Location => {
                    let name = self.fresh("location");
                    self.given.caller_location = Some((id, Expr::var(&name)));
                    name
                }
                Slot::Const(index) => {
                    let param = self.tcx.generics_of(id).param_at(index as usize, self.tcx);
                    let name = self.fresh(param.name.as_str());
                    self.given.const_params.push((index, Expr::var(&name)));
                    name
                }
                Slot::Bound(tr) => {
                    let name = self.fresh(&bound_word(self.tcx, tr));
                    self.given.evidence.push((tr, Expr::var(&name)));
                    // A std body's default, where a call gives none (ADR 0995).
                    if let Some(default) = self.default_dictionary(id, tr.def_id) {
                        params.push(js::Pattern::Defaulted(name, plain_dictionary(default)));
                        continue;
                    }
                    name
                }
                Slot::Fact(index, fact) => {
                    let of = drop_slot(self.tcx, self.slots(), id, index);
                    let name = self.fresh(&format!("{}{}", drop_word(self.tcx, of), fact.word()));
                    self.given.type_facts.push((of, fact, Expr::var(&name)));
                    name
                }
                Slot::Drop(index) => {
                    let ty = drop_slot(self.tcx, self.slots(), id, index);
                    let name = self.fresh(&format!("drop{}", drop_word(self.tcx, ty)));
                    self.give_drop_param(index, name.clone(), ty);
                    name
                }
            };
            params.push(name.into());
        }
        params
    }

    /// A const argument's value (ADR 0107): `3`, or the caller's own `N`.
    pub(in crate::lower) fn const_arg(&self, c: ty::Const<'tcx>, span: Span) -> R<Expr> {
        let c = self
            .tcx
            .normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(c));
        let value = match c.kind() {
            ty::ConstKind::Param(p) => self
                .given
                .const_params
                .iter()
                .find(|&&(index, _)| index == p.index)
                .map(|(_, value)| value.clone()),
            _ => c
                .try_to_value()
                .and_then(|value| const_js(self.tcx, &self.krate.flattened.keyed, value)),
        };
        value.ok_or_else(|| self.unsupported(span, "this const argument"))
    }
}
