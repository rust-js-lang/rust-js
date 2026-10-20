//! Which type parameters given as they are, with an `Iterator` dictionary
//! (ADR 0411), or an extension's, `DoubleEndedIterator`'s or
//! `ExactSizeIterator`'s (ADR 0487), each function steps by each: it's given
//! a dictionary only then, as a parameter nothing steps by it needs none.

use crate::lower::iterators::given_as_it_is;
use crate::lower::recognition::{
    StdItem, is_extension_entry, is_iterator_extension, is_std_crate, is_std_def, std_item, trait_method,
};
use crate::lower::{Body, FnInfo, fn_def};
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// Each `(function, parameter, trait)` whose dictionary is given: one given
/// as it is that the function steps by the trait, a method of it of a value
/// of it, `Iterator`'s of an `IntoIterator`'s, or of another iterator it's
/// given to, `chain(it)`'s; the crate's iterator holding it, or a function
/// it's passed to that does. An impl's, of its dictionary, if any of its
/// items does.
pub(in crate::lower) fn stepped_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    fns: &HashMap<DefId, FnInfo>,
) -> HashSet<(DefId, u32, DefId)> {
    let iterator = std_item(tcx, StdItem::Iterator);
    let next = trait_method(tcx, iterator, "next");
    let mut stepped: HashSet<(DefId, u32, DefId)> = HashSet::new();
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
            // What it steps by: `Iterator`, of its own methods and
            // `IntoIterator`'s, with the extensions one asks of its `Self`,
            // `rev()`'s `DoubleEndedIterator`; or an extension, of
            // `next_back` or `len`.
            let by: Vec<DefId> = match tcx.trait_of_assoc(callee) {
                Some(tr) if is_std_def(tcx, tr, StdItem::Iterator) || is_std_def(tcx, tr, StdItem::IntoIterator) => {
                    let self_param = ty::Ty::new_param(tcx, 0, rustc_span::symbol::kw::SelfUpper);
                    let asked = (tcx.clauses_of(callee).instantiate_identity(tcx).into_iter())
                        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
                        .filter(|p| p.self_ty().skip_binder() == self_param && is_iterator_extension(tcx, p.def_id()))
                        .map(|p| p.def_id());
                    std::iter::once(iterator).chain(asked).collect()
                }
                // One's own entry, by its dictionary alone; another of its
                // methods, run on a JS iterator made of it, by `Iterator` too
                // (ADR 0488).
                Some(tr) if is_iterator_extension(tcx, tr) => match is_extension_entry(tcx, callee) {
                    true => vec![tr],
                    false => vec![tr, iterator],
                },
                _ => Vec::new(),
            };
            if !by.is_empty()
                && let Some(self_ty) = args.types().next()
            {
                let self_ty = self_ty.peel_refs();
                // Another iterator it's given, `chain(other)`'s, stepped
                // with it.
                for given in args.types().skip(1) {
                    if let ty::Param(param) = given.peel_refs().kind() {
                        stepped.insert((caller, param.index, iterator));
                    }
                }
                // A value of the parameter itself, stepped; or a std adapter
                // of it, `it.map(f)`'s, stepped from its back or counted,
                // which asks that of it, as a `Map` is double-ended where its
                // source is (ADR 0488). Its `Iterator`'s, made where the
                // adapter's made, by `map`.
                if let ty::Param(param) = self_ty.kind() {
                    stepped.extend(by.iter().map(|&by| (caller, param.index, by)));
                    continue;
                }
                if let ty::Adt(adt, adapted) = self_ty.kind()
                    && is_std_crate(tcx, adt.did())
                {
                    for param in adapted.types().filter_map(|ty| match ty.peel_refs().kind() {
                        ty::Param(param) => Some(param.index),
                        _ => None,
                    }) {
                        let asked = by.iter().filter(|&&by| by != iterator);
                        stepped.extend(asked.map(|&by| (caller, param, by)));
                    }
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
    // A parameter given as a JS iterator (ADR 0061), not as it is, takes
    // one made of the dictionaries its bounds ask: its `Iterator`'s, and its
    // extensions', its back and its length (ADR 0487).
    let as_js_iterator = |(id, index): (DefId, u32)| -> Vec<DefId> {
        let generics = tcx.generics_of(id);
        if !matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn) || generics.count() <= index as usize {
            return Vec::new();
        }
        let param = generics.param_at(index as usize, tcx);
        if !matches!(param.kind, ty::GenericParamDefKind::Type { .. }) {
            return Vec::new();
        }
        let param = ty::ParamTy::new(param.index, param.name);
        let bounds: Vec<DefId> = (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .filter(|p| p.self_ty().skip_binder() == param.to_ty(tcx))
            .map(|p| p.def_id())
            .collect();
        let iterates = bounds
            .iter()
            .any(|&tr| tr == iterator || is_iterator_extension(tcx, tr));
        if !iterates || given_as_it_is(tcx, id, param) {
            return Vec::new();
        }
        let extensions = bounds.into_iter().filter(|&tr| is_iterator_extension(tcx, tr));
        std::iter::once(iterator).chain(extensions).collect()
    };
    let mut changed = true;
    while changed {
        changed = false;
        for &(from, to) in &passed {
            let by: Vec<DefId> = (stepped.iter())
                .filter(|&&(id, index, _)| (id, index) == to)
                .map(|&(.., by)| by)
                .chain(as_js_iterator(to))
                .collect();
            for by in by {
                if stepped.insert((from.0, from.1, by)) {
                    changed = true;
                }
            }
        }
        for &(item, imp) in &items {
            let asked: Vec<(u32, DefId)> = (stepped.iter())
                .filter(|(id, ..)| *id == item)
                .map(|&(_, index, by)| (index, by))
                .collect();
            for (index, by) in asked {
                if stepped.insert((imp, index, by)) {
                    changed = true;
                }
            }
        }
    }
    stepped.retain(|&(id, index, _)| {
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
