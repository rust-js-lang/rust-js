//! Closures' handle on what owns them (ADR 0317, amended, the user's rd573
//! decision): callbacks made before the value they need, given to the call
//! that makes it, as React's Fizz server's entries make `onAllReady` before
//! the request it flows:
//!
//! ```ignore
//! let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
//! let on_all_ready: Rc<dyn Fn()> = {
//!     let cell = Rc::clone(&request_cell);
//!     Rc::new(move || {
//!         let request = cell.get().and_then(Weak::upgrade).unwrap();
//!         start_flowing(&request, ..);
//!     })
//! };
//! let request = create_request(.., Some(on_all_ready), ..);
//! let _ = request_cell.set(Rc::downgrade(&request));
//! ```
//!
//! What the call returns alone owns what it's given (`owned_by_return`), so
//! each closure runs only while that `Rc`'s strong count is one at least,
//! and the cell, set right after the call returns and never again, holds
//! its `Weak`: each upgrade gives that `Rc` once it's set, and `None`
//! before, and asks no count. Where nothing else counts it, the cell is
//! upstream's `let`, the `Rc`'s variable, which the closures close over:
//!
//! ```js
//! let request;
//! const onAllReady = () => {
//!   startFlowing($unwrap(request), ..);
//! };
//! request = createRequest(.., onAllReady, ..);
//! ```
//!
//! its making the `let`, its clones, its setting and its drops no JS, the
//! call's value assigned to it, and each closure's `get().and_then(
//! Weak::upgrade)` the variable, `undefined` until it's set, as `None` is.

use super::*;

/// Closures' handle on what owns them, of their maker and of each closure.
pub(crate) struct OwnerHandle<'tcx> {
    /// The body that makes the closures, the cell and the call.
    pub(super) maker: DefId,
    /// The maker's local of the `Rc` the call returns: the variable the
    /// cell is.
    pub(super) handle: Local,
    /// The block whose call makes the cell: where its `let` is.
    pub(super) declared: BasicBlock,
    /// Each closure, and which of its captures is the cell's clone.
    pub(super) closures: Vec<(DefId, usize)>,
    /// What the `Rc` points at, the `Weak`'s: the cell is the variable only
    /// where nothing counts it.
    pub(super) pointee: Ty<'tcx>,
    /// What the maker writes of the cell: nothing, at each.
    pub(super) made: HashSet<Location>,
    /// Of each closure, what it writes of reading the cell, nothing at
    /// each, and each block whose `and_then` gives the variable, to the
    /// local it gives it to.
    pub(super) read: HashMap<DefId, (HashSet<Location>, HashMap<BasicBlock, Local>)>,
    /// Where each `Weak::upgrade` of the reads is written, by its closure,
    /// which asks no count.
    pub(super) upgrades: Vec<(DefId, Span)>,
}

/// The owner handle `cell`, of `body`, makes, where it's one, each of its
/// closures read of `of`; `owned` says whether what a function returns
/// alone owns its parameter.
pub(super) fn owner_made<'tcx>(
    tcx: TyCtxt<'tcx>,
    (maker, body, uses): (DefId, &mir::Body<'tcx>, &Uses),
    cell: Local,
    (of, owned): (&HashMap<DefId, &mir::Body<'tcx>>, &dyn Fn(DefId, u32) -> bool),
) -> Option<OwnerHandle<'tcx>> {
    // An `Rc<OnceCell<Weak<D>>>`.
    let pointee = once_weak(tcx, body.local_decls[cell].ty)?;
    let mut made = HashSet::new();
    // What it's made of: `Rc::new(OnceCell::new())`.
    let at = uses.def(cell)?;
    let (HandleStep::RcNew, args, _) = call_at(tcx, body, at)? else {
        return None;
    };
    made.insert(at);
    let declared = at.block;
    let inner = whole(args.first()?)?;
    let at = uses.def(inner)?;
    let (HandleStep::OnceNew, args, _) = call_at(tcx, body, at)? else {
        return None;
    };
    if !args.is_empty() {
        return None;
    }
    made.insert(at);
    if !only_made(body, uses, inner, &mut made) {
        return None;
    }
    // All made of it: a clone for each closure, and its one setting.
    let mut slice: Vec<Local> = vec![cell];
    let mut clones: Vec<Local> = Vec::new();
    let mut sets: Vec<(Location, Local, Local)> = Vec::new();
    let mut i = 0;
    while i < slice.len() {
        let local = slice[i];
        i += 1;
        for &at in uses.uses(local) {
            if let Some((place, rvalue)) = assign_at(body, at) {
                // `&cell`, or a reborrow of one.
                let borrowed = matches!(rvalue, Rvalue::Ref(_, mir::BorrowKind::Shared, p)
                    if p.local == local && matches!(p.projection.as_slice(), [] | [PlaceElem::Deref]));
                if !borrowed || !place.projection.is_empty() {
                    return None;
                }
                made.insert(at);
                slice.push(place.local);
                continue;
            }
            if drops(body, at, local) {
                made.insert(at);
                continue;
            }
            let (step, args, destination) = call_at(tcx, body, at)?;
            if args.first().and_then(|a| whole(a)) != Some(local) {
                return None;
            }
            match (step, &args[..]) {
                (HandleStep::RcClone, [_]) => clones.push(destination),
                (HandleStep::RcDeref, [_]) => slice.push(destination),
                (HandleStep::OnceSet, [_, value]) => sets.push((at, whole(value)?, destination)),
                _ => return None,
            }
            made.insert(at);
        }
    }
    // Set once, where it's set, of the `Rc` the call returns: `let _ =
    // cell.set(Rc::downgrade(&handle))`, what it gives only dropped.
    let [(set_at, value, result)] = sets[..] else {
        return None;
    };
    if body.basic_blocks[set_at.block].is_cleanup || !only_made(body, uses, result, &mut made) {
        return None;
    }
    let mut handle = None;
    let mut set = vec![value];
    while let Some(local) = set.pop() {
        if !only_made(body, uses, local, &mut made) {
            return None;
        }
        let at = uses.def(local)?;
        made.insert(at);
        if let Some((step, args, _)) = call_at(tcx, body, at) {
            if step != HandleStep::Downgrade || args.len() != 1 {
                return None;
            }
            set.push(whole(args[0])?);
            continue;
        }
        match assign_at(body, at)?.1 {
            Rvalue::Ref(_, mir::BorrowKind::Shared, place) => match place.projection.as_slice() {
                [] => {
                    if handle.replace(place.local).is_some() {
                        return None;
                    }
                }
                [PlaceElem::Deref] => set.push(place.local),
                _ => return None,
            },
            _ => return None,
        }
    }
    let handle = handle.filter(|&h| user_variable(&body.local_decls[h]))?;
    // Never changed after, through a `&mut` or a pointer, which would change
    // the variable the closures read, not the cell.
    let changed = uses.uses(handle).iter().any(|&at| {
        matches!(assign_at(body, at), Some((_, Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, p) | Rvalue::RawPtr(_, p)))
            if p.local == handle)
    });
    if changed {
        return None;
    }
    // The call that makes it, given each closure's `Rc`, as it is or in a
    // `Some`, whose return alone owns it.
    let call = uses.def(handle)?;
    let mut closures = Vec::new();
    for clone in clones {
        let mut capture = None;
        for &at in uses.uses(clone) {
            if drops(body, at, clone) {
                made.insert(at);
                continue;
            }
            let (_, Rvalue::Aggregate(kind, fields)) = assign_at(body, at)? else {
                return None;
            };
            let AggregateKind::Closure(closure, _) = **kind else {
                return None;
            };
            let index = fields.iter().position(|f| whole(f) == Some(clone))?;
            if capture.is_some() || fields.iter().filter(|f| rvalue_reads(f, clone)).count() != 1 {
                return None;
            }
            capture = Some((closure, index, assign_at(body, at)?.0.local));
        }
        let (closure, index, mut value) = capture?;
        // Moved on, made an `Rc`, coerced and put in a `Some`, to the call.
        loop {
            uses.def(value)?;
            let mut next = None;
            for &at in uses.uses(value) {
                if drops(body, at, value) {
                    continue;
                }
                if next.is_some() {
                    return None;
                }
                next = Some(at);
            }
            let at = next?;
            if at == call {
                let (_, args, _) = any_call_at(tcx, body, at)?;
                let given = args.iter().position(|a| whole(a) == Some(value))?;
                if args.iter().filter(|a| rvalue_reads(a, value)).count() != 1
                    || !matches!(args[given], Operand::Move(_))
                {
                    return None;
                }
                closures.push((closure, index, given as u32));
                break;
            }
            if let Some((place, rvalue)) = assign_at(body, at) {
                let moved = match rvalue {
                    Rvalue::Use(operand @ Operand::Move(_), ..) | Rvalue::Cast(_, operand @ Operand::Move(_), _) => {
                        whole(operand) == Some(value)
                    }
                    Rvalue::Aggregate(kind, fields) => {
                        matches!(**kind, AggregateKind::Adt(adt, variant, ..)
                            if tcx.is_lang_item(adt, rustc_hir::attrs::lang_items::LangItem::Option) && variant.as_u32() == 1)
                            && matches!(fields.iter().collect::<Vec<_>>()[..], [operand @ Operand::Move(_)] if whole(operand) == Some(value))
                    }
                    _ => false,
                };
                if !moved || !place.projection.is_empty() {
                    return None;
                }
                value = place.local;
                continue;
            }
            match call_at(tcx, body, at)? {
                (HandleStep::RcNew, args, destination) if args.len() == 1 && whole(args[0]) == Some(value) => {
                    value = destination;
                }
                _ => return None,
            }
        }
    }
    if closures.is_empty() {
        return None;
    }
    let (callee, _, _) = any_call_at(tcx, body, call)?;
    if !closures.iter().all(|&(_, _, given)| owned(callee, given)) {
        return None;
    }
    // Nothing runs between the call and the cell's setting but its steps,
    // so no closure reads the cell set where JS has it assigned; nor is it
    // set again, on a later turn.
    let TerminatorKind::Call {
        target: Some(after), ..
    } = body.basic_blocks[call.block].terminator().kind
    else {
        return None;
    };
    let after = Location {
        block: after,
        statement_index: 0,
    };
    if !set_next(body, (after, set_at), &made, &[]) {
        return None;
    }
    if reaches(body, set_at.block, set_at.block) {
        return None;
    }
    // Each closure reads the cell only by `cell.get().and_then(Weak::upgrade)`.
    let mut read = HashMap::new();
    let mut upgrades = Vec::new();
    for &(closure, index, _) in &closures {
        let (at, own, spans) = super::read(tcx, of.get(&closure)?, (index, true))?;
        upgrades.extend(spans.into_iter().map(|span| (closure, span)));
        read.insert(closure, (at, own));
    }
    Some(OwnerHandle {
        maker,
        handle,
        declared,
        closures: closures
            .into_iter()
            .map(|(closure, index, _)| (closure, index))
            .collect(),
        pointee,
        made,
        read,
        upgrades,
    })
}

/// `Rc<OnceCell<Weak<D>>>`'s `D`.
pub(super) fn once_weak<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let cell = crate::lower::recognition::rc_pointee(tcx, ty)?;
    let ty::Adt(adt, args) = *cell.kind() else { return None };
    if !crate::lower::recognition::is_std_def(tcx, adt.did(), crate::lower::recognition::StdItem::OnceCell) {
        return None;
    }
    weak_pointee(tcx, args.type_at(0))
}

/// The call at `at`, a block's terminator, of any function: it, its
/// arguments and its destination, a local whole.
fn any_call_at<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &'a mir::Body<'tcx>,
    at: Location,
) -> Option<(DefId, Vec<&'a Operand<'tcx>>, Local)> {
    let data = &body.basic_blocks[at.block];
    if at.statement_index != data.statements.len() {
        return None;
    }
    let TerminatorKind::Call {
        func,
        args,
        destination,
        ..
    } = &data.terminator().kind
    else {
        return None;
    };
    let (id, _) = fn_def(func.ty(body, tcx))?;
    destination.projection.is_empty().then_some(())?;
    Some((id, args.iter().map(|a| &a.node).collect(), destination.local))
}

/// Whether `to` is reached from `from`'s successors, by what doesn't unwind.
fn reaches(body: &mir::Body<'_>, from: BasicBlock, to: BasicBlock) -> bool {
    let mut seen = HashSet::new();
    let mut todo: Vec<BasicBlock> = body.basic_blocks[from].terminator().successors().collect();
    while let Some(block) = todo.pop() {
        if body.basic_blocks[block].is_cleanup || !seen.insert(block) {
            continue;
        }
        if block == to {
            return true;
        }
        todo.extend(body.basic_blocks[block].terminator().successors());
    }
    false
}
