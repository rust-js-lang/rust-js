//! The trait constants the crate's generic code reads, which its
//! dictionaries carry (ADR 0106), and a program's contract lists for its
//! tests (ADR 1076, amended, #597).

use crate::collections::IndexSet;

use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;

use crate::lower::Body;

/// The trait constants generic code reads, `S::SIDES` (ADR 0106): a
/// dictionary has only these, as rustc evaluates only the constants a
/// program uses, and a default no impl uses may not evaluate.
pub(super) fn generic_consts(tcx: TyCtxt<'_>, all_bodies: &[&Body<'_>]) -> IndexSet<DefId> {
    let generic = |def_id: DefId, args: ty::GenericArgsRef<'_>| {
        (tcx.trait_of_assoc(def_id).is_some() && args.has_non_region_param()).then_some(def_id)
    };
    let mut consts = IndexSet::default();
    // A `const { .. }` block of a type parameter's, which rustc can't say
    // as a value, is lowered as code where it's used, its MIR's
    // (`crate_const_as_code`): its trait constants are generic code's
    // reads too, `T::V` of `const { T::V * 10 + 1 }`, and so are an
    // inline constant's inside it (#1387).
    let mut blocks: Vec<DefId> = Vec::new();
    // A trait's default, `const A: u8 = Self::B`, reads only where it's
    // lowered, a library's dictionary, which has every constant: here its
    // `Self::B` would ask the program's of a default it never reads, which
    // may not evaluate, `B = Self::A` (rustc's defaults-cyclic-pass).
    for body in all_bodies.iter().filter(|body| {
        !(matches!(tcx.def_kind(body.def_id), DefKind::AssocConst) && tcx.trait_of_assoc(body.def_id).is_some())
    }) {
        consts.extend(body.thir.exprs.iter().filter_map(|expr| match expr.kind {
            ExprKind::NamedConst { def_id, args, .. } => generic(def_id, args),
            _ => None,
        }));
        blocks.extend(body.thir.exprs.iter().filter_map(|expr| match expr.kind {
            ExprKind::ConstBlock { did, args } if args.has_non_region_param() => Some(did),
            _ => None,
        }));
        // A library's body the crate compiles, of its MIR alone (ADR 0914):
        // `Self::CAPACITY` of arrayvec's `ArrayVecImpl::push`, which the
        // impl's dictionary the crate compiles beside it must have (ADR
        // 0176, amended).
        if body.root().is_none()
            && let Some(mir) = &body.mir
        {
            struct Named<'a, 'tcx>(
                &'a dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<DefId>,
                &'a mut IndexSet<DefId>,
            );
            impl<'tcx> rustc_middle::mir::visit::Visitor<'tcx> for Named<'_, 'tcx> {
                fn visit_const_operand(
                    &mut self,
                    constant: &rustc_middle::mir::ConstOperand<'tcx>,
                    _: rustc_middle::mir::Location,
                ) {
                    if let rustc_middle::mir::Const::Unevaluated(uv, _) = constant.const_
                        && uv.promoted.is_none()
                    {
                        self.1.extend((self.0)(uv.def, uv.args));
                    }
                }
            }
            rustc_middle::mir::visit::Visitor::visit_body(&mut Named(&generic, &mut consts), &mir.body);
        }
    }
    let mut seen: IndexSet<DefId> = IndexSet::default();
    while let Some(block) = blocks.pop() {
        if !seen.insert(block) {
            continue;
        }
        let mir = tcx.mir_for_ctfe(block);
        for constant in constants_of(mir) {
            if let rustc_middle::mir::Const::Unevaluated(uv, _) = constant
                && uv.promoted.is_none()
            {
                consts.extend(generic(uv.def, uv.args));
                if matches!(tcx.def_kind(uv.def), DefKind::AnonConst) && uv.args.has_non_region_param() {
                    blocks.push(uv.def);
                }
            }
        }
    }
    consts
}

/// Each constant `mir` reads.
fn constants_of<'tcx>(mir: &rustc_middle::mir::Body<'tcx>) -> Vec<rustc_middle::mir::Const<'tcx>> {
    struct Each<'tcx>(Vec<rustc_middle::mir::Const<'tcx>>);
    impl<'tcx> rustc_middle::mir::visit::Visitor<'tcx> for Each<'tcx> {
        fn visit_const_operand(
            &mut self,
            constant: &rustc_middle::mir::ConstOperand<'tcx>,
            _: rustc_middle::mir::Location,
        ) {
            self.0.push(constant.const_);
        }
    }
    let mut each = Each(Vec::new());
    rustc_middle::mir::visit::Visitor::visit_body(&mut each, mir);
    each.0
}
