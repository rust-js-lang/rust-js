//! Trait evidence stays separate from payloads: arguments for generics,
//! lazy dictionaries for impls, and `{ value, impl }` for trait objects.

use super::bindings;
use super::display::Pretty;
use super::drops::Drops;
use super::recognition::{
    Recognition, StdItem, TraitCall, TypeFact, in_std_dictionary, is_std_def, is_std_method, is_writer_default,
    known_derive, std_item,
};
use super::representation::{Num, const_js, eval_const};
use super::{FnCx, R, lower_first};
use crate::js::{self, Expr, Op, Prop, StmtKind};
use crate::runtime::Helper;
use rustc_hir::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::{BinOp, UnOp};
use rustc_middle::traits::{BuiltinImplSource, ImplSource};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeFoldable, TypeVisitableExt};
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
pub(super) fn bounds<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    copied: &HashSet<(DefId, u32)>,
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
        let clause = clause.skip_normalization();
        if let Some(tr) = bound_of(tcx, foreign, copied, clause, id)
            && !result.contains(&tr)
        {
            result.push(tr);
        }
    }
    result
}

/// Might any value have a destructor: has the crate a `Drop` impl of its own,
/// or a library, whose types might? Where it hasn't, what a caller gives
/// generic code without a drop function has nothing to drop (ADR 0106).
/// A generic trait method's own type parameters its caller gives a drop for
/// (ADR 0163): each one the trait declares that isn't `Copy`. Decided by the
/// trait's declaration, which a caller through a dictionary knows, so it and
/// each impl agree; of an impl's method, its own type parameters at those
/// places, as their indices are its.
pub(super) fn own_drop_params(tcx: TyCtxt<'_>, method: DefId) -> Vec<u32> {
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
    let own = types(method);
    types(declared)
        .into_iter()
        .enumerate()
        .filter(|(_, p)| !tcx.type_is_copy_modulo_regions(typing_env, Ty::new_param(tcx, p.index, p.name)))
        .filter_map(|(at, _)| own.get(at).map(|p| p.index))
        .collect()
}

pub(super) fn may_have_destructors(tcx: TyCtxt<'_>, foreign: &super::library::Foreign<'_, '_>) -> bool {
    let drop_trait = tcx.lang_items().drop_trait();
    drop_trait.is_some_and(|id| tcx.all_local_trait_impls(()).contains_key(&id)) || foreign.any()
}

/// A clause of a function's, as its evidence is for it, if it's given one.
fn bound_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    copied: &HashSet<(DefId, u32)>,
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
        && !copied.contains(&(tcx.typeck_root_def_id(id), param.index))
    {
        return None;
    }
    (operational(tcx, foreign, tr.def_id) && !is_marker(tcx, foreign, tr.def_id)).then_some(tr)
}

/// A trait of the crate's or a library's with nothing in it: no items, and
/// each supertrait one too, or one of no dictionary, `Send`. A bound of it
/// passes no dictionary, as nothing would read it: `named<T: Marker>(value)`
/// is `named(value)`. A `dyn` of it still carries its impl's, whose `$drop`
/// drops it (ADR 0098). `Copy` isn't one: its dictionary copies.
pub(super) fn is_marker(tcx: TyCtxt<'_>, foreign: &super::library::Foreign<'_, '_>, id: DefId) -> bool {
    (id.is_local() || foreign.in_library(id))
        && tcx.associated_items(id).in_definition_order().next().is_none()
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
    copied: &HashSet<(DefId, u32)>,
    id: DefId,
) -> Vec<ty::TraitRef<'tcx>> {
    let own: Vec<_> = tcx
        .clauses_of(id)
        .clauses
        .iter()
        .filter_map(|&(clause, _)| bound_of(tcx, foreign, copied, clause, id))
        .collect();
    bounds(tcx, foreign, copied, id)
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
        // but `Error`'s has its `Display` and `Debug` (ADR 0141).
        if !self.recognition.is_rust_trait(from.def_id) && !is_std_pair_trait(self.recognition.tcx, from.def_id) {
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

/// `Display` or `Error`: a std trait whose `dyn` is a pair (ADR 0141).
fn is_std_pair_trait(tcx: TyCtxt<'_>, id: DefId) -> bool {
    [StdItem::Display, StdItem::Error]
        .into_iter()
        .any(|item| is_std_def(tcx, id, item))
}

/// An associated type's drop in its impl's dictionary, `$dropOffset`: a
/// name no Rust method can have, as `$drop` is (ADR 0178).
pub(super) fn item_drop_key(tcx: TyCtxt<'_>, item: DefId) -> String {
    format!("$drop{}", tcx.item_name(item))
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

    /// `resolve_instance` of a call whose arguments a copied default's
    /// `Self` was replaced in (ADR 0049): they're the impl's, which resolve in
    /// the impl's typing environment, as its own body's types don't.
    pub(super) fn resolve_self_instance(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Result<Option<ty::Instance<'tcx>>, rustc_span::ErrorGuaranteed> {
        let typing_env = self.given.self_env.unwrap_or(self.typing_env);
        // Normalized first, or not resolved, as `resolve_instance`'s are.
        let Ok(args) = self
            .tcx
            .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(args))
        else {
            return Ok(None);
        };
        ty::Instance::try_resolve(self.tcx, typing_env, def_id, args)
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
        self.given
            .type_facts
            .iter()
            .find(|&&(at, given, _)| at == index && given == fact)
            .map(|(_, _, value)| value.clone())
    }

    pub(super) fn evidence_params(&mut self, id: DefId) -> Vec<js::Pattern> {
        // Each const parameter's value first, `N`, in the order they're
        // declared, the impl's before the method's (ADR 0107).
        let mut params: Vec<js::Pattern> = Vec::new();
        for param in const_params(self.tcx, id) {
            let name = self.fresh(param.name.as_str());
            self.given.const_params.push((param.index, Expr::var(&name)));
            params.push(name.into());
        }
        params.extend(
            bounds(self.tcx, self.krate.foreign, self.krate.copied, id)
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
            let param = self.tcx.generics_of(id).param_at(index as usize, self.tcx);
            let word = match fact {
                TypeFact::Size => "Size",
                TypeFact::Align => "Align",
                TypeFact::Name => "Name",
            };
            let name = self.fresh(&format!("{}{word}", param_word(param.name)));
            self.given.type_facts.push((index, fact, Expr::var(&name)));
            params.push(name.into());
        }
        // Then a drop function for each type parameter a caller gives a value
        // with a destructor, `dropT` (ADR 0098).
        for &index in self.krate.drop_params.get(&id).into_iter().flatten() {
            let param = self.tcx.generics_of(id).param_at(index as usize, self.tcx);
            let name = self.fresh(&format!("drop{}", param_word(param.name)));
            self.give_drop_param(index, name.clone());
            params.push(name.into());
        }
        params
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

    /// Is a dictionary for `tr` given? Without building it.
    pub(super) fn has_evidence(&self, tr: ty::TraitRef<'tcx>) -> bool {
        self.evidence_query().has_evidence(tr)
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

    pub(super) fn dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        // `<Words as Source>::Item: Debug` of a bound instantiated: `String: Debug`.
        let tr = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(tr))
            .unwrap_or(tr);
        // serde's: the function that writes or reads the type (ADR 0081).
        match super::serde::serde_trait(self.tcx, tr.def_id) {
            Some(true) => return self.json_writer(tr.self_ty(), span),
            Some(false) => return self.json_reader(tr.self_ty(), span),
            None => {}
        }
        if let Some(found) = self.evidence_for(tr) {
            return Ok(found);
        }
        // A marker's of a type a bound gave none for, as none passes one: a
        // type parameter made a `dyn`, `Box::new(value)` of a `T: Marker`.
        // What its impl's would have, its drop and its supertraits', made here.
        if tr.self_ty().has_param() && is_marker(self.tcx, self.krate.foreign, tr.def_id) {
            let mut props = Vec::new();
            for (name, supertrait, _) in supertraits(self.tcx, tr.def_id, tr.args) {
                if operational(self.tcx, self.krate.foreign, supertrait.def_id) {
                    let dictionary = self.dictionary(supertrait, span)?;
                    props.push(Prop::Field(
                        name,
                        Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
                    ));
                }
            }
            if self.drops(tr.self_ty()) == Drops::Runs
                && let Some(drop) = self.drop_function(tr.self_ty(), span)?
            {
                props.push(Prop::Field("$drop".into(), drop));
            }
            return Ok(Expr::object(props));
        }
        // Derived and std impls of `Default` and `Clone` (ADR 0052).
        let ty = tr.self_ty();
        // std's `Error` of one of its parse errors: its `Display` and `Debug`, as
        // a crate's impl's dictionary has them, and no `source`, which none of
        // them has (ADR 0186).
        if is_std_def(self.tcx, tr.def_id, StdItem::Error) && self.is_parse_error(ty) {
            let mut props = Vec::new();
            for (name, supertrait, _) in supertraits(self.tcx, tr.def_id, tr.args) {
                if !operational(self.tcx, self.krate.foreign, supertrait.def_id) {
                    continue;
                }
                let dictionary = self.dictionary(supertrait, span)?;
                props.push(Prop::Field(
                    name,
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
                ));
            }
            let none = Expr::arrow(
                Vec::new(),
                vec![StmtKind::Return(Some(Expr::undefined())).at(js::Span::NONE)],
            );
            props.push(Prop::Field("source".into(), none));
            return Ok(Expr::object(props));
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
        // written is added to it; of a `&mut` to one taken by value, to the
        // box the `&mut` given is to (ADR 0180). A `&mut` to an object
        // writer is the object, as is the `&mut` given to its own methods.
        if is_std_def(self.tcx, tr.def_id, StdItem::FmtWrite) {
            let text =
                |ty: Ty<'tcx>| self.is_lang_adt(ty, LangItem::String) || self.recognition().is_formatter_type(ty);
            if text(ty) {
                self.runtime.insert(Helper::StringWriter);
                return Ok(Expr::var("$stringWriter"));
            }
            if let ty::Ref(_, inner, Mutability::Mut) = *ty.kind() {
                if text(inner) {
                    self.runtime.insert(Helper::StringWriter);
                    return Ok(Expr::var("$mutStringWriter"));
                }
                if self.is_object(inner) {
                    return self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [inner]), span);
                }
            }
        }
        if is_std_def(self.tcx, tr.def_id, StdItem::AsRef) && !self.has_user_impl(tr.def_id, ty) {
            let (from, to) = (ty.peel_refs(), tr.args.type_at(1));
            let sequence = |t: Ty<'tcx>| t.is_array() || t.is_slice() || self.is_vec_like(t);
            if (self.is_string_like(from) && to.is_str()) || (sequence(from) && to.is_slice()) {
                let same = Expr::arrow(
                    vec!["value".into()],
                    vec![StmtKind::Return(Some(Expr::var("value"))).at(js::Span::NONE)],
                );
                return Ok(Expr::object(vec![Prop::Field("as_ref".into(), same)]));
            }
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
        // `x.into()` of a `T: Into<U>` (ADR 0108): std's conversion, the
        // crate's `From`, or of a `T` to itself, the value.
        if is_std_def(self.tcx, tr.def_id, StdItem::Into) {
            let into = self.tcx.associated_item_def_ids(tr.def_id)[0];
            let target = tr.args.type_at(1);
            let from = std_item(self.tcx, StdItem::From);
            let from = self.tcx.associated_item_def_ids(from)[0];
            let from_args = self.tcx.mk_args(&[target.into(), ty.into()]);
            let value = if let Some(known) = self.recognition().classify(into, tr.args)
                && let Some(f) =
                    self.std_fn_value(known, Ty::new_fn_def(self.tcx, into, ty::Binder::dummy(tr.args)), span)?
            {
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
            } else if self.tcx.erase_and_anonymize_regions(ty) == self.tcx.erase_and_anonymize_regions(target) {
                Expr::arrow(
                    vec!["value".into()],
                    vec![StmtKind::Return(Some(Expr::var("value"))).at(js::Span::NONE)],
                )
            } else {
                return Err(self.no_evidence(span, tr));
            };
            return Ok(Expr::object(vec![Prop::Field("into".into(), value)]));
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
        let selected = self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr));
        // The crate's own impl's accessor, or one a library exports (ADR 0100).
        if let Ok(ImplSource::UserDefined(imp)) = selected
            && (self.krate.trait_impls.contains(&imp.impl_def_id) || self.krate.foreign.item(imp.impl_def_id).is_some())
        {
            let callee = self.fn_ref(imp.impl_def_id);
            let args = self.evidence_args(imp.impl_def_id, imp.args, span)?;
            return Ok(Expr::call(callee, args));
        }
        // A trait object's: Rust's built-in `impl Trait for dyn Trait`.
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
        for item in self.tcx.associated_items(tr.def_id).in_definition_order() {
            if self.tcx.def_kind(item.def_id) != DefKind::AssocTy
                || item.is_impl_trait_in_trait()
                || !self.tcx.generics_of(item.def_id).own_params.is_empty()
            {
                continue;
            }
            let projection = Ty::new_projection(self.tcx, ty::IsRigid::No, item.def_id, tr.args);
            let Ok(item_ty) = self
                .tcx
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection))
            else {
                continue;
            };
            if self.drops(item_ty) == Drops::Runs
                && let Some(drop) = self.drop_function(item_ty, span)?
            {
                props.push(Prop::Field(item_drop_key(self.tcx, item.def_id), drop));
            }
        }
        Ok(props)
    }

    fn object_dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        let object = Expr::var("object");
        let principal = self
            .dyn_trait_ref(tr.self_ty(), tr.self_ty())
            .ok_or_else(|| self.no_evidence(span, tr))?;
        let mut props = Vec::new();
        let returning = |value: Expr| Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(value)).at(js::Span::NONE)]);
        for (name, supertrait, _) in supertraits(self.tcx, tr.def_id, tr.args) {
            if operational(self.tcx, self.krate.foreign, supertrait.def_id) {
                props.push(Prop::Field(name, returning(self.dictionary(supertrait, span)?)));
            }
        }
        for (name, bound) in item_bounds(self.tcx, tr.def_id, tr.args) {
            if operational(self.tcx, self.krate.foreign, bound.def_id) {
                props.push(Prop::Field(name, returning(self.dictionary(bound, span)?)));
            }
        }
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
                true => Expr::member(object.clone(), "value"),
                false => object.clone(),
            };
            let dictionary = self
                .super_evidence(principal, tr, Expr::member(pair.clone(), "impl"))
                .ok_or_else(|| self.no_evidence(span, tr))?;
            let this = match mutable {
                true => pair,
                false => Expr::member(pair, "value"),
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

    pub(super) fn evidence_args(&mut self, id: DefId, args: ty::GenericArgsRef<'tcx>, span: Span) -> R<Vec<Expr>> {
        self.check_drops_given(id, args, span)?;
        let mut values = const_params(self.tcx, id)
            .into_iter()
            .map(|param| self.const_arg(args.const_at(param.index as usize), span))
            .collect::<R<Vec<_>>>()?;
        for bound in bounds(self.tcx, self.krate.foreign, self.krate.copied, id) {
            let bound = ty::EarlyBinder::bind(self.tcx, bound)
                .instantiate(self.tcx, args)
                .skip_normalization();
            values.push(self.dictionary(bound, span)?);
        }
        // Each fact it asks of a type parameter, of the type given for it (ADR 0145).
        let facts = self.krate.type_facts.get(&id).cloned().unwrap_or_default();
        for (index, fact) in facts {
            values.push(self.type_fact_value(args.type_at(index as usize), fact, span)?);
        }
        // Each drop function it takes: a type's with nothing to drop is none,
        // left out at the end (ADR 0098).
        let mut drops = Vec::new();
        let given = match self.krate.foreign.item(id) {
            Some(item) => item.drops.as_slice(),
            None => self.krate.drop_params.get(&id).map_or(&[][..], Vec::as_slice),
        };
        for &index in given {
            drops.push(self.drop_function(args.type_at(index as usize), span)?);
        }
        while matches!(drops.last(), Some(None)) {
            drops.pop();
        }
        values.extend(drops.into_iter().map(|drop| drop.unwrap_or_else(Expr::undefined)));
        Ok(values)
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
                let b = super::numbers::shift_amount_of(op, b.expect("two operands"), ty, tr.args.type_at(1));
                self.binary(op, a, b, None, ty, span)
            }
            Err(op) => self.unary(op, a, ty, span),
        }
    }

    /// A generic impl's constant of its parameters (ADR 0176): its initializer,
    /// `() => [TConstZero.ZERO]`, lowered in its dictionary, which has the
    /// impl's evidence.
    /// The impl's own, `id`; a trait's default, of `Self`, isn't yet.
    fn impl_const_getter(&mut self, id: Option<DefId>, span: Span) -> R<Expr> {
        let id = id.ok_or_else(|| self.unsupported(span, "a generic impl's default constant of its parameters"))?;
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
        let mut stmts = Vec::new();
        let value = self.expr(body.expr, &mut stmts);
        self.leave_body(enclosing)?;
        stmts.push(StmtKind::Return(Some(value?)).at(js::Span::NONE));
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
            // The pair is read twice, so one with effects goes in a `const`
            // first. It's still first: `operands` put anything before it that
            // needed statements in `const`s of its own.
            let pair = if receiver.has_effects() {
                self.spill("receiver", receiver, out)
            } else {
                receiver
            };
            let principal = self.dyn_trait_ref(tr.self_ty(), tr.self_ty()).unwrap();
            let dictionary = self
                .super_evidence(principal, tr, Expr::member(pair.clone(), "impl"))
                .ok_or_else(|| self.unsupported(span, "this trait object supertrait"))?;
            // A `&mut self` method is given the pair, whose `value` a box's is: a
            // number's impl writes the place it reads (ADR 0099).
            let receiver = self
                .tcx
                .fn_sig(id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs()[0];
            let this = match receiver.kind() {
                ty::Ref(_, _, Mutability::Mut) => pair,
                _ => Expr::member(pair, "value"),
            };
            values.insert(0, this);
            values.extend(self.own_evidence(id, generic_args, span)?);
            let called = Expr::call(Expr::member(dictionary, bindings::fn_name(self.tcx, id)), values);
            return Ok(Some(self.fmt_result_value(id, generic_args, called)));
        }
        if let Some(instance) = self.resolve_self_instance(id, generic_args)?
            && self.is_rust_fn(instance.def_id())
            && self.tcx.trait_of_assoc(instance.def_id()).is_none()
        {
            let mut values = values;
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
                self.eq_value(a, b, tr.self_ty(), span, out)?
            };
            return Ok(Some(if negate { super::std_impls::negate(eq) } else { eq }));
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
        if (super::recognition::value_operator(self.tcx, trait_id).is_some()
            || is_std_def(self.tcx, trait_id, StdItem::Into))
            && !tr.args.has_non_region_param()
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
        // `s.borrow()` of std's where it's the same JS value: the value itself,
        // written in place, as the types are known (ADR 0167).
        if is_std_def(self.tcx, trait_id, StdItem::Borrow)
            && self.recognition().borrows_as_itself(tr.self_ty(), tr.args.type_at(1))
        {
            return Ok(None);
        }
        // Where the writer is known, `write!(c, ..)` is its own `write_str`, or
        // a string's `+=`, written in place (ADRs 0148, 0166): a dictionary is
        // for generic code (ADR 0180).
        if is_std_def(self.tcx, trait_id, StdItem::FmtWrite) && !tr.args.has_non_region_param() {
            return Ok(None);
        }
        // A std trait's dictionary has only its required methods, and those
        // provided ones it's given.
        if operational(self.tcx, self.krate.foreign, trait_id)
            && !self.is_rust_trait(trait_id)
            && self.tcx.defaultness(id).has_value()
            && !in_std_dictionary(self.tcx, id)
        {
            let what = format!("calling `{}`", self.tcx.def_path_str(id));
            return Err(self.unsupported(span, &what));
        }
        if operational(self.tcx, self.krate.foreign, trait_id) {
            let dictionary = self.dictionary(tr, span)?;
            let mut values = values;
            values.extend(self.own_evidence(id, generic_args, span)?);
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
    fn own_evidence(&mut self, id: DefId, generic_args: ty::GenericArgsRef<'tcx>, span: Span) -> R<Vec<Expr>> {
        // Its own const parameters' values first, `repeat::<3>`'s `3` (ADR 0135).
        let mut values = own_const_params(self.tcx, id)
            .into_iter()
            .map(|index| self.const_arg(generic_args.const_at(index as usize), span))
            .collect::<R<Vec<_>>>()?;
        for bound in own_bounds(self.tcx, self.krate.foreign, self.krate.copied, id) {
            values.push(
                self.dictionary(
                    ty::EarlyBinder::bind(self.tcx, bound)
                        .instantiate(self.tcx, generic_args)
                        .skip_normalization(),
                    span,
                )?,
            );
        }
        // Then a drop for each of its own type parameters the trait declares
        // one for, of the type given for it: none, left out at the end, for
        // one with nothing to drop (ADR 0163).
        let mut drops = Vec::new();
        for index in own_drop_params(self.tcx, id) {
            drops.push(self.drop_function(generic_args.type_at(index as usize), span)?);
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
        span: Span,
    ) -> R<Vec<Expr>> {
        let own = own_bounds(self.tcx, self.krate.foreign, self.krate.copied, method);
        // Its const parameters' values first, the impl's and its own (ADR 0135).
        let mut values = const_params(self.tcx, method)
            .into_iter()
            .map(|param| self.const_arg(args.const_at(param.index as usize), span))
            .collect::<R<Vec<_>>>()?;
        for bound in bounds(self.tcx, self.krate.foreign, self.krate.copied, method) {
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
        // Its drops are the impl's type parameters' (ADR 0098): its own, a
        // caller through a dictionary gives none of, are refused (`validate`).
        let mut drops = Vec::new();
        for &index in self.krate.drop_params.get(&method).map_or(&[][..], Vec::as_slice) {
            drops.push(self.drop_function(args.type_at(index as usize), span)?);
        }
        while matches!(drops.last(), Some(None)) {
            drops.pop();
        }
        values.extend(drops.into_iter().map(|drop| drop.unwrap_or_else(Expr::undefined)));
        Ok(values)
    }

    /// The trait a `dyn` of `ty` is a pair of, `{ value, impl }` (ADR 0049):
    /// one rust-js compiled, or std's `Display` or `Error`, whose
    /// dictionaries it makes (ADR 0141). A `dyn Debug` is a string instead.
    pub(super) fn dynamic_trait(&self, ty: Ty<'tcx>) -> Option<DefId> {
        let inner = self.pointee(ty);
        match inner.kind() {
            ty::Dynamic(predicates, ..) => predicates
                .principal_def_id()
                .filter(|&id| self.is_rust_trait(id) || self.is_std_pair_trait(id)),
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
        let Some(dictionary) = self.super_evidence(from, to, Expr::member(pair.clone(), "impl")) else {
            return Ok(None);
        };
        // Read twice: once for its dictionary, once for its value.
        if !pair.reads_same() {
            return Err(self.unsupported(span, &format!("showing a `{ty}` made where it's shown")));
        }
        let mut values = vec![Expr::member(pair, "value")];
        values.extend(options);
        Ok(Some(Expr::call(Expr::member(dictionary, "fmt"), values)))
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
        if self.is_string_like(from) {
            self.runtime.insert(Helper::StringError);
            return Ok(Some(Expr::call(Expr::var("$stringError"), Vec::new())));
        }
        // One of std's parse errors, which is its message (ADR 0063).
        if self.is_parse_error(from)
            && let ty::Adt(adt, _) = from.kind()
        {
            self.runtime.insert(Helper::ParseErrorDyn);
            let name = self.tcx.item_name(adt.did());
            return Ok(Some(Expr::call(
                Expr::var("$parseErrorDyn"),
                vec![Expr::str(name.as_str())],
            )));
        }
        // serde_json's, shown as serde_json shows it.
        if self.is_json_error(from) {
            self.runtime.insert(Helper::JsonErrorDyn);
            return Ok(Some(Expr::call(Expr::var("$jsonErrorDyn"), Vec::new())));
        }
        let error = std_item(self.tcx, StdItem::Error);
        let tr = ty::TraitRef::new(self.tcx, error, [from]);
        self.dictionary(tr, span).map(Some)
    }

    /// `Error::source`, which a `dyn Error`'s dictionary has (ADR 0141).
    fn is_error_source(&self, id: DefId) -> bool {
        is_std_method(self.tcx, id, StdItem::Error, "source")
    }

    /// `Display` or `Error`: a std trait whose `dyn` is a pair (ADR 0141).
    pub(super) fn is_std_pair_trait(&self, id: DefId) -> bool {
        is_std_pair_trait(self.tcx, id)
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

    fn pointee(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        match ty.kind() {
            ty::Ref(_, inner, _) => *inner,
            ty::Adt(_, args) if self.is_std_wrapper(ty) => args.type_at(0),
            _ => ty,
        }
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
        // A pointer of the crate's own, as `#[derive(CoercePointee)]` makes
        // one, would hold a `dyn`'s value and impl where it holds the value.
        if let ty::Adt(adt, _) = target.kind()
            && (adt.did().is_local() || self.krate.foreign.in_library(adt.did()))
        {
            return Err(self.unsupported(span, &format!("unsizing a `{target}`")));
        }
        // A `&dyn Debug` is the string it shows (ADR 0060).
        if self.is_dyn_debug(target) && !self.is_dyn_debug(source) {
            return self.dyn_debug_string(value, self.pointee(source), span);
        }
        // `&Fat<Bar>` to `&Fat<dyn ToBar>`: the struct's last field would
        // be a `dyn`'s value and impl, or a `dyn Debug`'s string, which it
        // isn't. One ending in a slice, or a `dyn FnMut`, the JS function,
        // is the same value either way.
        let pointee = self.pointee(target);
        let tail = self.tcx.struct_tail_for_codegen(pointee, self.typing_env);
        if pointee.is_adt()
            && matches!(tail.kind(), ty::Dynamic(traits, ..)
                if traits.principal_def_id().is_some_and(|id| self.is_rust_trait(id)) || self.is_dyn_debug(tail))
        {
            return Err(self.unsupported(span, &format!("a `{pointee}`, whose last field is a `dyn`")));
        }
        if self.dynamic_trait(target).is_none() {
            return Ok(value);
        }
        self.check_value_ty(target, span)?;
        if self.dynamic_trait(source).is_some() {
            let self_ty = self.pointee(source);
            let from = self.dyn_trait_ref(source, self_ty).unwrap();
            let to = self.dyn_trait_ref(target, self_ty).unwrap();
            // The same trait (a `Box<dyn T>` to a `Box<dyn T>`): the same pair.
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
                .super_evidence(from, to, Expr::member(pair.clone(), "impl"))
                .ok_or_else(|| self.unsupported(span, "this trait upcast"))?;
            // A `&mut dyn Sub` as a `&mut dyn Super`: a pair on the first's
            // `value`, which its `&mut self` methods write (ADR 0099).
            if matches!(target.kind(), ty::Ref(_, _, Mutability::Mut)) {
                return Ok(Expr::pair(Expr::member(pair, "value"), dictionary));
            }
            return Ok(Expr::object(vec![
                Prop::Field("value".into(), Expr::member(pair, "value")),
                Prop::Field("impl".into(), dictionary),
            ]));
        }
        let tr = self.dyn_trait_ref(target, self.pointee(source)).unwrap();
        let dictionary = self.dictionary(tr, span)?;
        // `&mut n as &mut dyn Trait` of a number: the pair reads and writes
        // the place the cell does (ADR 0099). A temporary's box is the pair's.
        if self.is_cell(source) {
            return Ok(match value.kind {
                js::ExprKind::Handle(place) => Expr::pair(*place, dictionary),
                js::ExprKind::Object(mut props) if matches!(props.as_slice(), [Prop::Field(name, _)] if name == "value") =>
                {
                    props.push(Prop::Field("impl".into(), dictionary));
                    Expr::object(props)
                }
                _ => {
                    let cell = if value.reads_same() {
                        value
                    } else {
                        self.spill("cell", value, out)
                    };
                    Expr::pair(Expr::member(cell, "value"), dictionary)
                }
            });
        }
        Ok(Expr::object(vec![
            Prop::Field("value".into(), value),
            Prop::Field("impl".into(), dictionary),
        ]))
    }

    pub(super) fn lower_dictionary(&mut self, id: DefId, cache: &str) -> R<js::Function> {
        let span = self.tcx.def_span(id);
        // An impl of a generic trait, `Convert<f64>`, or of a std one,
        // `PartialEq<Rhs>`, is for its arguments: a dictionary of its own.
        let tr = self.tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
        let params = self.evidence_params(id);
        let mut props = Vec::new();
        // What the dictionary's entries share, named once before it's made.
        let mut shared = Vec::new();
        for (name, supertrait, _) in supertraits(self.tcx, tr.def_id, tr.args) {
            if operational(self.tcx, self.krate.foreign, supertrait.def_id) {
                let dictionary = self.dictionary(supertrait, span)?;
                props.push(Prop::Field(
                    name,
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
                ));
            }
        }
        // And each bound its trait declares on an associated type, the impl's:
        // `LabelDisplay` of `type Label = u32` is `u32`'s `Display`.
        for (name, bound) in item_bounds(self.tcx, tr.def_id, tr.args) {
            if operational(self.tcx, self.krate.foreign, bound.def_id) {
                let dictionary = self.dictionary(bound, span)?;
                props.push(Prop::Field(
                    name,
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
                ));
            }
        }
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
                let Some(value) = eval_const(self.tcx, self.typing_env, item.def_id, tr.args, span)
                    .and_then(|value| const_js(self.tcx, value))
                else {
                    // Of its parameters, `Wrapping(T::ZERO)`: its initializer, read
                    // each time, as a constant is, of the impl's evidence (ADR 0176).
                    let own = self.tcx.impl_item_implementor_ids(id).get(&item.def_id).copied();
                    let getter = self.impl_const_getter(own, span)?;
                    props.push(Prop::Getter(bindings::fn_name(self.tcx, item.def_id), getter));
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
            if self.tcx.def_kind(item.def_id) != DefKind::AssocFn {
                continue;
            }
            // A std trait's provided methods, like `Clone::clone_from`,
            // aren't in its dictionary: nothing calls them through it. But a
            // `dyn Error` calls `source` (ADR 0141), and generic code writing
            // to any writer `write_char` and `write_fmt` (ADR 0180).
            let source = self.is_error_source(item.def_id);
            if !self.is_rust_trait(tr.def_id)
                && self.tcx.defaultness(item.def_id).has_value()
                && !in_std_dictionary(self.tcx, item.def_id)
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
            // std's own `write_char` and `write_fmt`: its `write_str`, given a
            // `char`'s text or a `write!`'s, as std's give it (ADR 0180).
            if is_writer_default(self.tcx, item.def_id) && !self.krate.fns.contains_key(&method) {
                let write_str = props.iter().find_map(|prop| match prop {
                    Prop::Field(name, value) if name == "write_str" => Some(value.clone()),
                    _ => None,
                });
                let write_str = write_str.ok_or_else(|| self.unsupported(span, "a writer without its `write_str`"))?;
                // One function for the three, named once where it's more than
                // a name: `{ write_str, write_char: write_str, .. }`.
                let write_str = match write_str.kind {
                    js::ExprKind::Var(_) | js::ExprKind::Symbol(_) => write_str,
                    _ => {
                        let name = self.fresh("write_str");
                        shared.push(StmtKind::Const(name.clone(), write_str).at(js::Span::NONE));
                        for prop in &mut props {
                            if let Prop::Field(field, value) = prop
                                && field == "write_str"
                            {
                                *value = Expr::var(&name);
                            }
                        }
                        Expr::var(&name)
                    }
                };
                props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), write_str));
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
            let declared: Vec<_> = own_bounds(self.tcx, self.krate.foreign, self.krate.copied, item.def_id)
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
            // A `&mut self` its caller through the dictionary gives in a box,
            // as a generic `&mut Self` is (ADR 0099), to a method that takes the
            // object itself: what's in it.
            for (i, value) in values.iter_mut().enumerate() {
                if self.param_is_box(item.def_id, i) && !self.param_is_box(method, i) {
                    *value = Expr::member(std::mem::replace(value, Expr::undefined()), "value");
                }
            }
            // The drops its caller gives for its own type parameters, which it
            // passes on as the method's, where the method takes them (ADR 0163).
            let own_drops: Vec<u32> = match self.krate.drop_params.get(&method) {
                Some(taken) if own_drop_params(self.tcx, method).iter().any(|i| taken.contains(i)) => {
                    own_drop_params(self.tcx, item.def_id)
                }
                _ => Vec::new(),
            };
            let mut drop_names = Vec::new();
            let mut replaced = Vec::new();
            for &index in &own_drops {
                let param = self.tcx.generics_of(item.def_id).param_at(index as usize, self.tcx);
                let name = self.fresh(&format!("drop{}", param_word(param.name)));
                replaced.push((index, self.lend_drop_param(index, name.clone())));
                drop_names.push(name);
            }
            // Everything the method takes that its caller through the dictionary
            // doesn't give: its dictionaries, then its drops (ADR 0098), which
            // are this impl's own, and its own type parameters' it's given.
            let evidence = match declared.is_empty() {
                true => self.evidence_args(method, instance.args, span),
                false => self.method_evidence(method, instance.args, &declared, &names, span),
            };
            self.given.const_params.truncate(mark);
            for (index, before) in replaced {
                self.return_drop_param(index, before);
            }
            let evidence = evidence?;
            params.extend(names);
            params.extend(drop_names);
            values.extend(evidence);
            // One that passes on just what it's given, in order, is the method.
            let passed = values.len() == params.len()
                && values
                    .iter()
                    .zip(&params)
                    .all(|(value, param)| matches!(&value.kind, js::ExprKind::Var(v) if v == param));
            let value = if passed {
                callee
            } else {
                Expr::arrow(
                    params.into_iter().map(Into::into).collect(),
                    vec![StmtKind::Return(Some(Expr::call(callee, values))).at(js::Span::NONE)],
                )
            };
            props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), value));
        }
        // Its type's drop, as Rust's vtable has it: what dropping a `dyn` of it
        // runs (ADR 0098). A name no Rust method can have.
        if self.drops(tr.self_ty()) == Drops::Runs
            && let Some(drop) = self.drop_function(tr.self_ty(), span)?
        {
            props.push(Prop::Field("$drop".into(), drop));
        }
        props.extend(self.item_drops(tr, span)?);
        let object = Expr::object(props);
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
            // Keyed by a const parameter's value first, a number, which only a
            // `Map` holds (ADR 0107).
            let map = if self.given.const_params.is_empty() {
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
            self.runtime.insert(Helper::TraitImpl);
            // One dictionary for each set of what it's given: its const
            // parameters' values, its dictionaries, and its drops, which may be
            // none.
            let keys = Expr::array(
                self.given
                    .const_params
                    .iter()
                    .map(|(_, value)| value.clone())
                    .chain(self.given.evidence.iter().map(|(_, value)| value.clone()))
                    .chain(self.given_drops().iter().map(|name| Expr::var(name)))
                    .collect(),
            );
            let make = Expr::arrow(
                Vec::new(),
                [shared, vec![StmtKind::Return(Some(object)).at(js::Span::NONE)]].concat(),
            );
            body.push(
                StmtKind::Return(Some(Expr::call(
                    Expr::var("$traitImpl"),
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
        let own = own_bounds(self.tcx, self.krate.foreign, self.krate.copied, id);
        let mut own_params = Vec::new();
        for bound in bounds(self.tcx, self.krate.foreign, self.krate.copied, id) {
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
        let mut const_names = Vec::new();
        for index in own_const_params(self.tcx, id) {
            let name = self.fresh(generics.param_at(index as usize, self.tcx).name.as_str());
            consts.push((index, Expr::var(&name)));
            const_names.push(name);
        }
        let outer_consts = std::mem::replace(&mut self.given.const_params, consts);
        let body = self.krate.bodies[&id];
        let nested = super::Nested::Default {
            evidence: specialized,
            self_args: args,
            typing_env: ty::TypingEnv::post_analysis(self.tcx, id),
            drops,
            unsupported,
        };
        let enclosing = self.enter_body(body, id, nested)?;
        let mut rest = Vec::new();
        let signature = self.lower_signature(id, &body.thir.params.raw, body.expr, &mut rest);
        self.given.const_params = outer_consts;
        let (mut params, is_async) = signature?;
        params.extend(const_names.into_iter().map(Into::into));
        params.extend(own_params.into_iter().map(Into::into));
        // Only the drops it uses: most defaults drop nothing of their `Self`.
        let used = self.used_drops();
        self.leave_body(enclosing)?;
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
