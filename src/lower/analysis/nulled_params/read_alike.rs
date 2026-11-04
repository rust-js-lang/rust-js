//! The crate's `#[rust_js::nullable]` thread-locals nothing reads apart,
//! `null` from `undefined` (#665). A nullable place stores `null` for
//! `None`, `x ?? null` of what may be `undefined`, because a read of it may
//! tell them apart: a `=== null` test, a store as it is into another
//! nullable place, a nullable function's return, what JS or a binding is
//! given. Where no read can, the coalesce changes nothing a program sees,
//! so a write stores what it's given as it is, React's `resolveFamily =
//! handler;` (ADR 0861, amended; ADR 0262's case "none").
//!
//! One is such only where every read of it is seen and each reads `null`
//! and `undefined` alike: it's private to its module, which alone reads
//! it, by code that isn't generic, so no other crate's or module's JS
//! does; each use is a `get`, `set`, `take`, `replace` or a
//! `with_borrow(Clone::clone)` of it, never a lend whose closure may do
//! anything; and what each read gives is only tested, `== null`, its
//! `Some`'s value read, unwrapped, given a fallback, `map_or`'d, set back
//! into one such, or moved, copied, borrowed, mapped or cloned into what's
//! only used so. Anything else, a call of the crate's function among them,
//! leaves it as it was: when unsure, `?? null`.

use crate::lower::Body;
use crate::lower::body_queries::strip;
use crate::lower::combinators::Comb;
use crate::lower::fn_def;
use crate::lower::recognition::{Recognition, Std, local_key_access};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{self, Local, Location, Operand, PlaceElem, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::thir::ExprKind;
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// The nullable thread-locals of `bodies`, the crate's every body, that
/// nothing reads apart.
pub(super) fn found<'tcx>(recognition: &Recognition<'_, 'tcx>, bodies: &[&Body<'tcx>]) -> HashSet<DefId> {
    let tcx = recognition.tcx;
    let nullable = |key: DefId| key.is_local() && crate::lower::bindings::nullable_local_cell(tcx, key);
    // Each read or set where the THIR shows it, and each read or used
    // where its reads may not all be seen.
    let mut alike: HashSet<DefId> = HashSet::new();
    let mut refused: HashSet<DefId> = HashSet::new();
    // How many calls of each the THIR shows, which MIR must show too.
    let mut called: HashMap<DefId, usize> = HashMap::new();
    for body in bodies {
        let thir = &body.thir;
        let at = body.def_id;
        // Each read or set of one, `KEY` of `KEY.get()`.
        let mut accessed = HashSet::new();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let Some((id, _)) = fn_def(thir[fun].ty)
                && local_key_access(tcx, id).is_some()
                && let Some(&first) = args.first()
                && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
                && let ExprKind::NamedConst { def_id: key, .. } = thir[strip(thir, arg)].kind
                && nullable(key)
            {
                accessed.insert(strip(thir, arg));
                *called.entry(key).or_default() += 1;
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            let ExprKind::NamedConst { def_id: key, .. } = expr.kind else {
                continue;
            };
            if !nullable(key) {
                continue;
            }
            // Exported, used by another module or by generic code another
            // crate may compile, or handed on, `is_set(&KEY)`: its reads
            // aren't all seen.
            let elsewhere = tcx.visibility(key).is_public()
                || !at.is_local()
                || tcx.parent_module_from_def_id(at.expect_local())
                    != tcx.parent_module_from_def_id(key.expect_local())
                || tcx.generics_of(at).requires_monomorphization(tcx);
            match elsewhere || !accessed.contains(&id) {
                true => _ = refused.insert(key),
                false => _ = alike.insert(key),
            }
        }
    }
    // Each call MIR shows of each, as the THIR shows them, and what each
    // read gives used alike.
    let mut seen: HashMap<DefId, usize> = HashMap::new();
    for body in bodies {
        let Some(mir) = &body.mir else { continue };
        let uses = Uses::new(recognition, &mir.body, &mir.promoted);
        for data in mir.body.basic_blocks.iter() {
            let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let Some(key) = uses.key_called(func, args).filter(|&key| alike.contains(&key)) else {
                continue;
            };
            *seen.entry(key).or_default() += 1;
            let (id, generic_args) = fn_def(func.ty(&mir.body, tcx)).expect("a thread-local's method");
            let read = match recognition.classify(id, generic_args) {
                Some(Std::CellSet) => true,
                Some(Std::CellGet | Std::CellTake | Std::CellReplace) => {
                    destination.projection.is_empty() && uses.alike(key, destination.local, &mut Vec::new())
                }
                // Its value cloned whole, `KEY.with_borrow(Clone::clone)`.
                Some(Std::LocalBorrow { mutable: false }) => {
                    let f = args.get(1).and_then(|f| fn_def(f.node.ty(&mir.body, tcx)));
                    let clone = f.and_then(|(f, _)| tcx.trait_of_assoc(f));
                    clone.is_some_and(|clone| tcx.is_lang_item(clone, LangItem::Clone))
                        && destination.projection.is_empty()
                        && uses.alike(key, destination.local, &mut Vec::new())
                }
                // A lend, `with_borrow_mut(f)`, whose `f` may do anything.
                _ => false,
            };
            if !read {
                refused.insert(key);
            }
        }
    }
    alike.retain(|key| !refused.contains(key) && seen.get(key) == called.get(key));
    alike
}

/// What a body does with an `Option` read whole from such a thread-local.
struct Uses<'u, 'tcx> {
    recognition: &'u Recognition<'u, 'tcx>,
    body: &'u mir::Body<'tcx>,
    promoted: &'u rustc_index::IndexVec<mir::Promoted, mir::Body<'tcx>>,
    /// Each local assigned once by a statement, and what it's assigned.
    assigned: HashMap<Local, Option<&'u Rvalue<'tcx>>>,
}

impl<'u, 'tcx> Uses<'u, 'tcx> {
    fn new(
        recognition: &'u Recognition<'u, 'tcx>,
        body: &'u mir::Body<'tcx>,
        promoted: &'u rustc_index::IndexVec<mir::Promoted, mir::Body<'tcx>>,
    ) -> Self {
        let mut assigned: HashMap<Local, Option<&Rvalue<'tcx>>> = HashMap::new();
        for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
            if let StatementKind::Assign(assign) = &statement.kind
                && assign.0.projection.is_empty()
            {
                let once = !assigned.contains_key(&assign.0.local);
                assigned.insert(assign.0.local, once.then_some(&assign.1));
            }
        }
        Uses {
            recognition,
            body,
            promoted,
            assigned,
        }
    }

    /// The nullable thread-local the call of `func` with `args` is a method
    /// of, `KEY` of `KEY.get()`.
    fn key_called(&self, func: &Operand<'tcx>, args: &[rustc_span::Spanned<Operand<'tcx>>]) -> Option<DefId> {
        let (id, _) = fn_def(func.ty(self.body, self.recognition.tcx))?;
        local_key_access(self.recognition.tcx, id)?;
        let first = args.first()?;
        crate::lower::mir::referred_key(
            self.promoted,
            |local| self.assigned.get(&local).copied().flatten(),
            &first.node,
        )
    }

    /// Each place in the body `local` is used, but where it's declared.
    fn mentions(&self, local: Local) -> Vec<Location> {
        struct Mentions(Local, Vec<Location>);
        impl<'tcx> Visitor<'tcx> for Mentions {
            fn visit_local(&mut self, local: Local, context: PlaceContext, location: Location) {
                if local == self.0 && !matches!(context, PlaceContext::NonUse(_)) {
                    self.1.push(location);
                }
            }
        }
        let mut mentions = Mentions(local, Vec::new());
        mentions.visit_body(self.body);
        mentions.1.dedup();
        mentions.1
    }

    /// Whether each use of `local`, an `Option` read from `key`, reads
    /// `null` and `undefined` alike, of the locals it's followed through,
    /// `seen`, each once. The function's return is read by its `return`,
    /// which isn't.
    fn alike(&self, key: DefId, local: Local, seen: &mut Vec<Local>) -> bool {
        if seen.contains(&local) {
            return true;
        }
        seen.push(local);
        (self.mentions(local).into_iter()).all(|location| self.used_alike((key, local), location, false, seen))
    }

    /// Whether each use of `local`, a shared reference to such an
    /// `Option`, reads it alike.
    fn alike_through(&self, key: DefId, local: Local, seen: &mut Vec<Local>) -> bool {
        if seen.contains(&local) {
            return true;
        }
        seen.push(local);
        (self.mentions(local).into_iter()).all(|location| self.used_alike((key, local), location, true, seen))
    }

    /// Whether the use of `local` at `location` reads it alike: of the
    /// `Option` itself, or of a shared reference to it (`through`).
    fn used_alike(
        &self,
        (key, local): (DefId, Local),
        location: Location,
        through: bool,
        seen: &mut Vec<Local>,
    ) -> bool {
        let data = &self.body.basic_blocks[location.block];
        let Some(statement) = data.statements.get(location.statement_index) else {
            return self.called_alike((key, local), data.terminator(), through, seen);
        };
        // The `Option`'s place, or what the reference refers to.
        let whole: &[PlaceElem<'tcx>] = if through { &[PlaceElem::Deref] } else { &[] };
        let is = |place: &mir::Place<'tcx>, projection: &[PlaceElem<'tcx>]| {
            place.local == local && place.projection.as_slice() == projection
        };
        // Its `Some`'s value, `(x as Some).0`, there either way.
        let payload = |place: &mir::Place<'tcx>| {
            place.local == local
                && matches!(place.projection.as_slice().strip_prefix(whole),
                    Some([PlaceElem::Downcast(_, variant), PlaceElem::Field(..), ..]) if variant.as_u32() == 1)
        };
        match &statement.kind {
            StatementKind::Assign(assign) => {
                let (target, rvalue) = &**assign;
                // Its own write, of what doesn't read it.
                if target.local == local {
                    return target.projection.is_empty() && !self.reads(rvalue, local);
                }
                let into = target.projection.is_empty().then_some(target.local);
                match rvalue {
                    // Its test, `== null`, a `match`'s, an `if let`'s.
                    Rvalue::Discriminant(of) => is(of, whole),
                    Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
                    | Rvalue::Ref(_, mir::BorrowKind::Shared | mir::BorrowKind::Fake(_), of)
                        if payload(of) =>
                    {
                        true
                    }
                    // A `match` guard's, which reads nothing.
                    Rvalue::Ref(_, mir::BorrowKind::Fake(_), of) if is(of, whole) => true,
                    // Moved or copied whole, into what's used so too.
                    Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) if is(of, &[]) => match through {
                        false => into.is_some_and(|into| self.alike(key, into, seen)),
                        true => into.is_some_and(|into| self.alike_through(key, into, seen)),
                    },
                    Rvalue::CopyForDeref(of) if through && is(of, &[]) => {
                        into.is_some_and(|into| self.alike_through(key, into, seen))
                    }
                    // Borrowed shared, or reborrowed, `&*r`.
                    Rvalue::Ref(_, mir::BorrowKind::Shared, of) if is(of, whole) => {
                        into.is_some_and(|into| self.alike_through(key, into, seen))
                    }
                    _ => false,
                }
            }
            StatementKind::FakeRead(..)
            | StatementKind::PlaceMention(..)
            | StatementKind::AscribeUserType(..)
            | StatementKind::Coverage(..)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop
            | StatementKind::BackwardIncompatibleDropHint { .. } => true,
            _ => false,
        }
    }

    /// Whether `rvalue` reads `local`.
    fn reads(&self, rvalue: &Rvalue<'tcx>, local: Local) -> bool {
        struct Reads(Local, bool);
        impl<'tcx> Visitor<'tcx> for Reads {
            fn visit_local(&mut self, local: Local, _: PlaceContext, _: Location) {
                self.1 |= local == self.0;
            }
        }
        let mut reads = Reads(local, false);
        reads.visit_rvalue(rvalue, Location::START);
        reads.1
    }

    /// Whether `terminator`'s use of `local` reads it alike: its drop, its
    /// write as a call's result, or a call of std's that tests it, reads
    /// its `Some`'s value or gives it a fallback, or one whose result is
    /// what it's of, used alike too, `map`'s, `as_ref()`'s, `clone()`'s;
    /// or its set back into `key`. Of a shared reference to it
    /// (`through`), the calls that take one.
    fn called_alike(
        &self,
        (key, local): (DefId, Local),
        terminator: &mir::Terminator<'tcx>,
        through: bool,
        seen: &mut Vec<Local>,
    ) -> bool {
        let tcx = self.recognition.tcx;
        let (func, args, destination) = match &terminator.kind {
            TerminatorKind::Drop { place, .. } => return place.local == local,
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => (func, args, destination),
            _ => return false,
        };
        let given = |operand: &Operand<'tcx>| operand.place().is_some_and(|place| place.local == local);
        if destination.local == local {
            return destination.projection.is_empty() && !args.iter().any(|arg| given(&arg.node));
        }
        // Given whole, once.
        let positions: Vec<usize> = (args.iter().enumerate())
            .filter(|(_, arg)| given(&arg.node))
            .map(|(i, _)| i)
            .collect();
        let [at] = positions[..] else { return false };
        if args[at].node.place().is_none_or(|place| !place.projection.is_empty()) || given(func) {
            return false;
        }
        let Some((def_id, generic_args)) = fn_def(func.ty(self.body, tcx)) else {
            return false;
        };
        let result = || destination.projection.is_empty().then_some(destination.local);
        let known = self.recognition.classify(def_id, generic_args);
        if through {
            return match known {
                Some(Std::IsSome | Std::IsNone) => at == 0,
                // `as_ref()`, `as_deref()` and `clone()`, the option itself
                // in JS, or what it holds, cloned (ADR 0023).
                Some(Std::Pointee | Std::Comb(Comb::AsDeref)) => {
                    at == 0
                        && matches!(
                            args[0].node.ty(self.body, tcx).kind(),
                            rustc_middle::ty::Ref(_, _, rustc_middle::ty::Mutability::Not)
                        )
                        && result().is_some_and(|result| self.alike(key, result, seen))
                }
                _ if (tcx.trait_of_assoc(def_id)).is_some_and(|t| tcx.is_lang_item(t, LangItem::Clone)) => {
                    at == 0 && result().is_some_and(|result| self.alike(key, result, seen))
                }
                _ => false,
            };
        }
        match known {
            // Tested, unwrapped, given a fallback: none of these gives it on.
            Some(
                Std::Unwrap
                | Std::UnwrapUnchecked
                | Std::UnwrapOr
                | Std::Comb(
                    Comb::UnwrapOrElse
                    | Comb::UnwrapOrDefault
                    | Comb::MapOr
                    | Comb::MapOrElse
                    | Comb::MapOrDefault
                    | Comb::IsSomeAnd
                    | Comb::IsNoneOr
                    | Comb::OkOr
                    | Comb::OkOrElse,
                ),
            ) => at == 0,
            // What it's of, or what it gives in its place, used alike.
            Some(Std::OptionMap | Std::OptionCloned | Std::Comb(Comb::AndThen | Comb::Filter | Comb::Inspect)) => {
                at == 0 && result().is_some_and(|result| self.alike(key, result, seen))
            }
            Some(Std::Comb(Comb::Or | Comb::OrElse | Comb::Xor)) => {
                result().is_some_and(|result| self.alike(key, result, seen))
            }
            Some(Std::Drop) => at == 0,
            // Set back into it, `KEY.set(previous)`.
            Some(Std::CellSet | Std::CellReplace) => {
                at == 1
                    && self.key_called(func, args) == Some(key)
                    && result().is_none_or(|result| self.alike(key, result, seen))
            }
            _ => false,
        }
    }
}
