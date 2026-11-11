//! The types the crate clones (ADRs 0522, 0525): each a `Clone` bound of a
//! function called is given, `Clone::clone`'s own `Self: Clone` too, and
//! what cloning each clones in turn, its `Clone` impl's bounds, as rustc
//! proves them: `Map<I, F>: Clone` clones its `I` and its `F`.

use crate::collections::{IndexMap, IndexSet};
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
    /// What a library's functions the crate compiles itself, of MIR alone
    /// (ADR 0914), clone: the library's own closures, whose clones its
    /// contract decides (#1202), so apart from `types`, which the crate's
    /// own decisions are of (`with_compiled_here`).
    compiled_here: IndexSet<Ty<'tcx>>,
    callers: crate::lower::instances::Callers<'tcx>,
    /// Of each type cloned as it is of its function's own type parameters,
    /// any type its callers may give them, a `pub` function's: that
    /// function, whose bounds say what they may be (`owners_of`, #1318).
    owners: IndexMap<Ty<'tcx>, IndexSet<DefId>>,
}

impl<'tcx> std::ops::Deref for Cloned<'tcx> {
    type Target = IndexSet<Ty<'tcx>>;

    fn deref(&self) -> &Self::Target {
        &self.types
    }
}

impl<'tcx> Cloned<'tcx> {
    /// Each type the crate clones, and what a library's function it
    /// compiles of MIR alone clones too: what an `Rc<dyn Fn>` may be made
    /// of twice, a closure cloned (ADR 0959, #1053, #1249).
    /// The functions whose own type parameters `ty`, cloned as it is, holds,
    /// any type their callers may give them within their bounds; none of
    /// another (#1318).
    pub(in crate::lower) fn owners_of(&self, ty: Ty<'tcx>) -> impl Iterator<Item = DefId> + '_ {
        self.owners.get(&ty).into_iter().flatten().copied()
    }

    pub(in crate::lower) fn with_compiled_here(&self) -> impl Iterator<Item = Ty<'tcx>> + '_ {
        self.types.iter().chain(&self.compiled_here).copied()
    }

    /// `with_compiled_here`'s, but a type parameter alone, generic code's
    /// clone of whatever a caller the crate doesn't see gives it: what may
    /// be a closure of the crate's cloned. No such caller can name one, and
    /// one the crate gives out behind an `impl Clone` is among them by its
    /// own type (`given_out_clone`), so an `Rc<dyn Fn>` of any other is made
    /// of it once (#1249, #1250). An `Rc` made of what generic code gives
    /// back, `Rc::new(dup(&f))`, is made apart, counted whatever's cloned.
    pub(in crate::lower) fn of_named_types(&self) -> impl Iterator<Item = Ty<'tcx>> + '_ {
        self.with_compiled_here()
            .filter(|ty| !matches!(ty.kind(), ty::Param(_)))
    }

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
        crate::lower::instances::instances_of(
            tcx,
            (&self.callers, crate::lower::instances::Fact::Shared),
            ty,
            tcx.typeck_root_def_id(owner),
        )
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
        compiled_here: (all_bodies.iter())
            .flat_map(|body| mir_called_clones(tcx, body))
            .map(|ty| tcx.erase_and_anonymize_regions(ty))
            .collect(),
        callers: crate::lower::instances::callers_of(tcx, all_bodies),
        owners: IndexMap::default(),
    };
    // A clone in generic code of a type of its own parameters, `Chain<J,
    // J>`, is of each type its callers give them, where every one is seen
    // (#1009): so it's told apart from another generic function's adapter
    // of its own, which rustc names alike, its `Chain<I, I>` (#1119); else
    // as it is, any type they may be.
    let instances = |owner: DefId, ty: Ty<'tcx>| -> Vec<(Ty<'tcx>, Option<DefId>)> {
        match cloned.instances_of(tcx, ty, owner) {
            Some(instances) => instances.into_iter().map(|ty| (ty, None)).collect(),
            None => vec![(ty, ty.has_non_region_param().then(|| tcx.typeck_root_def_id(owner)))],
        }
    };
    // What an `impl Clone` hides, revealed: a caller's clone of what a
    // function gives is of what that function makes. Not std's struct a
    // function makes, cloned by its compiled `Clone`, its value's (#1241).
    let mut compiled = IndexSet::default();
    let mut todo: Vec<(Ty<'tcx>, Option<DefId>)> = (all_bodies.iter())
        .flat_map(|body| {
            let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
            let owner = body.def_id;
            called_clones(tcx, &body.thir)
                .map(|ty| {
                    compiled.extend(crate::lower::recognition::compiled_behind_an_impl(tcx, typing_env, ty));
                    (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))).unwrap_or(ty)
                })
                .flat_map(|ty| instances(owner, ty))
                .collect::<Vec<_>>()
        })
        .collect();
    if library {
        todo.extend(given_out_clone(tcx, all_bodies).into_iter().map(|ty| (ty, None)));
    }
    // Each std's struct a function makes, never cloned apart, its parts
    // cloned in turn by its compiled `Clone` all the same (#1242).
    let mut compiled_seen: IndexSet<Ty<'tcx>> = IndexSet::default();
    while let Some((ty, owner)) = todo.pop() {
        let ty = tcx.erase_and_anonymize_regions(ty);
        if let Some(owner) = owner {
            cloned.owners.entry(ty).or_default().insert(owner);
        }
        let new = match compiled.contains(&ty) {
            true => compiled_seen.insert(ty),
            false => cloned.types.insert(ty),
        };
        if new {
            todo.extend(clones_in_turn(tcx, ty).into_iter().map(|part| (part, owner)));
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
        // Its lifetimes erased, not left bound: what `impl .. + '_` hides
        // holds the function's own, `Chars<'_>`, which no question of its
        // `Clone` may hold unbound (#1201).
        let output = tcx
            .instantiate_bound_regions_with_erased(tcx.fn_sig(id).instantiate_identity().skip_normalization())
            .output();
        // What it hides revealed through each `impl Trait` it holds in turn,
        // a private function's `impl Iterator + Clone`, whose closure a
        // clone clones too (#1202).
        let typing_env = ty::TypingEnv::post_analysis(tcx, id);
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
                let hidden = tcx.type_of(opaque).instantiate(tcx, alias.args).skip_normalization();
                given.push(
                    (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(hidden)))
                        .unwrap_or(hidden),
                );
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

/// The types a library's function the crate compiles itself, of MIR alone
/// (ADR 0914), clones, as `called_clones` reads them of its calls: a
/// closure of another crate's given twice (#1053).
fn mir_called_clones<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Vec<Ty<'tcx>> {
    use rustc_middle::mir::TerminatorKind;
    let Some(mir) = (body.mir.as_ref())
        .filter(|_| body.root().is_none() && !crate::lower::recognition::is_std_crate(tcx, body.def_id))
    else {
        return Vec::new();
    };
    let mir = &mir.body;
    (mir.basic_blocks.iter())
        .filter_map(|block| match &block.terminator().kind {
            TerminatorKind::Call { func, .. } => Some(func.ty(&mir.local_decls, tcx)),
            _ => None,
        })
        .flat_map(|callee| {
            let ty::FnDef(id, args) = *callee.kind() else {
                return Vec::new();
            };
            (tcx.clauses_of(id).instantiate(tcx, args.skip_binder()).into_iter())
                .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
                .filter(|p| is_clone(tcx, p.def_id()))
                .map(|p| p.self_ty().skip_binder())
                .collect()
        })
        .collect()
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
            let Ok(ImplSource::UserDefined(imp)) = crate::lower::traits::select(tcx, typing_env, tr) else {
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
