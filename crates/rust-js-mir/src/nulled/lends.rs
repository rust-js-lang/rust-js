//! A `&mut` lend of an `Option` local that never takes its `Some` away:
//! React's Flight client's `deferred.get_or_insert_with(Vec::new)`, and
//! getParentFragmentInstances' `match &mut found { None => .., Some(f) =>
//! f.push(..) }`. A write of the local through it sets a `Some`, or the
//! payload a `Some` holds, never `None`, so the local's own writes still
//! say what it is (#1933 step 4 (B)). Split by concept from `nulled`,
//! whose `whole_writes` asks it.

use super::*;
use rustc_middle::mir::visit::{PlaceContext, Visitor};

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// Whether `lent`, borrowed `&mut` into `reference`, a local written
    /// once, keeps its `Some`: `lent` is a `Some`'s payload, or every use of
    /// `reference` reads the `Option` it lends, its discriminant or a
    /// borrow of its payload, a `match &mut found`'s, or gives it to std's
    /// `get_or_insert`, `get_or_insert_with`, `get_or_insert_default`,
    /// `insert` or `as_mut`, none of which writes a `None`. Nothing else
    /// reads it: no other call, no copy, no write of the whole through it.
    pub(super) fn lend_keeps_some(&self, state: &State<'_, 'tcx>, lent: Place<'tcx>, reference: Place<'tcx>) -> bool {
        if matches!(lent.projection.first(), Some(PlaceElem::Downcast(_, variant)) if variant.as_u32() == 1) {
            return true;
        }
        if !lent.projection.is_empty() || !reference.projection.is_empty() || state.locals.writes[reference.local] != 1
        {
            return false;
        }
        let reference = reference.local;
        let body = state.body;
        // A read through it of the `Option` itself, or of a `Some`'s payload.
        let reads_through = |place: &Place<'tcx>| match place.projection.as_slice() {
            [PlaceElem::Deref] => true,
            [PlaceElem::Deref, PlaceElem::Downcast(_, variant), ..] => variant.as_u32() == 1,
            _ => false,
        };
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let mir::StatementKind::Assign(assign) = &statement.kind else {
                    if mentions(statement, reference)
                        && !matches!(
                            statement.kind,
                            mir::StatementKind::StorageLive(_)
                                | mir::StatementKind::StorageDead(_)
                                | mir::StatementKind::FakeRead(..)
                                | mir::StatementKind::PlaceMention(..)
                        )
                    {
                        return false;
                    }
                    continue;
                };
                let (place, rvalue) = &**assign;
                if place.local == reference {
                    // The lend itself; a write through it of a `Some`'s
                    // payload, never of the whole.
                    let payload = matches!(place.projection.as_slice(),
                        [PlaceElem::Deref, PlaceElem::Downcast(_, variant), ..] if variant.as_u32() == 1);
                    if !place.projection.is_empty() && !payload {
                        return false;
                    }
                }
                let read = match rvalue {
                    // A reborrow, `&mut *lent`, may be given anywhere, so a
                    // `&mut` through it is of a `Some`'s payload alone; a
                    // raw pointer never.
                    Rvalue::Ref(_, BorrowKind::Mut { .. }, of) if of.local == reference => {
                        matches!(of.projection.as_slice(),
                            [PlaceElem::Deref, PlaceElem::Downcast(_, variant), ..] if variant.as_u32() == 1)
                    }
                    Rvalue::Discriminant(of) | Rvalue::Ref(_, _, of) if of.local == reference => reads_through(of),
                    Rvalue::RawPtr(_, of) if of.local == reference => false,
                    Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) if of.local == reference => {
                        !of.projection.is_empty() && reads_through(of)
                    }
                    rvalue => !rvalue_mentions(rvalue, reference),
                };
                if !read {
                    return false;
                }
            }
            if let mir::TerminatorKind::Call { func, args, .. } = &data.terminator().kind {
                let given = args
                    .iter()
                    .position(|arg| arg.node.place().is_some_and(|p| p.local == reference));
                if args
                    .iter()
                    .filter(|arg| arg.node.place().is_some_and(|p| p.local == reference))
                    .count()
                    > 1
                {
                    return false;
                }
                if let Some(at) = given {
                    let Some((def_id, generic_args)) = fn_def(func.ty(body, self.tcx)) else {
                        return false;
                    };
                    let keeps = matches!(
                        self.recognition().classify(def_id, generic_args),
                        Some(
                            rust_js_lower::Std::OptionPlace(
                                rust_js_lower::std_types::option::OptionPlaceOp::GetOrInsert
                                    | rust_js_lower::std_types::option::OptionPlaceOp::GetOrInsertWith
                                    | rust_js_lower::std_types::option::OptionPlaceOp::GetOrInsertDefault
                                    | rust_js_lower::std_types::option::OptionPlaceOp::Insert
                            ) | rust_js_lower::Std::Pointee
                        )
                    );
                    if at != 0 || !keeps || !args[0].node.place().is_some_and(|p| p.projection.is_empty()) {
                        return false;
                    }
                }
            } else if terminator_mentions(data.terminator(), reference) {
                return false;
            }
        }
        true
    }
}

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// How many times `local` is lent `&mut`, each a write the body's
    /// count of writes counts (`Locals::writes`), which `whole_writes`
    /// leaves out where each keeps its `Some` (`lend_keeps_some`).
    pub(super) fn mut_lends(&self, state: &State<'_, 'tcx>, local: Local) -> usize {
        (state.body.basic_blocks.iter())
            .filter(|data| !data.is_cleanup)
            .flat_map(|data| &data.statements)
            .filter(|statement| {
                matches!(&statement.kind, mir::StatementKind::Assign(assign)
                if matches!(&assign.1, Rvalue::Ref(_, BorrowKind::Mut { .. }, lent) if lent.local == local))
            })
            .count()
    }
}

/// Whether `statement` mentions `local` at all.
fn mentions<'tcx>(statement: &mir::Statement<'tcx>, local: Local) -> bool {
    let mut found = Found(local, false);
    found.visit_statement(statement, mir::Location::START);
    found.1
}

/// Whether `rvalue` mentions `local` at all.
fn rvalue_mentions<'tcx>(rvalue: &Rvalue<'tcx>, local: Local) -> bool {
    let mut found = Found(local, false);
    found.visit_rvalue(rvalue, mir::Location::START);
    found.1
}

/// Whether `terminator` mentions `local` at all.
fn terminator_mentions<'tcx>(terminator: &mir::Terminator<'tcx>, local: Local) -> bool {
    let mut found = Found(local, false);
    found.visit_terminator(terminator, mir::Location::START);
    found.1
}

struct Found(Local, bool);

impl<'tcx> Visitor<'tcx> for Found {
    fn visit_local(&mut self, local: Local, _: PlaceContext, _: mir::Location) {
        self.1 |= local == self.0;
    }
}
