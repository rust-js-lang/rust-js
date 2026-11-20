//! What of its parameters a crate's function returns, its result's
//! provenance, one owner (#1934): a `&mut` that is its parameter, the place
//! its caller gave, reborrowed through `&raw mut *s`, pointer casts that
//! keep the place and `&mut *p`, httparse's `deinit_slice_mut` (#1513),
//! whose caller's pointer to what it returns is its argument's; and an
//! iterator that is its parameter's, as it is or through std's adapters a
//! collect in place reaches its source through, `it.map(f)`, or a `Vec`
//! parameter's `into_iter()`, whose caller's collect in place has that
//! argument's source `Vec` (`returned_iterator`).

use super::*;
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
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
        let Returned {
            param,
            via: Via::Reborrow,
        } = self.returned_from(fn_id)?
        else {
            return None;
        };
        let given = cell(body.local_decls[mir::Local::from_usize(param + 1)].ty)?;
        self.same_place(given, result).then_some(param)
    }

    /// What of its parameters the crate's function `fn_id` returns: a
    /// reborrow of one (`reborrowed_from`), else its iterator
    /// (`returned_iterator`). `None` of anything else, of another crate's
    /// function, whose body this crate doesn't see, and of a recursive one.
    fn returned_from(&self, fn_id: DefId) -> Option<Returned> {
        let body = local_fn_body(self.tcx, fn_id)?;
        match self.reborrowed_from(body, mir::RETURN_PLACE, false) {
            Some(param) => Some(Returned {
                param: param.as_usize() - 1,
                via: Via::Reborrow,
            }),
            None => returned_iterator(self.tcx, fn_id),
        }
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
            local = match crate::lower::body_queries::the_write(body, local, crate::lower::body_queries::Borrowed::Any)?
            {
                crate::lower::body_queries::Write::Statement(rvalue) => self.reborrow_of(body, rvalue)?,
                crate::lower::body_queries::Write::Call(func, args) if through_calls => {
                    let (callee, _) = func.const_fn_def()?;
                    let index = self.returns_its_parameter(callee)?;
                    match &args.get(index)?.node {
                        mir::Operand::Copy(place) | mir::Operand::Move(place) => place.as_local()?,
                        mir::Operand::Constant(_) | mir::Operand::RuntimeChecks(_) => return None,
                    }
                }
                crate::lower::body_queries::Write::Call(..) => return None,
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

/// How a function's result is made of its parameter `param`, its index
/// (`returned_from`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lower) struct Returned {
    pub(in crate::lower) param: usize,
    pub(in crate::lower) via: Via,
}

/// The steps from a parameter to what a function returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lower) enum Via {
    /// A reborrow of a `&mut` parameter, the place its caller gave.
    Reborrow,
    /// The parameter's iterator, as it is or through std's adapters a
    /// collect in place reaches its source through (`adapts_in_place`).
    Adapted,
    /// A `Vec` parameter's `into_iter()`, adapted so or not: the `Vec` is
    /// its collect's source.
    IntoIter,
}

/// The MIR of the crate's own function `fn_id`, where it has one.
fn local_fn_body<'tcx>(tcx: TyCtxt<'tcx>, fn_id: DefId) -> Option<&'tcx mir::Body<'tcx>> {
    (fn_id.is_local() && matches!(tcx.def_kind(fn_id), DefKind::Fn | DefKind::AssocFn) && tcx.is_mir_available(fn_id))
        .then(|| tcx.optimized_mir(fn_id))
}

/// The parameter whose iterator the crate's function `fn_id` returns, as it
/// is or adapted (`Via::Adapted`), or whose `Vec`'s `into_iter()` it
/// returns (`Via::IntoIter`), each local on the way written once: by a
/// move, an adapter's call of it (`adapts_in_place`), `into_iter()` of a
/// `Vec`, or a call of another of the crate's functions that returns its
/// argument so. `None` of anything else, of another crate's function, and
/// of one that reaches itself again, a recursive one's (#1934).
pub(in crate::lower) fn returned_iterator(tcx: TyCtxt<'_>, fn_id: DefId) -> Option<Returned> {
    returned_iterator_within(tcx, fn_id, &mut Vec::new())
}

fn returned_iterator_within(tcx: TyCtxt<'_>, fn_id: DefId, walking: &mut Vec<DefId>) -> Option<Returned> {
    use crate::lower::recognition::{StdItem, adapts_in_place, is_std_def, std_item, trait_method};
    // A function the walk is in already: recursive, never followed.
    if walking.contains(&fn_id) {
        return None;
    }
    let body = local_fn_body(tcx, fn_id)?;
    walking.push(fn_id);
    let typing_env = ty::TypingEnv::post_analysis(tcx, fn_id);
    let into_iter = trait_method(tcx, std_item(tcx, StdItem::IntoIterator), "into_iter");
    let operand_local = |operand: &mir::Operand<'_>| match operand {
        mir::Operand::Copy(place) | mir::Operand::Move(place) => place.as_local(),
        _ => None,
    };
    let mut local = mir::RETURN_PLACE;
    let mut via = Via::Adapted;
    let mut found = None;
    for _ in 0..STEPS {
        if (1..=body.arg_count).contains(&local.as_usize()) {
            found = Some(Returned {
                param: local.as_usize() - 1,
                via,
            });
            break;
        }
        let next = match crate::lower::body_queries::the_write(body, local, crate::lower::body_queries::Borrowed::Any) {
            Some(crate::lower::body_queries::Write::Statement(Rvalue::Use(operand, _))) => operand_local(operand),
            // What follows an `into_iter()` is the `Vec` itself, moved.
            Some(crate::lower::body_queries::Write::Call(..)) if via == Via::IntoIter => None,
            Some(crate::lower::body_queries::Write::Call(func, args)) => (|| {
                let (callee, _) = func.const_fn_def()?;
                let first = operand_local(&args.first()?.node)?;
                let made = body.local_decls[local].ty;
                let of = body.local_decls[first].ty;
                if callee == into_iter
                    && matches!(of.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Vec))
                {
                    via = Via::IntoIter;
                    return Some(first);
                }
                if adapts_in_place(tcx, typing_env, made, of) {
                    return Some(first);
                }
                let returned = returned_iterator_within(tcx, callee, walking)?;
                via = returned.via;
                operand_local(&args.get(returned.param)?.node)
            })(),
            _ => None,
        };
        let Some(next) = next else { break };
        local = next;
    }
    walking.pop();
    found
}
