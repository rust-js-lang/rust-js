//! Where std folds a zip to its shorter side's count of pairs, otherwise
//! than its `next()` steps it, and what shows that it does: what a step
//! clones and whether that runs the crate's code (ADRs 0973, 0997).

use super::*;

/// Whether `ty` is a zip std folds otherwise than it steps it, where that
/// shows: of sides both `TrustedLen` but not both `TrustedRandomAccess`,
/// whose `SpecFold` takes the shorter's count of pairs, where `next()` steps
/// `a` once more, and a step of `a` runs code, a closure's or a `fn`'s, or
/// gives an item with a destructor. The recognizers' JS iterator can't tell
/// a fold from its steps, so it's std's struct where it's made too (#380).
/// And a `TrustedLen` adapter whose fold is its source's, each item mapped,
/// a `map()`'s or an `enumerate()`'s, of one that folds apart: its fold is
/// that zip's, so it's std's struct too, which a collect into a `Vec` folds
/// as `extend_trusted` does (#654). Of a type rustc knows whole.
pub(crate) fn folds_apart<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    zip_folds_to_the_shorter(tcx, ty) == Some(false)
        || (super::mapped_source(tcx, ty))
            .or_else(|| super::enumerated_source(tcx, ty))
            .is_some_and(|source| folds_apart(tcx, source))
}

/// What a step of std's iterator `ty` clones: a `cloned()`'s items, a
/// `repeat()`'s or a `repeat_n()`'s value, a `cycle()`'s iterator. The one
/// catalog of it, `stepping_work`'s `Clones` and what a zip's step runs
/// (#708, #1162).
pub(crate) fn stepping_clones<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
) -> Option<Ty<'tcx>> {
    let ty::Adt(adt, args) = ty.kind() else { return None };
    let first = args.types().next();
    match std_path(tcx, adt.did()) {
        "std::iter::Repeat" | "std::iter::RepeatN" => first,
        "std::iter::Cycle" => first,
        "std::iter::Cloned" => {
            let item = Ty::new_projection(
                tcx,
                ty::IsRigid::No,
                trait_method(tcx, std_item(tcx, StdItem::Iterator), "Item"),
                [ty],
            );
            tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(item))
                .ok()
        }
        _ => None,
    }
}

/// Whether cloning a `ty` may run code the crate wrote, whose effects show:
/// a type parameter's or a closure's, anyone's; one of the crate's types,
/// or a library's, whose `Clone` it writes, or derives of a field that
/// may; std's of what it holds. Not what's `Copy`, nor a derived `Clone`
/// of what's plain (#1162).
fn clone_runs_crates_code<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
    seen: &mut Vec<Ty<'tcx>>,
) -> bool {
    if tcx.type_is_copy_modulo_regions(typing_env, ty) || seen.contains(&ty) {
        return false;
    }
    seen.push(ty);
    let clone = tcx.require_lang_item(LangItem::Clone, rustc_span::DUMMY_SP);
    let runs = match *ty.kind() {
        ty::Param(_) | ty::Closure(..) | ty::Alias(..) | ty::Dynamic(..) => true,
        ty::Adt(adt, args) if !is_std_crate(tcx, adt.did()) => {
            let derived = (tcx.all_impls(clone)).find(|&imp| {
                matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(), ty::Adt(of, _) if of.did() == adt.did())
            });
            match derived {
                Some(imp) if tcx.is_automatically_derived(imp) => adt
                    .all_fields()
                    .any(|f| clone_runs_crates_code(tcx, typing_env, f.ty(tcx, args).skip_normalization(), seen)),
                _ => true,
            }
        }
        ty::Adt(_, args) => args
            .types()
            .any(|arg| clone_runs_crates_code(tcx, typing_env, arg, seen)),
        ty::Tuple(parts) => parts
            .iter()
            .any(|part| clone_runs_crates_code(tcx, typing_env, part, seen)),
        ty::Array(item, _) | ty::Slice(item) => clone_runs_crates_code(tcx, typing_env, item, seen),
        _ => false,
    };
    seen.pop();
    runs
}

/// Of a zip of type `ty`, rustc knowing it whole, where std's fold of it
/// takes the shorter side's count of pairs, where `next()` steps `a` once
/// more, and a step of `a` runs code, a closure's or a `fn`'s, a clone that
/// may run the crate's (`stepping_clones`, #1162), or gives an
/// item with a destructor: `Some(true)` of sides both `TrustedRandomAccess`,
/// read by index (ADR 0973), `Some(false)` of sides both `TrustedLen` but not
/// both that, by `SpecFold` (ADR 0997). `None` where its fold steps as
/// `next()` does, or what it steps runs nothing.
pub(crate) fn zip_folds_to_the_shorter<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<bool> {
    let typing_env = ty::TypingEnv::fully_monomorphized();
    // Its sides as the types an `impl Trait` hides, which native Rust's
    // specialization sees (#1225, as #1142).
    let ty = tcx
        .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
        .unwrap_or(ty);
    let (a, b) = zipped_sources(tcx, ty)?;
    let (Some(trusted_len), Some(random_access)) = (
        super::fact_trait(tcx, TypeFact::TrustedLen),
        super::fact_trait(tcx, TypeFact::RandomAccess),
    ) else {
        return None;
    };
    let holds = |ty: Ty<'tcx>, tr: DefId| {
        let tr = tcx.erase_and_anonymize_regions(ty::TraitRef::new(tcx, tr, [ty]));
        tcx.codegen_select_candidate(typing_env.as_query_input(tr)).is_ok()
    };
    let iterator = std_item(tcx, StdItem::Iterator);
    let item = Ty::new_projection(tcx, ty::IsRigid::No, trait_method(tcx, iterator, "Item"), [a]);
    let item = tcx
        .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(item))
        .unwrap_or(item);
    // A clone that may run the crate's own code (`clone_runs_crates_code`).
    let clones_own = |t: Ty<'tcx>| {
        stepping_clones(tcx, typing_env, t).is_some_and(|v| clone_runs_crates_code(tcx, typing_env, v, &mut Vec::new()))
    };
    let runs_code = steps_run_code(tcx, typing_env, a)
        || a.walk().any(|part| part.as_type().is_some_and(clones_own))
        || item.needs_drop(tcx, typing_env);
    let by_index = holds(a, random_access) && holds(b, random_access);
    (runs_code && (by_index || holds(a, trusted_len) && holds(b, trusted_len))).then_some(by_index)
}

/// Whether stepping an iterator of type `ty` runs code a program sees, as a
/// function one of its parts holds does: a closure's, a function's, or a
/// `dyn Fn*`'s, a boxed one's (#1225). The one answer of what std's
/// shortcuts leave unrun, a zip folded by index, an `eq` of lengths first.
/// Of the types an `impl Trait` hides, revealed in `typing_env`, as native
/// Rust's specialization sees them (#1142).
pub(crate) fn steps_run_code<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty = match rustc_middle::ty::TypeVisitableExt::has_opaque_types(&ty) {
        true => tcx
            .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty),
        false => ty,
    };
    ty.walk().any(|part| {
        part.as_type().is_some_and(|t| {
            t.is_closure()
                || t.is_fn()
                || matches!(t.kind(), ty::Dynamic(predicates, ..)
                    if predicates.principal_def_id().is_some_and(|p| tcx.fn_trait_kind_from_def_id(p).is_some()))
        })
    })
}
