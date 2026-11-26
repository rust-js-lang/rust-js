//! The items a function's body writes as its own (ADR 0308): a function a
//! block makes and gives, JS's named function expression where the block
//! is (ADR 0296); a function only that body names, a declaration there;
//! and a `const` only what's written in that body reads, a `const` there
//! (ADR 0031, amended).

use crate::Body;
use crate::bindings::is_binding;
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};

/// The functions a block makes and gives, `{ fn Label(..) { .. } Label }`,
/// that nothing else names: each is JS's named function expression where
/// the block is, as react.dev's `memo(function Label(..) { .. })` (ADR 0296).
pub(super) fn named_expressions<'a, 'tcx: 'a>(
    tcx: TyCtxt<'tcx>,
    bodies: impl Iterator<Item = &'a Body<'tcx>>,
) -> IndexSet<DefId> {
    let mut named: IndexMap<DefId, usize> = IndexMap::default();
    for body in bodies {
        for expr in body.thir.exprs.iter() {
            if let ExprKind::ZstLiteral { .. } = expr.kind
                && let ty::FnDef(def_id, _) = *expr.ty.kind()
                && def_id.is_local()
            {
                *named.entry(def_id).or_default() += 1;
            }
        }
    }
    named
        .into_iter()
        .filter(|&(def_id, uses)| uses == 1 && given_by_its_block(tcx, def_id.expect_local()))
        .map(|(def_id, _)| def_id)
        .collect()
}

/// The functions written in a function's body that only it and others of
/// them name: each is a function declaration where it's written there
/// (ADR 0308). One a static's initializer names stays the module's, as that
/// initializer is.
pub(super) fn local_functions<'a, 'tcx: 'a>(
    tcx: TyCtxt<'tcx>,
    bodies: impl Iterator<Item = &'a Body<'tcx>>,
    (names, named_expressions): (&crate::call_graph::Names<'tcx>, &IndexSet<DefId>),
) -> IndexMap<DefId, rustc_span::Span> {
    let mut users: IndexMap<DefId, IndexSet<DefId>> = IndexMap::default();
    for body in bodies {
        let user = tcx.typeck_root_def_id(body.def_id);
        for &(named, _, _, _) in names.names_of(body.def_id) {
            users.entry(named).or_default().insert(user);
        }
    }
    let mut local: IndexSet<DefId> = (users.keys().copied())
        .filter(|&def_id| {
            def_id.as_local().is_some_and(|local| {
                tcx.def_kind(local) == DefKind::Fn
                    && !named_expressions.contains(&def_id)
                    && !is_binding(tcx, def_id)
                    && matches!(
                        tcx.parent_hir_node(tcx.local_def_id_to_hir_id(local)),
                        hir::Node::Stmt(_)
                    )
                    && matches!(
                        tcx.def_kind(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id())),
                        DefKind::Fn | DefKind::AssocFn
                    )
                    // Not of a trait's default, whose body each impl's
                    // dictionary copies into its own module (ADR 0185): the
                    // trait's module's, which each copy imports.
                    && tcx
                        .trait_of_assoc(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id()))
                        .is_none()
            })
        })
        .collect();
    // Named elsewhere than in its function, or in another of them that
    // isn't one, it's the module's.
    loop {
        let parent = |def_id: DefId| tcx.typeck_root_def_id(tcx.local_parent(def_id.expect_local()).to_def_id());
        let outside: Vec<DefId> = (local.iter().copied())
            .filter(|&def_id| {
                users[&def_id]
                    .iter()
                    .any(|&user| user != def_id && user != parent(def_id) && !local.contains(&user))
            })
            .collect();
        if outside.is_empty() {
            break;
        }
        for def_id in outside {
            local.swap_remove(&def_id);
        }
    }
    // Each in the block it's written in.
    (local.into_iter())
        .filter_map(|def_id| {
            let stmt = tcx.parent_hir_node(tcx.local_def_id_to_hir_id(def_id.expect_local()));
            let hir::Node::Stmt(stmt) = stmt else { return None };
            let hir::Node::Block(block) = tcx.parent_hir_node(stmt.hir_id) else {
                return None;
            };
            Some((def_id, block.span))
        })
        .collect()
}

/// The `const`s written in a function's body that only what's written in
/// that body reads: the function, its closures and the functions written
/// in it that are its own (ADR 0308). Each is a `const` in the body it's
/// written in (ADR 0031, amended), and what reads it there, by each
/// `const`, the bodies that do. One a function of the module's reads, a
/// static's or another constant's initializer, or a `const { .. }`'s, is
/// the module's. Not a thread-local's (ADR 0270), nor one of a trait's
/// default, whose body each impl's dictionary copies into its own module
/// (ADR 0185).
pub(super) fn local_consts<'a, 'tcx: 'a>(
    tcx: TyCtxt<'tcx>,
    bodies: impl Iterator<Item = &'a Body<'tcx>>,
    consts: &[LocalDefId],
    local_functions: &IndexMap<DefId, rustc_span::Span>,
) -> super::LocalConsts {
    let mut readers: IndexMap<DefId, IndexSet<DefId>> = IndexMap::default();
    for body in bodies {
        for named in crate::body_queries::consts_named(&body.thir) {
            readers.entry(named).or_default().insert(body.def_id);
        }
    }
    // The function of the module's an item is written in, through the
    // closures and the functions written in it, which are in its body too.
    let function = |def_id: DefId| {
        let mut at = tcx.parent(def_id);
        while tcx.is_closure_like(at) || local_functions.contains_key(&at) {
            at = tcx.parent(at);
        }
        at
    };
    (consts.iter())
        .filter(|&&local| {
            matches!(tcx.def_kind(local), DefKind::Const)
                && tcx.item_name(local) != rustc_span::kw::Underscore
                && !super::is_thread_local(tcx, local)
                && super::in_thread_local(tcx, local).is_none()
                && matches!(
                    tcx.def_kind(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id())),
                    DefKind::Fn | DefKind::AssocFn
                )
                && tcx
                    .trait_of_assoc(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id()))
                    .is_none()
        })
        .filter_map(|&local| {
            let def_id = local.to_def_id();
            let readers = readers.swap_remove(&def_id).unwrap_or_default();
            let at = function(def_id);
            let within = readers.iter().all(|&reader| {
                reader == at
                    || ((tcx.is_closure_like(reader) || local_functions.contains_key(&reader))
                        && function(reader) == at)
            });
            // In the block it's written in, as a statement.
            let hir::Node::Stmt(stmt) = tcx.parent_hir_node(tcx.local_def_id_to_hir_id(local)) else {
                return None;
            };
            let hir::Node::Block(block) = tcx.parent_hir_node(stmt.hir_id) else {
                return None;
            };
            within.then_some((def_id, (block.span, readers)))
        })
        .collect()
}

/// Is `def_id` a function its block makes and then gives as its value?
fn given_by_its_block(tcx: TyCtxt<'_>, def_id: LocalDefId) -> bool {
    if tcx.def_kind(def_id) != DefKind::Fn || tcx.generics_of(def_id).count() != 0 {
        return false;
    }
    let hir::Node::Stmt(stmt) = tcx.parent_hir_node(tcx.local_def_id_to_hir_id(def_id)) else {
        return false;
    };
    let hir::Node::Block(block) = tcx.parent_hir_node(stmt.hir_id) else {
        return false;
    };
    matches!(block.expr, Some(hir::Expr { kind: hir::ExprKind::Path(hir::QPath::Resolved(None, path)), .. })
        if path.res == Res::Def(DefKind::Fn, def_id.to_def_id()))
}
