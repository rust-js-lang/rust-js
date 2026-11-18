//! What a `RefCell` guard held across code says of its cell: no code but
//! through the guard writes it, as another borrow panics, in JS too where
//! `cell_borrows` counts the guard or has proved nothing borrows it
//! (#2112 B, ADR 1174 amended).

use super::*;

impl<'tcx> Flow<'_, 'tcx> {
    /// Of each location that runs code, what each `RefCell` guard surely held
    /// there points to, one pointer down: the cell, or what it's of. Held
    /// from its `borrow()` or `borrow_mut()` till it's dropped, moved or
    /// dies, on every way to it. Not of code that may be what can't be named
    /// where `unnamed` says that may borrow its cell.
    pub(super) fn guarded(&self, unnamed: &dyn Fn(Ty<'tcx>) -> bool) -> IndexMap<Location, IndexSet<Target>> {
        let blocks = &self.body.basic_blocks;
        // Each guard, by its local, and the type of the cell it borrows.
        let mut guards: IndexMap<Local, Ty<'tcx>> = IndexMap::default();
        for data in blocks.iter() {
            if let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
                && let Some((callee, _)) = func.const_fn_def()
                && refcell_borrow(self.tcx, callee).is_some()
                && let Some(guard) = destination.as_local()
                && let Some(cell) = args
                    .first()
                    .and_then(|a| a.node.ty(self.body, self.tcx).builtin_deref(true))
            {
                guards.insert(guard, cell);
            }
        }
        if guards.is_empty() {
            return IndexMap::default();
        }
        // Held before each block, on every way to it: `None` of none reached.
        let mut before: IndexVec<BasicBlock, Option<IndexSet<Local>>> = IndexVec::from_elem(None, blocks);
        before[mir::START_BLOCK] = Some(IndexSet::default());
        let mut changed = true;
        while changed {
            changed = false;
            for &block in blocks.reverse_postorder() {
                let Some(mut held) = before[block].clone() else {
                    continue;
                };
                for statement in &blocks[block].statements {
                    end_guards(Some(&statement.kind), None, &mut held);
                }
                let terminator = blocks[block].terminator();
                end_guards(None, Some(&terminator.kind), &mut held);
                let made = match &terminator.kind {
                    TerminatorKind::Call {
                        destination,
                        target: Some(target),
                        ..
                    } => destination
                        .as_local()
                        .filter(|g| guards.contains_key(g))
                        .map(|g| (*target, g)),
                    _ => None,
                };
                for succ in terminator.successors() {
                    let mut out = held.clone();
                    if let Some((target, guard)) = made
                        && target == succ
                    {
                        out.insert(guard);
                    }
                    let next = match &before[succ] {
                        None => out,
                        Some(old) => old.iter().copied().filter(|g| out.contains(g)).collect(),
                    };
                    if before[succ].as_ref() != Some(&next) {
                        before[succ] = Some(next);
                        changed = true;
                    }
                }
            }
        }
        let mut guarded = IndexMap::default();
        for (block, data) in blocks.iter_enumerated() {
            let Some(mut held) = before[block].clone() else {
                continue;
            };
            for statement in &data.statements {
                end_guards(Some(&statement.kind), None, &mut held);
            }
            let unseen = match &data.terminator().kind {
                TerminatorKind::Call { func, args, .. } => match func.const_fn_def() {
                    None => true,
                    Some((callee, generic)) => {
                        !callee.is_local()
                            && (!is_std_crate(self.tcx, callee)
                                || generic
                                    .types()
                                    .chain(args.iter().map(|a| a.node.ty(self.body, self.tcx)))
                                    .any(|t| may_be_unnamed(self.tcx, t)))
                    }
                },
                TerminatorKind::Drop { place, .. } => may_be_unnamed(self.tcx, place.ty(self.body, self.tcx).ty),
                _ => continue,
            };
            let mut covered: IndexSet<Target> = IndexSet::default();
            for guard in held {
                let Some(&cell) = guards.get(&guard) else {
                    continue;
                };
                // A guard the program holds in a variable is counted where
                // code run while it's held may borrow its cell, what can't be
                // named too (`cell_borrows`' `call_asks`), so a borrow it
                // makes panics as Rust's does, or none is made. Not one made
                // in a call's own arguments, which counts only what it sees.
                let named = (self.body.var_debug_info.iter()).any(
                    |info| matches!(info.value, mir::VarDebugInfoContents::Place(p) if p.as_local() == Some(guard)),
                );
                if !named && unseen && unnamed(cell) {
                    continue;
                }
                for map in [&self.writes, &self.reaches] {
                    covered.extend(
                        map.get(&Target::Whole(guard))
                            .into_iter()
                            .flat_map(|t| t.places.keys().copied()),
                    );
                }
            }
            if !covered.is_empty() {
                guarded.insert(self.body.terminator_loc(block), covered);
            }
        }
        guarded
    }
}

/// Whether `ty` holds code that can't be named: a `dyn`, a fn pointer, a
/// type parameter, an opaque type; of a closure, what it captures.
pub(super) fn may_be_unnamed<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    if let Some((to, _)) = raw_pointee(tcx, ty) {
        return may_be_unnamed(tcx, to);
    }
    match ty.kind() {
        ty::Dynamic(..) | ty::FnPtr(..) => true,
        _ if known_by_bounds(ty) => true,
        ty::Closure(_, args) => args.as_closure().upvar_tys().iter().any(|t| may_be_unnamed(tcx, t)),
        ty::Ref(_, to, _) | ty::Slice(to) | ty::Array(to, _) => may_be_unnamed(tcx, *to),
        ty::Tuple(items) => items.iter().any(|t| may_be_unnamed(tcx, t)),
        ty::Adt(_, args) => args.types().any(|t| may_be_unnamed(tcx, t)),
        _ => false,
    }
}

/// The guards a statement or a terminator ends, of `held`, the one place a
/// guard's end is said: its storage's end, a move of it, its drop, a call
/// it's moved into.
fn end_guards(
    statement: Option<&StatementKind<'_>>,
    terminator: Option<&TerminatorKind<'_>>,
    held: &mut IndexSet<Local>,
) {
    let mut ended: Vec<Local> = Vec::new();
    match statement {
        Some(StatementKind::StorageDead(local)) => ended.push(*local),
        Some(StatementKind::Assign(assign)) => {
            if let Rvalue::Use(Operand::Move(place), _) = &assign.1
                && place.projection.is_empty()
            {
                ended.push(place.local);
            }
        }
        _ => {}
    }
    match terminator {
        Some(TerminatorKind::Drop { place, .. }) => ended.push(place.local),
        Some(TerminatorKind::Call { args, .. }) => {
            ended.extend(args.iter().filter_map(|arg| match &arg.node {
                Operand::Move(place) => Some(place.local),
                _ => None,
            }));
        }
        _ => {}
    }
    for local in ended {
        held.shift_remove(&local);
    }
}
