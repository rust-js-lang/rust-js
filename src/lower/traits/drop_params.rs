//! The drops a generic function is given for its type parameters and
//! their associated types: which, in its own terms, and a dictionary's.

use super::*;

/// Whether `param`, of the generics `typing_env` is of, is a type its caller
/// gives a drop for: one that isn't `Copy` there (ADR 0163). The one test,
/// of a function's, an impl's or a trait's parameters alike.
pub(in crate::lower) fn given_a_drop<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    param: &ty::GenericParamDef,
) -> bool {
    matches!(param.kind, ty::GenericParamDefKind::Type { .. })
        && !tcx.type_is_copy_modulo_regions(typing_env, Ty::new_param(tcx, param.index, param.name))
}

/// A generic trait method's own type parameters its caller gives a drop for
/// (ADR 0163): each one the trait declares that isn't `Copy`, and each std
/// trait's associated type of one its declaration bounds it by,
/// `<I as IntoIterator>::IntoIter` (ADR 0421). Decided by the trait's
/// declaration, which a caller through a dictionary knows, so it and each
/// impl agree; of an impl's method, its own at those places, as their
/// indices are its.
pub(in crate::lower) fn own_drop_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    method: DefId,
) -> Vec<u32> {
    let declared = tcx.trait_item_of(method).unwrap_or(method);
    if tcx.trait_of_assoc(declared).is_none() {
        return Vec::new();
    }
    let types = |id: DefId| -> Vec<&ty::GenericParamDef> {
        tcx.generics_of(id)
            .own_params
            .iter()
            .filter(|p| matches!(p.kind, ty::GenericParamDefKind::Type { .. }))
            .collect()
    };
    let typing_env = ty::TypingEnv::non_body_analysis(tcx, declared);
    let (own, theirs) = (types(method), types(declared));
    let given: Vec<usize> = (0..theirs.len())
        .filter(|&at| given_a_drop(tcx, typing_env, theirs[at]) && at < own.len())
        .collect();
    let mut indices: Vec<u32> = given.iter().map(|&at| own[at].index).collect();
    // Each associated type of one, at the place the method's own bounds have
    // it, the declaration's own parameter as the method's.
    let items = drop_items(tcx, foreign, method);
    for item in drop_items(tcx, foreign, declared) {
        let Some(alias) = alias_of(item) else { continue };
        let ty::Param(param) = *alias.self_ty().kind() else {
            continue;
        };
        let Some(&at) = given.iter().find(|&&at| theirs[at].index == param.index) else {
            continue;
        };
        let place = items.iter().position(|&t| {
            alias_of(t).is_some_and(|a| {
                a.kind == alias.kind && matches!(*a.self_ty().kind(), ty::Param(p) if p.index == own[at].index)
            })
        });
        indices.extend(place.map(|place| item_slot(tcx, method, place)));
    }
    indices
}

/// What a std trait's required generic method's dictionary carries of its
/// own type parameters, the same for every crate (#794): of each one its
/// declaration bounds by `IntoIterator`, `InPlace` of its `IntoIter`, and
/// of one bounded by `Iterator`, its own, the buffer std's `collect()` into
/// a `Vec` may collect it in (#672), which an impl's `Vec::from_iter(iter)`
/// asks: `FromIterator::from_iter`'s, `Extend::extend`'s, `Sum::sum`'s.
/// Decided by the trait's declaration, as `own_drop_params` is: its impls
/// are its only bodies, a library's and a consumer's alike, so a union of
/// the impls one crate sees would leave a library calling it through a
/// dictionary giving less than another crate's impl asks. A provided
/// method's std body, which the crate compiles, asks its own (ADR 0561);
/// the crate's and a library's traits carry what their code asks (ADR
/// 0466).
pub(in crate::lower) fn std_method_facts<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    method: DefId,
) -> Vec<(u32, TypeFact)> {
    if method.is_local() || foreign.is_library(method.krate) || tcx.trait_of_assoc(method).is_none() {
        return Vec::new();
    }
    if tcx.associated_item(method).defaultness(tcx).has_value() {
        return Vec::new();
    }
    let generics = tcx.generics_of(method);
    let parent = generics.parent_count as u32;
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    let iterator = std_item(tcx, StdItem::Iterator);
    let into_iter = crate::lower::recognition::trait_method(tcx, into_iterator, "IntoIter");
    let items = drop_items(tcx, foreign, method);
    let mut facts = Vec::new();
    for (clause, _) in tcx.clauses_of(method).instantiate_identity(tcx) {
        let Some(bound) = clause
            .skip_normalization()
            .as_trait_clause()
            .and_then(|b| b.no_bound_vars())
        else {
            continue;
        };
        let ty::Param(param) = *bound.self_ty().kind() else {
            continue;
        };
        if param.index < parent {
            continue;
        }
        if bound.def_id() == iterator {
            facts.push((param.index, TypeFact::InPlace));
        } else if bound.def_id() == into_iterator {
            // Its `IntoIter`'s slot, after the type parameters (`drop_slot`).
            let place = items.iter().position(|&item| {
                alias_of(item).is_some_and(|alias| {
                    matches!(alias.kind, ty::Projection { def_id } if def_id == into_iter)
                        && matches!(*alias.self_ty().kind(), ty::Param(p) if p.index == param.index)
                })
            });
            facts.extend(place.map(|place| (item_slot(tcx, method, place), TypeFact::InPlace)));
            // And of `FromIterator::from_iter`'s, whether it's `TrustedLen`,
            // of which and `IInPlace` an entry that collects a `Vec` makes
            // how std's `SpecFromIter` collects it, where it knows its items'
            // layout, which a generic caller may not (#1239).
            if tcx.trait_of_assoc(method) == Some(std_item(tcx, StdItem::FromIterator)) {
                facts.extend(place.map(|place| (item_slot(tcx, method, place), TypeFact::TrustedLen)));
            }
        }
    }
    facts.sort();
    facts.dedup();
    facts
}

/// What a function compiled elsewhere asks of its type parameters, as the
/// analysis relays it to its callers and a call gives it: a std trait's
/// required method, what its declaration carries (`std_method_facts`); a
/// library's function, what its manifest says. The one answer of both.
pub(in crate::lower) fn foreign_facts<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    id: DefId,
) -> Vec<(u32, TypeFact)> {
    match std_method_facts(tcx, foreign, id) {
        facts if !facts.is_empty() => facts,
        _ => foreign.type_facts(id),
    }
}

/// The type a function's slot `index` is, in its own terms: its type
/// parameter's, `T`, below its generics' count, and from there each
/// associated type of a type parameter's bound by a trait no dictionary
/// carries, a std trait's, `<I as IntoIterator>::IntoIter`, in its bounds'
/// order (ADR 0421). Its callers know its bounds, so they and it agree, a
/// library's consumers too. One a bound names, `IntoIterator<Item = T>`,
/// is that type's, and isn't one. Past those, each type built of them its
/// layout is asked of, `Held<<H::Item as IntoIterator>::IntoIter>`, which
/// no bound says: the crate's own as `built` lists them, a library's as
/// its manifest does (`built_slot`, #737).
pub(in crate::lower) fn drop_slot<'tcx>(
    tcx: TyCtxt<'tcx>,
    (foreign, built): (&crate::lower::library::Foreign<'_, 'tcx>, &BuiltSlots<'tcx>),
    id: DefId,
    index: u32,
) -> Ty<'tcx> {
    let Some(at) = slot_item(tcx, id, index) else {
        let param = tcx.generics_of(id).param_at(index as usize, tcx);
        return Ty::new_param(tcx, param.index, param.name);
    };
    let items = drop_items(tcx, foreign, id);
    match items.get(at) {
        Some(&item) => item,
        None => {
            (built_slot(tcx, (foreign, built), id, index))
                .expect("a slot past the associated types is a built one")
                .ty
        }
    }
}

/// The index of `id`'s slot of the associated type `at` in its
/// `drop_items`' order: after its type parameters'. With `drop_slot`, an
/// index's type, and `slot_index`, a type's index, the one numbering of a
/// function's drop slots, which its callers pass and its body is given
/// (#887).
pub(in crate::lower) fn item_slot(tcx: TyCtxt<'_>, id: DefId, at: usize) -> u32 {
    tcx.generics_of(id).count() as u32 + at as u32
}

/// Which associated type, in `drop_items`' order, `id`'s slot `index` is;
/// `None` of a type parameter's (`item_slot`'s inverse).
pub(in crate::lower) fn slot_item(tcx: TyCtxt<'_>, id: DefId, index: u32) -> Option<usize> {
    index
        .checked_sub(tcx.generics_of(id).count() as u32)
        .map(|at| at as usize)
}

/// Which of `id`'s slots `ty` is, `drop_slot`'s inverse: a type parameter's
/// index, or a std associated type of one's, after them, `Self::Item` (ADR
/// 0466).
pub(in crate::lower) fn slot_index<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    id: DefId,
    ty: Ty<'tcx>,
) -> Option<u32> {
    match ty.kind() {
        // One of its own, not another's numbered past them, a copied
        // default's own in its impl's terms (#804).
        ty::Param(param) => (slot_item(tcx, id, param.index).is_none()).then_some(param.index),
        ty::Alias(..) => {
            let alias = alias_of(ty)?;
            let at = (drop_items(tcx, foreign, id).into_iter()).position(|item| alias_of(item) == Some(alias))?;
            Some(item_slot(tcx, id, at))
        }
        _ => None,
    }
}

/// A type built of a function's slots that it asks the layout of, a
/// collect's items' in place, `Held<J>` of a `Vec<Held<J>>` (#737): no
/// bound names it, so its callers read which it is of its own list, the
/// crate's analysis's or a library's manifest's. Every caller gives it, as
/// the collect's `Vec` grows by its size wherever it's called (#796).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::lower) struct BuiltSlot<'tcx> {
    pub(in crate::lower) ty: Ty<'tcx>,
}

/// Each of the crate's functions' built slots, in their slots' order, after
/// its associated types' (`drop_slot`).
pub(in crate::lower) type BuiltSlots<'tcx> = IndexMap<DefId, Vec<BuiltSlot<'tcx>>>;

/// `id`'s built slot `index`, if it's one: the crate's own function's, as
/// `built` lists them, or a library's, as its manifest says.
pub(in crate::lower) fn built_slot<'tcx>(
    tcx: TyCtxt<'tcx>,
    (foreign, built): (&crate::lower::library::Foreign<'_, 'tcx>, &BuiltSlots<'tcx>),
    id: DefId,
    index: u32,
) -> Option<BuiltSlot<'tcx>> {
    let first = item_slot(tcx, id, drop_items(tcx, foreign, id).len());
    let at = index.checked_sub(first)? as usize;
    match built.get(&id) {
        Some(slots) => slots.get(at).cloned(),
        None => foreign.built_slot(id, index),
    }
}

/// The associated types of a function's type parameters it may be given a
/// drop for, after its type parameters' (`drop_slot`); and of those a bound
/// names, `<I::Item as IntoIterator>::IntoIter` of an `I::Item: IntoIterator`
/// (ADR 0478), each found in turn, of each of `slot_bounds`.
pub(in crate::lower) fn drop_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    id: DefId,
) -> Vec<Ty<'tcx>> {
    let bounds = slot_bounds(tcx, id);
    let mut items: Vec<Ty<'tcx>> = Vec::new();
    loop {
        let found = items.len();
        for (clauses, typing_env) in &bounds {
            drop_items_of(tcx, foreign, (clauses, *typing_env), &mut items);
        }
        if items.len() == found {
            return items;
        }
    }
}

/// The bounds `drop_items` reads, each with where it holds: a function's
/// own, where its bounds hold; and, of an impl, each trait default it keeps'
/// own where clause in the impl's terms, `<T as Iterator>::Item:
/// IntoIterator` of a `Self::Item: IntoIterator`, where the impl's bounds
/// and it hold: the impl's dictionary is given the drops a default copied
/// into it asks of them, as no caller of the default gives any (ADR 0478,
/// #798); and each method's the impl writes, alike (#1080). Not one of the
/// default's own parameters, which its caller gives (`own_drop_params`).
fn slot_bounds<'tcx>(tcx: TyCtxt<'tcx>, id: DefId) -> Vec<(Vec<ty::Clause<'tcx>>, ty::TypingEnv<'tcx>)> {
    let typing_env = ty::TypingEnv::non_body_analysis(tcx, id);
    let own = (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
        .map(|(clause, _)| clause.skip_normalization())
        .collect();
    let mut bounds = vec![(own, typing_env)];
    if !matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true }) {
        return bounds;
    }
    let tr = tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
    let overridden = tcx.impl_item_implementor_ids(id);
    let trait_count = tcx.generics_of(tr.def_id).count() as u32;
    let mut methods: Vec<Vec<ty::Clause<'tcx>>> = Vec::new();
    for item in tcx.associated_items(tr.def_id).in_definition_order() {
        // A default the impl keeps.
        if !matches!(item.kind, ty::AssocKind::Fn { .. })
            || !tcx.defaultness(item.def_id).has_value()
            || overridden.contains_key(&item.def_id)
        {
            continue;
        }
        let args = tr.args.extend_to(tcx, item.def_id, |param, _| match param.kind {
            ty::GenericParamDefKind::Lifetime => tcx.lifetimes.re_erased.into(),
            _ => tcx.mk_param_from_def(param),
        });
        let args = default_args(tcx, id, item.def_id, args);
        // A trait's bound or an associated type's equality, of the impl's
        // parameters alone: none of the default's own, which aren't the
        // impl's, and its caller gives the drops of.
        methods.push(
            (tcx.clauses_of(item.def_id).clauses.iter())
                .filter(|&&(clause, _)| names_only(clause, |index| index < trait_count))
                .map(|&(clause, _)| {
                    ty::EarlyBinder::bind(tcx, clause)
                        .instantiate(tcx, args)
                        .skip_normalization()
                })
                .collect(),
        );
    }
    // And each method the impl writes itself, as a default it keeps: its own
    // where clause of the impl's parameters alone, `Self::Item:
    // IntoIterator` of a `Wrapper<I>`'s `held`, `<I as Iterator>::Item:
    // IntoIterator` in the impl's terms, whose drops its dictionary is given
    // too, as no caller through it knows them (#1080).
    let impl_count = tcx.generics_of(id).count() as u32;
    for &own in tcx.associated_item_def_ids(id) {
        if !matches!(tcx.def_kind(own), DefKind::AssocFn) {
            continue;
        }
        methods.push(
            (tcx.clauses_of(own).clauses.iter())
                .filter(|&&(clause, _)| names_only(clause, |index| index < impl_count))
                .map(|&(clause, _)| clause)
                .collect(),
        );
    }
    for clauses in methods.into_iter().filter(|clauses| !clauses.is_empty()) {
        let held =
            (typing_env.param_env.caller_bounds().iter()).chain(ty::elaborate::elaborate(tcx, clauses.iter().copied()));
        let param_env = ty::ParamEnv::new(tcx.mk_clauses_from_iter(held));
        bounds.push((clauses, ty::TypingEnv::new(param_env, typing_env.typing_mode())));
    }
    bounds
}

/// `drop_items`' of each of `clauses` on a type parameter, or on one of
/// `items`, its associated types those that `typing_env` doesn't name.
fn drop_items_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    (clauses, typing_env): (&[ty::Clause<'tcx>], ty::TypingEnv<'tcx>),
    items: &mut Vec<Ty<'tcx>>,
) {
    for clause in clauses {
        let Some(bound) = clause.as_trait_clause().and_then(|b| b.no_bound_vars()) else {
            continue;
        };
        // As the clause names it: an impl's method's where clause on its
        // `Self::Item`, `<Wrapper<I> as Iterator>::Item: IntoIterator`, is
        // no slot of the impl's, whose items' iterator the bound's own
        // dictionary drops, `$dropIntoIter` (ADR 0451, amended).
        let self_ty = bound.self_ty();
        let slotted = matches!(self_ty.kind(), ty::Param(_))
            || (alias_of(self_ty).is_some() && items.iter().any(|&t| alias_of(t) == alias_of(self_ty)));
        if !slotted {
            continue;
        }
        // Of a trait with a dictionary, which carries its own associated
        // types' facts, its supertraits' with none, std's `Iterator` of the
        // crate's `T: Sorted`, as `T: Sorted + Iterator` would (#934).
        let dictionary = has_dictionary_of(tcx, foreign, bound.def_id(), self_ty);
        for tr in supertrait_def_ids(tcx, bound.def_id()) {
            if dictionary && (tr == bound.def_id() || has_dictionary_of(tcx, foreign, tr, self_ty)) {
                continue;
            }
            // A supertrait's of no parameter but `Self`, whose arguments are its.
            let args = match tr == bound.def_id() {
                true => {
                    tcx.mk_args_from_iter(std::iter::once(self_ty.into()).chain(bound.trait_ref.args.iter().skip(1)))
                }
                false if tcx.generics_of(tr).count() == 1 => tcx.mk_args(&[self_ty.into()]),
                false => continue,
            };
            for item in tcx.associated_items(tr).in_definition_order() {
                if !matches!(item.kind, ty::AssocKind::Type { .. })
                    || tcx.is_impl_trait_in_trait(item.def_id)
                    || !tcx.generics_of(item.def_id).own_params.is_empty()
                {
                    continue;
                }
                let projection = Ty::new_projection(tcx, ty::IsRigid::No, item.def_id, args);
                let named = tcx
                    .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(projection))
                    .is_ok_and(|t| alias_of(t) != alias_of(projection));
                if !named && !items.iter().any(|&t| alias_of(t) == alias_of(projection)) {
                    items.push(projection);
                }
            }
        }
    }
}

/// A drop's type as a word of its name: a type parameter's, `T` of
/// `dropT`, or an associated type's, `IIntoIter` of `dropIIntoIter`.
pub(in crate::lower) fn drop_word<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
    match ty.kind() {
        ty::Param(param) => param_word(param.name),
        // A type built of a function's slots (#737): a tuple's of its
        // parts', `UsizeSelfItem` of `(usize, Self::Item)`; any's a name.
        ty::Tuple(parts) => (parts.iter())
            .map(|part| crate::lower::upper_first(&drop_word(tcx, part)))
            .collect(),
        _ => js_word(&evidence_word(tcx, ty)),
    }
}

/// An associated type's alias, whether rustc has found it rigid or not: what
/// two of them are the same type by.
pub(in crate::lower) fn alias_of(ty: Ty<'_>) -> Option<ty::AliasTy<'_>> {
    match *ty.kind() {
        ty::Alias(_, alias) => Some(alias),
        _ => None,
    }
}

/// Might any value have a destructor: is the crate a library, whose
/// consumers' types might (ADR 0163), has it a `Drop` impl of its own, or
/// does it use a library, whose types might? Where none, what a caller gives
/// generic code without a drop function has nothing to drop (ADR 0106).
pub(in crate::lower) fn may_have_destructors(
    tcx: TyCtxt<'_>,
    (foreign, library): (&crate::lower::library::Foreign<'_, '_>, bool),
) -> bool {
    let drop_trait = tcx.lang_items().drop_trait();
    library || drop_trait.is_some_and(|id| super::declared_impls_of(tcx, id).next().is_some()) || foreign.any()
}

/// An associated type's drop in its impl's dictionary, `$dropOffset`: a
/// name no Rust method can have, as `$drop` is (ADR 0178).
pub(in crate::lower) fn item_drop_key(tcx: TyCtxt<'_>, item: DefId) -> String {
    format!("$drop{}", super::words::item_word(tcx, item))
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The drops a trait method's own type parameters are given, as `method`
    /// takes them, its own drops' places (`own_drop_params`): each, where
    /// any is given one, as its caller gives them all; none, where none is,
    /// as of a crate whose values drop nothing (ADR 0163). Of the impl's
    /// method the dictionary's entry calls, or a default copied into it
    /// (#802).
    pub(in crate::lower) fn own_drops_taken(&self, method: DefId, at: DefId) -> Vec<u32> {
        let foreign = self.krate.foreign;
        match self.krate.drop_params.get(&method) {
            Some(taken)
                if own_drop_params(self.tcx, foreign, method)
                    .iter()
                    .any(|i| taken.contains(i)) =>
            {
                own_drop_params(self.tcx, foreign, at)
            }
            _ => Vec::new(),
        }
    }

    /// What resolves a function's slots (`drop_slot`): its bounds, and the
    /// built slots the crate's analysis and its libraries' manifests list.
    pub(in crate::lower) fn slots(&self) -> (&'a crate::lower::library::Foreign<'a, 'tcx>, &'a BuiltSlots<'tcx>) {
        (self.krate.foreign, self.krate.built_slots)
    }

    /// Whether a call of `id` with `args` gives its slot `index`'s facts:
    /// each but one of no type there, of a default whose where clause the
    /// arguments here can't meet, which nothing called here reads
    /// (`no_type_here`, #828).
    pub(in crate::lower) fn gives_slot(&self, id: DefId, index: u32, args: ty::GenericArgsRef<'tcx>) -> bool {
        !crate::lower::drops::no_type_here(self.tcx, self.typing_env, self.drop_slot_of(id, index, args))
    }

    /// The type `id`'s drop `index` drops, of the arguments it's called with
    /// here: the one given for its type parameter, or that type's associated
    /// type, as it's known here (ADR 0421).
    pub(in crate::lower) fn drop_slot_of(&self, id: DefId, index: u32, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
        let ty = ty::EarlyBinder::bind(self.tcx, drop_slot(self.tcx, self.slots(), id, index))
            .instantiate(self.tcx, args)
            .skip_normalization();
        // In a copied default, in the impl's terms, where its bounds and the
        // default's hold.
        let typing_env = self.typing_env;
        self.tcx
            .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    }

    /// A dictionary's `$drop`, its type's drop where it has one, as Rust's
    /// vtable has it: what dropping a `dyn` of it runs (ADR 0098). A name no
    /// Rust method can have.
    pub(super) fn dictionary_drop(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Prop>> {
        if self.drops(ty) != Drops::Runs {
            return Ok(None);
        }
        let drop = self.drop_function(ty, span)?;
        Ok(drop.map(|drop| Prop::Field(js::DICTIONARY_DROP.into(), drop)))
    }

    /// Rust's built-in `impl Trait for dyn Trait`, of `tr`, the `dyn`'s
    /// principal or a supertrait of it, as generic code is given it: each
    /// method calls the pair's own, as a call on the `dyn` does,
    /// `(object) => object.impl.area(object.value)`, and each supertrait's
    /// dictionary is one of these too.
    /// A dictionary's drops of its associated types, `$dropOffset`, which
    /// generic code holding one of them runs, as it can't name the type
    /// (ADR 0178): the impl's, or what a `dyn` names, `dyn Zone<Offset = T>`.
    /// Of a dictionary that gives what it makes as it is, `as_it_is`, std's
    /// `IntoIterator` of a type known here (ADR 0552), the value itself.
    pub(super) fn item_drops(&mut self, tr: ty::TraitRef<'tcx>, as_it_is: bool, span: Span) -> R<Vec<Prop>> {
        let mut props = Vec::new();
        for (item, item_ty) in self.item_types(tr) {
            // Of one generic code has as the JS iterator the dictionary made
            // of it, what it was made of (#978).
            let iterated = !as_it_is && self.item_made_js_iterator(item, item_ty);
            if self.drops(item_ty) == Drops::Runs
                && let Some(drop) = self.held_drop_function(item_ty, iterated, span)?
            {
                props.push(Prop::Field(item_drop_key(self.tcx, item), drop));
            }
        }
        Ok(props)
    }
}
