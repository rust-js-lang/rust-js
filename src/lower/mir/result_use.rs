//! What a call's `Result` is to the body that makes it, the one owner of
//! it (#1545, after #1524): a call whose error the lowering keeps as a
//! throw, a `write!` into a `Formatter` or a serde protocol call, is
//! written as it is, its error thrown on, only where that's all the body
//! does with it: returns it, or gives it on by `?`, whose `From` of the
//! error is the error itself. Anywhere else the body looks at it, and the
//! call gives its `Result` as a value.

use crate::lower::recognition::Recognition;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::visit::{MutatingUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{self, Local, Place, TerminatorKind};
use rustc_middle::ty;

/// What a body does with a call's `Result`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum ResultUse {
    /// Returned, or given on by `?` as it is: its error may be thrown.
    Propagated,
    /// Looked at, `match`ed, asked `is_err()`, mapped, dropped, kept: a value.
    Inspected,
}

/// What `body` does with the `Result` a call stores to `destination`:
/// `Propagated` where it's the body's own result, or a local read once, by
/// `?`'s `Try::branch`, in a body whose own result's error is the call's,
/// so `?`'s `From` changes nothing; else `Inspected`.
pub(in crate::lower) fn result_use<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    body: &mir::Body<'tcx>,
    destination: Place<'tcx>,
) -> ResultUse {
    struct Uses {
        local: Local,
        count: usize,
    }
    impl<'tcx> Visitor<'tcx> for Uses {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
            // Not its drop, a cleanup's as a panic unwinds: no use of its value.
            let dropped = matches!(context, PlaceContext::MutatingUse(MutatingUseContext::Drop));
            if local == self.local && context.is_use() && !context.is_place_assignment() && !dropped {
                self.count += 1;
            }
        }
    }
    if !destination.projection.is_empty() {
        return ResultUse::Inspected;
    }
    if destination.local == mir::RETURN_PLACE {
        return ResultUse::Propagated;
    }
    // `?`'s `From` of the error: the error itself, the body's own error
    // type the call's.
    let tcx = recognition.tcx;
    let error_of = |ty: ty::Ty<'tcx>| match ty.kind() {
        ty::Adt(_, args) if recognition.is_result(ty) => Some(tcx.erase_and_anonymize_regions(args.type_at(1))),
        _ => None,
    };
    let given = error_of(body.local_decls[destination.local].ty);
    if given.is_none() || given != error_of(body.return_ty()) {
        return ResultUse::Inspected;
    }
    let mut uses = Uses {
        local: destination.local,
        count: 0,
    };
    uses.visit_body(body);
    let tried = (body.basic_blocks.iter()).filter(|block| {
        matches!(&block.terminator().kind, TerminatorKind::Call { func, args, .. }
            if func.const_fn_def().is_some_and(|(id, _)| tcx.is_lang_item(id, LangItem::TryTraitBranch))
                && args.iter().any(|arg| arg.node.place().is_some_and(|p| p.local == destination.local && p.projection.is_empty())))
    });
    match uses.count == 1 && tried.count() == 1 {
        true => ResultUse::Propagated,
        false => ResultUse::Inspected,
    }
}
