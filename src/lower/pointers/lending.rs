//! What a body lends as its places' pointers: each reference made once and
//! given to a call that's given the pointer to its place for it, which a
//! raw pointer may leave the call with (ADR 1114).

use super::Pointed;
use crate::collections::IndexMap;
use crate::lower::FnCx;
use rustc_middle::mir::{self, Local, PlaceElem, Rvalue, StatementKind};
use rustc_middle::ty;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `Pointed::lent_as_pointers`: each reference made once of a place,
    /// `&mut n`, and given only to a call that's given the pointer to its
    /// place for it; the place, where it's a whole local of what isn't an
    /// object, a cell, as one a raw pointer is made of is (ADR 1114).
    pub(super) fn lent_as_pointers(&self, body: &mir::Body<'tcx>, pointed: &mut Pointed) {
        let mut made: IndexMap<Local, Option<mir::Place<'tcx>>> = IndexMap::default();
        // What a closure's call is given, its arguments' tuple's operands.
        let mut tuples: IndexMap<Local, Vec<mir::Operand<'tcx>>> = IndexMap::default();
        for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
            if let StatementKind::Assign(assign) = &statement.kind
                && assign.0.projection.is_empty()
            {
                let of = match &assign.1 {
                    Rvalue::Ref(_, _, of) => Some(*of),
                    _ => None,
                };
                made.entry(assign.0.local).and_modify(|once| *once = None).or_insert(of);
                if let Rvalue::Aggregate(kind, operands) = &assign.1
                    && let mir::AggregateKind::Tuple = **kind
                {
                    tuples.insert(assign.0.local, operands.iter().cloned().collect());
                }
            }
        }
        for (block, data) in body.basic_blocks.iter_enumerated() {
            let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let callee = func.ty(body, self.tcx);
            // Each argument a reference may be given as, and whether it's
            // given its place's pointer there: a function's, a function
            // pointer's by its signature, a closure's by its own, through
            // its call's tuple.
            let given: Vec<(mir::Operand<'tcx>, bool)> = match *callee.kind() {
                ty::FnPtr(sig, _) => {
                    let sig = sig.skip_binder();
                    let inputs = sig.inputs().to_vec();
                    (args.iter().enumerate())
                        .map(|(i, arg)| (arg.node.clone(), self.sig_given_its_pointer((&inputs, sig.output()), i)))
                        .collect()
                }
                _ => match crate::lower::fn_def(callee) {
                    // std's `ptr::from_mut(r)` and `ptr::from_ref(r)`, the
                    // polyfill's own: given its place's pointer, a shared
                    // reference's too.
                    Some((fn_id, fn_args))
                        if self.recognition().classify(fn_id, fn_args)
                            == Some(crate::lower::recognition::Std::Pointer(super::PointerOp::FromPlace)) =>
                    {
                        args.iter().map(|arg| (arg.node.clone(), true)).collect()
                    }
                    // A closure's, an `F: FnMut(&mut T) -> *mut T`'s or a
                    // `dyn`'s: its arguments' tuple and what it returns.
                    Some((fn_id, fn_args))
                        if self
                            .tcx
                            .trait_of_assoc(fn_id)
                            .is_some_and(|tr| self.tcx.fn_trait_kind_from_def_id(tr).is_some())
                            && let ty::Tuple(inputs) = *fn_args.type_at(1).kind()
                            && let Some(mir::Operand::Move(tuple)) = args.get(1).map(|arg| &arg.node)
                            && let Some(operands) = tuples.get(&tuple.local) =>
                    {
                        let output = destination.ty(body, self.tcx).ty;
                        (operands.iter().enumerate())
                            .map(|(i, operand)| (operand.clone(), self.sig_given_its_pointer((inputs, output), i)))
                            .collect()
                    }
                    // What the call calls, as its arguments are given,
                    // `call_callee`'s: an impl's method, not a `dyn`'s.
                    Some((fn_id, fn_args)) => {
                        // Of a function rust-js compiles, whose call gives what
                        // `boxed_args` gives; not std's, which its own lowering
                        // reads, `MaybeUninit::as_mut_ptr(&mut m)`'s.
                        let called = (self.call_callee(fn_id, fn_args).ok().flatten()).filter(|&(id, _)| {
                            self.is_rust_fn(id)
                                || self
                                    .tcx
                                    .trait_of_assoc(id)
                                    .is_some_and(|tr| self.krate.foreign.compiled_by_rust_js(tr))
                        });
                        (args.iter().enumerate())
                            .map(|(i, arg)| {
                                (
                                    arg.node.clone(),
                                    called.is_some_and(|(id, _)| self.given_its_pointer(id, i)),
                                )
                            })
                            .collect()
                    }
                    None => continue,
                },
            };
            for (arg, given_pointer) in given {
                let mir::Operand::Move(given) = arg else {
                    continue;
                };
                if !given.projection.is_empty() || !given_pointer {
                    continue;
                }
                let Some(Some(of)) = made.get(&given.local).copied() else {
                    continue;
                };
                pointed.lent_as_pointers.insert(given.local);
                pointed.lent_calls.insert(block);
                // A whole local, or what a reference made once of one refers
                // to, a reborrow's, `&mut *r` of `r = &mut n`.
                let whole = match of.projection.as_slice() {
                    [] => Some(of.local),
                    [PlaceElem::Deref] => pointed.refs.get(&of.local).copied(),
                    _ => None,
                };
                if let Some(local) = whole {
                    let ty = body.local_decls[local].ty;
                    if !self.is_object(ty) || self.mut_ref_form(ty).is_cell() {
                        pointed.locals.insert(local);
                    }
                }
            }
        }
    }
}

/// The place a reference `local` is made of, where `body` makes it once,
/// `r = &mut s.n`: `s.n`. None of one made more than once, or otherwise.
pub(in crate::lower) fn made_once_of<'tcx>(body: &mir::Body<'tcx>, local: Local) -> Option<mir::Place<'tcx>> {
    let mut made =
        (body.basic_blocks.iter())
            .flat_map(|data| &data.statements)
            .filter_map(|statement| match &statement.kind {
                StatementKind::Assign(assign) if assign.0.local == local && assign.0.projection.is_empty() => {
                    Some(match &assign.1 {
                        Rvalue::Ref(_, _, of) => Some(*of),
                        _ => None,
                    })
                }
                _ => None,
            });
    match (made.next(), made.next()) {
        (Some(of), None) => of,
        _ => None,
    }
}
