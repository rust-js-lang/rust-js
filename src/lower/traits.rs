//! Trait evidence stays separate from payloads: arguments for generics,
//! lazy dictionaries for impls, and `{ value, impl }` for trait objects.

use super::bindings;
use super::display::Pretty;
use super::drops::Drops;
use super::recognition::{
    Recognition, Std, StdItem, TraitCall, TypeFact, any_trait, error_default, in_std_dictionary,
    into_iter_by_dictionary, io_write_default, is_clone_from, is_hasher, is_std_def, is_std_method, is_writer_default,
    known_derive, std_item, tracks_caller,
};
use super::representation::{Num, const_js, eval_const};
use super::{FnCx, R, lower_first};
use crate::js::{self, Expr, Op, Prop, StmtKind};
use rustc_hir::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::{BinOp, UnOp};
use rustc_middle::traits::{BuiltinImplSource, ImplSource};
use rustc_middle::ty::elaborate::supertrait_def_ids;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeFoldable, TypeVisitableExt, Upcast};
use rustc_span::def_id::DefId;
use rustc_span::{Span, Symbol};
use std::collections::{HashMap, HashSet};

/// A trait whose bounds take dictionaries (ADR 0049): the crate's own, and
/// the std ones rust-js has dictionaries for (ADR 0052).
pub(super) use super::recognition::operational;

/// A trait the crate may implement. `From` and `TryFrom` have no
/// dictionaries: their impls are only called where the types are known
/// (ADR 0052). `Eq` has no
/// methods: a `T: Eq` bound is its `PartialEq` (ADR 0053). An `Iterator` is a
/// JS iterator, and has no dictionaries either (ADR 0055). An operator's
/// impl is called where `a + b` is, with the types known (ADR 0064).
pub(super) use super::recognition::implementable;

/// `Add`, `Neg`, `AddAssign` and the like: what `a + b`, `-a` and
/// `a += b` call on a type of the crate's own.
pub(super) use super::recognition::is_operator;

pub(super) fn validate(tcx: TyCtxt<'_>, foreign: &super::library::Foreign<'_, '_>) -> bool {
    let mut valid = true;
    for id in tcx.hir_crate_items(()).definitions() {
        if bindings::is_binding(tcx, id.to_def_id()) {
            continue;
        }
        let kind = tcx.def_kind(id);
        if !matches!(
            kind,
            DefKind::Trait | DefKind::Fn | DefKind::AssocFn | DefKind::Impl { .. }
        ) {
            continue;
        }
        // A derived impl's methods are never lowered: `#[derive(Hash)]`'s
        // generic `hash<H>` is no reason to reject the crate.
        let derived = |id: rustc_span::def_id::LocalDefId| known_derive(tcx, id.to_def_id());
        if derived(id)
            || (kind == DefKind::AssocFn && tcx.opt_local_parent(id).is_some_and(derived))
            || super::analysis::from_serde_derive(tcx, id)
        {
            continue;
        }
        if kind == DefKind::Trait {
            let mut names = std::collections::HashSet::new();
            let identity = ty::GenericArgs::identity_for_item(tcx, id);
            for (name, tr, span) in supertraits(tcx, id.to_def_id(), identity) {
                if operational(tcx, foreign, tr.def_id) && (name == "__proto__" || !names.insert(name)) {
                    tcx.dcx().span_err(span, "rust-js: supertrait dictionary names collide");
                    valid = false;
                }
            }
            for item in tcx.associated_items(id).in_definition_order() {
                // The type rustc makes of an `async fn`'s future has no name,
                // and no place in a dictionary; nor has an associated type.
                if item.is_impl_trait_in_trait() || tcx.def_kind(item.def_id) == DefKind::AssocTy {
                    continue;
                }
                let name = bindings::fn_name(tcx, item.def_id);
                if name == "__proto__" || !names.insert(name) {
                    tcx.dcx().span_err(
                        tcx.def_span(item.def_id),
                        "rust-js: trait dictionary names collide or use reserved `__proto__`",
                    );
                    valid = false;
                }
            }
            for (name, _) in item_bounds(tcx, id.to_def_id(), identity) {
                if !names.insert(name) {
                    tcx.dcx()
                        .span_err(tcx.def_span(id), "rust-js: trait dictionary names collide");
                    valid = false;
                }
            }
        }
    }
    valid
}

/// `id`'s const parameters, its parent's first, as rustc numbers them.
/// The indexes of `id`'s own const parameters, not its parent's: a trait
/// method's, which a caller through a dictionary gives (ADR 0135).
fn own_const_params(tcx: TyCtxt<'_>, id: DefId) -> Vec<u32> {
    tcx.generics_of(id)
        .own_params
        .iter()
        .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Const { .. }))
        .map(|param| param.index)
        .collect()
}

fn const_params(tcx: TyCtxt<'_>, id: DefId) -> Vec<&ty::GenericParamDef> {
    let generics = tcx.generics_of(id);
    (0..generics.count())
        .map(|index| generics.param_at(index, tcx))
        .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Const { .. }))
        .collect()
}

/// Signature order, including parent impl bounds. Never depend on body usage.
/// A component has none: React calls it with its props, never a dictionary
/// (ADR 0201).
/// What the crate's analyses say of its type parameters' bounds: those
/// whose `Copy` takes a copy function (ADR 0289), and those whose
/// `Iterator`, or an extension of it, is given its dictionary (ADRs 0411,
/// 0487), by their indices, with the trait's.
#[derive(Clone, Copy)]
pub(super) struct Given<'a> {
    pub copied: &'a HashSet<(DefId, u32)>,
    pub stepped: &'a HashSet<(DefId, u32, DefId)>,
}

/// The impl whose dictionary is the evidence of `tr`, and its arguments: a
/// user's, the crate's or another's, as `codegen_select_candidate` selects
/// it, where `tr` isn't a built-in one, a `dyn`'s.
pub(super) fn evidence_impl<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    tr: ty::TraitRef<'tcx>,
) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
    match tcx.codegen_select_candidate(typing_env.as_query_input(tr)) {
        Ok(ImplSource::UserDefined(imp)) => Some((imp.impl_def_id, imp.args)),
        _ => None,
    }
}

pub(super) fn bounds<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    given: Given<'_>,
    id: DefId,
) -> Vec<ty::TraitRef<'tcx>> {
    let mut result = Vec::new();
    if bindings::is_component(tcx, id) {
        return result;
    }
    if let Some(trait_id) = tcx.trait_of_assoc(id)
        && operational(tcx, foreign, trait_id)
    {
        result.push(ty::TraitRef::identity(tcx, trait_id));
    }
    for (clause, _) in tcx.clauses_of(id).instantiate_identity(tcx) {
        for tr in bounds_of(tcx, foreign, given, clause.skip_normalization(), id) {
            if !result.contains(&tr) {
                result.push(tr);
            }
        }
    }
    result
}

/// A clause's bounds that are given evidence: its own, `bound_of`'s, after
/// the `Iterator` an iterator's extension on a type parameter given as it
/// is implies, `I: DoubleEndedIterator`'s, as its supertrait (ADR 0488).
fn bounds_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    given: Given<'_>,
    clause: ty::Clause<'tcx>,
    id: DefId,
) -> impl Iterator<Item = ty::TraitRef<'tcx>> {
    let implied = clause.as_trait_clause().and_then(|p| {
        let tr = p.skip_binder().trait_ref;
        if !super::recognition::is_iterator_extension(tcx, tr.def_id) {
            return None;
        }
        let iterator = ty::TraitRef::new(tcx, std_item(tcx, StdItem::Iterator), [tr.self_ty()]);
        bound_of(tcx, foreign, given, ty::Binder::dummy(iterator).upcast(tcx), id)
    });
    implied.into_iter().chain(bound_of(tcx, foreign, given, clause, id))
}

/// Whether `param`, of the generics `typing_env` is of, is a type its caller
/// gives a drop for: one that isn't `Copy` there (ADR 0163). The one test,
/// of a function's, an impl's or a trait's parameters alike.
pub(super) fn given_a_drop<'tcx>(
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
pub(super) fn own_drop_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
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
pub(super) fn drop_slot<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
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
pub(super) fn drop_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
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
    foreign: &super::library::Foreign<'_, 'tcx>,
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
pub(super) fn drop_word<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
    match ty.kind() {
        ty::Param(param) => param_word(param.name),
        _ => evidence_word(tcx, ty),
    }
}

/// An associated type's alias, whether rustc has found it rigid or not: what
/// two of them are the same type by.
pub(super) fn alias_of(ty: Ty<'_>) -> Option<ty::AliasTy<'_>> {
    match *ty.kind() {
        ty::Alias(_, alias) => Some(alias),
        _ => None,
    }
}

/// Might any value have a destructor: has the crate a `Drop` impl of its own,
/// or a library, whose types might? Where it hasn't, what a caller gives
/// generic code without a drop function has nothing to drop (ADR 0106).
pub(super) fn may_have_destructors(tcx: TyCtxt<'_>, foreign: &super::library::Foreign<'_, '_>) -> bool {
    let drop_trait = tcx.lang_items().drop_trait();
    drop_trait.is_some_and(|id| tcx.all_local_trait_impls(()).contains_key(&id)) || foreign.any()
}

/// A clause of a function's, as its evidence is for it, if it's given one.
fn bound_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    given: Given<'_>,
    clause: ty::Clause<'tcx>,
    id: DefId,
) -> Option<ty::TraitRef<'tcx>> {
    // A higher-ranked bound, `for<'a> T: Foo<'a>`, is one dictionary:
    // lifetimes aren't in the JS, so its own are erased, not left bound.
    let ty::ClauseKind::Trait(predicate) = tcx.instantiate_bound_regions_with_erased(clause.kind()) else {
        return None;
    };
    let mut tr = predicate.trait_ref;
    // `Eq` promises more than `PartialEq`, but it's `PartialEq`'s `eq`
    // that's called.
    if is_std_def(tcx, tr.def_id, StdItem::Eq) {
        let partial_eq = tcx.require_lang_item(LangItem::PartialEq, tcx.def_span(id));
        tr = ty::TraitRef::new(tcx, partial_eq, [tr.self_ty(), tr.self_ty()]);
    }
    // A `Copy` bound only the crate's callers give, none of them a value
    // whose copy isn't itself, takes no copy function (ADR 0289).
    if tcx.is_lang_item(tr.def_id, LangItem::Copy)
        && super::analysis::copies_by_callers(tcx, id)
        && let ty::Param(param) = tr.self_ty().kind()
        && !given.copied.contains(&(tcx.typeck_root_def_id(id), param.index))
    {
        return None;
    }
    // An `Iterator` a type holds or a function gives back: its dictionary,
    // which steps it as it is (ADR 0411), and its extensions' (ADR 0487).
    if (is_std_def(tcx, tr.def_id, StdItem::Iterator) || super::recognition::is_iterator_extension(tcx, tr.def_id))
        && let ty::Param(param) = tr.self_ty().kind()
    {
        return given.stepped.contains(&(id, param.index, tr.def_id)).then_some(tr);
    }
    (has_dictionary_of(tcx, foreign, tr.def_id, tr.self_ty()) && !is_marker(tcx, foreign, tr.def_id)).then_some(tr)
}

/// Whether a bound of `trait_id` on `self_ty` is given a dictionary: one of
/// a trait that has them, but an `IntoIterator` on a type parameter, which
/// is given the JS iterator itself, where one on a reference to one, `for<'a>
/// &'a mut C: IntoIterator`, is given its dictionary (ADRs 0061, 0457).
pub(super) fn has_dictionary_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    trait_id: DefId,
    self_ty: Ty<'tcx>,
) -> bool {
    operational(tcx, foreign, trait_id)
        && (!is_std_def(tcx, trait_id, StdItem::IntoIterator) || into_iter_by_dictionary(tcx, trait_id, self_ty))
}

/// A trait of the crate's or a library's with nothing in it: no items, and
/// each supertrait one too, or one of no dictionary, `Send`. A bound of it
/// passes no dictionary, as nothing would read it: `named<T: Marker>(value)`
/// is `named(value)`. A `dyn` of it still carries its impl's, whose `$drop`
/// drops it (ADR 0098). `Copy` isn't one: its dictionary copies.
pub(super) fn is_marker(tcx: TyCtxt<'_>, foreign: &super::library::Foreign<'_, '_>, id: DefId) -> bool {
    (id.is_local() || foreign.in_library(id))
        && (tcx.associated_items(id).in_definition_order()).all(|item| bindings::is_binding(tcx, item.def_id))
        && supertraits(tcx, id, ty::GenericArgs::identity_for_item(tcx, id))
            .iter()
            .all(|(_, tr, _)| !operational(tcx, foreign, tr.def_id) || is_marker(tcx, foreign, tr.def_id))
}

/// The bounds of a function's own type parameters, which end its `bounds`:
/// a trait's generic method's, `T: Display` of `describe<T: Display>`, which
/// a caller through a dictionary gives where it calls, after its arguments,
/// in the trait's order, where an impl's are given when its dictionary is
/// made (ADR 0106).
pub(super) fn own_bounds<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    given: Given<'_>,
    id: DefId,
) -> Vec<ty::TraitRef<'tcx>> {
    let own: Vec<_> = tcx
        .clauses_of(id)
        .clauses
        .iter()
        .flat_map(|&(clause, _)| bounds_of(tcx, foreign, given, clause, id))
        .collect();
    bounds(tcx, foreign, given, id)
        .into_iter()
        .filter(|tr| own.contains(tr))
        .collect()
}

/// The type, then the trait, then the trait's arguments other than their
/// defaults: `circleShape`, `metersFromF64` for `impl From<f64> for
/// Meters`, and `versionPartialEq`, whose `Rhs` is `Self` (ADR 0052).
pub(super) fn impl_name(tcx: TyCtxt<'_>, id: DefId) -> String {
    let named = |id: DefId, full: bool, module: &str| {
        let tr = tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
        if !full {
            return format!(
                "{}{module}{}",
                lower_first(&js_word(&type_word(tcx, tr.self_ty()))),
                trait_word(tcx, tr)
            );
        }
        let mut name = lower_first(&js_word(&full_type_word(tcx, tr.self_ty())));
        name.push_str(module);
        name.push_str(tcx.item_name(tr.def_id).as_str());
        for arg in tr.args.iter().skip(1) {
            // A const argument by its value, `Scaled10` of `Scaled<10>` (ADR 0135).
            let word = match (arg.as_type(), arg.as_const()) {
                (Some(ty), _) => js_word(&full_type_word(tcx, ty)),
                (None, Some(c)) => js_word(&c.to_string()),
                (None, None) => continue,
            };
            let mut chars = word.chars();
            name.extend(chars.next().map(|c| c.to_ascii_uppercase()));
            name.extend(chars);
        }
        name
    };
    let tr = tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
    // Two traits of one name for one type, serde's `de::Error` and
    // `ser::Error`: each named with its trait's module too, `errorDeError`.
    let short = named(id, false, "");
    let clash = tcx
        .trait_impls_in_crate(id.krate)
        .iter()
        .any(|&other| tcx.impl_trait_id(other) != tr.def_id && named(other, false, "") == short);
    let module = match clash {
        true => {
            // A crate root's is its path, the crate's name, or `crate`.
            let parent = tcx.parent(tr.def_id);
            tcx.opt_item_name(parent)
                .map_or_else(|| tcx.def_path_str(parent), |name| name.to_string())
                .split('_')
                .flat_map(|part| {
                    let mut chars = part.chars();
                    chars.next().map(|c| c.to_ascii_uppercase()).into_iter().chain(chars)
                })
                .collect()
        }
        false => String::new(),
    };
    let name = named(id, false, &module);
    // Two impls of a trait for one type with other arguments, `Vec<i32>`'s and
    // `Vec<String>`'s: each named with the arguments, its type's and its
    // trait's, `vecI32Describe`; and, two types of one name, local to two
    // functions, numbered in the order they're declared.
    let mut alike: Vec<DefId> = tcx
        .all_impls(tr.def_id)
        .filter(|&other| other.krate == id.krate && named(other, false, &module) == name)
        .collect();
    if alike.len() < 2 {
        return name;
    }
    let full = named(id, true, &module);
    alike.retain(|&other| named(other, true, &module) == full);
    alike.sort_by_key(|other| other.index);
    match alike.iter().position(|&other| other == id) {
        Some(at) if at > 0 => format!("{full}{}", at + 1),
        _ => full,
    }
}

/// `type_word`, with the type's arguments other than their defaults:
/// `VecI32` of `Vec<i32>`, whose allocator is the default.
fn full_type_word<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
    match ty.kind() {
        ty::Adt(adt, args) => {
            let mut name = tcx.item_name(adt.did()).to_string();
            let generics = tcx.generics_of(adt.did());
            for (param, arg) in generics.own_params.iter().zip(args.iter()) {
                let Some(arg) = arg.as_type() else { continue };
                if param
                    .default_value(tcx)
                    .map(|d| d.instantiate(tcx, args).skip_normalization())
                    == Some(arg.into())
                {
                    continue;
                }
                let word = js_word(&full_type_word(tcx, arg));
                let mut chars = word.chars();
                name.extend(chars.next().map(|c| c.to_ascii_uppercase()));
                name.extend(chars);
            }
            name
        }
        _ => type_word(tcx, ty),
    }
}

/// A trait's supertraits, as its dictionary has them: each one's key, its
/// trait's name, `PartialEq`, or with its arguments as the trait declares
/// them where it has two of one trait, `LabelU32` and `LabelString` of
/// `trait Both: Label<u32> + Label<String>`, so that a generic impl's
/// dictionary and its caller agree; and the supertrait with
/// `args`, its own lifetimes erased: a `for<'a> B<&'a ()>` is one
/// dictionary, and rustc's trait selection takes no bound ones (ADR 0106).
pub(super) fn supertraits<'tcx>(
    tcx: TyCtxt<'tcx>,
    trait_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Vec<(String, ty::TraitRef<'tcx>, Span)> {
    let clauses = tcx.explicit_super_clauses_of(trait_id);
    let found: Vec<_> = clauses
        .iter_identity_copied()
        .map(|item| item.skip_normalization())
        .zip(
            clauses
                .iter_instantiated_copied(tcx, args)
                .map(|item| item.skip_normalization()),
        )
        .filter_map(|((declared, span), (instantiated, _))| {
            let ty::ClauseKind::Trait(declared) = declared.kind().skip_binder() else {
                return None;
            };
            let ty::ClauseKind::Trait(instantiated) = tcx.instantiate_bound_regions_with_erased(instantiated.kind())
            else {
                return None;
            };
            Some((declared.trait_ref, instantiated.trait_ref, span))
        })
        .collect();
    let twice = |id: DefId| found.iter().filter(|(declared, _, _)| declared.def_id == id).count() > 1;
    // Named by its arguments too where it's there twice, and by their
    // references as well where they alone tell them apart: `AddBase` and
    // `AddRefBase` of `Add<Base> + for<'r> Add<&'r Base>`.
    let alike = |tr: ty::TraitRef<'tcx>| {
        found
            .iter()
            .filter(|(declared, _, _)| trait_word(tcx, *declared) == trait_word(tcx, tr))
            .count()
            > 1
    };
    found
        .iter()
        .map(|&(declared, instantiated, span)| {
            let name = match (twice(declared.def_id), alike(declared)) {
                (true, true) => trait_word_with_refs(tcx, declared, true),
                (true, false) => trait_word(tcx, declared),
                (false, _) => tcx.item_name(declared.def_id).to_string(),
            };
            (name, instantiated, span)
        })
        .collect()
}

/// The bounds a trait declares on its associated types, as its dictionary
/// has them: each one's key, the type's name and the bound's as the trait
/// declares it, `LabelDisplay` of `type Label: Display`, and the bound with
/// `args`, the trait's. Generic code finds `<L as Labeled>::Label: Display`
/// in `L`'s `Labeled`, as rustc proves it from the trait (ADR 0106).
pub(super) fn item_bounds<'tcx>(
    tcx: TyCtxt<'tcx>,
    trait_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Vec<(String, ty::TraitRef<'tcx>)> {
    let mut found = Vec::new();
    for item in tcx.associated_items(trait_id).in_definition_order() {
        if tcx.def_kind(item.def_id) != DefKind::AssocTy || item.is_impl_trait_in_trait() {
            continue;
        }
        // A generic associated type's own lifetimes are erased, as JS has
        // none: one dictionary for every `Iter<'a>` (ADR 0146). One with other
        // parameters of its own has no bounds (`gat_supported`).
        let own = &tcx.generics_of(item.def_id).own_params;
        if own
            .iter()
            .any(|param| !matches!(param.kind, ty::GenericParamDefKind::Lifetime))
        {
            continue;
        }
        let item_args = ty::GenericArgs::for_item(tcx, item.def_id, |param, _| match args.get(param.index as usize) {
            Some(&arg) => arg,
            None => tcx.lifetimes.re_erased.into(),
        });
        let bounds = tcx.explicit_item_bounds(item.def_id);
        for ((declared, _), (instantiated, _)) in
            bounds.iter_identity_copied().map(|item| item.skip_normalization()).zip(
                bounds
                    .iter_instantiated_copied(tcx, item_args)
                    .map(|item| item.skip_normalization()),
            )
        {
            let ty::ClauseKind::Trait(declared) = declared.kind().skip_binder() else {
                continue;
            };
            let ty::ClauseKind::Trait(instantiated) = tcx.instantiate_bound_regions_with_erased(instantiated.kind())
            else {
                continue;
            };
            let name = format!("{}{}", tcx.item_name(item.def_id), trait_word(tcx, declared.trait_ref));
            found.push((name, instantiated.trait_ref));
        }
    }
    found
}

/// Can rust-js have `def_id`, a generic associated type (ADR 0146)? Its own
/// lifetimes are erased, and its own types and constants a dictionary entry
/// would vary by: one of those with no bound has none.
pub(super) fn gat_supported(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    // An impl's is its trait's, which has the bounds.
    let def_id = tcx.trait_item_of(def_id).unwrap_or(def_id);
    let own = &tcx.generics_of(def_id).own_params;
    own.iter()
        .all(|param| matches!(param.kind, ty::GenericParamDefKind::Lifetime))
        || tcx
            .explicit_item_bounds(def_id)
            .iter_identity_copied()
            .map(|item| item.skip_normalization())
            .all(|(clause, _)| match clause.kind().skip_binder() {
                ty::ClauseKind::Trait(bound) => tcx.is_lang_item(bound.def_id(), LangItem::Sized),
                _ => true,
            })
}

/// Where a dictionary is found, before its JS is built: one a function
/// was given, or an associated type's of one, then its supertraits'
/// accessors, `SShape.Named()`.
#[derive(Clone)]
struct Route {
    root: Root,
    supers: Vec<String>,
}

#[derive(Clone)]
enum Root {
    /// The function's given dictionary at this index.
    Given(usize),
    /// An associated type's, `LabelDisplay`, of its trait's dictionary.
    Item(Box<Route>, String),
}

/// Where a function's dictionaries are, by the bounds they're given for:
/// a question of types (ADR 0178), which what a type drops asks. Their JS,
/// `given`'s values, only `FnCx::route_expr` reads.
pub(in crate::lower) struct EvidenceQuery<'a, 'tcx> {
    recognition: Recognition<'a, 'tcx>,
    given: &'a [(ty::TraitRef<'tcx>, Expr)],
}

impl<'a, 'tcx> EvidenceQuery<'a, 'tcx> {
    /// A query given no dictionaries: of a body's own types, whose type
    /// parameters are then of no dictionary known (ADR 0362).
    pub(in crate::lower) fn without_evidence(recognition: Recognition<'a, 'tcx>) -> Self {
        EvidenceQuery {
            recognition,
            given: &[],
        }
    }

    /// The supertraits' accessors that lead from `from`'s dictionary to
    /// `to`'s, none if they're one.
    pub(super) fn super_route(&self, from: ty::TraitRef<'tcx>, to: ty::TraitRef<'tcx>) -> Option<Vec<String>> {
        // As rustc says what they are here: `<I as Int>::T: NonZero` is
        // `J: NonZero` of an `I: Int<T = J>`.
        let normalized = |tr: ty::TraitRef<'tcx>| {
            self.recognition
                .tcx
                .try_normalize_erasing_regions(self.recognition.typing_env, ty::Unnormalized::new_wip(tr))
                .unwrap_or_else(|_| self.recognition.tcx.erase_and_anonymize_regions(tr))
        };
        if normalized(from) == normalized(to) {
            return Some(Vec::new());
        }
        // A std trait's dictionary, like `Copy`'s, has no supertraits in it,
        // but `Error`'s has its `Display` and `Debug` (ADR 0141), and
        // `DerefMut`'s its `Deref`.
        if !self.recognition.holds_supertraits(from.def_id) {
            return None;
        }
        supertraits(self.recognition.tcx, from.def_id, from.args)
            .into_iter()
            .find_map(|(name, tr, _)| {
                let mut rest = self.super_route(tr, to)?;
                rest.insert(0, name);
                Some(rest)
            })
    }

    /// Where the dictionary for `tr` is, among those this function was
    /// given, or a supertrait's of one: a question of types alone, which
    /// what's dropped asks (ADR 0178) without building its JS.
    fn evidence_route(&self, tr: ty::TraitRef<'tcx>) -> Option<Route> {
        self.given
            .iter()
            .enumerate()
            .find_map(|(index, (bound, _))| {
                let supers = self.super_route(*bound, tr)?;
                Some(Route {
                    root: Root::Given(index),
                    supers,
                })
            })
            .or_else(|| self.item_route(tr))
    }

    /// `<L as Labeled>::Label: Display`, which the trait declares, from `L`'s
    /// `Labeled`: its `LabelDisplay`, or a supertrait's of it (ADR 0106).
    fn item_route(&self, tr: ty::TraitRef<'tcx>) -> Option<Route> {
        let ty::Alias(
            _,
            alias @ ty::AliasTy {
                kind: ty::Projection { .. },
                ..
            },
        ) = *tr.self_ty().kind()
        else {
            return None;
        };
        let owner = alias.trait_ref(self.recognition.tcx);
        let dictionary = self.evidence_route(owner)?;
        item_bounds(self.recognition.tcx, owner.def_id, owner.args)
            .into_iter()
            .find_map(|(name, bound)| {
                let supers = self.super_route(bound, tr)?;
                Some(Route {
                    root: Root::Item(Box::new(dictionary.clone()), name),
                    supers,
                })
            })
    }

    /// Is a dictionary for `tr` given? Without building it.
    pub(in crate::lower) fn has_evidence(&self, tr: ty::TraitRef<'tcx>) -> bool {
        self.evidence_route(tr).is_some()
    }
}

/// Whether std's trait `trait_id`'s call of `self_ty` is written in place,
/// as recognition lowers it, where a dictionary is for generic code (ADR
/// 0180). A writer's where its type is known, `known_here`, as it may write
/// to one it holds: `write!(c, ..)`'s own `write_str`, a string's `+=`
/// (ADRs 0148, 0166), an `io::Write`'s bytes (ADR 0392). A `Deref`'s, a
/// guard's value or a `Cow`'s text, and a collection's `extend` and
/// `from_iter`, of any items, where its own type is known, whatever it
/// holds, a `MutexGuard<C>`'s (ADRs 0394, 0445, 0450). An `IntoIterator`'s
/// but one of a reference to a type only a caller knows (ADR 0457). A
/// hasher's, and a `Hash`'s, where its types are known: a std hasher's own
/// method, a crate's hasher's provided one, a std type's writes (ADR 0463).
fn std_written_in_place(tcx: TyCtxt<'_>, trait_id: DefId, self_ty: Ty<'_>, known_here: bool) -> bool {
    let writer = is_std_def(tcx, trait_id, StdItem::FmtWrite)
        || is_std_def(tcx, trait_id, StdItem::IoWrite)
        || super::recognition::is_hashing(tcx, trait_id);
    let holder = tcx.is_lang_item(trait_id, LangItem::Deref)
        || tcx.is_lang_item(trait_id, LangItem::DerefMut)
        || super::recognition::is_extend(tcx, trait_id)
        || is_std_def(tcx, trait_id, StdItem::FromIterator);
    let into_iter =
        is_std_def(tcx, trait_id, StdItem::IntoIterator) && !into_iter_by_dictionary(tcx, trait_id, self_ty);
    (writer && known_here)
        || (holder && !matches!(self_ty.peel_refs().kind(), ty::Param(_) | ty::Alias(..)))
        || into_iter
}

/// Whether only generic code reads a dictionary of `trait_id`'s: an
/// operator's, `a + b` and `a += b` (ADRs 0108, 0396), a `Deref`'s (ADR
/// 0394), an `Extend`'s and a `FromIterator`'s (ADR 0445), an `AsMut`'s
/// (ADR 0162), an `IntoIterator`'s (ADR 0457), a `Hash`'s and a
/// `Hasher`'s (ADR 0463), a `Sum`'s and a `Product`'s (ADR 0476), a
/// `From`'s (ADR 0477), and a `BorrowMut`'s (ADR 0483). Where the
/// type is known, the impl's method is called as it is, so a crate's impl's
/// dictionary is written only where it's reached.
pub(super) fn read_only_generically(tcx: TyCtxt<'_>, trait_id: DefId) -> bool {
    super::recognition::value_operator(tcx, trait_id).is_some()
        || super::recognition::assign_operator(tcx, trait_id).is_some()
        || tcx.is_lang_item(trait_id, LangItem::Deref)
        || tcx.is_lang_item(trait_id, LangItem::DerefMut)
        || super::recognition::is_extend(tcx, trait_id)
        || is_std_def(tcx, trait_id, StdItem::FromIterator)
        || is_std_def(tcx, trait_id, StdItem::AsMut)
        || is_std_def(tcx, trait_id, StdItem::BorrowMut)
        || is_std_def(tcx, trait_id, StdItem::IntoIterator)
        || super::recognition::is_hashing(tcx, trait_id)
        || super::recognition::is_sum_or_product(tcx, trait_id)
        || is_std_def(tcx, trait_id, StdItem::From)
}

/// An associated type's drop in its impl's dictionary, `$dropOffset`: a
/// name no Rust method can have, as `$drop` is (ADR 0178).
pub(super) fn item_drop_key(tcx: TyCtxt<'_>, item: DefId) -> String {
    format!("$drop{}", tcx.item_name(item))
}

/// The key a dictionary has a fact of its associated type `item` under,
/// `ItemSize` (ADR 0145).
pub(super) fn item_fact_key(tcx: TyCtxt<'_>, item: DefId, fact: TypeFact) -> String {
    format!("{}{}", tcx.item_name(item), fact.word())
}

/// A type as a word of an evidence name: a type parameter's, `T`, or an
/// associated type's of one, `SItem` of `<S as Source>::Item`.
fn evidence_word<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
    match ty.kind() {
        ty::Alias(
            _,
            alias @ ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            },
        ) => {
            format!("{}{}", evidence_word(tcx, alias.self_ty()), tcx.item_name(*def_id))
        }
        // `name: impl Into<String>`'s, which rustc names as it's written: its
        // trait's word alone, `IntoString`.
        ty::Param(p) if p.name.as_str().starts_with("impl ") => String::new(),
        // An array's and a slice's, of their item's: `TArray` of `[T; N]`,
        // as a where clause's bound of one is `TArrayDefault`.
        ty::Array(item, _) => format!("{}Array", evidence_word(tcx, *item)),
        ty::Slice(item) => format!("{}Slice", evidence_word(tcx, *item)),
        // A reference's, of what it's to: `CRefMut` of `&'a mut C`, as a
        // bound on one, `CRefMutIntoIterator` (ADR 0457).
        ty::Ref(_, inner, mutability) => {
            let kind = if mutability.is_mut() { "RefMut" } else { "Ref" };
            format!("{}{kind}", evidence_word(tcx, *inner))
        }
        // A struct's or an enum's own name, `String`, not its path.
        ty::Adt(..) => type_word(tcx, ty),
        // A derive's own, `__H` of `#[derive(Hash)]`'s `hash<__H: Hasher>`:
        // `H`, as it's written by hand (ADR 0463).
        ty::Param(p) if p.name.as_str().starts_with('_') && !p.name.as_str().trim_start_matches('_').is_empty() => {
            p.name.as_str().trim_start_matches('_').to_string()
        }
        _ => ty.to_string(),
    }
}

/// A type as a word of a JS name: an ADT's own name, `Meters` of
/// `Meters<T>`; a reference's, `RefFlags` of `&'a Flags` and `RefMutI32`
/// of `&mut i32`, with no lifetime, which isn't in the JS.
fn type_word<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
    match ty.kind() {
        ty::Adt(adt, _) => tcx.item_name(adt.did()).to_string(),
        ty::Ref(_, inner, mutability) => {
            let inner = js_word(&type_word(tcx, *inner));
            let mut chars = inner.chars();
            let first = chars.next().map(|c| c.to_ascii_uppercase()).unwrap_or_default();
            let kind = if mutability.is_mut() { "RefMut" } else { "Ref" };
            format!("{kind}{first}{}", chars.as_str())
        }
        // An associated type's, `BItem` of `<B as Array>::Item`.
        ty::Alias(..) => evidence_word(tcx, ty),
        // A slice's, its item's, `U32Slice` of `[u32]`.
        ty::Slice(item) => {
            let item = js_word(&type_word(tcx, *item));
            let mut chars = item.chars();
            let first = chars.next().map(|c| c.to_ascii_uppercase()).unwrap_or_default();
            format!("{first}{}Slice", chars.as_str())
        }
        _ => ty.to_string(),
    }
}

/// The trait and its arguments other than their defaults, as a word of a JS
/// name: `ConvertF64` of `Convert<f64>`, `PartialEq` of `PartialEq<Self>`.
fn trait_word<'tcx>(tcx: TyCtxt<'tcx>, tr: ty::TraitRef<'tcx>) -> String {
    trait_word_with_refs(tcx, tr, false)
}

/// `trait_word`, each reference among its arguments a `Ref` if `refs`.
fn trait_word_with_refs<'tcx>(tcx: TyCtxt<'tcx>, tr: ty::TraitRef<'tcx>, refs: bool) -> String {
    let mut name = tcx.item_name(tr.def_id).to_string();
    let generics = tcx.generics_of(tr.def_id);
    for (param, arg) in generics.own_params.iter().zip(tr.args).skip(1) {
        let Some(arg) = arg.as_type() else { continue };
        if param
            .default_value(tcx)
            .map(|d| d.instantiate(tcx, tr.args).skip_normalization())
            == Some(arg.into())
        {
            continue;
        }
        // `&'r Base` is `RefBase`, a reference's word before its pointee's.
        let mut word = String::new();
        let mut pointee = arg;
        while let ty::Ref(_, inner, _) = *pointee.kind() {
            if refs {
                word.push_str("Ref");
            }
            pointee = inner;
        }
        let pointee = js_word(&type_word(tcx, pointee));
        let mut rest = pointee.chars();
        match word.is_empty() {
            true => word.extend(rest),
            false => {
                word.extend(rest.next().map(|c| c.to_ascii_uppercase()));
                word.extend(rest);
            }
        }
        let mut chars = word.chars();
        name.extend(chars.next().map(|c| c.to_ascii_uppercase()));
        name.extend(chars);
    }
    name
}

/// A type's name as part of a JS name: what isn't a letter or a digit is
/// `_`, so `Vec<T>` is `Vec_T_`.
fn js_word(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// A type parameter's name as part of a JS name: `T`, or of `x: impl
/// fmt::Display`, which rustc names as it's written, its trait's, `Display`.
fn param_word(name: Symbol) -> String {
    let Some(bound) = name.as_str().strip_prefix("impl ") else {
        return name.to_string();
    };
    let path: String = bound
        .chars()
        .take_while(|&c| c.is_alphanumeric() || c == '_' || c == ':')
        .collect();
    path.rsplit("::").next().unwrap_or_default().to_string()
}

/// What a copied default body replaced of the given of the item it's
/// lowered in, its evidence and arguments, to put back when it's done.
pub(super) struct GivenScope<'tcx> {
    evidence: Vec<(ty::TraitRef<'tcx>, Expr)>,
    self_args: Option<ty::GenericArgsRef<'tcx>>,
    self_env: Option<ty::TypingEnv<'tcx>>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Enter a trait's default body copied into an impl (ADR 0049): it's
    /// given `evidence`, and the impl's arguments for its trait's
    /// parameters, `self_args`, which resolve in the typing environment
    /// of the item it's lowered in, the impl's.
    pub(super) fn enter_default(
        &mut self,
        evidence: Vec<(ty::TraitRef<'tcx>, Expr)>,
        self_args: ty::GenericArgsRef<'tcx>,
    ) -> GivenScope<'tcx> {
        GivenScope {
            evidence: std::mem::replace(&mut self.given.evidence, evidence),
            self_args: self.given.self_args.replace(self_args),
            self_env: self.given.self_env.replace(self.typing_env),
        }
    }

    /// Leave the copied default `enter_default` entered.
    pub(super) fn leave_default(&mut self, scope: GivenScope<'tcx>) {
        self.given.evidence = scope.evidence;
        self.given.self_args = scope.self_args;
        self.given.self_env = scope.self_env;
    }

    /// `value` in the impl's terms, in a copied default: its trait's
    /// parameters, `Self` among them, replaced by the impl's arguments for
    /// them (ADR 0049). Anywhere else, `value` itself.
    pub(super) fn in_impl_terms<T: TypeFoldable<TyCtxt<'tcx>>>(&self, value: T) -> T {
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
    pub(super) fn impl_typing_env(&self) -> Option<ty::TypingEnv<'tcx>> {
        let impl_env = self.given.self_env?;
        let bounds =
            (impl_env.param_env.caller_bounds().iter()).chain(self.typing_env.param_env.caller_bounds().iter());
        let param_env = ty::ParamEnv::new(self.tcx.mk_clauses_from_iter(bounds));
        Some(ty::TypingEnv::new(param_env, impl_env.typing_mode()))
    }

    pub(super) fn impl_type(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        if self.given.self_args.is_none() {
            return ty;
        }
        let ty = self.in_impl_terms(ty);
        let typing_env = self.given.self_env.unwrap_or(self.typing_env);
        self.tcx
            .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    }

    /// `resolve_instance` of a call whose arguments a copied default's
    /// `Self` was replaced in (ADR 0049): they're the impl's, which resolve in
    /// the impl's typing environment, as its own body's types don't.
    pub(super) fn resolve_self_instance(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Result<Option<ty::Instance<'tcx>>, rustc_span::ErrorGuaranteed> {
        let typing_env = self.given.self_env.unwrap_or(self.typing_env);
        super::items::resolve_instance_in(self.tcx, typing_env, def_id, args)
    }

    /// Is this a trait's default body copied into an impl (ADR 0049), whose
    /// type parameters are its trait's?
    pub(super) fn in_copied_default(&self) -> bool {
        self.given.self_args.is_some()
    }

    /// The first dictionary this function was given whose bound `which`
    /// says is the one: `T: Copy`'s, or a codec's (ADRs 0049, 0081).
    pub(super) fn given_evidence(&self, which: impl Fn(ty::TraitRef<'tcx>) -> bool) -> Option<Expr> {
        self.given
            .evidence
            .iter()
            .find(|&&(tr, _)| which(tr))
            .map(|(_, value)| value.clone())
    }

    /// The fact of its type parameter `index` this function was given,
    /// `TSize` (ADR 0145).
    pub(super) fn given_type_fact(&self, index: u32, fact: TypeFact) -> Option<Expr> {
        (self.given.type_facts.iter())
            .find(|&&(of, given, _)| matches!(of.kind(), ty::Param(p) if p.index == index) && given == fact)
            .map(|(_, _, value)| value.clone())
    }

    /// The fact of `of`, a std associated type of a type parameter, this
    /// function was given, `IItemMap` (ADR 0466).
    pub(super) fn given_fact_of(&self, of: Ty<'tcx>, fact: TypeFact) -> Option<Expr> {
        let of = self.tcx.erase_and_anonymize_regions(of);
        (self.given.type_facts.iter())
            .find(|&&(at, given, _)| self.tcx.erase_and_anonymize_regions(at) == of && given == fact)
            .map(|(_, _, value)| value.clone())
    }

    /// The caller's location this body was given, of a function that takes
    /// one, in its own body, not a closure in it (ADR 0427).
    pub(super) fn given_location(&self) -> Option<Expr> {
        let (owner, location) = self.given.caller_location.as_ref()?;
        (*owner == self.body_owner).then(|| location.clone())
    }

    pub(super) fn evidence_params(&mut self, id: DefId) -> Vec<js::Pattern> {
        let mut params: Vec<js::Pattern> = Vec::new();
        // Its caller's location first, of a function that tracks it (ADR 0427).
        if self.takes_caller_location(id) {
            let name = self.fresh("location");
            self.given.caller_location = Some((id, Expr::var(&name)));
            params.push(name.into());
        }
        // Then each const parameter's value, `N`, in the order they're
        // declared, the impl's before the method's (ADR 0107).
        for param in const_params(self.tcx, id) {
            let name = self.fresh(param.name.as_str());
            self.given.const_params.push((param.index, Expr::var(&name)));
            params.push(name.into());
        }
        params.extend(
            bounds(self.tcx, self.krate.foreign, self.krate.given, id)
                .into_iter()
                .map(|tr| {
                    // `writeT` and `readT`, as a generic codec's (ADR 0081).
                    let name = match super::serde::serde_trait(self.tcx, tr.def_id) {
                        Some(true) => format!("write{}", tr.self_ty()),
                        Some(false) => format!("read{}", tr.self_ty()),
                        // `XConvertF64` and `XConvertString`, of two impls of one trait.
                        None => format!("{}{}", evidence_word(self.tcx, tr.self_ty()), trait_word(self.tcx, tr)),
                    };
                    let name = self.fresh(&js_word(&name));
                    self.given.evidence.push((tr, Expr::var(&name)));
                    js::Pattern::from(name)
                }),
        );
        // Then each fact it asks of a type parameter, `TSize` (ADR 0145).
        for &(index, fact) in self.krate.type_facts.get(&id).into_iter().flatten() {
            let of = drop_slot(self.tcx, self.krate.foreign, id, index);
            let name = self.fresh(&format!("{}{}", drop_word(self.tcx, of), fact.word()));
            self.given.type_facts.push((of, fact, Expr::var(&name)));
            params.push(name.into());
        }
        // Then a drop function for each type parameter a caller gives a value
        // with a destructor, `dropT` (ADR 0098).
        // And one for each std trait's associated type of one, `dropIIntoIter`
        // (ADR 0421).
        for &index in self.krate.drop_params.get(&id).into_iter().flatten() {
            let ty = drop_slot(self.tcx, self.krate.foreign, id, index);
            let name = self.fresh(&format!("drop{}", drop_word(self.tcx, ty)));
            self.give_drop_param(index, name.clone(), ty);
            params.push(name.into());
        }
        params
    }

    /// The type `id`'s drop `index` drops, of the arguments it's called with
    /// here: the one given for its type parameter, or that type's associated
    /// type, as it's known here (ADR 0421).
    pub(super) fn drop_slot_of(&self, id: DefId, index: u32, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
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

    /// A const argument's value (ADR 0107): `3`, or the caller's own `N`.
    pub(super) fn const_arg(&self, c: ty::Const<'tcx>, span: Span) -> R<Expr> {
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

    /// Where this function's dictionaries are, by their bounds (ADR 0178).
    pub(super) fn evidence_query(&self) -> EvidenceQuery<'_, 'tcx> {
        EvidenceQuery {
            recognition: self.recognition(),
            given: &self.given.evidence,
        }
    }

    /// A query given no dictionaries: another body's, asked of its own types,
    /// whose type parameters are then of no dictionary known (ADR 0362).
    /// Is a dictionary for `tr` given? Without building it.
    pub(super) fn has_evidence(&self, tr: ty::TraitRef<'tcx>) -> bool {
        self.evidence_query().has_evidence(tr)
    }

    /// A `dyn`'s `Any` dictionary, of a trait whose supertrait is `Any`,
    /// from its own (ADR 0331).
    pub(super) fn any_of_dyn(&self, ty: Ty<'tcx>, dictionary: Expr) -> Option<Expr> {
        let ty::Dynamic(traits, ..) = ty.kind() else {
            return None;
        };
        let from = self
            .tcx
            .instantiate_bound_regions_with_erased(traits.principal()?.with_self_ty(self.tcx, ty));
        let any = any_trait(self.tcx)?;
        self.super_evidence(from, ty::TraitRef::new(self.tcx, any, [ty]), dictionary)
    }

    fn super_evidence(&self, from: ty::TraitRef<'tcx>, to: ty::TraitRef<'tcx>, value: Expr) -> Option<Expr> {
        let route = self.evidence_query().super_route(from, to)?;
        Some(route.into_iter().fold(value, |dictionary, name| {
            Expr::call(Expr::member(dictionary, name), Vec::new())
        }))
    }

    /// The dictionary for `tr` among those this function was given, or
    /// a supertrait's of one.
    pub(super) fn evidence_for(&self, tr: ty::TraitRef<'tcx>) -> Option<Expr> {
        let route = self.evidence_query().evidence_route(tr)?;
        Some(self.route_expr(&route))
    }

    fn route_expr(&self, route: &Route) -> Expr {
        let call = |dictionary: Expr, name: &str| Expr::call(Expr::member(dictionary, name), Vec::new());
        let root = match &route.root {
            Root::Given(index) => self.given.evidence[*index].1.clone(),
            Root::Item(owner, name) => call(self.route_expr(owner), name),
        };
        route
            .supers
            .iter()
            .fold(root, |dictionary, name| call(dictionary, name))
    }

    /// A dictionary's `$drop`, its type's drop where it has one, as Rust's
    /// vtable has it: what dropping a `dyn` of it runs (ADR 0098). A name no
    /// Rust method can have.
    fn dictionary_drop(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Prop>> {
        if self.drops(ty) != Drops::Runs {
            return Ok(None);
        }
        let drop = self.drop_function(ty, span)?;
        Ok(drop.map(|drop| Prop::Field(js::DICTIONARY_DROP.into(), drop)))
    }

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
                let text = self.joined_text(items.clone(), None, span)?;
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

    pub(super) fn dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
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
        match super::serde::serde_trait(self.tcx, tr.def_id) {
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
            if let ty::Param(param) = tr.self_ty().kind()
                && super::iterators::given_as_it_is(self.tcx, self.tcx.typeck_root_def_id(self.item), *param)
            {
                return Err(self.unsupported(
                    span,
                    &format!("stepping `{}`, which rust-js found nothing here steps", tr.self_ty()),
                ));
            }
            return self.iterator_dictionary(tr.self_ty(), span);
        }
        // An iterator's extension's, of one given as it is (ADR 0487).
        if super::recognition::is_iterator_extension(self.tcx, tr.def_id) {
            return self.iterator_extension_dictionary(tr.def_id, tr.self_ty(), span);
        }
        // `Any`'s: its type's key, `{ type_id: () => "i32" }` (ADR 0331).
        if any_trait(self.tcx) == Some(tr.def_id) {
            let key = self.type_fact_value(tr.self_ty(), TypeFact::Id, span)?;
            let type_id = Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(key)).at(js::Span::NONE)]);
            let mut props = vec![Prop::Field("type_id".into(), type_id)];
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
            && super::recognition::range_bounds(self.tcx, ty).is_some()
        {
            let start_bound = super::recognition::trait_method(self.tcx, tr.def_id, "start_bound");
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
                    Prop::Field(name.into(), Expr::arrow(vec!["range".into()], body))
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
            return Ok(Expr::object(vec![Prop::Field("from_iter".into(), from_iter)]));
        }
        // std's `Sum` or `Product` of a number, `{ sum(items) }`: what
        // `sum()` of them makes (ADR 0476).
        if let Some(name) = super::recognition::accumulation(self.tcx, tr.def_id)
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
        if super::recognition::is_extend(self.tcx, tr.def_id)
            && let Some(dictionary) = self.extend_dictionary(ty, span)?
        {
            return Ok(dictionary);
        }
        // `usize`'s or a range's `SliceIndex` of a slice (ADR 0418): the
        // runtime's, of each kind, `$sliceIndexes.range`.
        if is_std_def(self.tcx, tr.def_id, StdItem::SliceIndex) {
            use super::recognition::RangeBound::{Field, Unbounded};
            let kind = match ty.is_usize() {
                true => Some("item"),
                false => match super::recognition::range_bounds(self.tcx, ty) {
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
        if (default || clone || eq || display || to_string || debug || ord || partial_ord)
            && !self.has_user_impl(tr.def_id, ty)
        {
            if debug {
                // Given a `Formatter`'s options, where the crate's writers are (ADRs 0058, 0137).
                let (params, pretty): (Vec<js::Pattern>, _) = match self.writers_take_options() {
                    true => (
                        vec!["value".into(), "options".into()],
                        super::display::Pretty::When(Expr::var("options")),
                    ),
                    false => (vec!["value".into()], super::display::Pretty::Plain),
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
                return Ok(Expr::object(vec![Prop::Field(name.into(), compare)]));
            }
            if display {
                let fmt = self.display_fn(ty, span)?;
                return Ok(Expr::object(vec![Prop::Field("fmt".into(), fmt)]));
            }
            // `ToString` is std's of a `Display`: what that shows.
            if to_string {
                let fmt = self.display_fn(ty, span)?;
                return Ok(Expr::object(vec![Prop::Field("to_string".into(), fmt)]));
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
            // A `&mut` to a writer taken by value, `W = &mut s`, given in a
            // box (ADR 0454), a generic one's too: its own dictionary,
            // through the box.
            if let ty::Ref(_, inner, Mutability::Mut) = *ty.kind() {
                let writer = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [inner]), span)?;
                return Ok(Expr::call_helper("$mutWriter", vec![writer]));
            }
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
            let deref = super::recognition::trait_method(self.tcx, tr.def_id, "deref");
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
            let deref_mut = super::recognition::trait_method(self.tcx, tr.def_id, "deref_mut");
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
                props.push(Prop::Field("deref_mut".into(), f));
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
            let into_iter = super::recognition::trait_method(self.tcx, tr.def_id, "into_iter");
            let known = self
                .recognition()
                .classify(into_iter, tr.args)
                .ok_or_else(|| self.no_evidence(span, tr))?;
            let into_iter_ty = super::recognition::trait_method(self.tcx, tr.def_id, "IntoIter");
            let output = Ty::new_projection(self.tcx, ty::IsRigid::No, into_iter_ty, tr.args);
            let output = (self.tcx)
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
                .unwrap_or(output);
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
            let handles = match mutability {
                Mutability::Mut => self.item_handles_of(known, vec![given.clone()], pointee, span, &mut body)?,
                Mutability::Not => None,
            };
            let items = match handles {
                Some(handles) => handles,
                None => self.std_by_values(known, into_iter, tr.args, &[ty], vec![given], output, span, &mut body)?,
            };
            body.push(StmtKind::Return(Some(self.std_iterator(items, output))).at(js::Span::NONE));
            let f = Expr::arrow(vec![name.into()], body);
            return Ok(Expr::object(vec![Prop::Field("into_iter".into(), f)]));
        }
        // An `io::Write`'s bytes, of a `Vec<u8>`, added to it, or of a
        // standard stream, written to it (ADR 0392).
        if is_std_def(self.tcx, tr.def_id, StdItem::IoWrite)
            && let Some(writer) = self.recognition().io_writer(ty)
        {
            return Ok(Expr::helper(writer));
        }
        // std's `AsMut` of text as a `str`, or of a sequence as a slice, given
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
            if self.recognition().views_as_itself(ty, tr.args.type_at(1)) {
                let f = Expr::arrow(
                    vec!["cell".into()],
                    vec![StmtKind::Return(Some(cell)).at(js::Span::NONE)],
                );
                return Ok(Expr::object(vec![Prop::Field("as_mut".into(), f)]));
            }
        }
        if is_std_def(self.tcx, tr.def_id, StdItem::AsRef)
            && !self.has_user_impl(tr.def_id, ty)
            && self.recognition().views_as_itself(ty, tr.args.type_at(1))
        {
            let same = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(Expr::var("value"))).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field("as_ref".into(), same)]));
        }
        // std's `AsRef<[u8]>` of text: its UTF-8 bytes, as `as_bytes()` gives
        // them (ADR 0162).
        if is_std_def(self.tcx, tr.def_id, StdItem::AsRef)
            && !self.has_user_impl(tr.def_id, ty)
            && self.is_string_like(ty)
            && matches!(tr.args.type_at(1).kind(), ty::Slice(item) if *item == self.tcx.types.u8)
        {
            let bytes = super::std_types::text::utf8_bytes(Expr::var("value"));
            let as_ref = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(bytes)).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field("as_ref".into(), as_ref)]));
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
            let mut props = vec![Prop::Field("borrow_mut".into(), same)];
            props.extend(self.supertrait_entries(tr, span)?);
            return Ok(Expr::object(props));
        }
        // std's `FromStr` of a number, a `bool`, a `char` or a `String`: what
        // `s.parse()` of it is (ADR 0161).
        let parsed_by_std = super::representation::Num::of(ty).is_some()
            || ty.is_bool()
            || ty.is_char()
            || self.is_lang_adt(ty, LangItem::String);
        if super::recognition::is_from_str(self.tcx, tr.def_id) && parsed_by_std {
            let mut body = Vec::new();
            let parsed = self.parse_as(Expr::var("s"), ty, span, &mut body)?;
            body.push(StmtKind::Return(Some(parsed)).at(js::Span::NONE));
            let from_str = Expr::arrow(vec!["s".into()], body);
            return Ok(Expr::object(vec![Prop::Field("from_str".into(), from_str)]));
        }
        // `std::num::Wrapping`'s, its number's in its `[x]` (ADR 0175).
        if let Some(op) = super::recognition::value_operator(self.tcx, tr.def_id)
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
        if let Some(op) = super::recognition::value_operator(self.tcx, tr.def_id)
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
        if let Some(op) = super::recognition::assign_operator(self.tcx, tr.def_id)
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
            let copy = self.copy(Expr::var("value"), ty);
            return Ok(Expr::object(vec![Prop::Field(
                "copy".into(),
                Expr::arrow(
                    vec!["value".into()],
                    vec![StmtKind::Return(Some(copy)).at(js::Span::NONE)],
                ),
            )]));
        }
        // The crate's own impl's accessor, or one a library exports (ADR 0100):
        // in a copied default, of the impl's terms, where its bounds and the
        // default's hold, `Map<T, F>: Counting` of a blanket impl's.
        let typing_env = self.impl_typing_env().unwrap_or(self.typing_env);
        if let Some((impl_id, impl_args)) = evidence_impl(self.tcx, typing_env, tr)
            && (self.krate.trait_impls.contains(&impl_id) || self.krate.foreign.item(impl_id).is_some())
        {
            let callee = self.fn_ref(impl_id);
            let args = self.evidence_args(impl_id, impl_args, span)?;
            return Ok(Expr::call(callee, args));
        }
        // A trait object's: Rust's built-in `impl Trait for dyn Trait`.
        let selected = self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr));
        if let Ok(ImplSource::Builtin(BuiltinImplSource::Object(_), _)) = selected {
            return self.object_dictionary(tr, span);
        }
        Err(self.no_evidence(span, tr))
    }

    /// Rust's built-in `impl Trait for dyn Trait`, of `tr`, the `dyn`'s
    /// principal or a supertrait of it, as generic code is given it: each
    /// method calls the pair's own, as a call on the `dyn` does,
    /// `(object) => object.impl.area(object.value)`, and each supertrait's
    /// dictionary is one of these too.
    /// A dictionary's drops of its associated types, `$dropOffset`, which
    /// generic code holding one of them runs, as it can't name the type
    /// (ADR 0178): the impl's, or what a `dyn` names, `dyn Zone<Offset = T>`.
    fn item_drops(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
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

    /// A dictionary's facts of its associated types, what generic code asks
    /// of `<A as Array>::Item` (ADR 0145): `ItemSize`, its type's for `tr`.
    fn item_facts(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
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
    fn item_types(&self, tr: ty::TraitRef<'tcx>) -> Vec<(DefId, Ty<'tcx>)> {
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
    /// taken as any `IntoIterator`, and a type taken as what its own
    /// `Borrow` gives (ADRs 0160, 0167).
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
        Ok(())
    }

    pub(super) fn evidence_args(&mut self, id: DefId, args: ty::GenericArgsRef<'tcx>, span: Span) -> R<Vec<Expr>> {
        let location = self.caller_location(span);
        self.evidence_args_with(id, args, location, span)
    }

    /// `evidence_args`, `location` the one it's given where it takes one: its
    /// caller's, or, called by a shim of it, a function pointer or a
    /// dictionary's entry, its definition's, as rustc's shim gives it (ADR
    /// 0427).
    pub(super) fn evidence_args_with(
        &mut self,
        id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        location: Expr,
        span: Span,
    ) -> R<Vec<Expr>> {
        self.check_drops_given(id, args, span)?;
        if let Some(held) = self.dyn_in_generic_cell(id, args) {
            let what = format!("a cell of a `{held}` given to generic code as a cell of a type parameter");
            return Err(self.unsupported(span, &what));
        }
        // Where it's called first, of a function that tracks its caller, as
        // `evidence_params` takes it (ADR 0427).
        let mut values: Vec<Expr> = self.takes_caller_location(id).then_some(location).into_iter().collect();
        for param in const_params(self.tcx, id) {
            values.push(self.const_arg(args.const_at(param.index as usize), span)?);
        }
        for bound in bounds(self.tcx, self.krate.foreign, self.krate.given, id) {
            let bound = ty::EarlyBinder::bind(self.tcx, bound)
                .instantiate(self.tcx, args)
                .skip_normalization();
            values.push(self.dictionary(bound, span)?);
        }
        // Each fact it asks of a type parameter, of the type given for it,
        // a library's as its manifest says (ADR 0145).
        for (index, fact) in self.facts_asked(id) {
            let of = self.drop_slot_of(id, index, args);
            // Of a `dyn`, generic code over a `T: ?Sized` resolves a blanket
            // impl's method where rustc calls the `dyn`'s own, its value's
            // type's (ADR 0145): refused, as its answer would be another's.
            if matches!(of.kind(), ty::Dynamic(..)) {
                return Err(self.unsupported(span, &format!("a type's fact of `{of}` given to generic code")));
            }
            values.push(self.type_fact_value(of, fact, span)?);
        }
        // Each drop function it takes: a type's with nothing to drop is none,
        // left out at the end (ADR 0098).
        let mut drops = Vec::new();
        let given = match self.krate.foreign.item(id) {
            Some(item) => item.drops.as_slice(),
            None => self.krate.drop_params.get(&id).map_or(&[][..], Vec::as_slice),
        };
        // One that takes only the drops it uses is given each as an argument
        // the pipeline keeps or leaves out (ADR 0300).
        let takes_used = self.krate.drop_uses.borrow().takes_used(id);
        for &index in given {
            let ty = self.drop_slot_of(id, index, args);
            let iterated = self.iterated_slot(id, index, args, ty);
            drops.push(match takes_used {
                true => Some(self.drop_argument(id, index, (ty, iterated), span)?),
                false => self.held_drop_function(ty, iterated, span)?,
            });
        }
        while matches!(drops.last(), Some(None)) {
            drops.pop();
        }
        values.extend(drops.into_iter().map(|drop| drop.unwrap_or_else(Expr::undefined)));
        Ok(values)
    }

    /// What `id` asks of its type parameters, `TSize` and `KMap`, by their
    /// index: the crate's, or a library's as its manifest says (ADR 0145).
    pub(super) fn facts_asked(&self, id: DefId) -> Vec<(u32, TypeFact)> {
        match id.is_local() {
            true => self.krate.type_facts.get(&id).cloned().unwrap_or_default(),
            false => self.krate.foreign.type_facts(id),
        }
    }

    /// Select user code before std intrinsics, so custom implementations win.
    /// A trait method call, or `None` if it isn't one rust-js dispatches.
    /// `out` gets what must run first, like a receiver computed once.
    /// Are an operator's types, `tr`'s, each a number or a `bool`: JS's own
    /// operator's (ADR 0108)?
    fn primitive_operands(&self, tr: ty::TraitRef<'tcx>) -> bool {
        tr.args
            .types()
            .all(|t| super::representation::Num::of(t.peel_refs()).is_some() || t.peel_refs().is_bool())
    }

    /// `a + b`, or `-a` where `b` is `None`, of `tr`'s numbers, as a number's
    /// `+` is (ADR 0108): a shift's amount of its own type, `Shl<u64>` of a `u32`.
    fn number_operator(
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
                let b = super::std_types::number::shift_amount_of(op, b.expect("two operands"), ty, tr.args.type_at(1));
                self.binary(op, a, b, None, ty, span)
            }
            Err(op) => self.unary(op, a, ty, span),
        }
    }

    /// A generic impl's constant of its parameters (ADR 0176): its initializer,
    /// `() => [TConstZero.ZERO]`, lowered in its dictionary, which has the
    /// impl's evidence.
    /// The impl's own, `id`, or the trait's default.
    fn impl_const_getter(&mut self, id: DefId, span: Span) -> R<Expr> {
        let body = id
            .as_local()
            .and_then(|local| self.krate.closures.get(&local).copied())
            .ok_or_else(|| self.unsupported(span, "a generic impl's constant of its parameters"))?;
        let enclosing = self.enter_body(
            body,
            id,
            super::Nested::Closure {
                names: self.names.clone(),
            },
        )?;
        // From its MIR (ADR 0364), its value given back.
        let mut stmts = Vec::new();
        let lowered = match &body.mir {
            Some(mir) => self.mir_signature(body, mir, &mut stmts),
            None => Err(self.unsupported(span, "this constant, without its MIR")),
        };
        let hoisted = self.leave_body(enclosing);
        lowered?;
        stmts.splice(0..0, hoisted);
        Ok(Expr::arrow(Vec::new(), stmts))
    }

    /// An operator trait's dictionary entry: its method, `add`, after its
    /// `Output`.
    fn operator_entry(&self, trait_id: DefId) -> String {
        let method = self
            .tcx
            .associated_items(trait_id)
            .in_definition_order()
            .find(|item| item.is_fn())
            .expect("an operator trait has a method");
        bindings::fn_name(self.tcx, method.def_id)
    }

    pub(super) fn trait_call(
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
            // A `&mut self` method is given the pair, whose `value` a box's is: a
            // number's impl writes the place it reads (ADR 0099).
            let by_mut = matches!(
                self.tcx
                    .fn_sig(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .skip_binder()
                    .inputs()[0]
                    .kind(),
                ty::Ref(_, _, Mutability::Mut)
            );
            // A pair made here of parts read the same, a struct's last field's
            // `{ value: x.ptr, impl: x.impl }` (ADR 0409), is its parts.
            let parts = match &receiver.kind {
                js::ExprKind::Object(props) if !by_mut => match props.as_slice() {
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
        if let Some(instance) = self.resolve_self_instance(id, generic_args)?
            && self.is_rust_fn(instance.def_id())
            && self.tcx.trait_of_assoc(instance.def_id()).is_none()
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
            self.given_iterators(instance.def_id(), &mut values, &given, span)?;
            values.extend(self.evidence_args(instance.def_id(), instance.args, span)?);
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
            // A hand-written `PartialEq<Rhs>` is its own `eq`, whatever `Rhs` is.
            let eq = if self.is_user_impl(tr) {
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
        if ordering && super::representation::Num::of(tr.self_ty().peel_refs()).is_some() {
            return Ok(None);
        }
        // Where the types are known, `"paren".into()` and `a.add(b)` are std's,
        // written in place, as ever: a dictionary is for generic code (ADR 0108).
        // A conversion no bound gives, `VecDeque::from(v)` of a `Vec<T>`, is
        // std's impl wherever it's called, written in place too (ADR 0477).
        let conversion = is_std_def(self.tcx, trait_id, StdItem::Into) || is_std_def(self.tcx, trait_id, StdItem::From);
        if (super::recognition::value_operator(self.tcx, trait_id).is_some()
            || super::recognition::assign_operator(self.tcx, trait_id).is_some()
            || conversion)
            && (!tr.args.has_non_region_param() || (conversion && self.evidence_for(tr).is_none()))
        {
            // In a copied default, the call is the trait's, of `Self`, which std's
            // path doesn't see as the number it is here: `self - Self::one()`.
            if self.given.self_args.is_some()
                && let Some(op) = super::recognition::value_operator(self.tcx, trait_id)
                && self.primitive_operands(tr)
            {
                let mut values = values.into_iter();
                let a = values.next().expect("an operand");
                return Ok(Some(self.number_operator(op, a, values.next(), tr, span)?));
            }
            return Ok(None);
        }
        if std_written_in_place(self.tcx, trait_id, tr.self_ty(), known_here) {
            return Ok(None);
        }
        // A std trait's dictionary has only its required methods, and those
        // provided ones it's given.
        if operational(self.tcx, self.krate.foreign, trait_id)
            && !self.is_rust_trait(trait_id)
            && self.tcx.defaultness(id).has_value()
            && !in_std_dictionary(self.tcx, id)
        {
            return Err(super::refusal::refuse(
                self.tcx,
                span,
                super::refusal::Refused::Call(id),
            ));
        }
        if operational(self.tcx, self.krate.foreign, trait_id) {
            let dictionary = self.dictionary(tr, span)?;
            let mut values = values;
            // A range or an iterator of the crate's where a std method's own
            // generic `IntoIterator` goes, a JS iterator, as a function's (ADR
            // 0445).
            if !self.is_rust_trait(trait_id) {
                let given = (self.tcx.fn_sig(id).instantiate(self.tcx, generic_args))
                    .skip_normalization()
                    .skip_binder()
                    .inputs()
                    .to_vec();
                self.given_iterators(id, &mut values, &given, span)?;
            }
            let location = self.caller_location(span);
            values.extend(self.own_evidence(id, generic_args, Some(location), span)?);
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
        // Where it's called first, `location`, of a method whose entries take
        // it, unless it's given none (ADR 0427).
        let mut values: Vec<Expr> = match self.takes_caller_location(id) {
            true => location.into_iter().collect(),
            false => Vec::new(),
        };
        // Its own const parameters' values, `repeat::<3>`'s `3` (ADR 0135).
        for index in own_const_params(self.tcx, id) {
            values.push(self.const_arg(generic_args.const_at(index as usize), span)?);
        }
        for bound in own_bounds(self.tcx, self.krate.foreign, self.krate.given, id) {
            values.push(
                self.dictionary(
                    ty::EarlyBinder::bind(self.tcx, bound)
                        .instantiate(self.tcx, generic_args)
                        .skip_normalization(),
                    span,
                )?,
            );
        }
        // Then what it asks of its own type parameters, `KMap`, of the types
        // given for them (ADR 0466).
        let parent = self.tcx.generics_of(id).parent_count as u32;
        for (index, fact) in self.facts_asked(id) {
            if index >= parent {
                let of = self.drop_slot_of(id, index, generic_args);
                values.push(self.type_fact_value(of, fact, span)?);
            }
        }
        // Then a drop for each of its own type parameters the trait declares
        // one for, of the type given for it: none, left out at the end, for
        // one with nothing to drop (ADR 0163).
        let mut drops = Vec::new();
        for index in own_drop_params(self.tcx, self.krate.foreign, id) {
            let ty = self.drop_slot_of(id, index, generic_args);
            drops.push(self.drop_function(ty, span)?);
        }
        while matches!(drops.last(), Some(None)) {
            drops.pop();
        }
        values.extend(drops.into_iter().map(|drop| drop.unwrap_or_else(Expr::undefined)));
        Ok(values)
    }

    /// `evidence_args` of an impl's generic method, for its dictionary's entry:
    /// its own bounds' evidence is the entry's caller's, `names`, given for the
    /// trait's, `declared`, each passed on as the impl's bound it is, which may
    /// be in another order; the impl's own are this dictionary's (ADR 0106).
    fn method_evidence(
        &mut self,
        method: DefId,
        args: ty::GenericArgsRef<'tcx>,
        declared: &[ty::TraitRef<'tcx>],
        names: &[String],
        location: Expr,
        span: Span,
    ) -> R<Vec<Expr>> {
        let own = own_bounds(self.tcx, self.krate.foreign, self.krate.given, method);
        // Where it's called first, of one that takes it, as `evidence_args`
        // gives it (ADR 0427).
        let mut values: Vec<Expr> = self
            .takes_caller_location(method)
            .then_some(location)
            .into_iter()
            .collect();
        // Its const parameters' values, the impl's and its own (ADR 0135).
        for param in const_params(self.tcx, method) {
            values.push(self.const_arg(args.const_at(param.index as usize), span)?);
        }
        for bound in bounds(self.tcx, self.krate.foreign, self.krate.given, method) {
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
        // What it asks of its type parameters: of the impl's, this
        // dictionary's, and of its own, its caller's (ADRs 0145, 0466).
        for (index, fact) in self.facts_asked(method) {
            let of = self.drop_slot_of(method, index, args);
            values.push(self.type_fact_value(of, fact, span)?);
        }
        // Its drops are the impl's type parameters' (ADR 0098): its own, a
        // caller through a dictionary gives none of, are refused (`validate`).
        let mut drops = Vec::new();
        for &index in self.krate.drop_params.get(&method).map_or(&[][..], Vec::as_slice) {
            let ty = self.drop_slot_of(method, index, args);
            drops.push(self.drop_function(ty, span)?);
        }
        while matches!(drops.last(), Some(None)) {
            drops.pop();
        }
        values.extend(drops.into_iter().map(|drop| drop.unwrap_or_else(Expr::undefined)));
        Ok(values)
    }

    /// A `dyn` given for one of `id`'s type parameters that its signature,
    /// or its impl's type, holds in a cell, `RefCell<T>`, or a guard of one,
    /// a function's or an impl's: a cell of a `dyn` is its pair (ADR 0397),
    /// which generic code would read as a cell of a value (ADR 0451).
    fn dyn_in_generic_cell(&self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Ty<'tcx>> {
        let tcx = self.tcx;
        let held: Vec<Ty<'tcx>> = match tcx.def_kind(id) {
            DefKind::Impl { .. } => {
                let mut tys = vec![tcx.type_of(id).instantiate_identity().skip_normalization()];
                if let Some(tr) = tcx.impl_opt_trait_ref(id) {
                    tys.extend(tr.instantiate_identity().skip_normalization().args.types());
                }
                tys
            }
            DefKind::Fn | DefKind::AssocFn => {
                let sig = tcx.fn_sig(id).instantiate_identity().skip_normalization().skip_binder();
                sig.inputs_and_output.iter().collect()
            }
            _ => return None,
        };
        let mut seen = Vec::new();
        held.into_iter().find_map(|ty| self.celled_dyn(ty, args, &mut seen))
    }

    /// `dyn_in_generic_cell`'s, of `ty` and what it's made of, the crate's
    /// structs' fields too, `seen` once each.
    fn celled_dyn(&self, ty: Ty<'tcx>, args: ty::GenericArgsRef<'tcx>, seen: &mut Vec<DefId>) -> Option<Ty<'tcx>> {
        let cells = [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock];
        ty.walk().filter_map(|part| part.as_type()).find_map(|part| {
            let ty::Adt(adt, part_args) = part.kind() else {
                return None;
            };
            let cell = cells.into_iter().any(|item| self.is_std_type(part, item)) || self.recognition().is_guard(part);
            if cell
                && let Some(&ty::Param(param)) = part_args.types().next().map(|t| t.kind())
                && let Some(given) = args.get(param.index as usize).and_then(|a| a.as_type())
                && self.cell_dyn_trait(given).is_some()
            {
                return Some(given);
            }
            if adt.did().is_local() && !seen.contains(&adt.did()) {
                seen.push(adt.did());
                let fields: Vec<Ty<'tcx>> = adt.all_fields().map(|f| self.field_ty(f, part_args)).collect();
                return fields.into_iter().find_map(|field| self.celled_dyn(field, args, seen));
            }
            None
        })
    }

    /// The trait of a `RefCell<dyn T>` that `ty` is or points at: the
    /// crate's own `T` (ADR 0397).
    pub(super) fn dyn_cell_trait(&self, ty: Ty<'tcx>) -> Option<DefId> {
        let cell = self.pointee(ty);
        if !self.is_std_type(cell, StdItem::RefCell) {
            return None;
        }
        let ty::Adt(_, args) = cell.kind() else { return None };
        self.cell_dyn_trait(args.type_at(0))
    }

    /// The trait of `held`, a `dyn` whose `RefCell` is its pair, `{ value,
    /// impl }`, its guard too: the crate's own (ADR 0397). A `dyn FnMut`'s
    /// holds the function.
    pub(super) fn cell_dyn_trait(&self, held: Ty<'tcx>) -> Option<DefId> {
        match held.kind() {
            ty::Dynamic(predicates, ..) => predicates.principal_def_id().filter(|&id| self.is_rust_trait(id)),
            _ => None,
        }
    }

    /// Another of the crate's traits that `held`, in a cell made a `dyn` of
    /// `trait_id`, has, with a method named as one of `trait_id`'s: neither
    /// the other's supertrait. A cell made a `dyn` of two has one `impl`,
    /// both's methods (ADR 0397), which two of one name can't be.
    fn dyn_cell_clash(&self, held: Ty<'tcx>, trait_id: DefId) -> Option<DefId> {
        let methods = |id: DefId| -> HashSet<Symbol> {
            supertrait_def_ids(self.tcx, id)
                .flat_map(|of| self.tcx.associated_items(of).in_definition_order())
                .filter(|item| item.is_fn())
                .map(|item| item.name())
                .collect()
        };
        let under = |sub: DefId, of: DefId| supertrait_def_ids(self.tcx, sub).any(|id| id == of);
        let named = methods(trait_id);
        let bounds = self.declared_bounds(held, self.typing_env.param_env);
        self.tcx.all_traits_including_private().find(|&other| {
            self.is_rust_trait(other)
                && !under(trait_id, other)
                && !under(other, trait_id)
                && (bounds.contains(&other) || self.has_user_impl(other, held))
                && !methods(other).is_disjoint(&named)
        })
    }

    /// The trait a `dyn` of `ty` is a pair of, `{ value, impl }` (ADR 0049):
    /// one rust-js compiled, or std's `Display` or `Error`, whose
    /// dictionaries it makes (ADR 0141). A `dyn Debug` is a string instead.
    /// Is `ty`, through its pointer, a `dyn` that's a pair, its value and
    /// its dictionary: of a pair trait, or of none, `dyn Send`.
    pub(super) fn is_pair_dyn(&self, ty: Ty<'tcx>) -> bool {
        matches!(self.pointee(ty).kind(), ty::Dynamic(predicates, ..) if self.recognition().is_pair_dyn(predicates))
    }

    pub(super) fn dynamic_trait(&self, ty: Ty<'tcx>) -> Option<DefId> {
        let inner = self.pointee(ty);
        match inner.kind() {
            ty::Dynamic(predicates, ..) => predicates
                .principal_def_id()
                .filter(|&id| self.recognition().is_pair_trait(id)),
            _ => None,
        }
    }

    /// `{}` or `{:?}` of `pair`, a `dyn` of `ty`, as `writer`, `Display` or
    /// `Debug`, says: through its dictionary, `d.impl.fmt(d.value)`, or a
    /// supertrait's, `e.impl.Display().fmt(e.value)` (ADR 0141), given
    /// `options` (ADR 0058). None if `ty` isn't such a `dyn`.
    pub(super) fn dyn_written(
        &self,
        pair: Expr,
        (ty, writer): (Ty<'tcx>, DefId),
        options: Option<Expr>,
        span: Span,
    ) -> R<Option<Expr>> {
        if !matches!(ty.kind(), ty::Dynamic(..)) || self.dynamic_trait(ty).is_none() {
            return Ok(None);
        }
        let from = self.dyn_trait_ref(ty, ty).expect("a principal");
        let to = ty::TraitRef::new(self.tcx, writer, [ty]);
        let Some(dictionary) = self.super_evidence(from, to, Expr::member(pair.clone(), js::DICTIONARY)) else {
            return Ok(None);
        };
        // Read twice: once for its dictionary, once for its value.
        if !pair.reads_same() {
            return Err(self.unsupported(span, &format!("showing a `{ty}` made where it's shown")));
        }
        let mut values = vec![Expr::held(pair)];
        values.extend(options);
        Ok(Some(Expr::call(Expr::member(dictionary, "fmt"), values)))
    }

    /// What `From::from` or `Into::into` makes, and from what.
    pub(super) fn converted(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        let tr = self.tcx.trait_of_assoc(def_id)?;
        let mut types = args.types();
        let (first, second) = (types.next()?, types.next()?);
        if is_std_def(self.tcx, tr, StdItem::From) {
            Some((first, second))
        } else if is_std_def(self.tcx, tr, StdItem::Into) {
            Some((second, first))
        } else {
            None
        }
    }

    /// The dictionary of a `Box<dyn Error>`, `to`, made from a `from` by std's
    /// `From` (ADR 0141): the error's own `Error`, or a message's,
    /// `$stringError()`. None if `to` isn't one.
    pub(super) fn dyn_error_from(&mut self, to: Ty<'tcx>, from: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let is_error = |id: DefId| is_std_def(self.tcx, id, StdItem::Error);
        let Some(inner) = to.boxed_ty() else {
            return Ok(None);
        };
        if !matches!(inner.kind(), ty::Dynamic(traits, ..) if traits.principal_def_id().is_some_and(is_error)) {
            return Ok(None);
        }
        self.error_dictionary(from, span).map(Some)
    }

    /// The `Error` dictionary of a `from` made a `dyn Error` (ADR 0141): the
    /// error's own `Error`, or a message's, `$stringError()`, a std error's
    /// its own.
    pub(super) fn error_dictionary(&mut self, from: Ty<'tcx>, span: Span) -> R<Expr> {
        if self.is_string_like(from) {
            return Ok(Expr::call(Expr::helper("$stringError"), Vec::new()));
        }
        if let Some(dictionary) = self.std_error_dictionary(from) {
            return Ok(dictionary);
        }
        let error = std_item(self.tcx, StdItem::Error);
        let tr = ty::TraitRef::new(self.tcx, error, [from]);
        self.dictionary(tr, span)
    }

    /// The `Error` dictionary of one of std's errors rust-js holds as its own
    /// value, shown as std shows it: a parse error's, its message (ADR
    /// 0063), serde_json's, and an `io::Error`'s (ADR 0462).
    fn std_error_dictionary(&self, ty: Ty<'tcx>) -> Option<Expr> {
        if self.is_parse_error(ty)
            && let ty::Adt(adt, _) = ty.kind()
        {
            let name = self.tcx.item_name(adt.did());
            return Some(Expr::call(
                Expr::helper("$parseErrorDyn"),
                vec![Expr::str(name.as_str())],
            ));
        }
        if self.is_json_error(ty) {
            return Some(Expr::call(Expr::helper("$jsonErrorDyn"), Vec::new()));
        }
        if self.recognition().is_io_error(ty) {
            return Some(Expr::call(Expr::helper("$ioErrorDyn"), Vec::new()));
        }
        None
    }

    /// `Error::source`, which a `dyn Error`'s dictionary has (ADR 0141).
    fn is_error_source(&self, id: DefId) -> bool {
        is_std_method(self.tcx, id, StdItem::Error, "source")
    }

    fn dyn_trait_ref(&self, ty: Ty<'tcx>, self_ty: Ty<'tcx>) -> Option<ty::TraitRef<'tcx>> {
        match self.pointee(ty).kind() {
            // `dyn for<'a> AsStr<'a, 'a>`: its lifetimes erased, as a
            // dictionary is the same for any.
            ty::Dynamic(predicates, ..) => predicates.principal().map(|p| {
                self.tcx
                    .instantiate_bound_regions_with_erased(p.with_self_ty(self.tcx, self_ty))
            }),
            _ => None,
        }
    }

    pub(super) fn pointee(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        match ty.kind() {
            ty::Ref(_, inner, _) => *inner,
            ty::Adt(_, args) if self.is_std_wrapper(ty) => args.type_at(0),
            _ => ty,
        }
    }

    /// `holder`, what holds a `dyn`'s value as its last field, a `RefCell`
    /// or a struct, given `dictionary`, the `impl` of `held`'s `trait_id`,
    /// in place (ADRs 0397, 0409): one made here, `{ value: .. }`, made with
    /// it; another, which may be a `dyn` of another trait already, given
    /// both's methods, unless they share a name.
    fn given_impl(
        &self,
        holder: Expr,
        (held, trait_id): (Ty<'tcx>, Option<DefId>),
        dictionary: Expr,
        target: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        if let js::ExprKind::Object(props) = &holder.kind
            && matches!(props.as_slice(), [Prop::Field(name, _)] if name == js::HELD)
        {
            let mut props = props.clone();
            props.push(Prop::Field(js::DICTIONARY.into(), dictionary));
            return Ok(Expr::object(props));
        }
        if let Some(other) = trait_id.and_then(|trait_id| self.dyn_cell_clash(held, trait_id)) {
            return Err(self.unsupported(
                span,
                &format!(
                    "a `{target}` of a value that may be a `dyn {}` too: its one `impl` can't hold two methods of one name",
                    self.tcx.def_path_str(other)
                ),
            ));
        }
        Ok(Expr::call(Expr::helper("$dynCell"), vec![holder, dictionary]))
    }

    /// Of `source` unsized as `target`, a reference to, or a `Box` of, a
    /// struct of the crate's whose last field, or its last field's, is a
    /// `dyn` that's a pair: the type that field is in `source`, and the `dyn`
    /// (ADRs 0409, 0436). Not a `dyn FnMut`, the JS function: the same struct
    /// either way.
    pub(super) fn dyn_tail(&self, source: Ty<'tcx>, target: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        let (from, to) = (self.pointee(source), self.pointee(target));
        if !(target.is_ref() || target.is_box())
            || !matches!(to.kind(), ty::Adt(adt, _) if adt.is_struct() && self.is_rust_adt(adt.did()))
        {
            return None;
        }
        let (concrete, tail) = self.tcx.struct_lockstep_tails_for_codegen(from, to, self.typing_env);
        matches!(tail.kind(), ty::Dynamic(traits, ..) if self.recognition().is_pair_dyn(traits))
            .then_some((concrete, tail))
    }

    /// The fields from a `ty` to the struct its last `dyn` field is in, each
    /// a struct and its index: none where it's `ty`'s own, or its last
    /// field's, `fat.inner`.
    pub(super) fn tail_path(&self, mut ty: Ty<'tcx>) -> Vec<(Ty<'tcx>, usize)> {
        let mut path = Vec::new();
        loop {
            let ty::Adt(adt, args) = ty.kind() else { return path };
            let Some(last) = adt.non_enum_variant().fields.iter().last() else {
                return path;
            };
            let field = self.field_ty(last, args);
            if matches!(field.kind(), ty::Dynamic(..)) || !matches!(field.kind(), ty::Adt(..)) {
                return path;
            }
            path.push((ty, adt.non_enum_variant().fields.len() - 1));
            ty = field;
        }
    }

    /// An unsizing JS can't make: of a pointer of the crate's own, or to a
    /// struct whose last field is a `dyn`.
    pub(super) fn check_unsize(&self, target: Ty<'tcx>, span: Span) -> R<()> {
        // A pointer of the crate's own, as `#[derive(CoercePointee)]` makes
        // one, would hold a `dyn`'s value and impl where it holds the value.
        if let ty::Adt(adt, _) = target.kind()
            && (adt.did().is_local() || self.krate.foreign.in_library(adt.did()))
        {
            return Err(self.unsupported(span, &format!("unsizing a `{target}`")));
        }
        // A `RefCell<dyn T>` is its cell with its `impl`, a pair (ADR 0397).
        if self.dyn_cell_trait(target).is_some() {
            return Ok(());
        }
        // `&Fat<Bar>` to `&Fat<dyn ToBar>`: the struct given its last field's
        // `impl` (ADR 0409), behind a reference or a `Box`, which drops its
        // last field through it (ADR 0436); an `Rc`'s, which may be counted,
        // not yet. A `dyn Debug`'s would be the string it shows, which the
        // field isn't. One ending in a slice, or a `dyn FnMut`, the JS
        // function, is the same value either way.
        let pointee = self.pointee(target);
        let tail = self.tcx.struct_tail_for_codegen(pointee, self.typing_env);
        if pointee.is_adt()
            && matches!(tail.kind(), ty::Dynamic(traits, ..)
                if self.recognition().is_pair_dyn(traits) || self.is_dyn_debug(tail))
            && (!(target.is_ref() || target.is_box()) || self.is_dyn_debug(tail))
        {
            return Err(self.unsupported(span, &format!("a `{pointee}`, whose last field is a `dyn`")));
        }
        Ok(())
    }

    /// `x as &dyn Trait`: `{ value, impl }`, or for a trait object, the same
    /// value with its supertrait's dictionary. `out` gets a value computed once.
    pub(super) fn unsize_trait(
        &mut self,
        source: Ty<'tcx>,
        target: Ty<'tcx>,
        value: Expr,
        span: Span,
        out: &mut Vec<js::Stmt>,
    ) -> R<Expr> {
        self.check_unsize(target, span)?;
        // A `&dyn Debug` is the string it shows (ADR 0060), and serde's `&dyn
        // Expected` its text (ADR 0459).
        if self.is_dyn_debug(target) && !self.is_dyn_debug(source) {
            return self.dyn_debug_string(value, self.pointee(source), span);
        }
        if super::recognition::is_dyn_expected(self.tcx, target)
            && !super::recognition::is_dyn_expected(self.tcx, source)
        {
            return self.expected_text(value, self.pointee(source), span);
        }
        // A `()` as a `dyn` JS is given the value of, a binding crate's or a
        // marker's, is what JS is given of a tuple, an array (ADR 0020):
        // `use_effect(f, Some(&()))`'s `[]`.
        let value = match self.pointee(source).is_unit()
            && (!self.is_pair_dyn(target) || self.given_as_value(self.pointee(target)))
        {
            true => Expr::array(Vec::new()),
            false => value,
        };
        // A `RefCell<C>` as a `RefCell<dyn T>`: the same cell, given its
        // `impl`, in place, so an `Rc` of it before is the same (ADR 0397).
        if self.dyn_cell_trait(target).is_some()
            && let (ty::Adt(_, from), ty::Adt(_, to)) = (self.pointee(source).kind(), self.pointee(target).kind())
            && let Some(tr) = self.dyn_trait_ref(to.type_at(0), from.type_at(0))
        {
            // A `dyn` of its supertrait, whose dictionary would be the cell's
            // `impl`'s: not yet.
            if matches!(from.type_at(0).kind(), ty::Dynamic(..)) {
                return Err(self.unsupported(span, &format!("a `{source}` as a `{target}`")));
            }
            let dictionary = self.dictionary(tr, span)?;
            // A `Weak`'s of one, whose `dyn` is a pair around it: not yet.
            if self.weak_of(target).is_some() {
                return Err(self.unsupported(span, &format!("a counted `{target}`")));
            }
            // A counted `Rc`'s (ADR 0320): its cell given the `impl`, as
            // every owner shares it, and the same `Rc`, its counts theirs.
            if self.counted_rc(target).is_some() {
                let rc = match value.reads_same() {
                    true => value,
                    false => self.spill("rc", value, out),
                };
                let cell = Expr::member(rc.clone(), js::HELD);
                let given = self.given_impl(cell, (from.type_at(0), Some(tr.def_id)), dictionary, target, span)?;
                out.push(StmtKind::Expr(given).at(js::Span::NONE));
                return Ok(rc);
            }
            return self.given_impl(value, (from.type_at(0), Some(tr.def_id)), dictionary, target, span);
        }
        // `&Fat<Bar>` as a `&Fat<dyn ToBar>`: the same struct, the one its
        // last field is in given `Bar`'s `ToBar` as its `impl`, as a
        // `RefCell`'s is (ADR 0409).
        if let Some((concrete, tail)) = self.dyn_tail(source, target) {
            // Of a `dyn` already, the same struct; as another trait's, its
            // supertrait's, not yet.
            if matches!(concrete.kind(), ty::Dynamic(..)) {
                let erased = |ty| self.tcx.erase_and_anonymize_regions(ty);
                if erased(concrete) == erased(tail) {
                    return Ok(value);
                }
                return Err(self.unsupported(span, &format!("a `{source}` as a `{target}`")));
            }
            let trait_id = self.dyn_trait_ref(tail, concrete).map(|tr| tr.def_id);
            let dictionary = self.dyn_dictionary(tail, concrete, span)?;
            // The struct itself holds it: given its `impl` as it's made.
            let path = self.tail_path(self.pointee(target));
            if path.is_empty() {
                return self.given_impl(value, (concrete, trait_id), dictionary, target, span);
            }
            let value = match value.reads_same() {
                true => value,
                false => self.spill("value", value, out),
            };
            let parent = (path.into_iter()).fold(value.clone(), |parent, (ty, i)| self.project(parent, ty, i));
            let given = self.given_impl(parent, (concrete, trait_id), dictionary, target, span)?;
            out.push(StmtKind::Expr(given).at(js::Span::NONE));
            return Ok(value);
        }
        if !self.is_pair_dyn(target) {
            return Ok(value);
        }
        self.check_value_ty(target, span)?;
        // A counted `Rc` as an `Rc<dyn T>` is `{ value: pair, strong, weak }`
        // (ADR 0320): of one made here, its value made the pair; of one made
        // elsewhere, which its other owners share as what it is, not yet.
        if self.counted_rc(source).is_some() || self.weak_of(source).is_some() {
            return self.counted_unsized(source, target, value, span);
        }
        if self.is_pair_dyn(source) {
            let self_ty = self.pointee(source);
            let from = self.dyn_trait_ref(source, self_ty);
            // The same trait (a `Box<dyn T>` to a `Box<dyn T>`), or none, a
            // `dyn T + Send` as a `dyn Send`, whose dictionary is its drop,
            // which the first's has: the same pair.
            let Some(to) = self.dyn_trait_ref(target, self_ty) else {
                return Ok(value);
            };
            let from = from.expect("a `dyn` of no trait made one of a trait");
            if self.tcx.erase_and_anonymize_regions(from) == self.tcx.erase_and_anonymize_regions(to) {
                return Ok(value);
            }
            // Read twice, so one with effects goes in a `const` first.
            let pair = if value.has_effects() {
                self.spill("receiver", value, out)
            } else {
                value
            };
            let dictionary = self
                .super_evidence(from, to, Expr::member(pair.clone(), js::DICTIONARY))
                .ok_or_else(|| self.unsupported(span, "this trait upcast"))?;
            // A `&mut dyn Sub` as a `&mut dyn Super`: a pair on the first's
            // `value`, which its `&mut self` methods write (ADR 0099).
            if matches!(target.kind(), ty::Ref(_, _, Mutability::Mut)) {
                return Ok(Expr::pair(Expr::held(pair), dictionary));
            }
            return Ok(Expr::object(vec![
                Prop::Field(js::HELD.into(), Expr::held(pair)),
                Prop::Field(js::DICTIONARY.into(), dictionary),
            ]));
        }
        let dictionary = self.dyn_dictionary(target, self.pointee(source), span)?;
        // `&mut n as &mut dyn Trait` of a number: the pair reads and writes
        // the place the cell does (ADR 0099). A temporary's box is the pair's.
        if self.is_cell(source) {
            return Ok(match value.kind {
                js::ExprKind::Handle(place) => Expr::pair(*place, dictionary),
                js::ExprKind::Object(mut props) if matches!(props.as_slice(), [Prop::Field(name, _)] if name == js::HELD) =>
                {
                    props.push(Prop::Field(js::DICTIONARY.into(), dictionary));
                    Expr::object(props)
                }
                _ => {
                    let cell = if value.reads_same() {
                        value
                    } else {
                        self.spill("cell", value, out)
                    };
                    Expr::pair(Expr::held(cell), dictionary)
                }
            });
        }
        Ok(Expr::object(vec![
            Prop::Field(js::HELD.into(), value),
            Prop::Field(js::DICTIONARY.into(), dictionary),
        ]))
    }

    /// The dictionary a `self_ty` made a `target`, a `dyn`, is given: its
    /// trait's, or of a `dyn` of no trait, `dyn Send`, its drop alone,
    /// `{ $drop }`, or nothing, `{}`.
    fn dyn_dictionary(&mut self, target: Ty<'tcx>, self_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        match self.dyn_trait_ref(target, self_ty) {
            Some(tr) => self.dictionary(tr, span),
            None => Ok(Expr::object(self.dictionary_drop(self_ty, span)?.into_iter().collect())),
        }
    }

    /// A counted `Rc` made here, `{ value: x, strong: 1, weak: 0 }`, as an
    /// `Rc<dyn T>`: its value made the pair, `{ value: x, impl }`, as every
    /// use of one reads it (ADR 0320); one of the same `dyn` already, itself.
    /// One made elsewhere, of another `dyn`, or a `Weak`, is refused: its
    /// other owners share it as the type it is.
    fn counted_unsized(&mut self, source: Ty<'tcx>, target: Ty<'tcx>, value: Expr, span: Span) -> R<Expr> {
        // Of the same `dyn` already, a slice's unsized: itself.
        if self.tcx.erase_and_anonymize_regions(source) == self.tcx.erase_and_anonymize_regions(target) {
            return Ok(value);
        }
        let what = format!("a counted `{source}` made elsewhere as a `{target}`");
        let made_here = match (self.counted_rc(source), value.kind) {
            (Some(pointee), js::ExprKind::Object(props)) if !self.is_pair_dyn(pointee) => Some((pointee, props)),
            _ => None,
        };
        let Some((pointee, mut props)) = made_here else {
            return Err(self.unsupported(span, &what));
        };
        let dictionary = self.dyn_dictionary(target, pointee, span)?;
        let Some(Prop::Field(_, held)) = props
            .iter_mut()
            .find(|p| matches!(p, Prop::Field(name, _) if name == js::HELD))
        else {
            return Err(self.unsupported(span, &what));
        };
        let inner = std::mem::replace(held, Expr::undefined());
        *held = Expr::object(vec![
            Prop::Field(js::HELD.into(), inner),
            Prop::Field(js::DICTIONARY.into(), dictionary),
        ]);
        Ok(Expr::object(props))
    }

    /// A dictionary's `method`, which a std default given as another entry
    /// is, a writer's `write_char` its `write_str` or an error's `cause` its
    /// `source`, named once where it's more than a name: `{ write_str,
    /// write_char: write_str, .. }`.
    fn shared_entry(&mut self, props: &mut [Prop], shared: &mut Vec<js::Stmt>, method: &str, span: Span) -> R<Expr> {
        let value = props.iter().find_map(|prop| match prop {
            Prop::Field(name, value) if name == method => Some(value.clone()),
            _ => None,
        });
        let value = value.ok_or_else(|| self.unsupported(span, &format!("a dictionary without its `{method}`")))?;
        if matches!(value.kind, js::ExprKind::Var(_) | js::ExprKind::Symbol(_)) {
            return Ok(value);
        }
        let name = self.fresh(method);
        shared.push(StmtKind::Const(name.clone(), value).at(js::Span::NONE));
        for prop in props.iter_mut() {
            if let Prop::Field(field, value) = prop
                && field == method
            {
                *value = Expr::var(&name);
            }
        }
        Ok(Expr::var(&name))
    }

    /// What recognition knows of a call of the trait method `def_id`, of
    /// `generic_args`: nothing of `into_iter` of a `&C` or a `&mut C`, which
    /// its bound's dictionary gives (ADR 0457), in its impl's terms where
    /// it's a copied default. Nothing but a bound makes one `IntoIterator`:
    /// a `&mut I` of an `I: Iterator` is an iterator, stepped as it is.
    pub(super) fn known_trait_call(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        if let Some(trait_id) = self.tcx.trait_of_assoc(def_id)
            && into_iter_by_dictionary(self.tcx, trait_id, self.in_impl_terms(generic_args).type_at(0))
        {
            return None;
        }
        self.recognition().classify(def_id, generic_args)
    }

    /// Whether generic code holds a value of `self_ty` in a box: a `&mut` to
    /// an object, given for a type parameter (ADR 0454), `for x in c` of a `c:
    /// &mut C`'s (ADR 0457). A `&C` is the value, and a `&mut` to a number
    /// or text its cell.
    fn held_in_box(&self, self_ty: Ty<'tcx>) -> bool {
        matches!(*self_ty.kind(), ty::Ref(_, pointee, Mutability::Mut) if !self.is_cell_pointee(pointee))
    }

    /// The entries of `tr`'s dictionary that give its supertraits', of those
    /// that have one, each made where it's asked: `Display: () =>
    /// innerDisplay()` of an `Error`'s.
    fn supertrait_entries(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
        let found = supertraits(self.tcx, tr.def_id, tr.args)
            .into_iter()
            .map(|(name, tr, _)| (name, tr));
        self.entries_of(found.collect(), span)
    }

    /// The entries of `tr`'s dictionary that give the bounds its trait
    /// declares on an associated type, the impl's: `LabelDisplay` of `type
    /// Label = u32` is `u32`'s `Display` (ADR 0106).
    fn item_bound_entries(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
        self.entries_of(item_bounds(self.tcx, tr.def_id, tr.args), span)
    }

    /// An entry giving each of `bounds`' dictionaries, of those that have one.
    fn entries_of(&mut self, bounds: Vec<(String, ty::TraitRef<'tcx>)>, span: Span) -> R<Vec<Prop>> {
        let mut props = Vec::new();
        for (name, bound) in bounds {
            if operational(self.tcx, self.krate.foreign, bound.def_id) {
                let dictionary = self.dictionary(bound, span)?;
                props.push(Prop::Field(
                    name,
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
                ));
            }
        }
        Ok(props)
    }

    pub(super) fn lower_dictionary(&mut self, id: DefId, cache: &str) -> R<js::Function> {
        let span = self.tcx.def_span(id);
        // An impl of a generic trait, `Convert<f64>`, or of a std one,
        // `PartialEq<Rhs>`, is for its arguments: a dictionary of its own.
        let tr = self.tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
        let params = self.evidence_params(id);
        // What the dictionary's entries share, named once before it's made.
        let mut shared = Vec::new();
        let mut props = self.supertrait_entries(tr, span)?;
        props.extend(self.item_bound_entries(tr, span)?);
        for item in self.tcx.associated_items(tr.def_id).in_definition_order() {
            // A constant, the impl's or the trait's default, as rustc computed
            // it: read on each use where it's of a type changed in place, so
            // each is a value of its own, as ADR 0031's are.
            if matches!(self.tcx.def_kind(item.def_id), DefKind::AssocConst { .. }) {
                // Only one generic code reads, here or where a library's consumers
                // may (ADR 0100).
                if !self.krate.library && !self.krate.generic_consts.contains(&item.def_id) {
                    continue;
                }
                let own = self.tcx.impl_item_implementor_ids(id).get(&item.def_id).copied();
                // A library's of a trait's default isn't computed: a consumer
                // may never read it, and rustc reports one it can't compute,
                // `360 / Self::SIDES` of 0, only where it's read (ADR 0176).
                let evaluated = match (self.krate.library, own) {
                    (true, None) => None,
                    _ => eval_const(self.tcx, self.typing_env, item.def_id, tr.args, span),
                };
                let Some(value) = evaluated.and_then(|value| const_js(self.tcx, value)) else {
                    // Of its parameters, `Wrapping(T::ZERO)`: its initializer, read
                    // each time, as a constant is, of the impl's evidence (ADR 0176);
                    // a trait's default's, of the dictionary's.
                    let getter = match own {
                        // A function's, a constant of its module (ADR 0410).
                        Some(own) if super::analysis::impl_const(self.tcx, own) => Expr::arrow(
                            Vec::new(),
                            vec![StmtKind::Return(Some(self.fn_ref(own))).at(js::Span::NONE)],
                        ),
                        Some(own) => self.impl_const_getter(own, span)?,
                        // The trait's default, copied into the impl as a default
                        // method is, `Self` the impl's type (ADR 0049).
                        None => self.default_method(item.def_id, tr.args)?,
                    };
                    let name = bindings::fn_name(self.tcx, item.def_id);
                    // A literal, `"shape"`, is the value itself: nothing to compute.
                    let literal = match &getter.kind {
                        js::ExprKind::Arrow(params, body) if params.is_empty() => match body.as_slice() {
                            [
                                js::Stmt {
                                    kind: StmtKind::Return(Some(value)),
                                    ..
                                },
                            ] if value.is_constant() => Some(value.clone()),
                            _ => None,
                        },
                        _ => None,
                    };
                    props.push(match literal {
                        Some(value) => Prop::Field(name, value),
                        None => Prop::Getter(name, getter),
                    });
                    continue;
                };
                let ty = self
                    .tcx
                    .type_of(item.def_id)
                    .instantiate(self.tcx, tr.args)
                    .skip_normalization();
                let ty = self
                    .tcx
                    .normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty));
                let name = bindings::fn_name(self.tcx, item.def_id);
                props.push(match self.contains_mutated(ty) {
                    true => Prop::Getter(
                        name,
                        Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(value)).at(js::Span::NONE)]),
                    ),
                    false => Prop::Field(name, value),
                });
                continue;
            }
            // A binding, which JS reads of the value itself, isn't in it
            // (ADR 0307).
            if self.tcx.def_kind(item.def_id) != DefKind::AssocFn || bindings::is_binding(self.tcx, item.def_id) {
                continue;
            }
            // A std trait's provided methods, like `Clone::clone_from`,
            // aren't in its dictionary: nothing calls them through it. But a
            // `dyn Error` calls `source` (ADR 0141), generic code writing
            // to any writer `write_char` and `write_fmt` (ADR 0180), and a
            // generic `clone_from_slice` an impl's own `clone_from`, as std's
            // does, which it asks the dictionary for.
            let source = self.is_error_source(item.def_id);
            let own_clone_from = is_clone_from(self.tcx, item.def_id) && self.recognition().has_clone_from(id);
            if !self.is_rust_trait(tr.def_id)
                && self.tcx.defaultness(item.def_id).has_value()
                && !in_std_dictionary(self.tcx, item.def_id)
                && !own_clone_from
            {
                continue;
            }
            // A `Hasher`'s provided method its impl doesn't write: `$hasher`'s,
            // std's over the dictionary's own (ADR 0463).
            if is_hasher(self.tcx, tr.def_id)
                && self.tcx.defaultness(item.def_id).has_value()
                && !self.tcx.impl_item_implementor_ids(id).contains_key(&item.def_id)
            {
                continue;
            }
            // The method's own parameters: lifetimes, `fn bar<'b>`, are erased,
            // as they aren't in the JS, but rustc resolves with them; a type,
            // `describe<T>`, is its caller's, the trait method's own (ADR 0106).
            let args = tr.args.extend_to(self.tcx, item.def_id, |param, _| match param.kind {
                ty::GenericParamDefKind::Lifetime => self.tcx.lifetimes.re_erased.into(),
                _ => self.tcx.mk_param_from_def(param),
            });
            // One whose `where` clauses this impl's type can't meet, a
            // default's `where Self::Item: Deref` of an `Item` that isn't,
            // nothing can call: it isn't in it, as rustc leaves it out of a
            // vtable.
            if self.tcx.instantiate_and_check_impossible_clauses((item.def_id, args)) {
                continue;
            }
            // rustc won't resolve one of a blanket impl over a `?Sized` type,
            // `impl<Q: ?Sized> Equivalent<K> for Q`, as a `dyn` might be that
            // type too: in the impl's own dictionary it's the impl's item.
            let own = || {
                let method = *self.tcx.impl_item_implementor_ids(id).get(&item.def_id)?;
                let args = ty::GenericArgs::for_item(self.tcx, method, |param, _| match param.kind {
                    ty::GenericParamDefKind::Lifetime => self.tcx.lifetimes.re_erased.into(),
                    _ => self.tcx.mk_param_from_def(param),
                });
                Some(ty::Instance::new_raw(method, args))
            };
            let instance = self
                .resolve_instance(item.def_id, args)?
                .or_else(own)
                .ok_or_else(|| self.unsupported(span, "this trait implementation"))?;
            let method = instance.def_id();
            // std's own `source`: `None`.
            if source && !self.krate.fns.contains_key(&method) {
                let none = Expr::arrow(
                    Vec::new(),
                    vec![StmtKind::Return(Some(Expr::undefined())).at(js::Span::NONE)],
                );
                props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), none));
                continue;
            }
            // std's own `Error::description`, deprecated: its text; and
            // `cause`, its `source` (ADR 0141).
            if let Some(cause) = error_default(self.tcx, item.def_id)
                && !self.krate.fns.contains_key(&method)
            {
                let entry = match cause {
                    true => self.shared_entry(&mut props, &mut shared, "source", span)?,
                    false => Expr::arrow(
                        Vec::new(),
                        vec![
                            StmtKind::Return(Some(Expr::str("description() is deprecated; use Display")))
                                .at(js::Span::NONE),
                        ],
                    ),
                };
                props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), entry));
                continue;
            }
            // std's own `write_char` and `write_fmt`: its `write_str`, given a
            // `char`'s text or a `write!`'s, as std's give it (ADR 0180).
            if is_writer_default(self.tcx, item.def_id) && !self.krate.fns.contains_key(&method) {
                let write_str = self.shared_entry(&mut props, &mut shared, "write_str", span)?;
                props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), write_str));
                continue;
            }
            // std's own `write_all` and `write_fmt` of a writer of bytes of the
            // crate's: its `write`'s till all's written, as std's (ADR 0392).
            if let Some(all) = io_write_default(self.tcx, item.def_id)
                && !self.krate.fns.contains_key(&method)
            {
                let write = self.shared_entry(&mut props, &mut shared, "write", span)?;
                let helper = match all {
                    true => "$ioWriteAll",
                    false => "$ioWriteFmt",
                };
                props.push(Prop::Field(
                    bindings::fn_name(self.tcx, item.def_id),
                    Expr::call(Expr::helper(helper), vec![write]),
                ));
                continue;
            }
            // std's own `hash_slice` of a `Hash` of the crate's: each item's
            // `hash`, as std's (ADR 0463).
            if is_std_method(self.tcx, item.def_id, StdItem::Hash, "hash_slice")
                && !self.krate.fns.contains_key(&method)
            {
                let hash = self.shared_entry(&mut props, &mut shared, "hash", span)?;
                props.push(Prop::Field(
                    bindings::fn_name(self.tcx, item.def_id),
                    Expr::call(Expr::helper("$hashSlice"), vec![hash]),
                ));
                continue;
            }
            // A library's trait's default, whose body is the library's (ADR 0100):
            // its function over `Self`, given this impl's dictionary as any of
            // its generic functions is (ADR 0185).
            let library_default = self.krate.foreign.in_library(method);
            if library_default && self.krate.foreign.item(method).is_none() {
                let what = format!(
                    "implementing another crate's trait without its default `{}`: write the method in the impl",
                    self.tcx.def_path_str(method)
                );
                return Err(self.unsupported(span, &what));
            }
            if !library_default && !self.krate.fns.contains_key(&method) {
                return Err(self.unsupported(span, &format!("trait method `{}`", self.tcx.def_path_str(method))));
            }
            if !library_default && self.tcx.trait_of_assoc(method).is_some() {
                let value = self.default_method(method, instance.args)?;
                props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), value));
                continue;
            }
            let callee = self.fn_ref(method);
            // Without a `Formatter`, which isn't a JS parameter (ADR 0054).
            let count = self
                .tcx
                .fn_sig(method)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs()
                .len()
                - usize::from(self.formatter_param(method).is_some())
                // A writer's options, where the crate's take them (ADRs 0058, 0137).
                + usize::from(self.writers_take_options() && self.formatter_param(method).is_some());
            let mut params: Vec<String> = (0..count).map(|i| format!("arg{i}")).collect();
            let mut values: Vec<Expr> = params.iter().map(|name| Expr::var(name)).collect();
            // Where it's called, after its arguments, of one whose entries take
            // it (ADR 0427): its caller's, generic code's or a `dyn`'s where
            // its declaration tracks its caller, and generic code's alone
            // where only an impl does, whose entry's is its own definition's
            // where a `dyn`'s call gives none, as rustc's shim gives it.
            let at_definition = self.location_at(self.tcx.def_span(method));
            let mut defaulted = None;
            let location = match self.takes_caller_location(item.def_id) {
                true => {
                    let name = self.fresh("location");
                    params.push(name.clone());
                    if !tracks_caller(self.tcx, item.def_id) && self.takes_caller_location(method) {
                        defaulted = Some((name.clone(), at_definition));
                    }
                    Expr::var(&name)
                }
                false => at_definition,
            };
            // Its own const parameters' values, given after its arguments
            // (ADR 0135), for the method, as the trait method's are named.
            let mark = self.given.const_params.len();
            for index in own_const_params(self.tcx, item.def_id) {
                if self.given.const_params.iter().any(|&(at, _)| at == index) {
                    return Err(self.unsupported(span, "a trait method's const parameter here"));
                }
                let name = self.fresh(
                    self.tcx
                        .generics_of(item.def_id)
                        .param_at(index as usize, self.tcx)
                        .name
                        .as_str(),
                );
                self.given.const_params.push((index, Expr::var(&name)));
                params.push(name);
            }
            // A generic method's own evidence is its caller's, after the arguments.
            let declared: Vec<_> = own_bounds(self.tcx, self.krate.foreign, self.krate.given, item.def_id)
                .into_iter()
                .map(|bound| {
                    ty::EarlyBinder::bind(self.tcx, bound)
                        .instantiate(self.tcx, args)
                        .skip_normalization()
                })
                .collect();
            let names: Vec<String> = declared
                .iter()
                .map(|&d| {
                    let word = format!("{}{}", evidence_word(self.tcx, d.self_ty()), trait_word(self.tcx, d));
                    self.fresh(&js_word(&word))
                })
                .collect();
            // What its caller through the dictionary gives in a box for each:
            // a generic `&mut Self` (ADR 0099), and a `Self` that's a `&mut` to
            // an object, an impl for `&mut Bag`'s (ADRs 0454, 0457); to a method
            // that takes a cell for each `&mut` it can't hold as the object,
            // `&mut u32`'s or `&mut &mut Money`'s: what's in each box more.
            let self_held = self.held_in_box(tr.self_ty());
            for (i, value) in values.iter_mut().enumerate() {
                let given = usize::from(self.param_is_box(item.def_id, i)) + usize::from(i == 0 && self_held);
                for _ in usize::from(self.param_is_box(method, i))..given {
                    *value = Expr::held(std::mem::replace(value, Expr::undefined()));
                }
            }
            // The drops its caller gives for its own type parameters, which it
            // passes on as the method's, where the method takes them (ADR 0163).
            let foreign = self.krate.foreign;
            let own_drops: Vec<u32> = match self.krate.drop_params.get(&method) {
                Some(taken)
                    if own_drop_params(self.tcx, foreign, method)
                        .iter()
                        .any(|i| taken.contains(i)) =>
                {
                    own_drop_params(self.tcx, foreign, item.def_id)
                }
                _ => Vec::new(),
            };
            let mut drop_names = Vec::new();
            let mut replaced = Vec::new();
            for &index in &own_drops {
                let ty = drop_slot(self.tcx, foreign, item.def_id, index);
                let name = self.fresh(&format!("drop{}", drop_word(self.tcx, ty)));
                replaced.push(self.lend_drop_param(index, name.clone(), ty));
                drop_names.push(name);
            }
            // The facts its caller gives of its own type parameters, `KMap`,
            // which it passes on as the method's (ADR 0466), as the trait
            // method's are named.
            let parent = self.tcx.generics_of(item.def_id).parent_count as u32;
            let own_facts: Vec<(u32, TypeFact)> = (self.krate.type_facts.get(&item.def_id).into_iter().flatten())
                .copied()
                .filter(|&(index, _)| index >= parent)
                .collect();
            let fact_mark = self.given.type_facts.len();
            let mut fact_names = Vec::new();
            for (index, fact) in own_facts {
                let of = drop_slot(self.tcx, self.krate.foreign, item.def_id, index);
                if self.given.type_facts.iter().any(|&(at, _, _)| at == of) {
                    return Err(self.unsupported(span, "a trait method's type parameter's fact here"));
                }
                let name = self.fresh(&format!("{}{}", drop_word(self.tcx, of), fact.word()));
                self.given.type_facts.push((of, fact, Expr::var(&name)));
                fact_names.push(name);
            }
            // Everything the method takes that its caller through the dictionary
            // doesn't give: its dictionaries, then its drops (ADR 0098), which
            // are this impl's own, and its own type parameters' it's given.
            let evidence = match declared.is_empty() {
                true => self.evidence_args_with(method, instance.args, location, span),
                false => self.method_evidence(method, instance.args, &declared, &names, location, span),
            };
            self.given.const_params.truncate(mark);
            self.given.type_facts.truncate(fact_mark);
            for lent in replaced {
                self.return_drop_param(lent);
            }
            let evidence = evidence?;
            params.extend(names);
            params.extend(fact_names);
            params.extend(drop_names);
            values.extend(evidence);
            // What generic code takes for an associated type bound by
            // `Iterator`, `fn items(&self) -> Self::Iter`, is a JS iterator
            // (ADR 0061): a range or the crate's own iterator the impl gives,
            // made one, as one given where a generic iterator goes is.
            let declared_output = (self.tcx.fn_sig(item.def_id).instantiate_identity())
                .skip_normalization()
                .skip_binder()
                .output();
            let output = (self.tcx.fn_sig(method).instantiate(self.tcx, instance.args))
                .skip_normalization()
                .skip_binder()
                .output();
            let iterates = matches!(declared_output.kind(), ty::Alias(..))
                && (self
                    .declared_bounds(declared_output, self.tcx.param_env(item.def_id))
                    .into_iter())
                .any(|id| is_std_def(self.tcx, id, StdItem::Iterator))
                && (self.range_kind(output).is_some() || self.is_user_iterator(output));
            let call = Expr::call(callee.clone(), values.clone());
            let call = match iterates {
                true => Some(self.as_js_iterator(call, output, span)?),
                false => None,
            };
            // A `&mut` generic code has in a box by the trait's signature, `&mut
            // Self::Target` of a `Vec`, which the method gives as the object:
            // a box of it, which a write of the whole changes in place (ADR
            // 0454), as the caller through the dictionary takes it out. Of an
            // impl's own type parameter, the trait's is generic too, and the
            // method's box is it.
            let call = match self.boxed_in_call(item.def_id, declared_output, args) {
                Some(_) => Some(Expr::call(
                    Expr::helper("$cellOf"),
                    vec![call.unwrap_or_else(|| Expr::call(callee.clone(), values.clone()))],
                )),
                None => call,
            };
            // A `fmt::Result` the trait's signature has as a `Result`, `fn
            // write(&mut self) -> Result<(), Self::Error>`'s of an impl whose
            // `Error` is `fmt::Error`: one (ADR 0461).
            let call = match self.generic_fmt_result(declared_output, args) {
                true => {
                    let made = call.unwrap_or_else(|| Expr::call(callee.clone(), values.clone()));
                    Some(Expr::call_helper(
                        "$writeResult",
                        vec![Expr::arrow(
                            Vec::new(),
                            vec![js::StmtKind::Return(Some(made)).at(js::Span::NONE)],
                        )],
                    ))
                }
                false => call,
            };
            // One that passes on just what it's given, in order, is the method.
            let passed = call.is_none()
                && defaulted.is_none()
                && values.len() == params.len()
                && values
                    .iter()
                    .zip(&params)
                    .all(|(value, param)| matches!(&value.kind, js::ExprKind::Var(v) if v == param));
            let value = if passed {
                callee
            } else {
                let patterns = params.into_iter().map(|name| match &defaulted {
                    Some((location, at)) if *location == name => js::Pattern::Defaulted(name, at.clone()),
                    _ => name.into(),
                });
                Expr::arrow(
                    patterns.collect(),
                    vec![StmtKind::Return(Some(call.unwrap_or_else(|| Expr::call(callee, values)))).at(js::Span::NONE)],
                )
            };
            props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), value));
        }
        props.extend(self.dictionary_drop(tr.self_ty(), span)?);
        props.extend(self.item_drops(tr, span)?);
        props.extend(self.item_facts(tr, span)?);
        let object = match is_hasher(self.tcx, tr.def_id) {
            true => Expr::call(Expr::helper("$hasher"), vec![Expr::object(props)]),
            false => Expr::object(props),
        };
        let undefined = Expr::bin(Op::Eq, Expr::var(cache), Expr::undefined());
        let mut body = Vec::new();
        if params.is_empty() {
            body.push(
                StmtKind::If(
                    undefined,
                    [
                        shared,
                        vec![StmtKind::Assign(Expr::var(cache), object).at(js::Span::NONE)],
                    ]
                    .concat(),
                    None,
                )
                .at(js::Span::NONE),
            );
            body.push(StmtKind::Return(Some(Expr::var(cache))).at(js::Span::NONE));
        } else {
            // Keyed by a const parameter's value first, or a type's fact, a
            // number, which only a `Map` holds (ADRs 0107, 0145).
            let map = if self.given.const_params.is_empty() && self.given.type_facts.is_empty() {
                "WeakMap"
            } else {
                "Map"
            };
            body.push(
                StmtKind::If(
                    undefined,
                    vec![StmtKind::Assign(Expr::var(cache), Expr::new_(Expr::var(map), Vec::new())).at(js::Span::NONE)],
                    None,
                )
                .at(js::Span::NONE),
            );
            // One dictionary for each set of what it's given: its const
            // parameters' values, its dictionaries, what it asks of its type
            // parameters, `TSize` (ADR 0145), and its drops, which may be none.
            let keys = Expr::array(
                self.given
                    .const_params
                    .iter()
                    .map(|(_, value)| value.clone())
                    .chain(self.given.evidence.iter().map(|(_, value)| value.clone()))
                    .chain(self.given.type_facts.iter().map(|(_, _, value)| value.clone()))
                    .chain(self.given_drops().iter().map(|name| Expr::var(name)))
                    .collect(),
            );
            let make = Expr::arrow(
                Vec::new(),
                [shared, vec![StmtKind::Return(Some(object)).at(js::Span::NONE)]].concat(),
            );
            body.push(
                StmtKind::Return(Some(Expr::call(
                    Expr::helper("$traitImpl"),
                    vec![Expr::var(cache), keys, make],
                )))
                .at(js::Span::NONE),
            );
        }
        Ok(js::Function {
            name: self.krate.fns[&id].name.clone(),
            params,
            body,
            export: self.tcx.visibility(tr.def_id).is_public(),
            is_async: false,
            span: self.js_span(span),
            name_span: js::Span::NONE,
        })
    }

    /// Copy the default body into this implementation. Its Rust bindings still
    /// refer to the trait definition; only its evidence is specialized here.
    fn default_method(&mut self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> R<Expr> {
        let span = self.tcx.def_span(id);
        let mut specialized = Vec::new();
        // A generic default's own evidence is its caller's, after its
        // arguments (ADR 0106); the rest is the impl's, made here.
        let own = own_bounds(self.tcx, self.krate.foreign, self.krate.given, id);
        let mut own_params = Vec::new();
        for bound in bounds(self.tcx, self.krate.foreign, self.krate.given, id) {
            if own.contains(&bound) {
                let word = format!(
                    "{}{}",
                    evidence_word(self.tcx, bound.self_ty()),
                    trait_word(self.tcx, bound)
                );
                let name = self.fresh(&js_word(&word));
                specialized.push((bound, Expr::var(&name)));
                // And as the body asks for it, in the impl's terms: a method's
                // `where Self: PartialEq` of a `Wrapping<T>`'s.
                let concrete = ty::EarlyBinder::bind(self.tcx, bound)
                    .instantiate(self.tcx, args)
                    .skip_normalization();
                if concrete != bound {
                    specialized.push((concrete, Expr::var(&name)));
                }
                own_params.push(name);
                continue;
            }
            let concrete = ty::EarlyBinder::bind(self.tcx, bound)
                .instantiate(self.tcx, args)
                .skip_normalization();
            specialized.push((bound, self.dictionary(concrete, span)?));
        }
        // And the impl's own, `SClone` of `impl<S: Clone> Movable<S> for
        // Point<S>`: a call on the impl's types, which the body's resolve
        // to, is given them.
        specialized.extend(self.given.evidence.iter().cloned());
        // Its trait's type parameters, `Self` among them, drop as the impl's
        // arguments for them do, with the impl's drops (ADR 0098): each is
        // made here, and the body is given it by name.
        let mut made = Vec::new();
        let mut drops = HashMap::new();
        // One rust-js can't make is an error only if the body drops one.
        let mut unsupported = HashMap::new();
        let generics = self.tcx.generics_of(id);
        for (index, arg) in args.iter().enumerate() {
            let Some(ty) = arg.as_type() else {
                continue;
            };
            match self.drops(ty) {
                Drops::Nothing => continue,
                Drops::Unsupported(t, what) => {
                    unsupported.insert(index as u32, (t, what));
                    continue;
                }
                Drops::Runs => {}
            }
            let Some(drop) = self.drop_function(ty, span)? else {
                continue;
            };
            let name = match &drop.kind {
                js::ExprKind::Var(name) => name.clone(),
                _ => {
                    let name = self.fresh(&format!("drop{}", param_word(generics.param_at(index, self.tcx).name)));
                    made.push((index as u32, StmtKind::Const(name.clone(), drop).at(js::Span::NONE)));
                    name
                }
            };
            drops.insert(index as u32, name);
        }
        // And each drop of a std associated type the impl's given, `dropTItem`
        // of `T::Item`, as the body asks in the impl's terms (ADR 0421).
        let mut items = HashMap::new();
        for (at, (alias, name)) in self.given_item_drops().into_iter().enumerate() {
            let index = (generics.count() + at) as u32;
            drops.insert(index, name);
            items.insert(alias, index);
        }
        // Its trait's const parameters are the impl's arguments for them, `10`
        // of `Scaled<10>` or the impl's own `K`, and its own are given after
        // its arguments (ADR 0135).
        let mut consts = Vec::new();
        let generics = self.tcx.generics_of(id);
        for index in 0..generics.parent_count {
            let param = generics.param_at(index, self.tcx);
            if matches!(param.kind, ty::GenericParamDefKind::Const { .. }) {
                consts.push((param.index, self.const_arg(args.const_at(index), span)?));
            }
        }
        // A C-variadic default's own, which can't come after what it's given
        // past the rest (ADR 0428).
        if super::recognition::is_c_variadic(self.tcx, id)
            && (!own_params.is_empty() || !own_const_params(self.tcx, id).is_empty())
        {
            return Err(self.unsupported(span, "a C-variadic default given a dictionary or a const"));
        }
        let mut const_names = Vec::new();
        for index in own_const_params(self.tcx, id) {
            let name = self.fresh(generics.param_at(index as usize, self.tcx).name.as_str());
            consts.push((index, Expr::var(&name)));
            const_names.push(name);
        }
        // And the impl's, which its type, `Self`, names: `CAP` of an
        // `ArrayVec<T, CAP>`. One numbered as one of the trait's is would
        // be either: refused, as a method's own is (above).
        let names_const = |index: u32| {
            args.iter().any(|arg| {
                arg.walk()
                    .any(|part| matches!(part.as_const().map(|c| c.kind()), Some(ty::ConstKind::Param(p)) if p.index == index))
            })
        };
        for (index, value) in self.given.const_params.clone() {
            match consts.iter().any(|&(at, _)| at == index) {
                true if names_const(index) => {
                    return Err(self.unsupported(span, "a trait default's const parameter numbered as its impl's"));
                }
                true => {}
                false => consts.push((index, value)),
            }
        }
        let outer_consts = std::mem::replace(&mut self.given.const_params, consts);
        // The facts its caller gives of its own type parameters, `KMap`,
        // after its dictionaries (ADR 0466).
        let fact_mark = self.given.type_facts.len();
        let mut fact_names = Vec::new();
        for &(index, fact) in self.krate.type_facts.get(&id).into_iter().flatten() {
            if index < generics.parent_count as u32 {
                continue;
            }
            let of = drop_slot(self.tcx, self.krate.foreign, id, index);
            if self.given.type_facts.iter().any(|&(at, _, _)| at == of) {
                self.given.const_params = outer_consts;
                return Err(self.unsupported(span, "a trait default's type parameter's fact here"));
            }
            let name = self.fresh(&format!("{}{}", drop_word(self.tcx, of), fact.word()));
            self.given.type_facts.push((of, fact, Expr::var(&name)));
            // And in the impl's terms, as the body asks: `T::Item` of the
            // trait's `Self::Item`.
            let concrete = ty::EarlyBinder::bind(self.tcx, of)
                .instantiate(self.tcx, args)
                .skip_normalization();
            if concrete != of {
                self.given.type_facts.push((concrete, fact, Expr::var(&name)));
            }
            fact_names.push(name);
        }
        // A method's, or a constant's initializer, a trait's default (ADR 0176).
        let body = match self.krate.bodies.get(&id) {
            Some(body) => *body,
            None => id
                .as_local()
                .and_then(|local| self.krate.closures.get(&local).copied())
                .ok_or_else(|| self.unsupported(span, "a trait's default of another crate"))?,
        };
        let nested = super::Nested::Default {
            evidence: specialized,
            self_args: args,
            typing_env: ty::TypingEnv::post_analysis(self.tcx, id),
            drops,
            unsupported,
            items,
        };
        // Where it's called, after its arguments, of one that takes it, which
        // `Location::caller()` in it reads (ADR 0427).
        let location = self.takes_caller_location(id).then(|| self.fresh("location"));
        let outer_location = std::mem::replace(
            &mut self.given.caller_location,
            location.as_ref().map(|name| (id, Expr::var(name))),
        );
        let enclosing = match self.enter_body(body, id, nested) {
            Ok(enclosing) => enclosing,
            Err(error) => {
                self.given.caller_location = outer_location;
                return Err(error);
            }
        };
        let mut rest = Vec::new();
        // From its MIR (ADR 0364), a constant's initializer of nothing, its
        // value given back.
        // From its MIR, as the trait has it, `Self` the impl's type where
        // it's asked (ADR 0364).
        let signature = match &body.mir {
            Some(mir) => self.mir_signature(body, mir, &mut rest),
            None => Err(self.unsupported(span, "this default, without its MIR")),
        };
        self.given.const_params = outer_consts;
        self.given.caller_location = outer_location;
        self.given.type_facts.truncate(fact_mark);
        let (mut params, is_async) = signature?;
        params.extend(location.map(Into::into));
        params.extend(const_names.into_iter().map(Into::into));
        params.extend(own_params.into_iter().map(Into::into));
        params.extend(fact_names.into_iter().map(Into::into));
        // Only the drops it uses: most defaults drop nothing of their `Self`.
        let used = self.used_drops();
        rest.splice(0..0, self.leave_body(enclosing));
        let mut out: Vec<_> = made
            .into_iter()
            .filter(|(index, _)| used.contains(index))
            .map(|(_, stmt)| stmt)
            .collect();
        out.extend(rest);
        Ok(if is_async {
            Expr::async_arrow(params, out)
        } else {
            Expr::arrow(params, out)
        })
    }

    /// Can a `dyn` of the trait `id` be a pair (ADR 0049): its items are
    /// methods, and neither it nor a supertrait has type parameters. A `&mut
    /// self` method is given the pair, whose `value` a box's is (ADR 0099).
    pub(super) fn dyn_supported(&self, id: DefId) -> bool {
        self.tcx
            .associated_items(id)
            .in_definition_order()
            .all(|item| match self.tcx.def_kind(item.def_id) {
                DefKind::AssocFn => true,
                // A `dyn Source<Item = u32>` says what it is (ADR 0106).
                DefKind::AssocTy => self.tcx.generics_of(item.def_id).own_params.is_empty(),
                _ => false,
            })
            && self
                .tcx
                .explicit_super_clauses_of(id)
                .iter_identity_copied()
                .map(|item| item.skip_normalization())
                .all(|(clause, _)| match clause.kind().skip_binder() {
                    ty::ClauseKind::Trait(p) if operational(self.tcx, self.krate.foreign, p.trait_ref.def_id) => {
                        self.dyn_supported(p.trait_ref.def_id)
                    }
                    _ => true,
                })
    }

    /// `method` of the hand-written impl for the trait's `args`, called
    /// directly: `counterClone_clone(c)`.
    pub(super) fn impl_call(
        &mut self,
        method: DefId,
        args: ty::GenericArgsRef<'tcx>,
        mut values: Vec<Expr>,
        span: Span,
    ) -> R<Expr> {
        let args = self.tcx.erase_and_anonymize_regions(args);
        let instance = self
            .resolve_instance(method, args)?
            .filter(|i| self.is_rust_fn(i.def_id()))
            .ok_or_else(|| self.unsupported(span, "this implementation"))?;
        values.extend(self.evidence_args(instance.def_id(), instance.args, span)?);
        let called = Expr::call(self.fn_ref(instance.def_id()), values);
        Ok(self.fmt_result_value(instance.def_id(), instance.args, called))
    }
}
