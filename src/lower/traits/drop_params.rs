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
    let count = tcx.generics_of(method).count() as u32;
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
        indices.extend(place.map(|place| count + place as u32));
    }
    indices
}

/// The type a function's drop `index` drops, in its own terms: its type
/// parameter's, `T`, below its generics' count, and from there each
/// associated type of a type parameter's bound by a trait no dictionary
/// carries, a std trait's, `<I as IntoIterator>::IntoIter`, in its bounds'
/// order (ADR 0421). Its callers know its bounds, so they and it agree, a
/// library's consumers too. One a bound names, `IntoIterator<Item = T>`,
/// is that type's, and isn't one.
pub(in crate::lower) fn drop_slot<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    id: DefId,
    index: u32,
) -> Ty<'tcx> {
    let generics = tcx.generics_of(id);
    match index.checked_sub(generics.count() as u32) {
        None => {
            let param = generics.param_at(index as usize, tcx);
            Ty::new_param(tcx, param.index, param.name)
        }
        Some(at) => drop_items(tcx, foreign, id)[at as usize],
    }
}

/// The associated types of a function's type parameters it may be given a
/// drop for, after its type parameters' (`drop_slot`); and of those a bound
/// names, `<I::Item as IntoIterator>::IntoIter` of an `I::Item: IntoIterator`
/// (ADR 0478), each found in turn.
pub(in crate::lower) fn drop_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    id: DefId,
) -> Vec<Ty<'tcx>> {
    let typing_env = ty::TypingEnv::non_body_analysis(tcx, id);
    let mut items: Vec<Ty<'tcx>> = Vec::new();
    loop {
        let found = items.len();
        drop_items_of(tcx, foreign, (id, typing_env), &mut items);
        if items.len() == found {
            return items;
        }
    }
}

/// `drop_items`' of each bound on a type parameter, or on one of `items`.
fn drop_items_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    (id, typing_env): (DefId, ty::TypingEnv<'tcx>),
    items: &mut Vec<Ty<'tcx>>,
) {
    for (clause, _) in tcx.clauses_of(id).instantiate_identity(tcx) {
        let Some(bound) = clause
            .skip_normalization()
            .as_trait_clause()
            .and_then(|b| b.no_bound_vars())
        else {
            continue;
        };
        let self_ty = bound.self_ty();
        let slotted = matches!(self_ty.kind(), ty::Param(_))
            || (alias_of(self_ty).is_some() && items.iter().any(|&t| alias_of(t) == alias_of(self_ty)));
        if !slotted || has_dictionary_of(tcx, foreign, bound.def_id(), self_ty) {
            continue;
        }
        for tr in supertrait_def_ids(tcx, bound.def_id()) {
            // A supertrait's of no parameter but `Self`, whose arguments are its.
            let args = match tr == bound.def_id() {
                true => bound.trait_ref.args,
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
        _ => evidence_word(tcx, ty),
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

/// Might any value have a destructor: has the crate a `Drop` impl of its own,
/// or a library, whose types might? Where it hasn't, what a caller gives
/// generic code without a drop function has nothing to drop (ADR 0106).
pub(in crate::lower) fn may_have_destructors(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
) -> bool {
    let drop_trait = tcx.lang_items().drop_trait();
    drop_trait.is_some_and(|id| tcx.all_local_trait_impls(()).contains_key(&id)) || foreign.any()
}

/// An associated type's drop in its impl's dictionary, `$dropOffset`: a
/// name no Rust method can have, as `$drop` is (ADR 0178).
pub(in crate::lower) fn item_drop_key(tcx: TyCtxt<'_>, item: DefId) -> String {
    format!("$drop{}", tcx.item_name(item))
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The type `id`'s drop `index` drops, of the arguments it's called with
    /// here: the one given for its type parameter, or that type's associated
    /// type, as it's known here (ADR 0421).
    pub(in crate::lower) fn drop_slot_of(&self, id: DefId, index: u32, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
        let ty = ty::EarlyBinder::bind(self.tcx, drop_slot(self.tcx, self.krate.foreign, id, index))
            .instantiate(self.tcx, args)
            .skip_normalization();
        // In a copied default, in the impl's terms, where its bounds and the
        // default's hold.
        let typing_env = self.impl_typing_env().unwrap_or(self.typing_env);
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
    pub(super) fn item_drops(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
        let mut props = Vec::new();
        for (item, item_ty) in self.item_types(tr) {
            if self.drops(item_ty) == Drops::Runs
                && let Some(drop) = self.drop_function(item_ty, span)?
            {
                props.push(Prop::Field(item_drop_key(self.tcx, item), drop));
            }
        }
        Ok(props)
    }
}
