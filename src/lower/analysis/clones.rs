//! The types the crate clones (ADRs 0522, 0525): each a `Clone` bound of a
//! function named, called or as a value, is given, `Clone::clone`'s own
//! `Self: Clone` too, and each its other bounds give through the impls
//! selected for them (#1968); and what cloning each clones in turn, its
//! `Clone` impl's bounds, as rustc proves them: `Map<I, F>: Clone` clones
//! its `I` and its `F`.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::Body;
use rustc_hir::def_id::DefId;
use rustc_middle::thir::ExprKind;
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::adjustment::PointerCoercion;
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
        self.with_compiled_here().filter(|&ty| !bare_param(ty))
    }

    /// `with_compiled_here`'s type parameters alone, each generic code's
    /// clone of a type parameter as it is, where its instances aren't seen
    /// (`instances_of`): what its namings give it is elaborated where
    /// they're named, but of a caller the crate doesn't see (#1896).
    pub(in crate::lower) fn of_bare_params(&self) -> impl Iterator<Item = Ty<'tcx>> + '_ {
        self.with_compiled_here().filter(|&ty| bare_param(ty))
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
pub(super) fn cloned_types<'tcx>(
    tcx: TyCtxt<'tcx>,
    (all_bodies, names): (&[&Body<'tcx>], &crate::lower::call_graph::Names<'tcx>),
    library: bool,
) -> Cloned<'tcx> {
    let mut cloned = Cloned {
        types: IndexSet::default(),
        compiled_here: (all_bodies.iter())
            .flat_map(|body| mir_given_clones(tcx, body))
            .map(|ty| tcx.erase_and_anonymize_regions(ty))
            .collect(),
        callers: crate::lower::instances::callers_of(tcx, names, all_bodies),
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
            given_clones(tcx, body)
                .into_iter()
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

/// The types `thir` clones (#1968): each type a `Clone` bound, or a bound of
/// a trait that extends it, is given by an obligation the body gives, as
/// `Given` elaborates it: of each function it names, called or as a value,
/// `Some(&held).map(Rc::clone)`'s `Rc<..>: Clone` too, and of each value it
/// makes a `dyn` of, `&held as &dyn Dup`'s `Rc<..>: Dup` (#1896).
fn given_clones<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Vec<Ty<'tcx>> {
    let thir = &body.thir;
    let mut given = Given::new(tcx, ty::TypingEnv::post_analysis(tcx, body.def_id));
    for expr in thir.exprs.iter() {
        if let ty::FnDef(id, args) = *expr.ty.kind() {
            given.named(id, args.skip_binder());
        }
        if let ExprKind::PointerCoercion {
            cast: PointerCoercion::Unsize,
            source,
            ..
        } = expr.kind
        {
            given.made_dyn(thir[source].ty, expr.ty);
        }
    }
    given.cloned
}

/// The types a library's function the crate compiles itself, of MIR alone
/// (ADR 0914), clones, as `given_clones` reads them of THIR: of each
/// function its MIR names, called or as a value, and of each unsizing, a
/// closure of another crate's given twice (#1053).
fn mir_given_clones<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Vec<Ty<'tcx>> {
    use rustc_middle::mir::{self, CastKind, Rvalue, visit::Visitor};
    let Some(mir) = (body.mir.as_ref())
        .filter(|_| body.root().is_none() && !crate::lower::recognition::is_std_crate(tcx, body.def_id))
    else {
        return Vec::new();
    };
    struct Found<'a, 'tcx> {
        obligations: Given<'tcx>,
        body: &'a mir::Body<'tcx>,
    }
    impl<'tcx> Visitor<'tcx> for Found<'_, 'tcx> {
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, location: mir::Location) {
            if let ty::FnDef(id, args) = *constant.ty().kind() {
                self.obligations.named(id, args.skip_binder());
            }
            self.super_const_operand(constant, location);
        }
        fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, location: mir::Location) {
            if let Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::Unsize, _), operand, to) = rvalue {
                let from = operand.ty(&self.body.local_decls, self.obligations.tcx);
                self.obligations.made_dyn(from, *to);
            }
            self.super_rvalue(rvalue, location);
        }
    }
    let mut found = Found {
        obligations: Given::new(tcx, ty::TypingEnv::post_analysis(tcx, body.def_id)),
        body: &mir.body,
    };
    found.visit_body(&mir.body);
    found.obligations.cloned
}

/// What a body's obligations clone (#1968): each type a `Clone` bound, or
/// one of a trait that extends it, is given, and through the impl selected
/// for each other obligation, its own bounds in turn: `held.to_owned()`'s
/// `Rc<..>: ToOwned` is std's `impl<T: Clone> ToOwned for T`, which clones
/// the `Rc`, and `outer(&held)`'s `Rc<..>: Dup` a crate's `impl<T: Clone>
/// Dup for T`. An obligation of a body's own type parameter, which its
/// `where` says, is its callers' to give, each theirs elaborated where
/// they name it. One whose impl selection can't decide is taken as
/// cloning its type, never as cloning nothing.
struct Given<'tcx> {
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    /// Each obligation elaborated, by its trait and types, regions erased.
    seen: IndexSet<ty::TraitRef<'tcx>>,
    /// Each function named, with its arguments.
    named: IndexSet<(DefId, ty::GenericArgsRef<'tcx>)>,
    cloned: Vec<Ty<'tcx>>,
}

impl<'tcx> Given<'tcx> {
    fn new(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>) -> Self {
        Given {
            tcx,
            typing_env,
            seen: IndexSet::default(),
            named: IndexSet::default(),
            cloned: Vec::new(),
        }
    }

    /// A function `id` named with `args`, called or as a value: each of its
    /// bounds, `Clone::clone`'s own `Self: Clone` too.
    fn named(&mut self, id: DefId, args: ty::GenericArgsRef<'tcx>) {
        if !self.named.insert((id, args)) {
            return;
        }
        let clauses = self.tcx.clauses_of(id).instantiate(self.tcx, args);
        self.clauses(clauses.into_iter().map(|(clause, _)| clause.skip_normalization()));
    }

    /// A value of `from` made a `dyn` as `to`: of the `dyn`'s trait, the
    /// value's evidence, whose impl its dictionary calls, and each trait it
    /// extends (`obligation`). One whose pointee can't be told is taken as
    /// cloned.
    fn made_dyn(&mut self, from: Ty<'tcx>, to: Ty<'tcx>) {
        let Some((from, to)) = unsized_parts(self.tcx, from, to) else {
            self.cloned.push(from);
            return;
        };
        let ty::Dynamic(predicates, ..) = *to.kind() else {
            return;
        };
        if let Some(principal) = predicates.principal() {
            let tr = self
                .tcx
                .instantiate_bound_regions_with_erased(principal.with_self_ty(self.tcx, from));
            self.obligation(tr);
        }
    }

    fn clauses(&mut self, clauses: impl IntoIterator<Item = ty::Clause<'tcx>>) {
        for clause in clauses {
            if let Some(p) = clause.as_trait_clause()
                && p.polarity() == ty::ClausePolarity::Positive
            {
                self.obligation(
                    self.tcx
                        .instantiate_bound_regions_with_erased(p.map_bound(|p| p.trait_ref)),
                );
            }
        }
    }

    /// `tr`'s self type, given a `Clone` bound or one extending it, is
    /// cloned; of any other trait, the impl selected for it is elaborated,
    /// and the evidence of each trait it extends, which a dictionary of it
    /// holds.
    fn obligation(&mut self, tr: ty::TraitRef<'tcx>) {
        let tcx = self.tcx;
        let tr = tcx.erase_and_anonymize_regions(tr);
        if tcx.trait_is_auto(tr.def_id) || tcx.is_lang_item(tr.def_id, rustc_hir::attrs::lang_items::LangItem::Sized) {
            return;
        }
        // Its type as it's written: what an `impl Clone` hides is revealed
        // by `cloned_types`, which tells std's struct apart (#1241).
        if is_clone(tcx, tr.def_id) {
            self.cloned.push(tr.self_ty());
            if Some(tr.def_id) == tcx.lang_items().clone_trait() {
                // What a clone clones in turn is `clones_in_turn`'s.
                return;
            }
        }
        let tr = (tcx.try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(tr))).unwrap_or(tr);
        if !self.seen.insert(tr) {
            return;
        }
        let supertraits: Vec<ty::TraitRef<'tcx>> = ty::elaborate::supertraits(tcx, ty::Binder::dummy(tr))
            .skip(1)
            .map(|supertrait| tcx.instantiate_bound_regions_with_erased(supertrait))
            .collect();
        for supertrait in supertraits {
            self.obligation(supertrait);
        }
        match crate::lower::traits::select(tcx, self.typing_env, tr) {
            Ok(ImplSource::UserDefined(imp)) => {
                let clauses = tcx.clauses_of(imp.impl_def_id).instantiate(tcx, imp.args);
                self.clauses(clauses.into_iter().map(|(clause, _)| clause.skip_normalization()));
            }
            // A `where` of the body's own, its callers' to give, or what's
            // built in, a closure's `Fn`, a `dyn`'s own.
            Ok(ImplSource::Param(_) | ImplSource::Builtin(..)) => {}
            Err(_) => self.cloned.push(tr.self_ty()),
        }
    }
}

/// Whether `ty` is a type parameter as it is, `T`.
fn bare_param(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Param(_))
}

/// What an unsizing coercion from `from` to `to` makes a `dyn` or a slice
/// of, and what of: a `&`'s, a raw pointer's, and a `Box`'s, an `Rc`'s or
/// another pointer struct's, its one argument that changes.
fn unsized_parts<'tcx>(tcx: TyCtxt<'tcx>, from: Ty<'tcx>, to: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    let pointee = |ty: Ty<'tcx>| match *ty.kind() {
        ty::Ref(_, pointee, _) => Some(pointee),
        _ => crate::lower::pointers::raw_pointee(tcx, ty).map(|(pointee, _)| pointee),
    };
    if let (Some(from), Some(to)) = (pointee(from), pointee(to)) {
        return Some((from, to));
    }
    match (from.kind(), to.kind()) {
        (ty::Adt(a, from), ty::Adt(b, to)) if a == b => {
            let (from, to) = from.types().zip(to.types()).find(|(from, to)| from != to)?;
            match to.kind() {
                ty::Dynamic(..) | ty::Slice(_) | ty::Str => Some((from, to)),
                _ => unsized_parts(tcx, from, to),
            }
        }
        _ => None,
    }
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
            // Its impl's bounds, elaborated as a body's obligations are: a
            // `Clone` impl's `T: Dup` whose impl clones `T` clones it too.
            let mut given = Given::new(tcx, typing_env);
            let clauses = tcx.clauses_of(imp.impl_def_id).instantiate(tcx, imp.args);
            given.clauses(clauses.into_iter().map(|(clause, _)| clause.skip_normalization()));
            given.cloned
        }
    }
}
