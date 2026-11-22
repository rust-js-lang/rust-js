//! The constant a promoted body or a thread-local's key refers to, read of
//! MIR, the base's since #2299 moved them from `mir::queries`.
use rustc_index::IndexVec;
use rustc_middle::mir::{self, BorrowKind, ConstOperand, Local, Operand, PlaceElem, Rvalue, StatementKind};

/// The constant a promoted body refers to, `_1 = const LOG; _0 = &_1`, if
/// that's all it is.
pub fn referred_constant<'tcx>(body: &mir::Body<'tcx>) -> Option<ConstOperand<'tcx>> {
    if body.basic_blocks.len() != 1 {
        return None;
    }
    let data = &body.basic_blocks[mir::START_BLOCK];
    let mut constant = None;
    let mut referred = None;
    for statement in &data.statements {
        match &statement.kind {
            StatementKind::Assign(assign) => match &**assign {
                (place, Rvalue::Use(Operand::Constant(c), ..)) if place.projection.is_empty() => {
                    constant = Some((place.local, **c));
                }
                (place, Rvalue::Ref(_, BorrowKind::Shared, target))
                    if place.local == mir::RETURN_PLACE && target.projection.is_empty() =>
                {
                    referred = Some(target.local);
                }
                _ => return None,
            },
            StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
            _ => return None,
        }
    }
    let (local, c) = constant?;
    (Some(local) == referred).then_some(c)
}

/// The constant `operand` refers to, `&KEY` of a thread-local's
/// `KEY.get()`: a promoted reference to it, held by a local `assigned`
/// once, or one reborrowed, `&*r`, of a body whose promoted constants are
/// `promoted`. `assigned` says what a local is assigned, where it's
/// assigned once.
pub fn referred_key<'a, 'tcx: 'a>(
    promoted: &IndexVec<mir::Promoted, mir::Body<'tcx>>,
    assigned: impl Fn(Local) -> Option<&'a Rvalue<'tcx>>,
    operand: &Operand<'tcx>,
) -> Option<rustc_span::def_id::DefId> {
    let mut operand = operand.clone();
    let constant = loop {
        let Some(place) = operand.place() else {
            break operand.constant().copied();
        };
        match assigned(place.local) {
            Some(Rvalue::Use(Operand::Constant(c), _)) if place.projection.is_empty() => break Some(**c),
            Some(Rvalue::Ref(_, _, lent))
                if place.projection.is_empty() && lent.projection.as_slice() == [PlaceElem::Deref] =>
            {
                operand = Operand::Copy(lent.local.into());
            }
            _ => break None,
        }
    };
    let referred = match constant.map(|c| c.const_) {
        Some(mir::Const::Unevaluated(uv, _)) => match uv.promoted {
            Some(at) => referred_constant(promoted.get(at)?).map(|c| c.const_),
            None => None,
        },
        _ => None,
    };
    match referred {
        Some(mir::Const::Unevaluated(uv, _)) if uv.promoted.is_none() => Some(uv.def),
        _ => None,
    }
}
