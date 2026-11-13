//! `MaybeUninit`, and a `Box` made before its value (ADR 0332): what it
//! holds, or `undefined` before it's written, as JS has no memory that isn't
//! a value. A `&mut` to one is a box, the place it's written to.

use crate::collections::IndexSet;
use crate::js::{self, Expr, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::temporary_names::Role;
use crate::lower::{FnCx, R};
use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{self, Local, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty;
use rustc_middle::ty::consts::ConstExt as _;

/// What's asked of a `MaybeUninit` or a `Box` of one.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum UninitOp {
    /// `MaybeUninit::uninit()`, `Box::new_uninit()` (`boxed`): nothing yet.
    Uninit { boxed: bool },
    /// `MaybeUninit::zeroed()`, `Box::new_zeroed()`: its type's zero.
    Zeroed,
    /// `Box::new_uninit_slice(n)`, `new_zeroed_slice(n)`.
    Slice { zeroed: bool },
    /// `Box::write(b, v)`: `v`.
    BoxWrite,
    /// A `MaybeUninit`'s `write(v)`: written, and a `&mut` to it given.
    Write,
    /// `assume_init_mut()`: a `&mut` to what it holds.
    InitMut,
    /// `assume_init_read()`: a copy of what it holds, as a pointer's read
    /// is (#1265).
    Read,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `MaybeUninit`'s call, or a `Box`'s of one: `None` if `known` is another.
    pub(in crate::lower) fn uninit_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Uninit(op) = known else {
            return Ok(None);
        };
        let Call {
            generic_args,
            tys,
            span,
            written_whole,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        // What it holds: the function's type argument, or a `Box`'s item.
        let item = generic_args.types().next().expect("a type argument");
        Ok(Some(match op {
            // An array of `MaybeUninit`s, written an item at a time, is its
            // items, each not yet written: arrayvec's `MaybeUninit::uninit()
            // .assume_init()` of `[MaybeUninit<T>; CAP]`; and so a
            // `MaybeUninit` of an array of any items, which its pointer may
            // write an item at a time, `m.as_mut_ptr() as *mut T` (#1255),
            // where a `Box` made before its value, `vec![..]`'s, and one
            // only ever written whole and read whole (`written_whole`,
            // #1258) are written whole, `const m = [..]`; a type parameter
            // that's an array, smallvec's `A: Array`, its slots, none yet
            // (ADR 0417). Another's is written whole.
            UninitOp::Uninit { boxed } => match *item.kind() {
                ty::Array(each, len) if !(boxed || written_whole) || self.recognition().uninit_of(each).is_some() => {
                    let n = match len.try_to_target_usize(self.tcx) {
                        Some(n) => Expr::int(i128::from(n)),
                        None => self.const_arg(len, span)?,
                    };
                    crate::lower::plain::filled(n, None)
                }
                // Of a `MaybeUninit` of it, smallvec's `uninit().assume_init()`
                // of a `MaybeUninit<A>`, too: that's its value (ADR 0332).
                _ if std::iter::successors(Some(item), |&t| self.recognition().uninit_of(t))
                    .any(|t| self.is_array_param(t)) =>
                {
                    Expr::array(Vec::new())
                }
                _ => Expr::undefined(),
            },
            UninitOp::Zeroed => self.zero(item, span)?,
            UninitOp::Read => self.copy_if_needed(arg(), item),
            // `Array.from({ length: n })`; of a zero, `new Array(n).fill(0)`,
            // or an object made for each.
            UninitOp::Slice { zeroed } => {
                let item = match item.kind() {
                    ty::Slice(item) => *item,
                    _ => item,
                };
                let n = arg();
                let zero = if zeroed { Some(self.zero(item, span)?) } else { None };
                crate::lower::plain::filled(n, zero)
            }
            UninitOp::BoxWrite => {
                let boxed = arg();
                if boxed.has_effects() {
                    out.push(StmtKind::Expr(boxed).at(self.js_span(span)));
                }
                arg()
            }
            // Written through its box; a `&mut` to a number or text is the box,
            // to an object the object (ADR 0074).
            UninitOp::Write | UninitOp::InitMut => {
                let slot = arg();
                // A handle on a place is the place itself, written as it is.
                let place = match slot.kind {
                    js::ExprKind::Handle(place, _) => *place,
                    kind => {
                        let slot = Expr { kind, ..slot };
                        let slot = if slot.reads_same() {
                            slot
                        } else {
                            self.spill(Role::Slot, slot, out)
                        };
                        Expr::held(slot)
                    }
                };
                if op == UninitOp::Write {
                    let value = arg();
                    out.push(StmtKind::Assign(place.clone(), value).at(self.js_span(span)));
                }
                let target = tys[0].peel_refs();
                let held = self.recognition().uninit_of(target).unwrap_or(target);
                match self.mut_ref_form(held).is_cell() {
                    true => Expr::handle(place),
                    false => place,
                }
            }
        }))
    }

    /// Whether the `MaybeUninit` the call ending `at` makes,
    /// `MaybeUninit::<[T; N]>::uninit()`, is only ever written whole,
    /// `m.write([..])`, and read whole, `assume_init()` and its kin, of
    /// the local it's made in or a copy of it (#1258): no pointer is taken
    /// to it, nor a reference given to anything else, which might write it
    /// an item at a time. Such a one is written whole, as JS writes it,
    /// not its slots.
    pub(in crate::lower) fn uninit_written_whole(&self, body: &mir::Body<'tcx>, at: mir::BasicBlock) -> bool {
        let TerminatorKind::Call { func, destination, .. } = &body[at].terminator().kind else {
            return false;
        };
        let made = body.local_decls[destination.local].ty;
        if !destination.projection.is_empty()
            || !self.recognition().uninit_of(made).is_some_and(|item| item.is_array())
            || self.std_call_of(body, func) != Some(Std::Uninit(UninitOp::Uninit { boxed: false }))
        {
            return false;
        }
        // The local and each copy of it, and each reference to one.
        let mut held: IndexSet<Local> = IndexSet::default();
        held.insert(destination.local);
        let mut refs: IndexSet<Local> = IndexSet::default();
        let mut grew = true;
        while grew {
            grew = false;
            for data in body.basic_blocks.iter() {
                for statement in &data.statements {
                    let StatementKind::Assign(assign) = &statement.kind else {
                        continue;
                    };
                    let (place, rvalue) = &**assign;
                    let of = match rvalue {
                        Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) => of,
                        Rvalue::Ref(_, _, of) => of,
                        _ => continue,
                    };
                    if !place.projection.is_empty() || !of.projection.is_empty() || !held.contains(&of.local) {
                        continue;
                    }
                    let into = if matches!(rvalue, Rvalue::Ref(..)) {
                        &mut refs
                    } else {
                        &mut held
                    };
                    grew |= into.insert(place.local);
                }
            }
        }
        // Each use of them, which must be one of those.
        struct Uses<'b, 'tcx> {
            body: &'b mir::Body<'tcx>,
            of: IndexSet<Local>,
            at: Vec<(Local, mir::Location, PlaceContext)>,
        }
        impl<'tcx> Visitor<'tcx> for Uses<'_, 'tcx> {
            fn visit_local(&mut self, local: Local, context: PlaceContext, at: mir::Location) {
                if self.of.contains(&local) && !matches!(context, PlaceContext::NonUse(_)) {
                    self.at.push((local, at, context));
                }
            }
        }
        let mut uses = Uses {
            body,
            of: held.iter().chain(&refs).copied().collect(),
            at: Vec::new(),
        };
        uses.visit_body(body);
        uses.at.iter().all(|&(local, at, _)| {
            let data = &uses.body[at.block];
            match data.statements.get(at.statement_index).map(|statement| &statement.kind) {
                // A copy of one, or a reference to one, made.
                Some(StatementKind::Assign(assign)) => {
                    let (place, rvalue) = &**assign;
                    place.projection.is_empty()
                        && (held.contains(&place.local) || refs.contains(&place.local))
                        && matches!(rvalue,
                            Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) | Rvalue::Ref(_, _, of)
                                if of.projection.is_empty() && held.contains(&of.local))
                }
                // Where it's only named, a `let`'s.
                Some(StatementKind::FakeRead(..) | StatementKind::PlaceMention(..) | StatementKind::AscribeUserType(..)) => {
                    true
                }
                Some(_) => false,
                None => match &data.terminator().kind {
                    TerminatorKind::Drop { place, .. } => place.projection.is_empty(),
                    TerminatorKind::Call { func, args, destination, .. } => {
                        let called = self.std_call_of(body, func);
                        let given = args.iter().any(|arg| {
                            matches!(&arg.node, Operand::Copy(p) | Operand::Move(p) if p.local == local && p.projection.is_empty())
                        });
                        match called {
                            // Made here.
                            _ if destination.local == local => {
                                destination.projection.is_empty()
                                    && called == Some(Std::Uninit(UninitOp::Uninit { boxed: false }))
                            }
                            // Written whole through a reference.
                            Some(Std::Uninit(UninitOp::Write)) => given,
                            // Read whole, `assume_init()`, `assume_init_ref()`,
                            // `assume_init_read()`: what it holds, or a shared
                            // reference to it; not a function that's what it's
                            // given in another, `Box::new(m)`, whose `MaybeUninit`
                            // may be written through its pointer after.
                            Some(Std::Same | Std::Uninit(UninitOp::Read)) => {
                                let gives = body.local_decls[destination.local].ty;
                                given
                                    && !matches!(gives.kind(), ty::Ref(_, _, ty::Mutability::Mut))
                                    && Some(gives.peel_refs()) == self.recognition().uninit_of(made)
                            }
                            _ => false,
                        }
                    }
                    _ => false,
                },
            }
        })
    }

    /// What std function `func` is, of those rust-js knows.
    fn std_call_of(&self, body: &mir::Body<'tcx>, func: &Operand<'tcx>) -> Option<Std> {
        let (def_id, args) = crate::lower::fn_def(func.ty(body, self.tcx))?;
        self.recognition().classify(def_id, args)
    }
}
