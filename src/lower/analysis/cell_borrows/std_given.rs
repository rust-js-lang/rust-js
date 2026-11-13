//! What std's code may run of the crate's, called of some generic
//! arguments (ADR 0328): what it's given, the crate's impls of the bounds
//! it's called under, a derive's reads of its fields, and a cell its own
//! impl borrows. `cell_borrows`' answers of a std call, apart (split by
//! concept, #1334).

use super::*;

/// What std's code may run of the crate's, called of some generic
/// arguments, by the bounds it's called under: it calls what it's given
/// only through them (ADR 0328).
#[derive(Default)]
pub(super) struct Given<'tcx> {
    /// The cells it borrows, a `RefCell<T>`'s by `RefCell<T>: Clone`, say.
    pub(super) cells: Vec<Borrow<'tcx>>,
    /// The crate's bodies it may run, and of what args where its type says:
    /// a closure or function it's given, an impl's method, whose args no
    /// type says (#240).
    pub(super) bodies: Vec<(DefId, Option<ty::GenericArgsRef<'tcx>>)>,
    /// What rust-js can't see: a type parameter's, a `dyn`'s or a
    /// library's type's impl.
    pub(super) any: bool,
}

/// What `id`, std's, of `args`, may run of the crate's code (`Given`): for
/// each trait with methods that bounds a type it's given, `F: FnMut(&T) ->
/// bool` or `T: Clone`, the closure or function, the crate's impl of it,
/// through what a derive reads of its fields, the cell whose impl borrows
/// it, or what rust-js can't see.
pub(super) fn std_given<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    impls: &Impls,
    id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Given<'tcx> {
    let tcx = recognition.tcx;
    let mut given = Given::default();
    let mut seen = IndexSet::default();
    // A derived method reads its type's fields by its trait.
    if let Some(imp) = derived_method(tcx, id) {
        let own = tcx.mk_args(&args[..tcx.generics_of(imp).count().min(args.len())]);
        let ty = tcx.type_of(imp).instantiate(tcx, own).skip_normalization();
        let trait_id = tcx
            .impl_trait_ref(imp)
            .instantiate_identity()
            .skip_normalization()
            .def_id;
        bound_runs(recognition, impls, (ty, trait_id), &mut seen, &mut given);
        return given;
    }
    for (clause, _) in tcx.clauses_of(id).instantiate(tcx, args) {
        let Some(bound) = clause.skip_normalization().as_trait_clause() else {
            continue;
        };
        let bound = bound.skip_binder();
        bound_runs(
            recognition,
            impls,
            (bound.self_ty(), bound.def_id()),
            &mut seen,
            &mut given,
        );
    }
    // A cell's own trait impl's method borrows it: `RefCell`'s
    // `PartialEq::eq` borrows both it's given, its `Clone::clone` the one
    // it clones (ADR 0863).
    if let Some(imp) = tcx.trait_impl_of_assoc(id) {
        let own = tcx.mk_args(&args[..tcx.generics_of(imp).count().min(args.len())]);
        let cell = tcx.type_of(imp).instantiate(tcx, own).skip_normalization();
        if recognition.is_borrowed_cell(cell) {
            given.cells.push(Borrow {
                cell: Cell::Type(tcx.erase_and_anonymize_regions(cell)),
                mutable: !recognition.is_std_type(cell, StdItem::RefCell),
                class: None,
            });
        }
    }
    given
}

/// The impl std derives that `id`, the crate's, is a method of:
/// `#[derive(Clone)]`'s `clone`.
pub(super) fn derived_method(tcx: TyCtxt<'_>, id: DefId) -> Option<DefId> {
    tcx.trait_impl_of_assoc(id)
        .filter(|&imp| crate::lower::recognition::known_derive(tcx, imp))
}

/// Whether a trait or one it extends has a method: what a bound by it may
/// run. `Sized`, `Copy` and `Send` have none.
pub(super) fn has_methods(tcx: TyCtxt<'_>, trait_id: DefId) -> bool {
    rustc_middle::ty::elaborate::supertrait_def_ids(tcx, trait_id).any(|t| {
        // A trait alias, `ptr::Thin`, may stand for any.
        tcx.def_kind(t) != DefKind::Trait
            || (tcx.associated_item_def_ids(t).iter()).any(|&item| matches!(tcx.def_kind(item), DefKind::AssocFn))
    })
}

/// What std's code may run of `ty` by its bound `trait_id` (`std_given`).
pub(super) fn bound_runs<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    impls: &Impls,
    (ty, trait_id): (Ty<'tcx>, DefId),
    seen: &mut IndexSet<(Ty<'tcx>, DefId)>,
    given: &mut Given<'tcx>,
) {
    let tcx = recognition.tcx;
    if given.any || !seen.insert((ty, trait_id)) || !has_methods(tcx, trait_id) {
        return;
    }
    let traits: Vec<DefId> = rustc_middle::ty::elaborate::supertrait_def_ids(tcx, trait_id).collect();
    let mut walk = ty.walk();
    while let Some(part) = walk.next() {
        let Some(part) = part.as_type() else { continue };
        // A raw pointer's, which std's code reads nothing of.
        if crate::lower::pointers::raw_pointee(tcx, part).is_some() {
            continue;
        }
        // A type std's impls are of runs of what it's made of what those
        // impls' bounds say, as std's functions do (ADR 0868): `Rc<T>`'s
        // `Clone` clones no `T`, nor a `&T`'s, which copies it.
        if let Some(bounds) = std_impl_bounds(recognition, part, &traits) {
            walk.skip_current_subtree();
            for bound in bounds {
                bound_runs(recognition, impls, bound, seen, given);
            }
            continue;
        }
        match *part.kind() {
            // Called, by an `Fn` trait; or, by another, `F: Clone`, what it
            // took is. Its signature, among its arguments, isn't given.
            ty::Closure(closure, closure_args) => {
                walk.skip_current_subtree();
                if !closure.is_local() {
                    given.any = true;
                    continue;
                }
                given.bodies.push((closure, Some(closure_args)));
                if tcx.fn_trait_kind_from_def_id(trait_id).is_none() {
                    for upvar in closure_args.as_closure().upvar_tys() {
                        bound_runs(recognition, impls, (upvar, trait_id), seen, given);
                    }
                }
            }
            ty::CoroutineClosure(closure, args) | ty::Coroutine(closure, args) => {
                walk.skip_current_subtree();
                match closure.is_local() {
                    true => given.bodies.push((closure, Some(args))),
                    false => given.any = true,
                }
            }
            ty::FnDef(f, args) if f.is_local() && !is_binding(tcx, f) => {
                given.bodies.push((f, Some(args.skip_binder())))
            }
            // Std's, `get_or_insert_with(Vec::new)`'s, called by an `Fn`
            // trait: what a call of it runs, by its own bounds (ADR 0868),
            // not each type it's of, `Vec::<F>::new`'s `F`.
            ty::FnDef(f, args) if recognition.is_std(f) => {
                walk.skip_current_subtree();
                let of = std_given(recognition, impls, f, args.skip_binder());
                given.cells.extend(of.cells);
                given.bodies.extend(of.bodies);
                given.any |= of.any;
            }
            ty::Adt(..) if recognition.is_borrowed_cell(part) => given.cells.push(Borrow {
                cell: Cell::Type(tcx.erase_and_anonymize_regions(part)),
                mutable: !recognition.is_std_type(part, StdItem::RefCell),
                class: None,
            }),
            ty::Adt(adt, adt_args) if adt.did().is_local() => {
                let all = impls.get(&adt.did()).into_iter().flatten();
                let of: Vec<DefId> = (all.clone())
                    .filter(|(t, _)| traits.contains(t))
                    .map(|&(_, imp)| imp)
                    .collect();
                // One a blanket impl of std's meets by another it has,
                // `ToString` by its `Display`: any of its impls but its
                // `Drop`, which runs where it drops. Not one std derives,
                // `Clone` or `Debug`, which only its own impl is.
                let items = tcx.lang_items();
                let derived = [
                    items.clone_trait(),
                    items.eq_trait(),
                    items.partial_ord_trait(),
                    Some(crate::lower::recognition::std_item(tcx, StdItem::Ord)),
                    Some(crate::lower::recognition::std_item(tcx, StdItem::Debug)),
                ];
                let of = match of.is_empty() && !derived.contains(&Some(trait_id)) {
                    true => all
                        .filter(|(t, _)| Some(*t) != items.drop_trait())
                        .map(|&(_, imp)| imp)
                        .collect(),
                    false => of,
                };
                for imp in of {
                    // A derive reads its fields by the trait it derives.
                    if crate::lower::recognition::known_derive(tcx, imp) {
                        for field in adt.all_fields() {
                            let field_ty = field.ty(tcx, adt_args).skip_normalization();
                            for &t in &traits {
                                bound_runs(recognition, impls, (field_ty, t), seen, given);
                            }
                        }
                        continue;
                    }
                    given.bodies.extend(
                        (tcx.associated_item_def_ids(imp).iter())
                            .filter(|&&item| matches!(tcx.def_kind(item), DefKind::AssocFn))
                            .map(|&item| (item, None)),
                    );
                }
            }
            // A library's, whose impls' bodies the crate can't see: what
            // it implements of `traits` may run anything, but what it's
            // equal only to itself by, `==` and `Hash`, its address's, as
            // its contract says (ADR 0864, amended). What it has no impl of,
            // its own or a blanket one outside std's, runs nothing.
            ty::Adt(adt, _) if recognition.foreign.in_library(adt.did()) => {
                let identity = recognition.foreign.equal_only_to_itself(adt.did());
                let by_address = |t: DefId| {
                    tcx.is_lang_item(t, rustc_hir::attrs::lang_items::LangItem::PartialEq)
                        || crate::lower::recognition::is_std_def(tcx, t, StdItem::Eq)
                        || crate::lower::recognition::is_std_def(tcx, t, StdItem::Hash)
                };
                let implemented = |t: DefId| {
                    let impls = tcx.trait_impls_of(t);
                    let own = rustc_middle::ty::fast_reject::SimplifiedType::Adt(adt.did());
                    impls.non_blanket_impls().contains_key(&own)
                        || (impls.blanket_impls().iter()).any(|&imp| !recognition.in_sysroot(imp))
                };
                if traits.iter().any(|&t| implemented(t) && !(identity && by_address(t))) {
                    given.any = true;
                }
            }
            ty::Adt(..)
            | ty::Bool
            | ty::Char
            | ty::Int(_)
            | ty::Uint(_)
            | ty::Float(_)
            | ty::Str
            | ty::Never
            | ty::Array(..)
            | ty::Slice(_)
            | ty::Ref(..)
            | ty::Tuple(_)
            | ty::Foreign(_)
            | ty::FnDef(..) => {}
            // A type parameter's, or a projection's, by a trait no impl of
            // which is the program's, each std's: what std's code runs,
            // `H: Hasher`'s, which runs nothing of the crate's.
            ty::Param(_) | ty::Alias(..)
                if traits.iter().all(|&t| {
                    tcx.all_impls(t)
                        .all(|imp| !imp.is_local() && recognition.in_sysroot(imp))
                }) => {}
            // A type parameter's, a `dyn`'s, a function pointer's.
            _ => given.any = true,
        }
    }
}

/// The bounds std's impls of `traits` for `ty` are under, each a type and
/// the trait it's bound by, `T: Clone` of `Option<T>`'s `Clone`: what
/// they may run of what `ty` is made of (ADR 0868), of a type of a type
/// parameter too, `Rc<Node<K>>`'s `Clone`, which clones no `K` (#499).
/// `None` where `ty` is the crate's, a cell, which its own impl borrows, or
/// has a trait of `traits` that isn't std's impl, a built-in one or one not
/// selected, a type parameter's own.
pub(super) fn std_impl_bounds<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    ty: Ty<'tcx>,
    traits: &[DefId],
) -> Option<Vec<(Ty<'tcx>, DefId)>> {
    let tcx = recognition.tcx;
    let ours = matches!(ty.kind(), ty::Adt(adt, _) if adt.did().is_local());
    if ours || recognition.is_borrowed_cell(ty) || ty.has_aliases() {
        return None;
    }
    // One of a const parameter, `[u8; SIZE]`, which only its own function's
    // environment, not this one, says the type of, as selecting its impl
    // asks (#1017).
    if ty.walk().any(|part| {
        part.as_const()
            .is_some_and(|c| matches!(c.kind(), ty::ConstKind::Param(_)))
    }) {
        return None;
    }
    // A trait alias, `ptr::Thin`, which may stand for any, and has no impls.
    if traits.iter().any(|&t| tcx.def_kind(t) != DefKind::Trait) {
        return None;
    }
    let ty = tcx.erase_and_anonymize_regions(ty);
    // A function pointer's impls, a copy, a comparison of where it points,
    // never call it; only an `Fn` trait's does.
    if ty.is_fn_ptr() {
        let calls = traits.iter().any(|&t| tcx.fn_trait_kind_from_def_id(t).is_some());
        return (!calls).then(Vec::new);
    }
    let own_methods =
        |t: DefId| (tcx.associated_item_def_ids(t).iter()).any(|&item| matches!(tcx.def_kind(item), DefKind::AssocFn));
    let mut bounds = Vec::new();
    for &t in traits.iter().filter(|&&t| own_methods(t)) {
        // A trait of arguments, `Fn(&T)`'s, `PartialEq<Rhs>`'s, which a
        // bound here doesn't say.
        if tcx.generics_of(t).count() != 1 {
            return None;
        }
        let tr = ty::TraitRef::new(tcx, t, [ty]);
        let (imp, args) = crate::lower::traits::evidence_impl(tcx, recognition.typing_env, tr)?;
        if !recognition.in_sysroot(imp) {
            return None;
        }
        for (clause, _) in tcx.clauses_of(imp).instantiate(tcx, args) {
            if let Some(bound) = clause.skip_normalization().as_trait_clause() {
                let bound = bound.skip_binder();
                bounds.push((bound.self_ty(), bound.def_id()));
            }
        }
    }
    Some(bounds)
}
