//! The types the crate clones (ADRs 0522, 0525): each a `Clone` bound of a
//! function called is given, `Clone::clone`'s own `Self: Clone` too, and
//! what cloning each clones in turn, its `Clone` impl's bounds, as rustc
//! proves them: `Map<I, F>: Clone` clones its `I` and its `F`.

use crate::collections::IndexSet;
use crate::lower::Body;
use rustc_hir::def_id::DefId;
use rustc_middle::thir::{ExprKind, Thir};
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};

/// The types the crate clones (`cloned_types`), and who calls each function
/// with what, by which generic code's types are told apart, each its
/// callers' instances (`instances_of`, #1119).
pub(in crate::lower) struct Cloned<'tcx> {
    types: IndexSet<Ty<'tcx>>,
    callers: crate::lower::instances::Callers<'tcx>,
}

impl<'tcx> std::ops::Deref for Cloned<'tcx> {
    type Target = IndexSet<Ty<'tcx>>;

    fn deref(&self) -> &Self::Target {
        &self.types
    }
}

impl<'tcx> Cloned<'tcx> {
    /// What `ty`, made in `owner`, of its type parameters, may be: each type
    /// its callers give them, where every one is seen; `None` of another,
    /// any type they may be.
    pub(in crate::lower) fn instances_of(
        &self,
        tcx: TyCtxt<'tcx>,
        ty: Ty<'tcx>,
        owner: DefId,
    ) -> Option<IndexSet<Ty<'tcx>>> {
        if !ty.has_non_region_param() {
            return None;
        }
        crate::lower::instances::instances_of(tcx, &self.callers, ty, tcx.typeck_root_def_id(owner))
    }

    /// Whether `is` holds of `ty`, made in `owner`, as one of its
    /// instances, where all are seen (`instances_of`); else of `ty` itself,
    /// any type its parameters may be: so generic code's adapter is asked
    /// whether it's cloned as each type its callers give it (#1119).
    pub(in crate::lower) fn as_any_instance(
        &self,
        tcx: TyCtxt<'tcx>,
        ty: Ty<'tcx>,
        owner: DefId,
        mut is: impl FnMut(Ty<'tcx>) -> bool,
    ) -> bool {
        match self.instances_of(tcx, ty, owner) {
            Some(made) => made.into_iter().any(is),
            None => is(ty),
        }
    }
}

/// The types `all_bodies` clone, regions erased, and those their clones
/// clone, to a fixed point; of a `library`, what each of its public
/// functions gives behind an `impl .. + Clone` too, which any consumer may
/// clone (#1032).
pub(super) fn cloned_types<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>], library: bool) -> Cloned<'tcx> {
    let mut cloned = Cloned {
        types: IndexSet::default(),
        callers: crate::lower::instances::callers_of(tcx, all_bodies),
    };
    // A clone in generic code of a type of its own parameters, `Chain<J,
    // J>`, is of each type its callers give them, where every one is seen
    // (#1009): so it's told apart from another generic function's adapter
    // of its own, which rustc names alike, its `Chain<I, I>` (#1119); else
    // as it is, any type they may be.
    let instances = |owner: DefId, ty: Ty<'tcx>| -> Vec<Ty<'tcx>> {
        match cloned.instances_of(tcx, ty, owner) {
            Some(instances) => instances.into_iter().collect(),
            None => vec![ty],
        }
    };
    // What an `impl Clone` hides, revealed: a caller's clone of what a
    // function gives is of what that function makes.
    let mut todo: Vec<Ty<'tcx>> = (all_bodies.iter())
        .flat_map(|body| {
            let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
            let owner = body.def_id;
            called_clones(tcx, &body.thir)
                .map(move |ty| {
                    (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))).unwrap_or(ty)
                })
                .flat_map(|ty| instances(owner, ty))
                .collect::<Vec<_>>()
        })
        .collect();
    if library {
        todo.extend(given_out_clone(tcx, all_bodies));
    }
    while let Some(ty) = todo.pop() {
        let ty = tcx.erase_and_anonymize_regions(ty);
        if cloned.types.insert(ty) {
            todo.extend(clones_in_turn(tcx, ty));
        }
    }
    cloned
}

/// What each of a library's public functions in `all_bodies` gives behind
/// an `impl .. + Clone`, revealed: `counter()`'s closure, which a consumer
/// may clone, its library never cloning it.
fn given_out_clone<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> Vec<Ty<'tcx>> {
    let mut given = Vec::new();
    for body in all_bodies {
        let id = body.def_id;
        if tcx.typeck_root_def_id(id) != id
            || !matches!(
                tcx.def_kind(id),
                rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
            )
            || !crate::lower::library::reachable(tcx, id)
        {
            continue;
        }
        let output = tcx
            .fn_sig(id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder()
            .output();
        for part in output.walk().filter_map(|part| part.as_type()) {
            if let ty::Alias(_, alias) = *part.kind()
                && let ty::Opaque { def_id: opaque } = alias.kind
                && (tcx
                    .item_bounds(opaque)
                    .instantiate_identity()
                    .skip_normalization()
                    .iter())
                .filter_map(|clause| clause.as_trait_clause())
                .any(|p| is_clone(tcx, p.def_id()))
            {
                given.push(tcx.type_of(opaque).instantiate(tcx, alias.args).skip_normalization());
            }
        }
    }
    given
}

/// Whether `id` is `Clone`, or a trait that extends it.
fn is_clone(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let clone = tcx.lang_items().clone_trait();
    Some(id) == clone || ty::elaborate::supertrait_def_ids(tcx, id).any(|s| Some(s) == clone)
}

/// The types `thir`'s calls clone: of each, each type a `Clone` bound of
/// the function called, or of a trait that extends it, is given.
fn called_clones<'a, 'tcx>(tcx: TyCtxt<'tcx>, thir: &'a Thir<'tcx>) -> impl Iterator<Item = Ty<'tcx>> + 'a {
    thir.exprs.iter().flat_map(move |expr| {
        let ExprKind::Call { fun, .. } = expr.kind else {
            return Vec::new();
        };
        let ty::FnDef(id, args) = *thir[fun].ty.kind() else {
            return Vec::new();
        };
        (tcx.clauses_of(id).instantiate(tcx, args.skip_binder()).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .filter(|p| is_clone(tcx, p.def_id()))
            .map(|p| p.self_ty().skip_binder())
            .collect()
    })
}

/// What a clone of `ty` clones: its `Clone` impl's bounds' types, or of
/// what's cloned field by field, a tuple's, an array's or a closure's, each
/// part. Of a type a caller knows, nothing here: the caller's clone gives it.
fn clones_in_turn<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
    match ty.kind() {
        ty::Tuple(parts) => parts.to_vec(),
        ty::Array(item, _) => vec![*item],
        ty::Closure(_, args) => args.as_closure().upvar_tys().to_vec(),
        _ if ty.has_non_region_param() || ty.has_aliases() => Vec::new(),
        _ => {
            let Some(clone) = tcx.lang_items().clone_trait() else {
                return Vec::new();
            };
            let typing_env = ty::TypingEnv::fully_monomorphized();
            let tr = ty::TraitRef::new(tcx, clone, [ty]);
            let Ok(ImplSource::UserDefined(imp)) = tcx.codegen_select_candidate(typing_env.as_query_input(tr)) else {
                return Vec::new();
            };
            (tcx.clauses_of(imp.impl_def_id).instantiate(tcx, imp.args).into_iter())
                .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
                .filter(|p| is_clone(tcx, p.def_id()))
                .map(|p| p.self_ty().skip_binder())
                .collect()
        }
    }
}
