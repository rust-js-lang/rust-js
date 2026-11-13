//! A `&mut` a function returns that is its parameter: the place its caller
//! gave, reborrowed through `&raw mut *s`, pointer casts that keep the
//! place and `&mut *p`, httparse's `deinit_slice_mut` (#1513). Its caller's
//! pointer to what such a call returns is the pointer of the argument it
//! returns.

use super::*;
use rustc_hir::def::DefKind;
use rustc_span::def_id::DefId;

/// How far a reborrow is followed back, each step one local's one write:
/// no chain of reborrows a function writes is longer.
const STEPS: usize = 32;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The index of the parameter a crate's own function returns, where on
    /// every path its result is a reborrow of that one `&mut` parameter
    /// (`reborrowed_from`), both to what JS holds in a cell for every
    /// instance (a cell, not generic code's box) and at the same place: the cell its
    /// caller gave it. `None` of anything else, another parameter on one
    /// path, a field, a fresh allocation, a call's result, a library's
    /// function, whose body this crate doesn't see.
    pub(in crate::lower) fn returns_its_parameter(&self, fn_id: DefId) -> Option<usize> {
        let tcx = self.tcx;
        if !fn_id.is_local()
            || !matches!(tcx.def_kind(fn_id), DefKind::Fn | DefKind::AssocFn)
            || !tcx.is_mir_available(fn_id)
        {
            return None;
        }
        let body = tcx.optimized_mir(fn_id);
        let cell = |ty: Ty<'tcx>| match *ty.kind() {
            ty::Ref(_, referent, ty::Mutability::Mut) => {
                let form = self.mut_ref_form(referent);
                (form.is_cell() && !form.is_generic_box()).then_some(referent)
            }
            _ => None,
        };
        let result = cell(body.local_decls[mir::RETURN_PLACE].ty)?;
        let param = self.reborrowed_from(body, mir::RETURN_PLACE, false)?;
        let given = cell(body.local_decls[param].ty)?;
        self.same_place(given, result).then(|| param.as_usize() - 1)
    }

    /// The parameter `local` of `body` is a reborrow of, where each local
    /// on the way is written once: by `&mut *l`, `&raw mut *l`, a copy or
    /// a move of `l`, a cast of a pointer `l` to one at the same place, or,
    /// `through_calls`, a call of a function that returns its parameter
    /// (`returns_its_parameter`) given `l` there. `None` where any write is
    /// something else, or where `local` is written more than once.
    pub(in crate::lower) fn reborrowed_from(
        &self,
        body: &mir::Body<'tcx>,
        mut local: Local,
        through_calls: bool,
    ) -> Option<Local> {
        for _ in 0..STEPS {
            if (1..=body.arg_count).contains(&local.as_usize()) {
                return Some(local);
            }
            local = match only_write(body, local)? {
                Write::Statement(rvalue) => self.reborrow_of(body, rvalue)?,
                Write::Call(func, args) if through_calls => {
                    let (callee, _) = func.const_fn_def()?;
                    let index = self.returns_its_parameter(callee)?;
                    match &args.get(index)?.node {
                        mir::Operand::Copy(place) | mir::Operand::Move(place) => place.as_local()?,
                        mir::Operand::Constant(_) | mir::Operand::RuntimeChecks(_) => return None,
                    }
                }
                Write::Call(..) => return None,
            };
        }
        None
    }

    /// The local `rvalue` reborrows, as `reborrowed_from` follows it.
    fn reborrow_of(&self, body: &mir::Body<'tcx>, rvalue: &Rvalue<'tcx>) -> Option<Local> {
        let deref_of = |place: &mir::Place<'tcx>| match place.projection.as_slice() {
            [PlaceElem::Deref] => Some(place.local),
            _ => None,
        };
        match rvalue {
            Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, place) | Rvalue::RawPtr(mir::RawPtrKind::Mut, place) => {
                deref_of(place)
            }
            Rvalue::Use(mir::Operand::Copy(place) | mir::Operand::Move(place), _) => place.as_local(),
            Rvalue::Cast(mir::CastKind::PtrToPtr, mir::Operand::Copy(place) | mir::Operand::Move(place), to) => {
                let from = raw_pointee(self.tcx, place.ty(body, self.tcx).ty)?.0;
                let to = raw_pointee(self.tcx, *to)?.0;
                self.same_place(from, to).then_some(place.as_local()?)
            }
            _ => None,
        }
    }
}

/// What writes a local: a statement's rvalue or a call.
enum Write<'b, 'tcx> {
    Statement(&'b Rvalue<'tcx>),
    Call(&'b mir::Operand<'tcx>, &'b [rustc_span::Spanned<mir::Operand<'tcx>>]),
}

/// The one write of `local` in `body`, whole: `None` where it's written
/// more than once, in part, or by anything but an assignment or a call,
/// or where it's borrowed itself, `&mut l` or `&raw mut l`, which may
/// write it.
fn only_write<'b, 'tcx>(body: &'b mir::Body<'tcx>, local: Local) -> Option<Write<'b, 'tcx>> {
    let mut found = None;
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    if let Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place) = &assign.1
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
