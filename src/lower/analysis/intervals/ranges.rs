//! The ranges of integers a `for` loop's `next()` alone steps (#1672 M3,
//! ADR 1137): which locals hold one only `next()` changes.

use super::*;

/// Each use of a local, as MIR's visitor tells it: how, where, and whether
/// of a projection of it.
type Use = (mir::visit::PlaceContext, Location, bool);

/// Whether `lent`, a temporary `&mut` of a range, is `next()`'s argument
/// alone, itself or a reborrow of it, `&mut (*lent)`, at most a few deep.
fn given_to_next<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    uses: &IndexVec<Local, Vec<Use>>,
    lent: Local,
    depth: u32,
) -> bool {
    use rustc_data_structures::either::Either;
    use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext, PlaceContext};
    depth < 4
        && (uses[lent].iter()).all(|&(context, at, projected)| match (context, body.stmt_at(at)) {
            (PlaceContext::NonUse(_), _) => true,
            (PlaceContext::MutatingUse(MutatingUseContext::Store), _) => !projected,
            (
                PlaceContext::NonMutatingUse(NonMutatingUseContext::Copy | NonMutatingUseContext::Move),
                Either::Right(terminator),
            ) => {
                if let TerminatorKind::Call { func, args, .. } = &terminator.kind
                    && let Some((callee, callee_args)) = func.const_fn_def()
                {
                    !projected
                        && range_step(tcx, callee, callee_args) == Some(RangeStep::Next)
                        && args.first().and_then(|arg| arg.node.place()).and_then(|p| p.as_local()) == Some(lent)
                } else {
                    false
                }
            }
            (PlaceContext::MutatingUse(MutatingUseContext::Borrow), Either::Left(statement)) => {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) = &assign.1
                    && let Some(again) = assign.0.as_local()
                {
                    of.local == lent
                        && matches!(of.projection.as_slice(), [mir::PlaceElem::Deref])
                        && given_to_next(tcx, body, uses, again, depth + 1)
                } else {
                    false
                }
            }
            _ => false,
        })
}

/// The ranges of integers, by their locals, `next()` alone changes, the
/// only ones a range's record is kept of: never written a field at a time,
/// `r.end = 100`, which no record of `r` would see; each
/// `&mut` of one a temporary its `next()` alone is given, and nothing
/// written into its fields. Its end never changes, and its start only
/// grows, so each value `next()` gives is below the one and at least
/// what the other was.
pub(super) fn stepped_ranges<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> IndexSet<Local> {
    use rustc_middle::mir::visit::{MutatingUseContext, PlaceContext, Visitor};
    struct Uses {
        uses: IndexVec<Local, Vec<Use>>,
    }
    impl<'tcx> Visitor<'tcx> for Uses {
        fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, location: Location) {
            self.uses[place.local].push((context, location, !place.projection.is_empty()));
        }
    }
    let mut uses = Uses {
        uses: IndexVec::from_elem_n(Vec::new(), body.local_decls.len()),
    };
    uses.visit_body(body);
    // A temporary `&mut` of a range, `next()`'s argument alone.
    (body.local_decls.indices())
        .filter(|&local| {
            let ty = body.local_decls[local].ty;
            matches!(ty.kind(), ty::Adt(adt, item) if tcx.is_lang_item(adt.did(), LangItem::Range)
                && item.types().next().is_some_and(|item| item.is_integral()))
        })
        .filter(|&range| {
            (uses.uses[range].iter()).all(|&(context, at, projected)| match context {
                PlaceContext::NonUse(_) | PlaceContext::NonMutatingUse(_) => true,
                PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call) => !projected,
                PlaceContext::MutatingUse(MutatingUseContext::Borrow) => {
                    !projected
                        && match body.stmt_at(at) {
                            rustc_data_structures::either::Either::Left(statement) => {
                                if let StatementKind::Assign(assign) = &statement.kind
                                    && let Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) = &assign.1
                                    && let Some(lent) = assign.0.as_local()
                                {
                                    of.as_local() == Some(range) && given_to_next(tcx, body, &uses.uses, lent, 0)
                                } else {
                                    false
                                }
                            }
                            rustc_data_structures::either::Either::Right(_) => false,
                        }
                }
                _ => false,
            })
        })
        .collect()
}
