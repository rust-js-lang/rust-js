//! What a function was given: a trait's default body copied into an impl,
//! in the impl's terms, and its dictionaries, facts and location.

use super::*;

/// What a copied default body replaced of the given of the item it's
/// lowered in, its evidence, arguments, const parameters and caller's
/// location, and how many facts it had, to put back when it's done.
pub(in crate::lower) struct GivenScope<'tcx> {
    evidence: Vec<(ty::TraitRef<'tcx>, Expr)>,
    self_args: Option<ty::GenericArgsRef<'tcx>>,
    self_env: Option<ty::TypingEnv<'tcx>>,
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
            self_env: self.given.self_env.replace(self.typing_env),
            const_params: std::mem::replace(&mut self.given.const_params, const_params),
            caller_location: std::mem::replace(&mut self.given.caller_location, caller_location),
            facts,
        }
    }

    /// Leave the copied default `enter_default` entered.
    pub(in crate::lower) fn leave_default(&mut self, scope: GivenScope<'tcx>) {
        self.given.evidence = scope.evidence;
        self.given.self_args = scope.self_args;
        self.given.self_env = scope.self_env;
        self.given.const_params = scope.const_params;
        self.given.caller_location = scope.caller_location;
        self.given.type_facts.truncate(scope.facts);
    }

    /// `value` in the impl's terms, in a copied default: its trait's
    /// parameters, `Self` among them, replaced by the impl's arguments for
    /// them (ADR 0049). Anywhere else, `value` itself.
    pub(in crate::lower) fn in_impl_terms<T: TypeFoldable<TyCtxt<'tcx>>>(&self, value: T) -> T {
        match self.given.self_args {
            Some(args) => ty::EarlyBinder::bind(self.tcx, value)
                .instantiate(self.tcx, args)
                .skip_normalization(),
            None => value,
        }
    }

    /// `ty` of a copied default's body in the impl's terms, normalized there
    /// where it can be: its `Self::Item<'a>` the impl's `&String`.
    /// Where a copied default's types in its impl's terms are known: both
    /// the impl's parameters and the method's own, `Buffer<SIZE>` or the
    /// method's `N`, the bounds of each (ADR 0107). `None` elsewhere.
    pub(in crate::lower) fn impl_typing_env(&self) -> Option<ty::TypingEnv<'tcx>> {
        let impl_env = self.given.self_env?;
        // The default's own, in its trait's terms, which its body names; and
        // its `Self: Sized` in the impl's, `T: Sized`, which `Enumerate<T>`
        // asks of an impl over `?Sized` types (ADR 0541). No other: one in
        // the impl's terms would be chosen over the impl that meets it.
        let own = self.typing_env.param_env.caller_bounds().iter();
        let sized = (own.clone())
            .filter(|clause| {
                (clause.as_trait_clause()).is_some_and(|tr| self.tcx.is_lang_item(tr.def_id(), LangItem::Sized))
            })
            .map(|clause| self.in_impl_terms(clause));
        let bounds = (impl_env.param_env.caller_bounds().iter()).chain(own).chain(sized);
        let param_env = ty::ParamEnv::new(self.tcx.mk_clauses_from_iter(bounds));
        Some(ty::TypingEnv::new(param_env, impl_env.typing_mode()))
    }

    pub(in crate::lower) fn impl_type(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        if self.given.self_args.is_none() {
            return ty;
        }
        let ty = self.in_impl_terms(ty);
        let typing_env = self.impl_typing_env().unwrap_or(self.typing_env);
        self.tcx
            .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
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
        let typing_env = self.impl_typing_env().unwrap_or(self.typing_env);
        crate::lower::items::resolve_instance_in(self.tcx, typing_env, def_id, args)
    }

    /// Is this a trait's default body copied into an impl (ADR 0049), whose
    /// type parameters are its trait's?
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
    /// function was given, `IItemMap` (ADR 0466).
    pub(in crate::lower) fn given_fact_of(&self, of: Ty<'tcx>, fact: TypeFact) -> Option<Expr> {
        let of = self.tcx.erase_and_anonymize_regions(of);
        (self.given.type_facts.iter())
            .find(|&&(at, given, _)| self.tcx.erase_and_anonymize_regions(at) == of && given == fact)
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
                    let name = match crate::lower::serde::serde_trait(self.tcx, tr.def_id) {
                        Some(true) => format!("write{}", tr.self_ty()),
                        Some(false) => format!("read{}", tr.self_ty()),
                        None => format!("{}{}", evidence_word(self.tcx, tr.self_ty()), trait_word(self.tcx, tr)),
                    };
                    let name = self.fresh(&js_word(&name));
                    self.given.evidence.push((tr, Expr::var(&name)));
                    // A std body's default, where a call gives none (ADR 0995).
                    if let Some(default) = self.default_dictionary(id, tr.def_id) {
                        params.push(js::Pattern::Defaulted(name, plain_dictionary(default)));
                        continue;
                    }
                    name
                }
                Slot::Fact(index, fact) => {
                    let of = drop_slot(self.tcx, self.krate.foreign, id, index);
                    let name = self.fresh(&format!("{}{}", drop_word(self.tcx, of), fact.word()));
                    self.given.type_facts.push((of, fact, Expr::var(&name)));
                    name
                }
                Slot::Drop(index) => {
                    let ty = drop_slot(self.tcx, self.krate.foreign, id, index);
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
            _ => c.try_to_value().and_then(|value| const_js(self.tcx, value)),
        };
        value.ok_or_else(|| self.unsupported(span, "this const argument"))
    }
}
