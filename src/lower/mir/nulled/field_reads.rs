//! An `Option` a struct's `RefCell` field holds, read where a test of the
//! same field proved it there: `while node.sibling.borrow().is_none() {
//! .. }` then `node.sibling.borrow().clone().unwrap()`, React's
//! `$unwrap(node.sibling)` (#1933, step 1(b)); and one a thread-local's
//! `Cell` holds, `KEY.get()`, read where a test of another `KEY.get()`
//! proved it, React's resolveCurrentlyRenderingComponent's `if
//! CURRENT.get().is_none() { panic!(..) } CURRENT.get().unwrap()` (rd443,
//! #1933's removal). Split by concept from `nulled`, whose
//! `proved_present` answers a local's own tests.
//!
//! A read is proved only where nothing runs between the test and it but
//! that same cell's reads, none of which changes it: its `borrow()`, the
//! std guard's deref and drop, the test itself, an `Rc`'s or an `Arc`'s
//! deref on the way to it, and a clone that runs no code of the crate's
//! (an `Option` of a reference, a number, a string, an `Rc` or an `Arc`,
//! std's). The variable the field is reached from is written nowhere
//! between; the struct is one no getter answers the fields of, the
//! program's own data (`fields_read_once`). Of a thread-local, nothing
//! runs between but its own `get()`s and the test: no call, which may
//! `set` it, and no write of its cell, which only a call makes.

use super::*;

/// A cell field's place, from the variable it's reached through: that
/// variable, and each step after it, a field or a deref, an `Rc`'s or an
/// `Arc`'s `Deref` call a deref too.
#[derive(PartialEq, Eq, Debug)]
struct CellPath<'tcx> {
    root: Local,
    steps: Vec<PlaceElem<'tcx>>,
}

/// What a read of a cell reads: a struct's `RefCell` field, by its path,
/// or a thread-local's `Cell`, by its key.
#[derive(PartialEq, Eq, Debug)]
enum CellRead<'tcx> {
    Field(CellPath<'tcx>),
    Local(rustc_span::def_id::DefId),
}

impl CellRead<'_> {
    /// The variable its path begins at, which nothing between may write.
    fn root(&self) -> Option<Local> {
        match self {
            CellRead::Field(path) => Some(path.root),
            CellRead::Local(_) => None,
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `local`, an `Option` read once from a struct's `RefCell`
    /// field, `node.sibling.borrow().clone()`, is proved there where the
    /// block lowered now runs: every path to it takes the edge of a test of
    /// the same field that `None` never takes, `is_some()`'s true edge or
    /// `is_none()`'s false one, and between that test and the read nothing
    /// runs that may change the field (the module's rule).
    pub(in crate::lower::mir) fn field_read_proved(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        let Some((read, read_at)) = self.cell_field_read(state, local) else {
            return false;
        };
        let blocks = &state.body.basic_blocks;
        let dominators = blocks.dominators();
        blocks.iter_enumerated().any(|(_, data)| {
            let TerminatorKind::SwitchInt { discr, targets } = &data.terminator().kind else {
                return false;
            };
            let Some(flag) = discr.place().filter(|p| p.projection.is_empty()) else {
                return false;
            };
            let Some(writes) = self.whole_writes(state, flag.local) else {
                return false;
            };
            let [Write::Call(test, func, [given])] = writes.as_slice() else {
                return false;
            };
            let Some((callee, _)) = func.const_fn_def() else {
                return false;
            };
            let Some(some) = crate::lower::recognition::option_presence_test(self.tcx, callee) else {
                return false;
            };
            let values: Vec<(u128, BasicBlock)> = targets.iter().collect();
            let [(0, zero)] = values.as_slice() else {
                return false;
            };
            // `is_some()`'s true edge, `is_none()`'s false one, `0`.
            let edge = match some {
                true => targets.otherwise(),
                false => *zero,
            };
            // Entered from the switch alone, before what's read.
            let entered = blocks.predecessors()[edge].len() == 1;
            entered
                && dominators.dominates(edge, state.at)
                && self.tested_cell(state, &given.node).as_ref() == Some(&read)
                && self.quiet_between(state, *test, (&read, read_at))
        })
    }

    /// Of `local`, written once by a clone of what a `RefCell` field's
    /// guard derefs to, `node.sibling.borrow().clone()`, or a copy of it,
    /// that field's path and the block its write ends; or by a
    /// thread-local's `KEY.get()`, its key.
    fn cell_field_read(&self, state: &State<'_, 'tcx>, local: Local) -> Option<(CellRead<'tcx>, BasicBlock)> {
        let writes = self.whole_writes(state, local)?;
        let [write] = writes.as_slice() else { return None };
        match write {
            Write::Call(block, func, [key]) if let Some(key) = self.local_cell_got(state, func, &key.node) => {
                Some((CellRead::Local(key), *block))
            }
            Write::Call(block, func, [cloned]) => {
                let (def_id, _) = fn_def(func.ty(state.body, self.tcx))?;
                let clone = self.tcx.trait_of_assoc(def_id)?;
                if !self.tcx.is_lang_item(clone, LangItem::Clone) {
                    return None;
                }
                Some((CellRead::Field(self.option_ref_cell(state, &cloned.node)?), *block))
            }
            _ => None,
        }
    }

    /// The thread-local a call of `func` given `key` is `KEY.get()` of, its
    /// `Cell`'s value.
    fn local_cell_got(
        &self,
        state: &State<'_, 'tcx>,
        func: &Operand<'tcx>,
        key: &Operand<'tcx>,
    ) -> Option<rustc_span::def_id::DefId> {
        let (def_id, _) = fn_def(func.ty(state.body, self.tcx))?;
        if crate::lower::recognition::local_key_cell_method(self.tcx, def_id)
            != Some(crate::lower::recognition::LocalKeyCellMethod::Get)
        {
            return None;
        }
        let assigned = crate::lower::body_queries::assigned_once(state.body);
        crate::lower::mir::referred_key(
            state.facts.promoted(),
            |local| assigned.get(&local).copied().flatten(),
            key,
        )
    }

    /// The cell `given`, a test's reference to an `Option`, reads: a
    /// `RefCell` field's (`option_ref_cell`), or of `&x`, `x` written once
    /// by a thread-local's `KEY.get()`, its key.
    fn tested_cell(&self, state: &State<'_, 'tcx>, given: &Operand<'tcx>) -> Option<CellRead<'tcx>> {
        if let Some(path) = self.option_ref_cell(state, given) {
            return Some(CellRead::Field(path));
        }
        let reference = given.place().filter(|p| p.projection.is_empty())?.local;
        let Some([Write::Assign(Rvalue::Ref(_, BorrowKind::Shared, of))]) =
            self.whole_writes(state, reference).as_deref()
        else {
            return None;
        };
        if !of.projection.is_empty() {
            return None;
        }
        match self.whole_writes(state, of.local).as_deref() {
            Some([Write::Call(_, func, [key])]) => self.local_cell_got(state, func, &key.node).map(CellRead::Local),
            _ => None,
        }
    }

    /// Whether a clone of `local`'s `Option` runs no code of the crate's:
    /// an item a clone gives as it is, as `written_given`'s are.
    fn clone_runs_nothing(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        self.option_of(state.body.local_decls[local].ty).is_some_and(|item| {
            item.is_ref()
                || item.is_primitive()
                || self.is_string_like(item)
                || self.is_std_type(item, StdItem::Rc)
                || self.is_std_type(item, StdItem::Arc)
        })
    }

    /// The `RefCell` field `given`, a reference to an `Option`, is what its
    /// std guard derefs to of: `&*guard`, `guard` what `borrow()` gave of
    /// the field.
    fn option_ref_cell(&self, state: &State<'_, 'tcx>, given: &Operand<'tcx>) -> Option<CellPath<'tcx>> {
        let reference = given.place().filter(|p| p.projection.is_empty())?.local;
        let reference = match self.whole_writes(state, reference).as_deref() {
            // A reborrow, `&(*_13)`, of what `_13` is.
            Some([Write::Assign(Rvalue::Ref(_, BorrowKind::Shared, of))])
                if of.projection[..] == [PlaceElem::Deref] =>
            {
                of.local
            }
            _ => reference,
        };
        let writes = self.whole_writes(state, reference)?;
        let [Write::Call(_, deref, [guard])] = writes.as_slice() else {
            return None;
        };
        let (deref, _) = fn_def(deref.ty(state.body, self.tcx))?;
        let receiver = guard.node.ty(state.body, self.tcx).peel_refs();
        if !self
            .tcx
            .trait_of_assoc(deref)
            .is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Deref))
            || !self.is_std_type(receiver, StdItem::RefCellRef)
        {
            return None;
        }
        let guard = self.borrowed(state, &guard.node).filter(|p| p.projection.is_empty())?;
        let writes = self.whole_writes(state, guard.local)?;
        let [Write::Call(_, borrow, [cell])] = writes.as_slice() else {
            return None;
        };
        let (borrow, borrow_args) = fn_def(borrow.ty(state.body, self.tcx))?;
        if self.recognition().classify(borrow, borrow_args)
            != Some(crate::lower::Std::Borrow {
                mutable: false,
                lock: false,
            })
        {
            return None;
        }
        let cell = self.borrowed(state, &cell.node)?;
        // A struct's field, of the program's own data, no getter's.
        let (base, PlaceElem::Field(..)) = cell.as_ref().last_projection()? else {
            return None;
        };
        let base = base.ty(state.body, self.tcx);
        if base.variant_index.is_some() || self.fields_read_once(base.ty) {
            return None;
        }
        self.cell_path(state, cell, 0)
    }

    /// `place`'s path from the variable it's reached through: of a
    /// temporary an `Rc`'s or an `Arc`'s `Deref` wrote, the path of what it
    /// derefs, then a deref. Each step a field, or a deref of a shared
    /// reference or of such a temporary; nothing indexed, nothing raw.
    fn cell_path(&self, state: &State<'_, 'tcx>, place: Place<'tcx>, depth: usize) -> Option<CellPath<'tcx>> {
        if depth > 4 {
            return None;
        }
        let mut ty = mir::PlaceTy::from_ty(state.body.local_decls[place.local].ty);
        for elem in place.projection {
            match elem {
                PlaceElem::Field(..) => {}
                PlaceElem::Deref if matches!(ty.ty.kind(), ty::Ref(_, _, ty::Mutability::Not)) => {}
                _ => return None,
            }
            ty = ty.projection_ty(self.tcx, elem);
        }
        let local = place.local;
        if local.as_usize() <= state.body.arg_count || state.locals.writes[local] != 1 {
            return Some(CellPath {
                root: local,
                steps: place.projection.to_vec(),
            });
        }
        let written = match self.whole_writes(state, local).as_deref() {
            Some([Write::Call(_, deref, [of])]) => {
                let (deref, _) = fn_def(deref.ty(state.body, self.tcx))?;
                let receiver = of.node.ty(state.body, self.tcx).peel_refs();
                let rc = self.is_std_type(receiver, StdItem::Rc) || self.is_std_type(receiver, StdItem::Arc);
                (rc && self
                    .tcx
                    .trait_of_assoc(deref)
                    .is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Deref)))
                .then(|| {
                    self.borrowed(state, &of.node)
                        .or_else(|| of.node.place().map(|p| p.project_deeper(&[PlaceElem::Deref], self.tcx)))
                })
                .flatten()
            }
            _ => None,
        };
        match written {
            Some(of) => {
                let mut path = self.cell_path(state, of, depth + 1)?;
                path.steps.extend(place.projection.iter());
                Some(path)
            }
            None => Some(CellPath {
                root: local,
                steps: place.projection.to_vec(),
            }),
        }
    }

    /// Whether, on each way from the test's call, `test` the block that
    /// ends with it, to the read, `read_at` the block its clone ends,
    /// and the block lowered now, nothing runs but the field's own reads
    /// and what runs no code (the module's rule), and nothing writes the
    /// variable its path begins at. Each way is from the call's return
    /// on, not through the test again, which reads the field anew.
    fn quiet_between(
        &self,
        state: &State<'_, 'tcx>,
        test: BasicBlock,
        (read, read_at): (&CellRead<'tcx>, BasicBlock),
    ) -> bool {
        let blocks = &state.body.basic_blocks;
        // Each way from the test to the read, not back through the test,
        // which reads the field anew (`ways_from_test`).
        let Some(ways) = self.ways_from_test(state, test) else {
            return false;
        };
        if !ways.contains(&read_at) {
            return false;
        }
        ways.iter().all(|&b| {
            let data = &blocks[b];
            let statements = data.statements.iter().all(|statement| match &statement.kind {
                mir::StatementKind::Assign(assign) => {
                    assign.0.projection.is_empty() && Some(assign.0.local) != read.root()
                }
                mir::StatementKind::SetDiscriminant { .. } => false,
                _ => true,
            });
            statements && (b == state.at || self.quiet_terminator(state, &data.terminator().kind, read))
        })
    }

    /// Whether `terminator` runs nothing that may change the field `read`
    /// is of, nor writes the variable its path begins at.
    fn quiet_terminator(
        &self,
        state: &State<'_, 'tcx>,
        terminator: &TerminatorKind<'tcx>,
        read: &CellRead<'tcx>,
    ) -> bool {
        match terminator {
            TerminatorKind::Goto { .. }
            | TerminatorKind::SwitchInt { .. }
            | TerminatorKind::FalseEdge { .. }
            | TerminatorKind::FalseUnwind { .. }
            | TerminatorKind::Assert { .. } => true,
            // A guard's drop, std's, which gives its borrow back.
            TerminatorKind::Drop { place, .. } => {
                let ty = place.ty(state.body, self.tcx).ty;
                self.is_std_type(ty, StdItem::RefCellRef)
            }
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                if !destination.projection.is_empty() || Some(destination.local) == read.root() {
                    return false;
                }
                // A thread-local's own `get()`, and the test, alone.
                if let CellRead::Local(key) = read {
                    return match args.as_ref() {
                        [given] => {
                            self.local_cell_got(state, func, &given.node) == Some(*key)
                                || fn_def(func.ty(state.body, self.tcx)).is_some_and(|(def_id, _)| {
                                    crate::lower::recognition::option_presence_test(self.tcx, def_id).is_some()
                                })
                        }
                        _ => false,
                    };
                }
                let Some((def_id, generic_args)) = fn_def(func.ty(state.body, self.tcx)) else {
                    return false;
                };
                let known = self.recognition().classify(def_id, generic_args);
                let deref = self
                    .tcx
                    .trait_of_assoc(def_id)
                    .is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Deref));
                let clone = self
                    .tcx
                    .trait_of_assoc(def_id)
                    .is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Clone));
                let receiver = args.first().map(|arg| arg.node.ty(state.body, self.tcx).peel_refs());
                let std_type = |item| receiver.is_some_and(|receiver| self.is_std_type(receiver, item));
                match known {
                    // The same field's `borrow()`.
                    Some(crate::lower::Std::Borrow {
                        mutable: false,
                        lock: false,
                    }) => {
                        (args.first())
                            .and_then(|cell| self.borrowed(state, &cell.node))
                            .and_then(|cell| self.cell_path(state, cell, 0))
                            .map(CellRead::Field)
                            .as_ref()
                            == Some(read)
                    }
                    Some(crate::lower::Std::IsSome | crate::lower::Std::IsNone) => true,
                    _ if deref => std_type(StdItem::RefCellRef) || std_type(StdItem::Rc) || std_type(StdItem::Arc),
                    _ if clone => self.clone_runs_nothing(state, destination.local),
                    _ => false,
                }
            }
            _ => false,
        }
    }
}
