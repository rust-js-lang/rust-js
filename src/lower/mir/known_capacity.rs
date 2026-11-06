//! Which `extend`s of a `Vec`, a `VecDeque` or a `BinaryHeap` extend one
//! whose capacity is known where it's extended (#849, the user's (B)): std's
//! `extend` asks its iterator's `size_hint` as the collection fills, which
//! the crate's own `size_hint` makes seen, so when it's asked is its
//! capacity's. Known where the collection is a variable made in the same
//! body by `new`, `with_capacity`, `vec![..]`, or a collect std makes of
//! exactly its length, a `TrustedLen` one's not in place, and lent `&mut`
//! since only to an `extend`, a `reserve` or a `reserve_exact`, each of
//! which keeps the capacity native Rust's has (`$grownBy`, `$reserve`).

use super::*;
use crate::lower::recognition::{CapacityOp, InPlaceSource, capacity_op, in_place_source, is_trusted_len};
use rustc_span::def_id::DefId;

impl<'a, 'tcx> crate::lower::recognition::Recognition<'a, 'tcx> {
    /// Of `body`'s calls, each `extend` of a collection whose capacity is
    /// known where it's called, in this typing environment, the body's.
    pub(in crate::lower) fn known_capacity_extends(&self, body: &mir::Body<'tcx>) -> IndexSet<BasicBlock> {
        let mut known = IndexSet::default();
        // Each local assigned once by a statement, and what it's assigned.
        let mut assigned: IndexMap<Local, Option<&Rvalue<'tcx>>> = IndexMap::default();
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && assign.0.projection.is_empty()
                {
                    let once = !assigned.contains_key(&assign.0.local);
                    assigned.insert(assign.0.local, once.then_some(&assign.1));
                }
            }
        }
        // The variable a `&mut` lends, through its reborrows.
        let lent = |mut local: Local| -> Option<Local> {
            for _ in 0..body.local_decls.len() {
                match assigned.get(&local) {
                    Some(Some(Rvalue::Ref(_, BorrowKind::Mut { .. }, place))) => match place.projection.as_slice() {
                        [] => return Some(place.local),
                        [PlaceElem::Deref] => local = place.local,
                        _ => return None,
                    },
                    Some(Some(Rvalue::Use(Operand::Copy(place) | Operand::Move(place), _)))
                        if place.projection.is_empty() =>
                    {
                        local = place.local
                    }
                    _ => return None,
                }
            }
            None
        };
        for (block, data) in body.basic_blocks.iter_enumerated() {
            let mir::TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                continue;
            };
            let Some((def_id, generic_args)) = func.const_fn_def() else {
                continue;
            };
            if !self.extend_grows(def_id, generic_args) {
                continue;
            }
            let receiver = args
                .first()
                .and_then(|arg| arg.node.place())
                .filter(|p| p.projection.is_empty());
            if let Some(variable) = receiver.and_then(|receiver| lent(receiver.local))
                && self.capacity_known(body, variable)
            {
                known.insert(block);
            }
        }
        known
    }

    /// Whether `def_id` is an `extend` std's collection asks its hint of as
    /// it fills: a `Vec`'s, a `VecDeque`'s or a `BinaryHeap`'s.
    pub(in crate::lower) fn extend_grows(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        self.tcx
            .trait_of_assoc(def_id)
            .is_some_and(|t| crate::lower::recognition::is_extend(self.tcx, t))
            && generic_args
                .types()
                .next()
                .is_some_and(|ty| self.grows_as_it_extends(ty.peel_refs()))
    }

    /// Whether `variable`, a collection, has the capacity native Rust's has
    /// wherever it's extended: made once in `body` by a maker whose capacity
    /// is known, and lent `&mut` only to an `extend` or a `reserve`.
    fn capacity_known(&self, body: &mir::Body<'tcx>, variable: Local) -> bool {
        // Not a parameter, whose value its caller made: MIR writes one again
        // by a statement, which the scan below refuses, but a call's
        // destination it may be written to directly would be its maker.
        if variable.as_usize() <= body.arg_count {
            return false;
        }
        let mut made = 0;
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                if assign.0.local == variable {
                    return false;
                }
                // A `&mut` of it only to be given an `extend` or a `reserve`.
                if let Rvalue::Ref(_, BorrowKind::Mut { .. }, place) | Rvalue::RawPtr(_, place) = &assign.1
                    && place.local == variable
                    && !self.lent_to_capacity_calls(body, assign.0.local)
                {
                    return false;
                }
            }
            if let mir::TerminatorKind::Call { func, destination, .. } = &data.terminator().kind
                && destination.local == variable
            {
                if !destination.projection.is_empty() || !self.makes_known_capacity(func) {
                    return false;
                }
                made += 1;
            }
        }
        made == 1
    }

    /// Whether each use of `reference`, a `&mut` of a collection, and of
    /// each reborrow or copy of it, is the collection given an `extend` or a
    /// `reserve` as what it's called on.
    fn lent_to_capacity_calls(&self, body: &mir::Body<'tcx>, reference: Local) -> bool {
        // The references, `reference` and what reborrows or copies one.
        let mut refs: IndexSet<Local> = IndexSet::from_iter([reference]);
        loop {
            let before = refs.len();
            for data in body.basic_blocks.iter() {
                for statement in &data.statements {
                    let StatementKind::Assign(assign) = &statement.kind else {
                        continue;
                    };
                    let of = match &assign.1 {
                        Rvalue::Ref(_, BorrowKind::Mut { .. }, place)
                            if place.projection.as_slice() == [PlaceElem::Deref] =>
                        {
                            Some(place.local)
                        }
                        Rvalue::Use(Operand::Copy(place) | Operand::Move(place), _) if place.projection.is_empty() => {
                            Some(place.local)
                        }
                        _ => None,
                    };
                    if assign.0.projection.is_empty() && of.is_some_and(|of| refs.contains(&of)) {
                        refs.insert(assign.0.local);
                    }
                }
            }
            if refs.len() == before {
                break;
            }
        }
        let mentions = |operand: &Operand<'tcx>| operand.place().is_some_and(|place| refs.contains(&place.local));
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                // Made one of them, as above, or not reading one.
                if assign.0.projection.is_empty() && refs.contains(&assign.0.local) {
                    continue;
                }
                let reads = match &assign.1 {
                    Rvalue::Use(operand, _) => mentions(operand),
                    Rvalue::Ref(_, _, place)
                    | Rvalue::RawPtr(_, place)
                    | Rvalue::Discriminant(place)
                    | Rvalue::CopyForDeref(place) => refs.contains(&place.local),
                    Rvalue::Aggregate(_, operands) => operands.iter().any(mentions),
                    _ => false,
                };
                if reads || refs.contains(&assign.0.local) {
                    return false;
                }
            }
            if let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
            {
                if refs.contains(&destination.local) {
                    return false;
                }
                if !args.iter().any(|arg| mentions(&arg.node)) {
                    continue;
                }
                let Some((def_id, generic_args)) = func.const_fn_def() else {
                    return false;
                };
                let reserves = matches!(capacity_op(self.tcx, def_id), Some(CapacityOp::Reserve { .. }));
                let first_alone = args.first().is_some_and(|arg| mentions(&arg.node))
                    && !args.iter().skip(1).any(|arg| mentions(&arg.node));
                if !(first_alone && (reserves || self.extend_grows(def_id, generic_args))) {
                    return false;
                }
            }
        }
        true
    }

    /// Whether `func` makes a collection whose capacity is known where it's
    /// made: `new`, `with_capacity`, `vec![..]` and `vec![x; n]`, each
    /// exactly what it holds or is asked; or a collect std makes of exactly
    /// its length, a `TrustedLen` iterator's, not in place.
    fn makes_known_capacity(&self, func: &Operand<'tcx>) -> bool {
        let Some((def_id, generic_args)) = func.const_fn_def() else {
            return false;
        };
        if capacity_op(self.tcx, def_id) == Some(CapacityOp::WithCapacity) {
            return true;
        }
        if matches!(
            self.classify(def_id, generic_args),
            Some(Std::VecNew | Std::VecMacro | Std::FromElem)
        ) {
            return true;
        }
        crate::lower::recognition::collected_items(self.tcx, self.typing_env, def_id, generic_args).is_some_and(
            |(iter, _, _)| {
                is_trusted_len(self.tcx, self.typing_env, iter)
                    && matches!(
                        in_place_source(self.tcx, self.typing_env, iter),
                        Some(InPlaceSource::Never)
                    )
            },
        )
    }
}
