//! Trait evidence stays separate from payloads: arguments for generics,
//! lazy dictionaries for impls, and `{ value, impl }` for trait objects.

mod calls;
mod dictionaries;
mod drop_params;
mod dyns;
mod evidence;
mod given;
mod impls;
mod routes;
mod words;

pub(super) use self::{drop_params::*, evidence::*, given::*, routes::*, words::*};

use super::bindings;
use super::display::Pretty;
use super::drops::Drops;
use super::passed_on::{Taker, passed_on};
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
/// whose `Copy` takes a copy function (ADR 0289), those whose `Clone`
/// takes its dictionary (ADR 0914, amended), and those whose `Iterator`,
/// or an extension of it, is given its dictionary (ADRs 0411, 0487), by
/// their indices, with the trait's.
#[derive(Clone, Copy)]
pub(super) struct Given<'a> {
    pub copied: &'a HashSet<(DefId, u32)>,
    pub clones: &'a HashSet<(DefId, u32)>,
    pub stepped: &'a HashSet<(DefId, u32, DefId)>,
    /// The type parameters a `Some` of which is the value (ADR 0051, amended).
    pub unboxed: &'a HashSet<(DefId, u32)>,
    /// What each function's JS takes its type parameters as, and the copies
    /// its calls need besides (ADR 0051, amended).
    pub js_given: &'a super::analysis::JsGiven,
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
/// the `Iterator` a trait that extends it on a type parameter given as it
/// is implies, `I: DoubleEndedIterator`'s or `T: Sorted`'s, as its
/// supertrait (ADRs 0488, 0511).
fn bounds_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    given: Given<'_>,
    clause: ty::Clause<'tcx>,
    id: DefId,
) -> impl Iterator<Item = ty::TraitRef<'tcx>> {
    let implied = clause.as_trait_clause().and_then(|p| {
        let tr = p.skip_binder().trait_ref;
        if !super::recognition::implies_iterator(tcx, tr.def_id) {
            return None;
        }
        let iterator = ty::TraitRef::new(tcx, std_item(tcx, StdItem::Iterator), [tr.self_ty()]);
        bound_of(tcx, foreign, given, ty::Binder::dummy(iterator).upcast(tcx), id)
    });
    implied.into_iter().chain(bound_of(tcx, foreign, given, clause, id))
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
    // `Vec<T>: Index<J>` holds by `J: SliceIndex<[T]>`, whose dictionary
    // indexes (ADR 0534).
    if let Some(slice_index) = super::recognition::index_by_slice_index(tcx, tr) {
        tr = slice_index;
    }
    // `Eq` promises more than `PartialEq`, but it's `PartialEq`'s `eq`
    // that's called.
    if is_std_def(tcx, tr.def_id, StdItem::Eq) {
        let partial_eq = tcx.require_lang_item(LangItem::PartialEq, tcx.def_span(id));
        tr = ty::TraitRef::new(tcx, partial_eq, [tr.self_ty(), tr.self_ty()]);
    }
    // A `Copy` bound only the crate's callers give, none of them a value
    // whose copy isn't itself, takes no copy function (ADR 0289).
    if tcx.is_lang_item(tr.def_id, LangItem::Copy)
        && super::analysis::copies_by_callers(tcx, foreign, id)
        && let ty::Param(param) = tr.self_ty().kind()
        && !given.copied.contains(&(tcx.typeck_root_def_id(id), param.index))
    {
        return None;
    }
    // A library's export's, as its contract says: one it takes no copy
    // function for is given none, as its JS takes none, the dictionaries
    // after it in their places (ADR 0914).
    if tcx.is_lang_item(tr.def_id, LangItem::Copy)
        && let ty::Param(param) = tr.self_ty().kind()
        && let Some(item) = foreign.item(id)
        && !(item.calling.param(param.index)).is_some_and(|param| param.has(crate::library::ParamFact::Copy))
    {
        return None;
    }
    // So is a `Clone` bound, whose clone of a value that's its own clone is
    // the value: only the crate's callers give it, none such, or a library's
    // export's that its contract says takes none (ADR 0914, amended).
    if tcx.is_lang_item(tr.def_id, LangItem::Clone)
        && let ty::Param(param) = tr.self_ty().kind()
        && match super::analysis::copies_by_callers(tcx, foreign, id) {
            true => super::analysis::clones_by_itself(tcx, foreign, given.clones, (id, param.index)),
            false => (foreign.item(id)).is_some_and(|item| {
                !(item.calling.param(param.index)).is_some_and(|param| param.has(crate::library::ParamFact::Clone))
            }),
        }
    {
        return None;
    }
    // An `Iterator` a type holds or a function gives back: its dictionary,
    // which steps it as it is (ADR 0411), and its extensions' (ADR 0487).
    if (is_std_def(tcx, tr.def_id, StdItem::Iterator) || super::recognition::is_iterator_extension(tcx, tr.def_id))
        && let ty::Param(param) = tr.self_ty().kind()
    {
        // This crate's, as its analysis says; a library's, as its manifest
        // does (ADR 0548).
        let steps = given.stepped.contains(&(id, param.index, tr.def_id)) || foreign.steps(id, param.index, tr.def_id);
        return steps.then_some(tr);
    }
    // An extension's of an associated type a parameter's bound names,
    // `U::IntoIter: DoubleEndedIterator` of std's `FlatMap`'s back, which
    // no `IntoIterator` dictionary carries: its own (ADR 0936).
    if super::recognition::is_iterator_extension(tcx, tr.def_id)
        && let ty::Alias(_, alias) = tr.self_ty().kind()
        && matches!(alias.kind, ty::Projection { .. })
        && matches!(alias.self_ty().kind(), ty::Param(_))
    {
        return Some(tr);
    }
    // An `IntoIterator` whose `IntoIter` is held as it is: its dictionary,
    // which makes it so (ADR 0552).
    if is_std_def(tcx, tr.def_id, StdItem::IntoIterator)
        && super::iterators::into_iter_held_as_it_is(tcx, id, tr.self_ty())
    {
        return Some(tr);
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

/// A trait's supertraits, as its dictionary has them: each one's key, its
/// trait's name, `PartialEq`, or with its arguments as the trait declares
/// them where it has two of one trait, `LabelU32` and `LabelString` of
/// `trait Both: Label<u32> + Label<String>`, so that a generic impl's
/// dictionary and its caller agree; and the supertrait with
/// `args`, its own lifetimes erased: a `for<'a> B<&'a ()>` is one
/// dictionary, and rustc's trait selection takes no bound ones (ADR 0106).
/// The supertraits a dictionary of `trait_id` has an entry of, each by its
/// name: those whose dictionaries are passed, not `Iterator`'s of a crate's
/// extension of it, which a function is given apart (ADRs 0411, 0548). The
/// one answer, of what's made and the route that reads it.
pub(super) fn held_supertraits<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    trait_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Vec<(String, ty::TraitRef<'tcx>)> {
    (supertraits(tcx, trait_id, args).into_iter())
        .filter(|(_, tr, _)| operational(tcx, foreign, tr.def_id))
        .map(|(name, tr, _)| (name, tr))
        .collect()
}

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
