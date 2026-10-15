//! Which type parameters given as they are, with an `Iterator` dictionary
//! (ADR 0411), each function steps: it's given the dictionary only then, as
//! a parameter nothing steps needs none.

use crate::lower::iterators::given_as_it_is;
use crate::lower::recognition::{StdItem, is_std_def, std_item, trait_method};
use crate::lower::{Body, FnInfo, fn_def};
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// Each `(function, parameter)` whose `Iterator` dictionary is given: one
/// given as it is that the function steps, an `Iterator` method of a value
/// of it, the crate's iterator holding it, or a function it's passed to
/// that does. An impl's, of its dictionary, if any of its items does.
pub(in crate::lower) fn stepped_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    fns: &HashMap<DefId, FnInfo>,
) -> HashSet<(DefId, u32)> {
    let iterator = std_item(tcx, StdItem::Iterator);
    let next = trait_method(tcx, iterator, "next");
    let mut stepped: HashSet<(DefId, u32)> = HashSet::new();
    // A caller's parameter given on as a callee's, which steps it if that does.
    let mut passed: Vec<((DefId, u32), (DefId, u32))> = Vec::new();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id.to_def_id());
        let typing_env = ty::TypingEnv::post_analysis(tcx, caller);
        let pass_on = |passed: &mut Vec<_>, callee: DefId, args: ty::GenericArgsRef<'tcx>| {
            for (index, arg) in args.iter().enumerate() {
                if let Some(ty) = arg.as_type()
                    && let ty::Param(param) = ty.kind()
                {
                    passed.push(((caller, param.index), (callee, index as u32)));
                }
            }
        };
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            let of_iterator = tcx
                .trait_of_assoc(callee)
                .is_some_and(|tr| is_std_def(tcx, tr, StdItem::Iterator) || is_std_def(tcx, tr, StdItem::IntoIterator));
            if of_iterator && let Some(self_ty) = args.types().next() {
                let self_ty = self_ty.peel_refs();
                // A value of the parameter itself, stepped.
                if let ty::Param(param) = self_ty.kind() {
                    stepped.insert((caller, param.index));
                    continue;
                }
                // The crate's iterator holding it, stepped by its impl's `next`.
                if let Ok(Some(instance)) =
                    ty::Instance::try_resolve(tcx, typing_env, next, tcx.mk_args(&[self_ty.into()]))
                    && fns.contains_key(&instance.def_id())
                {
                    pass_on(&mut passed, instance.def_id(), instance.args);
                }
                continue;
            }
            // A function of the crate's, or an impl's item, a dictionary's
            // made of the impl's evidence.
            let resolved = match ty::Instance::try_resolve(tcx, typing_env, callee, args) {
                Ok(Some(instance)) if fns.contains_key(&instance.def_id()) => instance,
                _ => continue,
            };
            pass_on(&mut passed, resolved.def_id(), resolved.args);
            if let Some(imp) = tcx.trait_impl_of_assoc(resolved.def_id()) {
                pass_on(&mut passed, imp, resolved.args);
            }
            // The crate's iterator holding it, given where a generic iterator
            // goes: a JS iterator, stepped by its impl's `next` (ADR 0061).
            for given in resolved.args.types().filter(|t| !matches!(t.kind(), ty::Param(_))) {
                if let Ok(Some(instance)) =
                    ty::Instance::try_resolve(tcx, typing_env, next, tcx.mk_args(&[given.into()]))
                    && fns.contains_key(&instance.def_id())
                {
                    pass_on(&mut passed, instance.def_id(), instance.args);
                }
            }
        }
    }
    // An impl's, of each item's.
    let items: Vec<(DefId, DefId)> = fns
        .keys()
        .filter_map(|&id| Some((id, tcx.trait_impl_of_assoc(id)?)))
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for &(from, to) in &passed {
            if stepped.contains(&to) && stepped.insert(from) {
                changed = true;
            }
        }
        for &(item, imp) in &items {
            let asked: Vec<u32> = stepped.iter().filter(|(id, _)| *id == item).map(|&(_, i)| i).collect();
            for index in asked {
                if stepped.insert((imp, index)) {
                    changed = true;
                }
            }
        }
    }
    stepped.retain(|&(id, index)| {
        matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn | DefKind::Impl { .. })
            && tcx.generics_of(id).count() > index as usize
            && match tcx.generics_of(id).param_at(index as usize, tcx).kind {
                ty::GenericParamDefKind::Type { .. } => {
                    let param = tcx.generics_of(id).param_at(index as usize, tcx);
                    given_as_it_is(tcx, id, ty::ParamTy::new(param.index, param.name))
                }
                _ => false,
            }
    });
    stepped
}
