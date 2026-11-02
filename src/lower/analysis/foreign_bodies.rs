//! The bodies of another crate's functions and constants a crate lowers,
//! from the MIR its metadata keeps: std's (ADR 0561), and a library's a
//! consumer compiles itself (ADR 0914), its impls' constants too (ADR
//! 0176, amended).

use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

use super::super::Body;

/// The bodies of `ids`, another crate's, from the MIR its metadata keeps,
/// with no THIR beside them: std's (ADR 0561), and a library's a consumer
/// compiles itself (ADR 0914), its impls' constants too (ADR 0176).
pub(in crate::lower) fn foreign_bodies(tcx: TyCtxt<'_>, mut ids: Vec<DefId>) -> Vec<Body<'_>> {
    // Each body named, and each closure a body makes, which its MIR names
    // as it's made: a body of its own, as the crate's closures are.
    let mut at = 0;
    while at < ids.len() {
        let def_id = ids[at];
        at += 1;
        let Some(body) = foreign_mir(tcx, def_id) else {
            continue;
        };
        // Made of what it captures, or, capturing nothing, named as a
        // constant.
        struct Closures(Vec<DefId>);
        impl<'tcx> rustc_middle::mir::visit::Visitor<'tcx> for Closures {
            fn visit_rvalue(&mut self, rvalue: &rustc_middle::mir::Rvalue<'tcx>, _: rustc_middle::mir::Location) {
                if let rustc_middle::mir::Rvalue::Aggregate(kind, _) = rvalue
                    && let rustc_middle::mir::AggregateKind::Closure(closure, _) = **kind
                {
                    self.0.push(closure);
                }
            }
            fn visit_const_operand(
                &mut self,
                constant: &rustc_middle::mir::ConstOperand<'tcx>,
                _: rustc_middle::mir::Location,
            ) {
                if let rustc_middle::ty::Closure(closure, _) = *constant.ty().kind() {
                    self.0.push(closure);
                }
            }
        }
        let mut closures = Closures(Vec::new());
        rustc_middle::mir::visit::Visitor::visit_body(&mut closures, body);
        // And each a local holds, of a body rustc inlined into this one,
        // `sum`'s in `Filter::count` of a std built to unwind (#424).
        for decl in &body.local_decls {
            for part in decl.ty.walk().filter_map(|part| part.as_type()) {
                if let rustc_middle::ty::Closure(closure, _) = *part.kind() {
                    closures.0.push(closure);
                }
            }
        }
        for closure in closures.0 {
            if !ids.contains(&closure) {
                ids.push(closure);
            }
        }
    }
    (ids.into_iter())
        .filter_map(|def_id| Some((def_id, foreign_mir(tcx, def_id)?)))
        .map(|(def_id, body)| {
            // What rustc clears of MIR across crates, each local's kind, is
            // what an optimized body has none of but the plain: rustc's own
            // dataflow, which says what's dropped, asks it of each local.
            let mut body = body.clone();
            for decl in body.local_decls.iter_mut() {
                decl.local_info =
                    rustc_middle::mir::ClearCrossCrate::Set(Box::new(rustc_middle::mir::LocalInfo::Boring));
            }
            let mir = super::super::mir::Mir::new(tcx, body, tcx.promoted_mir(def_id).clone());
            Body {
                def_id,
                thir: rustc_middle::thir::Thir::new(rustc_middle::thir::BodyTy::Const(tcx.types.unit)),
                expr: rustc_middle::thir::ExprId::from_u32(0),
                facts: super::super::body_queries::BodyFacts::default(),
                mir: Some(mir),
            }
        })
        .collect()
}

/// The MIR another crate's metadata keeps of `def_id`: a function's, as
/// it's optimized; an impl's constant's, as rustc evaluates it, all it
/// keeps of one, which the impl's dictionary lowers where rustc can't
/// compute it (ADR 0176, amended): of one with an initializer, as a `type
/// const` has none.
fn foreign_mir(tcx: TyCtxt<'_>, def_id: DefId) -> Option<&rustc_middle::mir::Body<'_>> {
    match tcx.def_kind(def_id) {
        DefKind::AssocConst { is_type_const } => {
            (!is_type_const && tcx.defaultness(def_id).has_value()).then(|| tcx.mir_for_ctfe(def_id))
        }
        _ => tcx.is_mir_available(def_id).then(|| tcx.optimized_mir(def_id)),
    }
}
