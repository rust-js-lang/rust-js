//! Pointers into what a call lends that's a copy in JS, text's bytes,
//! `s.as_bytes()`, `$utf8Bytes(s)` (ADR 0126, amended, #1183): a JS string
//! is a value with no place, so its bytes have no address of Rust's, and
//! two `String`s of the same text lend the same bytes. Such a pointer reads
//! what Rust's does, and goes along its array, compared with another of
//! the same bytes, as Rust's does; compared with any other, or kept past
//! what the function sees, it would show the copy's place: refused.

use crate::collections::IndexSet;
use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{self, Local, Operand, PlaceElem, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty;
use rustc_span::Span;

use super::super::recognition::Std;
use super::super::refusal::{Refused, refuse};
use super::super::std_types::text::TextOp;
use super::super::{FnCx, R};
use super::{COPIED, PointerOp};
use crate::js::Expr;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Each reference a call gives that's a copy in JS, its place not
    /// Rust's: text's bytes, `s.as_bytes()`.
    pub(super) fn lent_copies(&self, body: &mir::Body<'tcx>) -> IndexSet<Local> {
        (body.basic_blocks.iter())
            .filter_map(|data| match &data.terminator().kind {
                TerminatorKind::Call { func, destination, .. } if destination.projection.is_empty() => {
                    (self.std_called(body, func) == Some(Std::Text(TextOp::Bytes))).then_some(destination.local)
                }
                _ => None,
            })
            .collect()
    }

    /// A slice's `as_ptr()` or `as_mut_ptr()` of a reference a call lends
    /// that's a copy, `Pointed::lent_copies`', `s.as_bytes().as_ptr()`:
    /// refused unless it's kept within its copy (`kept_within_its_copy`).
    pub(in crate::lower) fn check_copy_pointer(
        &self,
        body: &mir::Body<'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        destination: mir::Place<'tcx>,
        span: Span,
    ) -> R<()> {
        let op = self.recognition().classify(def_id, generic_args);
        if !matches!(op, Some(Std::Pointer(PointerOp::FirstItem | PointerOp::ItemsRange))) {
            return Ok(());
        }
        // Asked only of a slice's pointer, which few calls make.
        let lent = self.pointed_locals(body).lent_copies;
        let of_copy = matches!(args.first().map(|arg| &arg.node),
            Some(Operand::Copy(of) | Operand::Move(of)) if of.projection.is_empty() && lent.contains(&of.local));
        // A range of them, `as_ptr_range()`'s, whose pointers are read from
        // its fields, is never kept within its copy: refused.
        if !of_copy || destination.projection.is_empty() && self.kept_within_its_copy(body, destination.local) {
            return Ok(());
        }
        Err(refuse(self.tcx, span, Refused::Memory(COPIED)))
    }

    /// Whether the pointer `first` makes, into a copy, and each it's
    /// stepped to, `p.add(i)`, `p.sub(i)`, `p.cast()`, or copied to, is
    /// only read through, `*p`, `p.read()`, what `ptr::copy` copies from,
    /// and compared with another of them, `p < end`, `p.offset_from(q)`:
    /// never written through, kept in a place, returned, given to another
    /// call or compared with another pointer, which would see the copy's
    /// place, not Rust's one.
    pub(super) fn kept_within_its_copy(&self, body: &mir::Body<'tcx>, first: Local) -> bool {
        if first == mir::RETURN_PLACE {
            return false;
        }
        // Each local stepped or copied from one of them, to the end.
        let mut within: IndexSet<Local> = IndexSet::default();
        within.insert(first);
        let of = |within: &IndexSet<Local>, operand: &Operand<'tcx>| matches!(operand, Operand::Copy(p) | Operand::Move(p) if p.projection.is_empty() && within.contains(&p.local));
        let mut changed = true;
        while changed {
            changed = false;
            for data in body.basic_blocks.iter() {
                for statement in &data.statements {
                    if let StatementKind::Assign(assign) = &statement.kind
                        && let (place, Rvalue::Use(operand, _) | Rvalue::Cast(_, operand, _)) = &**assign
                        && place.projection.is_empty()
                        && of(&within, operand)
                    {
                        changed |= within.insert(place.local);
                    }
                }
                if let TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } = &data.terminator().kind
                    && destination.projection.is_empty()
                    && self.steps(body, func)
                    && args.first().is_some_and(|arg| of(&within, &arg.node))
                {
                    changed |= within.insert(destination.local);
                }
            }
        }
        if within.contains(&mir::RETURN_PLACE) {
            return false;
        }
        struct Uses<'b, 'a, 'tcx> {
            cx: &'b FnCx<'a, 'tcx>,
            body: &'b mir::Body<'tcx>,
            within: &'b IndexSet<Local>,
            first: Local,
            kept: bool,
        }
        impl<'tcx> Visitor<'tcx> for Uses<'_, '_, 'tcx> {
            fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, at: mir::Location) {
                if !self.within.contains(&place.local) {
                    return;
                }
                let through = place.projection.first() == Some(&PlaceElem::Deref);
                self.kept &= match context {
                    PlaceContext::NonUse(_) => true,
                    PlaceContext::NonMutatingUse(_) if through => true,
                    PlaceContext::NonMutatingUse(NonMutatingUseContext::Inspect) => true,
                    // Assigned one of them, `p = p.add(1)`, or the first,
                    // its `as_ptr()`, a slice's or text's; anything else
                    // is another pointer.
                    PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call)
                        if place.projection.is_empty() =>
                    {
                        self.assigned_within(place.local, at)
                    }
                    PlaceContext::NonMutatingUse(NonMutatingUseContext::Copy | NonMutatingUseContext::Move) => {
                        self.used_within(at)
                    }
                    _ => false,
                };
            }
        }
        impl<'tcx> Uses<'_, '_, 'tcx> {
            fn of(&self, operand: &Operand<'tcx>) -> bool {
                matches!(operand, Operand::Copy(p) | Operand::Move(p) if p.projection.is_empty() && self.within.contains(&p.local))
            }

            fn assigned_within(&self, local: Local, at: mir::Location) -> bool {
                let block = &self.body.basic_blocks[at.block];
                match block
                    .statements
                    .get(at.statement_index)
                    .map(|statement| &statement.kind)
                {
                    Some(StatementKind::Assign(assign)) => {
                        matches!(&assign.1, Rvalue::Use(operand, _) | Rvalue::Cast(_, operand, _) if self.of(operand))
                    }
                    Some(_) => false,
                    None => match &block.terminator().kind {
                        TerminatorKind::Call { func, args, .. } => {
                            let made = matches!(
                                self.cx.std_called(self.body, func),
                                Some(Std::Pointer(PointerOp::FirstItem) | Std::Text(TextOp::BytesPointer))
                            );
                            local == self.first && made
                                || self.cx.steps(self.body, func) && args.first().is_some_and(|arg| self.of(&arg.node))
                        }
                        _ => false,
                    },
                }
            }

            fn used_within(&self, at: mir::Location) -> bool {
                let block = &self.body.basic_blocks[at.block];
                match block
                    .statements
                    .get(at.statement_index)
                    .map(|statement| &statement.kind)
                {
                    Some(StatementKind::Assign(assign)) => match &assign.1 {
                        // Copied or stepped to another of them.
                        Rvalue::Use(..) | Rvalue::Cast(..) => {
                            assign.0.projection.is_empty() && self.within.contains(&assign.0.local)
                        }
                        // Compared with another of them.
                        Rvalue::BinaryOp(op, operands) => {
                            matches!(
                                op,
                                mir::BinOp::Eq
                                    | mir::BinOp::Ne
                                    | mir::BinOp::Lt
                                    | mir::BinOp::Le
                                    | mir::BinOp::Gt
                                    | mir::BinOp::Ge
                            ) && self.of(&operands.0)
                                && self.of(&operands.1)
                        }
                        _ => false,
                    },
                    Some(_) => false,
                    None => match &block.terminator().kind {
                        TerminatorKind::Call {
                            func,
                            args,
                            destination,
                            ..
                        } => {
                            let op = self.cx.std_called(self.body, func);
                            let given = |i: usize| args.get(i).is_some_and(|arg| self.of(&arg.node));
                            match op {
                                Some(Std::Pointer(PointerOp::Read)) => true,
                                // What it copies from, never where to.
                                Some(Std::Pointer(PointerOp::Copy { dst_first })) => !given(usize::from(!dst_first)),
                                Some(Std::Pointer(PointerOp::OffsetFrom | PointerOp::ByteOffsetFrom)) => {
                                    given(0) && given(1)
                                }
                                _ => {
                                    self.cx.steps(self.body, func)
                                        && destination.projection.is_empty()
                                        && self.within.contains(&destination.local)
                                }
                            }
                        }
                        _ => false,
                    },
                }
            }
        }
        let mut uses = Uses {
            cx: self,
            body,
            within: &within,
            first,
            kept: true,
        };
        uses.visit_body(body);
        uses.kept
    }

    /// A part of a slice the crate's own code makes in place, `items` from
    /// `from` to `to` or their end, each in bounds: JS's own
    /// `items.slice(from, to)`; or, of a crate that takes a slice's address,
    /// `$part(items, from, to)`, which records where the part is in what
    /// it's of, as the runtime's parts are, so a pointer into it is one
    /// into that (#1185). Neither is an array made here alone, which a
    /// part with its record isn't: one that's owned is copied.
    pub(in crate::lower) fn slice_part(&self, items: Expr, from: Expr, to: Option<Expr>) -> Expr {
        let range = std::iter::once(from).chain(to).collect();
        if self.krate.stored_places.takes_slice_addresses() {
            return Expr::call(Expr::helper("$part"), std::iter::once(items).chain(range).collect());
        }
        let sliced = Expr::builtin(items, "slice", range);
        Expr {
            facts: crate::js::Facts::of(crate::js::Known::Sliced),
            ..sliced
        }
    }

    /// What std function `func` is, of those rust-js knows.
    fn std_called(&self, body: &mir::Body<'tcx>, func: &Operand<'tcx>) -> Option<Std> {
        let (def_id, args) = crate::lower::fn_def(func.ty(body, self.tcx))?;
        self.recognition().classify(def_id, args)
    }

    /// Whether `func` steps a pointer along its array, or casts it.
    fn steps(&self, body: &mir::Body<'tcx>, func: &Operand<'tcx>) -> bool {
        matches!(
            self.std_called(body, func),
            Some(Std::Pointer(
                PointerOp::Add
                    | PointerOp::Sub
                    | PointerOp::WrappingAdd { .. }
                    | PointerOp::ByteAdd { .. }
                    | PointerOp::Cast
            ))
        )
    }
}
