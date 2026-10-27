//! Which type parameters given as they are, with an `Iterator` dictionary
//! (ADR 0411), or an extension's, `DoubleEndedIterator`'s or
//! `ExactSizeIterator`'s (ADR 0487), each function steps by each: it's given
//! a dictionary only then, as a parameter nothing steps by it needs none.

use crate::lower::iterators::{given_as_it_is, given_by_its_trait};
use crate::lower::recognition::{
    StdItem, is_extension_entry, is_iterator_extension, is_std_crate, is_std_def, std_item, trait_method,
};
use crate::lower::{Body, FnInfo, fn_def};
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// A function's, or an impl's, type parameter, by its index.
type Param = (DefId, u32);

/// Each struct or enum a body read from its MIR alone makes, by its type:
/// what it holds of the body's parameters, its impls may step.
pub(super) fn body_aggregates<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Vec<ty::Ty<'tcx>> {
    if !body.thir.exprs.is_empty() {
        return Vec::new();
    }
    let Some(mir) = body.mir.as_ref() else {
        return Vec::new();
    };
    let mut made = Vec::new();
    for data in mir.body.basic_blocks.iter() {
        // And std's struct a std function makes where it's no iterator,
        // `StepBy::new`'s, which this body makes one (ADR 0917).
        if let Some(terminator) = &data.terminator
            && let rustc_middle::mir::TerminatorKind::Call { func, destination, .. } = &terminator.kind
            && func
                .const_fn_def()
                .is_some_and(|(callee, _)| crate::lower::recognition::makes_no_iterator(tcx, callee))
        {
            made.push(destination.ty(&mir.body.local_decls, tcx).ty);
        }
        for statement in &data.statements {
            if let rustc_middle::mir::StatementKind::Assign(assign) = &statement.kind
                && let rustc_middle::mir::Rvalue::Aggregate(kind, _) = &assign.1
                && let rustc_middle::mir::AggregateKind::Adt(..) = **kind
            {
                made.push(assign.0.ty(&mir.body.local_decls, tcx).ty);
            }
        }
    }
    made
}

/// Each closure a body read from its MIR alone makes whose own function
/// isn't the body's, as rustc's MIR inlines that function, std's
/// `flatten::<U, Acc>` into `FlattenCompat::rfold`: that function, and its
/// generics as the body gives them, the closure's parent args (ADR 0899).
pub(super) fn body_inlined_closures<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
) -> Vec<(DefId, ty::GenericArgsRef<'tcx>)> {
    let Some(mir) = body.mir.as_ref() else {
        return Vec::new();
    };
    let root = tcx.typeck_root_def_id(body.def_id);
    let mut made = Vec::new();
    for data in mir.body.basic_blocks.iter() {
        for statement in &data.statements {
            if let rustc_middle::mir::StatementKind::Assign(assign) = &statement.kind
                && let rustc_middle::mir::Rvalue::Aggregate(kind, _) = &assign.1
                && let rustc_middle::mir::AggregateKind::Closure(closure, args) = **kind
                && tcx.typeck_root_def_id(closure) != root
            {
                let own = tcx.typeck_root_def_id(closure);
                made.push((own, tcx.mk_args(args.as_closure().parent_args())));
            }
        }
    }
    made
}

/// Each function a body names, called or taken as a value, with its
/// arguments: of its THIR, each function value; of a body read from its
/// MIR alone, a std one's (ADR 0561), each constant of a
/// function's type.
pub(super) fn body_fn_defs<'tcx>(body: &Body<'tcx>) -> Vec<(DefId, ty::GenericArgsRef<'tcx>)> {
    if !body.thir.exprs.is_empty() {
        return (body.thir.exprs.iter())
            .filter_map(|expr| match (&expr.kind, fn_def(expr.ty)) {
                (ExprKind::ZstLiteral { .. }, Some(def)) => Some(def),
                _ => None,
            })
            .collect();
    }
    let Some(mir) = body.mir.as_ref() else {
        return Vec::new();
    };
    struct FnDefs<'tcx>(Vec<(DefId, ty::GenericArgsRef<'tcx>)>);
    impl<'tcx> rustc_middle::mir::visit::Visitor<'tcx> for FnDefs<'tcx> {
        fn visit_const_operand(
            &mut self,
            constant: &rustc_middle::mir::ConstOperand<'tcx>,
            _: rustc_middle::mir::Location,
        ) {
            if let Some(def) = fn_def(constant.ty()) {
                self.0.push(def);
            }
        }
    }
    let mut found = FnDefs(Vec::new());
    rustc_middle::mir::visit::Visitor::visit_body(&mut found, &mir.body);
    found.0
}

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
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
) -> HashSet<(DefId, u32, DefId)> {
    let iterator = std_item(tcx, StdItem::Iterator);
    let next = trait_method(tcx, iterator, "next");
    let size_hint = trait_method(tcx, iterator, "size_hint");
    let mut stepped: HashSet<(DefId, u32, DefId)> = HashSet::new();
    // What a trait's default steps of an associated type of its `Self`.
    let mut stepped_assoc: Vec<(DefId, DefId, DefId)> = Vec::new();
    // A caller's parameter given on as a callee's, which steps it if that does.
    let mut passed: Vec<(Param, Param)> = Vec::new();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id);
        let typing_env = ty::TypingEnv::post_analysis(tcx, caller);
        // A caller's parameter given to a library's function, a compiled std
        // body's among them (ADR 0561), which its manifest says it steps by
        // a trait of its bounds, or a supertrait of one, `Iterator` of an
        // `I: DoubleEndedIterator`: stepped by it here.
        let steps_through =
            |stepped: &mut HashSet<(DefId, u32, DefId)>, callee: DefId, args: ty::GenericArgsRef<'tcx>| {
                for (index, arg) in args.iter().enumerate() {
                    let Some(ty) = arg.as_type() else { continue };
                    let ty::Param(param) = ty.peel_refs().kind() else {
                        continue;
                    };
                    let generics = tcx.generics_of(callee);
                    if generics.count() <= index {
                        continue;
                    }
                    let theirs = generics.param_at(index, tcx);
                    let theirs = ty::Ty::new_param(tcx, theirs.index, theirs.name);
                    for (clause, _) in tcx.clauses_of(callee).instantiate_identity(tcx) {
                        let Some(bound) = clause.skip_normalization().as_trait_clause() else {
                            continue;
                        };
                        if bound.self_ty().skip_binder() != theirs {
                            continue;
                        }
                        for by in rustc_middle::ty::elaborate::supertrait_def_ids(tcx, bound.def_id()) {
                            if foreign.steps(callee, index as u32, by) {
                                stepped.insert((caller, param.index, by));
                            }
                        }
                    }
                }
            };
        let pass_on = |passed: &mut Vec<_>, callee: DefId, args: ty::GenericArgsRef<'tcx>| {
            for (index, arg) in args.iter().enumerate() {
                // Or a reference to one, `&mut Self` given as an `I:
                // IntoIterator`, made a JS iterator of it (ADR 0551).
                if let Some(ty) = arg.as_type()
                    && let ty::Param(param) = ty.peel_refs().kind()
                    && !given_by_its_trait(tcx, callee, index as u32)
                {
                    passed.push(((caller, param.index), (callee, index as u32)));
                }
            }
        };
        // A closure it makes of a function rustc's MIR inlined: that
        // function's parameters, as if it were called (ADR 0899).
        for (own, args) in body_inlined_closures(tcx, body) {
            pass_on(&mut passed, own, args);
        }
        // A parameter a body read from its MIR holds in a struct, `Peekable {
        // iter: self, .. }` of std's `peekable` (ADR 0561):
        // stepped as the struct's impl steps it, its `next`'s and its
        // `size_hint`'s, which a compiled std iterator is made knowing: a
        // `Rev<I>`'s steps `I` by `DoubleEndedIterator` and hints by
        // `Iterator`.
        for held in body_aggregates(tcx, body) {
            for method in [next, size_hint] {
                if let Ok(Some(instance)) =
                    crate::lower::items::resolve_instance_in(tcx, typing_env, method, tcx.mk_args(&[held.into()]))
                {
                    if fns.contains_key(&instance.def_id()) {
                        pass_on(&mut passed, instance.def_id(), instance.args);
                    } else if foreign.item(instance.def_id()).is_some() {
                        steps_through(&mut stepped, instance.def_id(), instance.args);
                    }
                }
            }
        }
        for (callee, args) in body_fn_defs(body) {
            // A std-private trait's method whose impl the caller's type picks,
            // `FuseImpl::next`'s of a `Fuse<I>` (ADR 0561): stepped as either
            // impl's method steps it.
            if let Some(spec) = crate::lower::recognition::fact_specialization(tcx, typing_env, callee, args) {
                let impl_args = spec.selected.1;
                for method in spec.methods() {
                    if fns.contains_key(&method) {
                        pass_on(&mut passed, method, impl_args);
                    } else if foreign.item(method).is_some() {
                        steps_through(&mut stepped, method, impl_args);
                    }
                    // A std adapter of the caller's parameter it's given,
                    // `FlatMap`'s `Map<I, F>` that `FlattenCompat`'s back
                    // asks a `DoubleEndedIterator` of: what that adapter's
                    // impl asks of it, `I`'s (ADR 0903).
                    let Some(imp) = tcx.impl_of_assoc(method) else { continue };
                    for (clause, _) in tcx.clauses_of(imp).instantiate(tcx, impl_args) {
                        let Some(bound) = clause.skip_normalization().as_trait_clause() else {
                            continue;
                        };
                        let held = bound.self_ty().skip_binder();
                        if matches!(held.kind(), ty::Adt(..)) && is_iterator_extension(tcx, bound.def_id()) {
                            // And `Iterator`, which each extends.
                            let asked = impl_asks(tcx, typing_env, bound.def_id(), held).unwrap_or_default();
                            for (param, by) in asked {
                                stepped.extend([(caller, param, by), (caller, param, iterator)]);
                            }
                        }
                    }
                }
                continue;
            }
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
                // Another std trait's method that asks its `Self` be an
                // iterator, `TrustedRandomAccessNoCoerce::size`'s `where Self:
                // Iterator`, compiled as a function over it (ADR 0975).
                Some(tr) if is_std_crate(tcx, tr) && asks_self_iterator(tcx, callee, iterator) => vec![iterator],
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
                // Of a trait's default, an associated type of its `Self`,
                // `Self::Iter`: stepped of what each impl keeping it says it
                // is, below (ADR 0977).
                if let ty::Alias(_, alias) = self_ty.kind()
                    && let ty::Projection { def_id: assoc } = alias.kind
                    && alias
                        .args
                        .types()
                        .next()
                        .is_some_and(|of| matches!(of.kind(), ty::Param(p) if p.index == 0))
                    && tcx.trait_of_assoc(caller).is_some()
                {
                    stepped_assoc.extend(by.iter().map(|&by| (caller, assoc, by)));
                    continue;
                }
                if let ty::Adt(adt, adapted) = self_ty.kind()
                    && is_std_crate(tcx, adt.did())
                {
                    for &by in by.iter().filter(|&&by| by != iterator) {
                        // A compiled std iterator's, `Peekable<I>`'s (ADR
                        // 0561): its compiled impl's, by what its manifest
                        // says it steps; a rewrite's, `len()`'s, nothing.
                        // Of std's own bodies being compiled, one of them,
                        // `Skip::rfold`'s `self.try_rfold(..)`: what it's given
                        // passed on, as it steps it (ADR 0927).
                        if crate::lower::recognition::is_compiled_std_iterator(tcx, self_ty) {
                            if let Ok(Some(instance)) =
                                crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args)
                            {
                                if fns.contains_key(&instance.def_id()) {
                                    pass_on(&mut passed, instance.def_id(), instance.args);
                                } else if foreign.item(instance.def_id()).is_some() {
                                    steps_through(&mut stepped, instance.def_id(), instance.args);
                                }
                            }
                            // And, where it's asked of it, `rev()`'s, what its
                            // compiled methods a use site gives `$knowing`
                            // step, its `next_back` and `nth_back`, and its
                            // `len`, by their manifests: a held
                            // `Enumerate<I>`'s back asks `I`'s length too (ADR
                            // 0871).
                            if tcx.trait_of_assoc(callee) != Some(by)
                                && crate::lower::recognition::is_double_ended(tcx, by)
                            {
                                let exact = crate::lower::recognition::exact_size_iterator_trait(tcx);
                                let given = [(Some(by), "next_back"), (Some(by), "nth_back"), (exact, "len")];
                                for (tr, name) in given {
                                    let Some(tr) = tr else { continue };
                                    let method = trait_method(tcx, tr, name);
                                    if let Ok(Some(instance)) = ty::Instance::try_resolve(
                                        tcx,
                                        typing_env,
                                        method,
                                        tcx.mk_args(&[self_ty.into()]),
                                    ) && foreign.item(instance.def_id()).is_some()
                                    {
                                        steps_through(&mut stepped, instance.def_id(), instance.args);
                                    }
                                }
                            }
                            continue;
                        }
                        // What its impl of it asks of what it holds, as rustc
                        // proves it: `Enumerate<I>`'s back asks `I`'s length
                        // too (ADR 0542); else the same of each.
                        match impl_asks(tcx, typing_env, by, self_ty) {
                            Some(asked) => stepped.extend(asked.into_iter().map(|(param, by)| (caller, param, by))),
                            None => stepped.extend(adapted.types().filter_map(|ty| match ty.peel_refs().kind() {
                                ty::Param(param) => Some((caller, param.index, by)),
                                _ => None,
                            })),
                        }
                    }
                }
                // The crate's iterator holding it, stepped by its impl's `next`.
                // A compiled std iterator holding it, `Peekable<I>` (ADR 0561),
                // is a JS iterator, stepped with no dictionary of it; what its
                // compiled impl of an extension takes, `next_back`'s of `I:
                // DoubleEndedIterator`, its manifest says.
                if let Ok(Some(instance)) =
                    crate::lower::items::resolve_instance_in(tcx, typing_env, next, tcx.mk_args(&[self_ty.into()]))
                    && fns.contains_key(&instance.def_id())
                {
                    pass_on(&mut passed, instance.def_id(), instance.args);
                }
                continue;
            }
            // A function of the crate's, or an impl's item, a dictionary's
            // made of the impl's evidence.
            let resolved = match crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args) {
                Ok(Some(instance)) if fns.contains_key(&instance.def_id()) => instance,
                // A library's, by what its manifest says it steps.
                Ok(Some(instance)) if foreign.item(instance.def_id()).is_some() => {
                    steps_through(&mut stepped, instance.def_id(), instance.args);
                    continue;
                }
                // A trait's method of a type parameter, its dictionary's: what
                // it's given as the trait declares it, a parameter given as a
                // JS iterator made of one given on (ADR 0551).
                Ok(None) if tcx.trait_of_assoc(callee).is_some() => {
                    pass_on(&mut passed, callee, args);
                    continue;
                }
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
                    crate::lower::items::resolve_instance_in(tcx, typing_env, next, tcx.mk_args(&[given.into()]))
                    && fns.contains_key(&instance.def_id())
                {
                    pass_on(&mut passed, instance.def_id(), instance.args);
                }
            }
        }
    }
    // A trait's default an impl keeps is copied into it (ADR 0049): what it
    // steps of its `Self`, or of a trait's parameter, it steps of the impl's
    // argument for it, a type parameter of the impl's, `T` of `impl<T:
    // Iterator> Trait for T`, as a function steps one passed on; or, of the
    // crate's iterator holding them, `Holder<I>`, what its impl's `next`
    // steps, where it's stepped (ADR 0492).
    let mut kept_through: Vec<(Param, (Param, Param))> = Vec::new();
    for (&trait_id, impls) in tcx.all_local_trait_impls(()) {
        for &imp in impls {
            let imp = imp.to_def_id();
            let typing_env = ty::TypingEnv::post_analysis(tcx, imp);
            let trait_ref = tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization();
            let overridden = tcx.impl_item_implementor_ids(imp);
            let kept = (tcx.associated_items(trait_id).in_definition_order()).filter(|item| {
                item.is_fn() && item.defaultness(tcx).has_value() && !overridden.contains_key(&item.def_id)
            });
            for item in kept {
                // An associated type of its `Self` it steps, of what this
                // impl says it is, a type parameter of the impl's, `I` of
                // `type Iter = I` (ADR 0977).
                for &(_, assoc, by) in stepped_assoc.iter().filter(|&&(default, ..)| default == item.def_id) {
                    if let Some(&own) = overridden.get(&assoc)
                        && let ty::Param(param) = tcx.type_of(own).instantiate_identity().skip_normalization().kind()
                    {
                        stepped.insert((imp, param.index, by));
                    }
                }
                for (index, arg) in trait_ref.args.iter().enumerate() {
                    let Some(ty) = arg.as_type() else { continue };
                    let default = (item.def_id, index as u32);
                    if let ty::Param(param) = ty.kind() {
                        passed.push(((imp, param.index), default));
                    } else if let Ok(Some(instance)) =
                        crate::lower::items::resolve_instance_in(tcx, typing_env, next, tcx.mk_args(&[ty.into()]))
                        && fns.contains_key(&instance.def_id())
                    {
                        for (at, held) in instance.args.iter().enumerate() {
                            if let Some(held) = held.as_type()
                                && let ty::Param(param) = held.kind()
                            {
                                kept_through.push((default, ((imp, param.index), (instance.def_id(), at as u32))));
                            }
                        }
                    }
                }
            }
        }
    }
    // An impl's associated type bound by `Iterator`, `IntoIterator`'s
    // `IntoIter`: its dictionary gives that type's, made of the impl's
    // evidence, so the impl steps what that type's `next` steps, of a
    // parameter passed on as it is, or held in a std adapter, `Fuse<I>`
    // (ADR 0527).
    for impls in tcx.all_local_trait_impls(()).values() {
        for &imp in impls {
            let imp = imp.to_def_id();
            let typing_env = ty::TypingEnv::post_analysis(tcx, imp);
            for item in tcx
                .associated_items(imp)
                .in_definition_order()
                .filter(|item| item.is_type())
            {
                let Some(declared) = item.trait_item_def_id() else {
                    continue;
                };
                let iterates = (tcx.item_bounds(declared).skip_binder().iter())
                    .filter_map(|clause| clause.as_trait_clause())
                    .any(|p| p.def_id() == iterator);
                let value = tcx.type_of(item.def_id).instantiate_identity().skip_normalization();
                let Ok(Some(instance)) =
                    crate::lower::items::resolve_instance_in(tcx, typing_env, next, tcx.mk_args(&[value.into()]))
                else {
                    continue;
                };
                if !iterates || !fns.contains_key(&instance.def_id()) {
                    continue;
                }
                for (at, held) in instance.args.iter().enumerate() {
                    let Some(held) = held.as_type() else { continue };
                    match held.kind() {
                        ty::Param(param) => passed.push(((imp, param.index), (instance.def_id(), at as u32))),
                        _ => stepped.extend(held.walk().filter_map(|part| match part.as_type()?.kind() {
                            ty::Param(param) => Some((imp, param.index, iterator)),
                            _ => None,
                        })),
                    }
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
        // An `IntoIterator`'s too, given the JS iterator of what it's given.
        let iterates = bounds
            .iter()
            .any(|&tr| tr == iterator || is_iterator_extension(tcx, tr) || is_std_def(tcx, tr, StdItem::IntoIterator));
        if !iterates || given_as_it_is(tcx, id, param) {
            return Vec::new();
        }
        let extensions = bounds.into_iter().filter(|&tr| is_iterator_extension(tcx, tr));
        std::iter::once(iterator).chain(extensions).collect()
    };
    let mut changed = true;
    while changed {
        changed = false;
        // The crate's iterator a kept default steps, its `Self`: its impl's
        // `next`'s, passed on.
        for &(default, edge) in &kept_through {
            if stepped.iter().any(|&(id, index, _)| (id, index) == default) && !passed.contains(&edge) {
                passed.push(edge);
                changed = true;
            }
        }
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

/// What std's impl of `by`, an iterator's extension, for `adapter` asks of
/// the type parameters it holds, as rustc proves it: `I: ExactSizeIterator`
/// and `I: DoubleEndedIterator` of `Enumerate<I>: DoubleEndedIterator` (ADR
/// 0542). `None` where rustc names no impl.
fn impl_asks<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    by: DefId,
    adapter: ty::Ty<'tcx>,
) -> Option<Vec<(u32, DefId)>> {
    let tr = ty::TraitRef::new(tcx, by, [adapter]);
    let Ok(rustc_middle::traits::ImplSource::UserDefined(imp)) =
        tcx.codegen_select_candidate(typing_env.as_query_input(tr))
    else {
        return None;
    };
    let clauses = tcx.clauses_of(imp.impl_def_id).instantiate(tcx, imp.args);
    Some(
        (clauses.into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .filter(|p| is_iterator_extension(tcx, p.def_id()))
            // Through what refers to it, `&mut R` of `Map<&mut R, F>`.
            .filter_map(|p| match p.self_ty().skip_binder().peel_refs().kind() {
                ty::Param(param) => Some((param.index, p.def_id())),
                _ => None,
            })
            .collect(),
    )
}

/// Whether `method`, a trait's, bounds its `Self` by `iterator`, `where
/// Self: Iterator`.
fn asks_self_iterator(tcx: TyCtxt<'_>, method: DefId, iterator: DefId) -> bool {
    let self_param = ty::Ty::new_param(tcx, 0, rustc_span::symbol::kw::SelfUpper);
    (tcx.clauses_of(method).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .any(|bound| bound.self_ty().skip_binder() == self_param && bound.def_id() == iterator)
}
