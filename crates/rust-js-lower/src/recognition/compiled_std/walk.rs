//! Finding std's bodies `@rust-js/std` compiles by walking std's MIR: each
//! the catalog's rows name, and each private one a compiled body reaches,
//! a `?`'s conversion or a `{:?}` it shows. The walk `catalogue` keeps
//! (`stored.rs`) and std's own build makes. Split from `compiled_std.rs` by
//! concept (#1334).

use super::*;

/// std's bodies `@rust-js/std` compiles of a build that calls the adapters
/// `called` says, of std's MIR: each the catalog's rows name, and each
/// private one a compiled body reaches, in the order found.
pub(super) fn walk(tcx: TyCtxt<'_>, called: &dyn Fn(StdPath) -> bool) -> rust_js_program::collections::IndexSet<DefId> {
    let rows = compiled_rows(tcx);
    let mut found = rust_js_program::collections::IndexSet::default();
    for &((of, ..), ref methods) in &rows.iter {
        if let Of::Primitives = of {
            found.extend(methods);
        } else if let Of::Struct(item) = of
            && called(item)
        {
            found.extend(methods);
        }
    }
    for (_, methods) in &rows.io {
        found.extend(methods);
    }
    // Its makers, but a `MEANING_ONLY` adapter's outside a build of it.
    let made_for_build = |method: &str| {
        (HELD_COMPILED.iter())
            .find(|&&(_, maker, _)| maker == method)
            .is_none_or(|&(item, ..)| called(item))
    };
    for &(trait_name, method) in COMPILED_STD_PROVIDED
        .iter()
        .filter(|&&(_, method)| made_for_build(method))
    {
        found.insert(rows.provided(trait_name, method));
    }
    // `io`'s traits' provided methods (#1156): each a function over `Self`,
    // given its dictionary, as `Iterator`'s are.
    found.extend(&rows.io_provided);
    // Std's impls of a trait for tuples, each method (#2395).
    found.extend(&rows.tuples);
    for &(id, makes) in &rows.fns {
        if makes.is_none_or(&called) {
            found.insert(id);
        }
    }
    // And each function std keeps private that a compiled body calls,
    // `Fuse`'s `and_then_or_clear`, compiled with it, as its closures are:
    // no program can name it, so no recognizer rewrites it, but `io`'s and
    // `Option::expect`'s panic, `expect_failed`, which one does.
    let answered = |id| super::io::answered_io_fn(tcx, id) || super::std_items::is_expect_failed(tcx, id);
    let mut at = 0;
    while let Some(&def_id) = found.get_index(at) {
        at += 1;
        if !tcx.is_mir_available(def_id) {
            continue;
        }
        let typing_env = ty::TypingEnv::post_analysis(tcx, def_id);
        // Of the blocks it runs, a precondition check's left out, as
        // rust-js reads `UbChecks` as off (ADRs 0364, 0917).
        let body = tcx.optimized_mir(def_id);
        let blocks = crate::std_coverage::reachable(tcx, body);
        // And each one std keeps private that it names as a value, which
        // what it's given to calls, `FlattenCompat::last`'s own `last` given
        // to its `iter_fold`, or a closure it makes holds, `into_item` in
        // an inlined `filter_map` (#645).
        for value in named_fns(body, &blocks) {
            if matches!(tcx.def_kind(value), DefKind::Fn)
                && !tcx.visibility(value).is_public()
                && !answered(value)
                && tcx.is_mir_available(value)
            {
                found.insert(value);
            }
        }
        // And the `Debug` of a type std keeps private that it shows through
        // a builder, `.field("inner", &self.inner)` of a `Flatten`'s
        // `FlattenCompat`, by the `&dyn Debug` it makes of it (#2252).
        found.extend(shown_private(tcx, typing_env, body, &blocks));
        for &block in &blocks {
            let Some(terminator) = &body.basic_blocks[block].terminator else {
                continue;
            };
            let (TerminatorKind::Call { func, .. } | TerminatorKind::TailCall { func, .. }) = &terminator.kind else {
                continue;
            };
            let Some((callee, args)) = func.const_fn_def() else {
                continue;
            };
            // Not a `?` of an `Option` or a `Result`, which the lowering
            // writes itself, `Chain::read`'s `self.first.read(buf)?`: no
            // body of `Try`'s is compiled for it (#1156).
            if super::is_question_mark_written(tcx, callee, args) {
                continue;
            }
            // And each method of a trait's impl for a type std keeps
            // private, `FlattenCompat`'s `next` (ADR 0899), or of a trait
            // std keeps private, `SpecTake`'s `spec_fold` of a `Take`
            // (ADR 0913).
            let of_private_type = |called: DefId| {
                tcx.trait_impl_of_assoc(called).is_some_and(|imp| {
                    matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(adt, _) if !tcx.visibility(adt.did()).is_public())
                        || std_private(tcx, tcx.impl_trait_id(imp))
                })
            };
            // Of one whose impl a fact picks, `FlattenCompat`'s `size_hint`
            // of a `OneShot`'s or not, both impls' (ADR 0899).
            // And where rustc resolves its default, which a fact picks all the
            // same, `SpecFold`'s of a zip on `TrustedLen` (#380).
            let spec = super::fact_specialization(tcx, typing_env, callee, args);
            let called: Vec<DefId> = match crate::items::resolve_instance_in(tcx, typing_env, callee, args) {
                Ok(Some(instance)) if spec.as_ref().is_none_or(|spec| spec.selected.0 != instance.def_id()) => {
                    match instance.def {
                        ty::InstanceKind::Item(called) => vec![called],
                        _ => Vec::new(),
                    }
                }
                _ => spec.map(|spec| spec.methods().collect()).unwrap_or_default(),
            };
            for called in called {
                if matches!(tcx.def_kind(called), DefKind::Fn | DefKind::AssocFn)
                    && !answered(called)
                    && ((!tcx.visibility(called).is_public()
                        && tcx.trait_of_assoc(called).is_none()
                        && tcx.trait_impl_of_assoc(called).is_none())
                        || of_private_type(called))
                    && tcx.is_mir_available(called)
                {
                    found.insert(called);
                }
            }
        }
    }
    found
}

/// Each function the constants of the blocks `body` runs, `blocks`, name,
/// as the value they are or one a closure they are holds.
fn named_fns<'tcx>(body: &rustc_middle::mir::Body<'tcx>, blocks: &[rustc_middle::mir::BasicBlock]) -> Vec<DefId> {
    use rustc_middle::mir;
    struct Named(Vec<DefId>);
    impl<'tcx> mir::visit::Visitor<'tcx> for Named {
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
            for part in constant.ty().walk() {
                if let Some(ty) = part.as_type()
                    && let ty::FnDef(def_id, _) = *ty.kind()
                {
                    self.0.push(def_id);
                }
            }
        }
    }
    let mut named = Named(Vec::new());
    for &block in blocks {
        mir::visit::Visitor::visit_basic_block_data(&mut named, block, &body.basic_blocks[block]);
    }
    named.0
}

/// The `Debug::fmt` of each type std keeps private that `body` makes a
/// `&dyn Debug` of where it runs, of `blocks`, a builder's field's, `&self.inner` of a
/// `FlattenCompat` (#2252): its impl's, as rustc selects it there.
fn shown_private<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    blocks: &[rustc_middle::mir::BasicBlock],
) -> Vec<DefId> {
    use rustc_middle::mir::{CastKind, Rvalue, StatementKind};
    use rustc_middle::ty::adjustment::PointerCoercion;
    let Some(debug) = tcx.get_diagnostic_item(Symbol::intern("Debug")) else {
        return Vec::new();
    };
    let fmt = super::trait_method(tcx, debug, "fmt");
    let mut shown = Vec::new();
    for &block in blocks {
        for statement in &body.basic_blocks[block].statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::Unsize, _), operand, to) = &assign.1 else {
                continue;
            };
            let to_debug = matches!(to.builtin_deref(true).map(|ty| ty.kind()),
                Some(ty::Dynamic(preds, ..)) if preds.principal_def_id() == Some(debug));
            let from = operand.ty(&body.local_decls, tcx).builtin_deref(true);
            let Some(from @ ty) = from.filter(|_| to_debug) else {
                continue;
            };
            let ty::Adt(adt, _) = ty.kind() else { continue };
            if tcx.visibility(adt.did()).is_public() || !super::is_std_crate(tcx, adt.did()) {
                continue;
            }
            if let Ok(Some(instance)) =
                crate::items::resolve_instance_in(tcx, typing_env, fmt, tcx.mk_args(&[from.into()]))
                && let ty::InstanceKind::Item(id) = instance.def
                && tcx.is_mir_available(id)
            {
                shown.push(id);
            }
        }
    }
    shown
}
