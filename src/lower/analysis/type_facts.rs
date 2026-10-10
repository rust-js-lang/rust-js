//! What each generic function asks of its type parameters (ADR 0145):
//! `size_of::<T>()`, `align_of::<T>()` and `type_name::<T>()`, which a
//! caller's type answers.

use crate::lower::recognition::{TypeFact, bound_by_any, pointer_target, points_at_dyn_any, type_fact};
use crate::lower::{Body, FnInfo, fn_def};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty;
use rustc_middle::ty::TyCtxt;
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// The facts each of the crate's generic functions asks of its type
/// parameters, by their index, directly, in a closure of its own, or
/// through a generic function of the crate's it passes the parameter on to:
/// a caller gives each, after the function's dictionaries. A trait's method
/// and an impl's, called through dictionaries, and a library's function its
/// consumers can reach, which never see this, ask none: theirs are errors.
pub(super) fn type_fact_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    fns: &HashMap<DefId, FnInfo>,
    library: bool,
) -> HashMap<DefId, Vec<(u32, TypeFact)>> {
    let mut asked: HashSet<(DefId, u32, TypeFact)> = HashSet::new();
    // A caller's type parameter passed on as a callee's: `relay<U>` calling `describe::<U>`.
    let mut passed: Vec<((DefId, u32), (DefId, u32))> = Vec::new();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id.to_def_id());
        let typing_env = ty::TypingEnv::non_body_analysis(tcx, caller);
        for expr in body.thir.exprs.iter() {
            // A type parameter's value made a `dyn Any`, whose dictionary is
            // its `TypeId` (ADR 0331).
            if let ExprKind::PointerCoercion {
                cast: PointerCoercion::Unsize,
                source,
                ..
            } = expr.kind
                && let ty::Param(param) = pointer_target(tcx, body.thir[source].ty).kind()
                && points_at_dyn_any(tcx, expr.ty)
                && !bound_by_any(tcx, caller, param.index)
            {
                asked.insert((caller, param.index, TypeFact::Id));
                continue;
            }
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            if let Some(fact) = type_fact(tcx, callee) {
                // `size_of_val` of an unsized `T` is the value's, not the type's.
                // A downcast's type is its last, after its `Box`'s allocator.
                // A `T: Any`'s `TypeId` is its dictionary's (ADR 0331).
                if let Some(of) = args.types().last()
                    && let ty::Param(param) = of.kind()
                    && of.is_sized(tcx, typing_env)
                    && !(fact == TypeFact::Id && bound_by_any(tcx, caller, param.index))
                {
                    asked.insert((caller, param.index, fact));
                }
                continue;
            }
            if !fns.contains_key(&callee) || tcx.trait_of_assoc(callee).is_some() {
                continue;
            }
            for (index, arg) in args.iter().enumerate() {
                if let Some(ty) = arg.as_type()
                    && let ty::Param(param) = ty.kind()
                {
                    passed.push(((caller, param.index), (callee, index as u32)));
                }
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for &((caller, from), (callee, to)) in &passed {
            for fact in [TypeFact::Size, TypeFact::Align, TypeFact::Name, TypeFact::Id] {
                if asked.contains(&(callee, to, fact)) && asked.insert((caller, from, fact)) {
                    changed = true;
                }
            }
        }
    }
    let mut params: HashMap<DefId, Vec<(u32, TypeFact)>> = HashMap::new();
    for (id, index, fact) in asked {
        let through_dictionary = tcx.trait_of_assoc(id).is_some() || tcx.trait_impl_of_assoc(id).is_some();
        let reached = library && crate::lower::library::reachable(tcx, id);
        if !through_dictionary && !reached {
            params.entry(id).or_default().push((index, fact));
        }
    }
    for facts in params.values_mut() {
        facts.sort();
    }
    params
}
