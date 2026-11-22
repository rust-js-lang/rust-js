//! What a place is the value of: a local, or what its box holds, which is
//! the local in JS, a box being the value it holds (ADR 0023). So a write
//! of `*b` is a write of `b`, as `b = v` is: each check of whether a local
//! is written asks here, where three asked of the local alone and read a
//! box written through as unchanged (#980, #997, #1000); and `indirect`,
//! which a box's deref inside a field isn't (#1033). `FnCx::whole`
//! extends `value_local` with what else is its value in JS, a transparent
//! wrapper's field; a body's THIR asks `changes`, which reaches a variable
//! through any deref.

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::{self, Local, Place, PlaceElem, Rvalue, StatementKind};
use rustc_middle::ty::{Ty, TyCtxt};

/// What a step `elem` from a value of type `ty` reaches that's still that
/// value: what a box holds, through its deref. The one answer of whether a
/// step through a box is the value itself, which each question here and
/// `FnCx::through_values` take it from.
pub fn through_box<'tcx>(elem: PlaceElem<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match elem {
        PlaceElem::Deref => ty.boxed_ty(),
        _ => None,
    }
}

/// The local `place` is the value of: itself, `_1`, or what its box holds,
/// `(*_1)` of a `Box<T>`, through boxes of boxes too. `None` of a part of
/// it, a field's, an item's, or what a reference points to.
pub fn value_local<'tcx>(decls: &mir::LocalDecls<'tcx>, place: &Place<'tcx>) -> Option<Local> {
    let mut ty = decls[place.local].ty;
    for elem in place.projection {
        ty = through_box(elem, ty)?;
    }
    Some(place.local)
}

/// Whether `place` is reached through a reference or a pointer, so what's
/// there is what that points at, not its local's own: a deref of anything
/// but a box. MIR's `is_indirect` counts a box's deref too, where its
/// content is its local's, `*w.0` of a `W(Box<i32>)` and `(*b).0` of a
/// `Box<V>`, whose writes left the local alike, a read of it moved past
/// them (#1033).
pub fn indirect<'tcx>(tcx: TyCtxt<'tcx>, decls: &mir::LocalDecls<'tcx>, place: &Place<'tcx>) -> bool {
    (place.iter_projections())
        .any(|(base, elem)| elem == PlaceElem::Deref && through_box(elem, base.ty(decls, tcx).ty).is_none())
}

/// Each local a statement assigns, and what it's assigned where that's
/// once: `None` of one assigned again, or written through its box, `*b =
/// v`, which assigns it as `b = v` does. Not a call's result, a
/// terminator's: this asks what value a statement builds a local of;
/// whether one write alone, a call's too, makes a local is `made_once`'s.
pub fn assigned_once<'b, 'tcx>(body: &'b mir::Body<'tcx>) -> IndexMap<Local, Option<&'b Rvalue<'tcx>>> {
    let mut assigned: IndexMap<Local, Option<&'b Rvalue<'tcx>>> = IndexMap::default();
    for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
        let StatementKind::Assign(assign) = &statement.kind else {
            continue;
        };
        let Some(local) = value_local(&body.local_decls, &assign.0) else {
            continue;
        };
        let once = assign.0.projection.is_empty() && !assigned.contains_key(&local);
        assigned.insert(local, once.then_some(&assign.1));
    }
    assigned
}

/// Each local one write alone makes: `assigned_once`'s, where no
/// terminator's result is it too, or one terminator's, a call's, where no
/// statement assigns it; through its box as `assigned_once` counts. Where
/// `assigned_once` asks what a statement builds a local of, this asks
/// whether it holds one value all its life, a `RefCell`'s guard a
/// `borrow_mut()` gives, never another's past it (#1815).
pub fn made_once(body: &mir::Body<'_>) -> IndexSet<Local> {
    use rustc_middle::mir::visit::{MutatingUseContext, PlaceContext, Visitor};
    struct Results<'a, 'tcx> {
        decls: &'a mir::LocalDecls<'tcx>,
        counts: IndexMap<Local, usize>,
    }
    impl<'tcx> Visitor<'tcx> for Results<'_, 'tcx> {
        fn visit_place(&mut self, place: &Place<'tcx>, context: PlaceContext, _: mir::Location) {
            let result = matches!(
                context,
                PlaceContext::MutatingUse(
                    MutatingUseContext::Call | MutatingUseContext::Yield | MutatingUseContext::AsmOutput
                )
            );
            if result && let Some(local) = value_local(self.decls, place) {
                *self.counts.entry(local).or_default() += 1;
            }
        }
    }
    let mut results = Results {
        decls: &body.local_decls,
        counts: IndexMap::default(),
    };
    for (block, data) in body.basic_blocks.iter_enumerated() {
        results.visit_terminator(data.terminator(), body.terminator_loc(block));
    }
    let assigned = assigned_once(body);
    (body.local_decls.indices())
        .filter(|local| {
            matches!(
                (assigned.get(local), results.counts.get(local)),
                (Some(Some(_)), None) | (None, Some(1))
            )
        })
        .collect()
}

/// What writes a local: a statement's rvalue or a call.
pub enum Write<'b, 'tcx> {
    Statement(&'b Rvalue<'tcx>),
    Call(&'b mir::Operand<'tcx>, &'b [rustc_span::Spanned<mir::Operand<'tcx>>]),
}

/// Which borrows of a local `the_write` takes for a write it can't see.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Borrowed {
    /// Any, shared too: what `pointers/returned.rs` follows takes none.
    Any,
    /// Only one that may write it, `&mut l` or `&raw mut l`.
    Mutably,
}

/// The one write of `local` in `body`, whole, a statement's or a call's,
/// what it was made of (#2211, moved from pointers/returned.rs): `None` where it's written
/// more than once, in part, or by anything but an assignment or a call,
/// or where it's borrowed itself as `borrowed` says.
pub fn the_write<'b, 'tcx>(body: &'b mir::Body<'tcx>, local: Local, borrowed: Borrowed) -> Option<Write<'b, 'tcx>> {
    let mut found = None;
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    let may_write = |kind: &mir::BorrowKind| borrowed == Borrowed::Any || kind.mutability().is_mut();
                    let lent = match &assign.1 {
                        Rvalue::Ref(_, kind, place) if may_write(kind) => Some(place),
                        Rvalue::RawPtr(kind, place)
                            if borrowed == Borrowed::Any || !matches!(kind, mir::RawPtrKind::Const) =>
                        {
                            Some(place)
                        }
                        _ => None,
                    };
                    if let Some(place) = lent
                        && place.local == local
                        && place.projection.first() != Some(&PlaceElem::Deref)
                    {
                        return None;
                    }
                    if assign.0.local == local {
                        if !assign.0.projection.is_empty() || found.is_some() {
                            return None;
                        }
                        found = Some(Write::Statement(&assign.1));
                    }
                }
                StatementKind::SetDiscriminant { place, .. } if place.local == local => return None,
                _ => {}
            }
        }
        match &data.terminator().kind {
            mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } if destination.local == local => {
                if !destination.projection.is_empty() || found.is_some() {
                    return None;
                }
                found = Some(Write::Call(func, args));
            }
            mir::TerminatorKind::InlineAsm { .. } => return None,
            _ => {}
        }
    }
    found
}
