//! What tidy is told of a body's clones: the locals a `clone()` makes, and
//! what's fixed in a closure, which a clone of may be read as (ADRs 0846,
//! 0879, amended).

use super::*;

impl<'tcx> FnCx<'_, 'tcx> {
    /// The names of the locals a `clone()` makes, `let resolved =
    /// payload.clone()`: where that's the value itself in JS, an uncounted
    /// `Rc`'s, tidy reads it by the name it's of (ADR 0846). And each a
    /// tuple of clones gives a pattern of it, `let (chunk, resolved) =
    /// (Rc::clone(self), Rc::clone(&wrapper));`'s `chunk` (rd515).
    pub(in crate::lower::mir) fn clone_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> std::collections::HashSet<String> {
        let cloned: std::collections::HashSet<Local> = (mir_body.basic_blocks.iter())
            .filter_map(|block| match &block.terminator().kind {
                mir::TerminatorKind::Call { func, destination, .. }
                    if destination.projection.is_empty()
                        && func
                            .const_fn_def()
                            .is_some_and(|(def_id, _)| self.tcx.is_lang_item(def_id, LangItem::CloneFn)) =>
                {
                    Some(destination.local)
                }
                _ => None,
            })
            .collect();
        // Each tuple's part that's a clone, by the tuple and its index.
        let statements = || (mir_body.basic_blocks.iter()).flat_map(|block| &block.statements);
        let mut parts: std::collections::HashSet<(Local, usize)> = std::collections::HashSet::new();
        for statement in statements() {
            if let mir::StatementKind::Assign(assign) = &statement.kind
                && let (place, mir::Rvalue::Aggregate(kind, operands)) = &**assign
                && matches!(**kind, mir::AggregateKind::Tuple)
                && place.projection.is_empty()
            {
                for (i, operand) in operands.iter().enumerate() {
                    if let Operand::Move(part) | Operand::Copy(part) = operand
                        && part.projection.is_empty()
                        && cloned.contains(&part.local)
                    {
                        parts.insert((place.local, i));
                    }
                }
            }
        }
        let taken = statements().filter_map(|statement| match &statement.kind {
            mir::StatementKind::Assign(assign) => match &**assign {
                (to, mir::Rvalue::Use(Operand::Move(from) | Operand::Copy(from), _))
                    if to.projection.is_empty()
                        && let [PlaceElem::Field(field, _)] = from.projection.as_slice()
                        && parts.contains(&(from.local, field.as_usize())) =>
                {
                    Some(to.local)
                }
                _ => None,
            },
            _ => None,
        });
        let locals: Vec<Local> = cloned.iter().copied().chain(taken).collect();
        (locals.into_iter())
            .map(|local| state.locals.names[local].clone())
            .filter(|name| !name.is_empty())
            .collect()
    }
}

/// A closure's captures it took by value, each a variable its maker never
/// assigns again from where it makes the closure (`mir_closure_fn`'s
/// copy of one it does): as fixed in it as a parameter, which a clone of
/// it may be read as (ADR 0879). And what it borrowed of a binding that
/// holds one value while it's in scope, one not `mut`, `self` of a method
/// too, as fixed (`analysis::borrowed_fixed_bindings`, rd515).
pub(in crate::lower::mir) fn kept_captures(
    tcx: ty::TyCtxt<'_>,
    closure: rustc_span::def_id::DefId,
    state: &State<'_, '_>,
) -> std::collections::HashSet<String> {
    let fixed = crate::lower::analysis::borrowed_fixed_bindings(tcx, closure);
    let fixed = (fixed.len() == state.facts.captures().len()).then_some(fixed);
    (state.facts.captures().iter().enumerate())
        .filter(|(i, (_, place))| !place || fixed.as_ref().is_some_and(|fixed| fixed[*i]))
        .filter_map(|(_, (capture, _))| match &capture.kind {
            js::ExprKind::Var(name) => Some(name.clone()),
            _ => None,
        })
        .collect()
}
