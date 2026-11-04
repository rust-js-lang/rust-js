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
//!
//! Where nothing the call runs before it returns may call any of them
//! (`OwnedByReturn::quiet`), none runs before the cell is set, so each
//! `unwrap` of its upgrade is the variable too, never `undefined` where
//! it's read: `startFlowing(request, ..)` (rd621).

use super::*;

/// Closures' handle on what owns them, of their maker and of each closure.
pub(crate) struct OwnerHandle<'tcx> {
    /// The body that makes the closures, the cell and the call.
    pub(super) maker: DefId,
    /// The maker's local of the `Rc` the call returns: the variable the
    /// cell is.
    pub(super) handle: Local,
    /// The block whose call makes the cell: where its `let` is; none of a
    /// value cell, whose call's value is its `const` (`value_made`).
    pub(super) declared: Option<BasicBlock>,
    /// Each closure, the body that makes it, the maker's or the closure's
    /// it's given to (`value_made`), and which of its captures is the
    /// cell's clone, or that closure's borrow of the cell.
    pub(super) closures: Vec<(DefId, DefId, usize)>,
    /// What the `Rc` points at, the `Weak`'s, or the cell of a value cell:
    /// the cell is the variable only where nothing counts it.
    pub(super) pointee: Ty<'tcx>,
    /// Whether the cell holds the `Rc` itself, not its `Weak` (#577): the
    /// variable only where a `Some` of it isn't boxed.
    pub(super) strong: bool,
    /// What the maker writes of the cell: nothing, at each.
    pub(super) made: HashSet<Location>,
    /// Of each closure, what it writes of reading the cell, nothing at
    /// each, and each block whose `and_then`, or a value cell's `unwrap`,
    /// gives the variable, to the local it gives it to; of the closure a
    /// value cell's readers are made in, its clones of the cell, and none.
    pub(super) read: HashMap<DefId, (HashSet<Location>, HashMap<BasicBlock, Local>)>,
    /// Where each `Weak::upgrade` of the reads is written, by its closure,
    /// which asks no count.
    pub(super) upgrades: Vec<(DefId, Span)>,
    /// The function the call is of, and the parameters it's given the
    /// closures as; none of a value cell's.
    pub(super) called: Option<(DefId, Vec<u32>)>,
    /// Of each closure, each read whose `unwrap` of the upgrade is the
    /// variable where none runs before the cell is set (`direct`).
    pub(super) unwrapped: Vec<(DefId, Unwrapped)>,
    /// Of a value cell's executor, each clone of it a tuple holds beside
    /// what isn't one: a local of no value (`value_cells`, #641).
    pub(super) unwritten: Vec<(DefId, Local)>,
}

/// A closure's `unwrap` of its owner's upgrade, `cell.get().and_then(
/// Weak::upgrade).unwrap()`: where the `unwrap` is, the local it gives the
/// `Rc` to, and the drops of what the `and_then` gives.
#[derive(Clone)]
pub(super) struct Unwrapped {
    at: Location,
    value: Local,
    drops: Vec<Location>,
}

impl OwnerHandle<'_> {
    /// Each closure's `unwrap` of its upgrade the variable: none runs
    /// before the cell is set, so it's never `None` where it's read.
    pub(super) fn direct(&mut self) {
        for (closure, unwrapped) in std::mem::take(&mut self.unwrapped) {
            let Some((read, own)) = self.read.get_mut(&closure) else {
                continue;
            };
            own.insert(unwrapped.at.block, unwrapped.value);
            read.insert(unwrapped.at);
            read.extend(unwrapped.drops);
        }
    }
}

/// A cell made as an owner handle's is, but given, in a closure's `Rc`, to
/// a call whose return doesn't alone own it, so it's counted, as written:
/// what a refusal of what that counts names (ADR 0317, amended).
pub(crate) struct Unowned<'tcx> {
    /// The closure, and the cell's clone's type, which it holds.
    pub(crate) closure: DefId,
    pub(crate) cell: Ty<'tcx>,
    /// The function it's given to, and which of its parameters it is.
    pub(crate) callee: DefId,
    pub(crate) given: u32,
}

/// The owner handle `cell`, of `body`, makes, where it's one, each of its
/// closures read of `of`; `owned` says whether what a function returns
/// alone owns its parameter. A cell given to a call whose return doesn't
/// own its closures is `unowned`'s.
pub(super) fn owner_made<'tcx>(
    tcx: TyCtxt<'tcx>,
    (maker, body, uses): (DefId, &mir::Body<'tcx>, &Uses),
    cell: Local,
    (of, owned): (&HashMap<DefId, &mir::Body<'tcx>>, &dyn Fn(DefId, u32) -> bool),
    unowned: &mut Vec<Unowned<'tcx>>,
) -> Option<OwnerHandle<'tcx>> {
    // An `Rc<OnceCell<Weak<D>>>`, or an `Rc<OnceCell<Rc<D>>>` (#577).
    let (pointee, held) = once_held(tcx, body.local_decls[cell].ty)?;
    let mut made = HashSet::new();
    // What it's made of: `Rc::new(OnceCell::new())`, or `Rc::default()`.
    let at = uses.def(cell)?;
    let declared = at.block;
    match call_at(tcx, body, at)? {
        (HandleStep::RcDefault, args, _) if args.is_empty() => _ = made.insert(at),
        (HandleStep::RcNew, args, _) => {
            made.insert(at);
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
        }
        _ => return None,
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
    // cell.set(Rc::downgrade(&handle))`, or `Rc::clone(&handle)` of a
    // strong one, what it gives only dropped.
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
            let step_of = match held {
                Held::Weak => HandleStep::Downgrade,
                Held::Strong => HandleStep::RcClone,
            };
            if step != step_of || args.len() != 1 {
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
        // A strong one's closures may be anywhere: none can read what the
        // cell holds before the call returns, whoever calls it, and each
        // reads `None` before it as the `let` is `undefined`; after, the
        // value, which no closure owns (#577).
        if held == Held::Strong {
            closures.push((closure, index, 0));
            continue;
        }
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
    let not_owned: Vec<Unowned<'tcx>> = (closures.iter())
        .filter(|&&(_, _, given)| !owned(callee, given))
        .map(|&(closure, _, given)| Unowned {
            closure,
            cell: body.local_decls[cell].ty,
            callee,
            given,
        })
        .collect();
    if held == Held::Weak && !not_owned.is_empty() {
        unowned.extend(not_owned);
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
    // Each closure reads the cell only by `cell.get().and_then(Weak::upgrade)`,
    // or a strong one's by `cell.get()`.
    let chain = match held {
        Held::Weak => Chain::Owner,
        Held::Strong => Chain::Strong,
    };
    let mut read = HashMap::new();
    let mut upgrades = Vec::new();
    let mut unwrapped = Vec::new();
    for &(closure, index, _) in &closures {
        let body = of.get(&closure)?;
        let (at, own, spans) = super::read(tcx, body, (index, chain))?;
        upgrades.extend(spans.into_iter().map(|span| (closure, span)));
        let reads = Uses::of(body);
        // A strong one's `unwrap` stays: its call isn't asked what it may
        // call before it returns (`called`).
        for &option in own.values().filter(|_| held == Held::Weak) {
            if let Some(at) = unwrapped_at(tcx, body, &reads, option) {
                unwrapped.push((closure, at));
            }
        }
        read.insert(closure, (at, own));
    }
    let given: Vec<u32> = closures.iter().map(|&(_, _, given)| given).collect();
    // What owns a strong one's closures isn't asked, so neither is what it
    // may call before it returns (`quiet`).
    let called = (held == Held::Weak).then_some((callee, given));
    Some(OwnerHandle {
        maker,
        handle,
        declared: Some(declared),
        closures: closures
            .into_iter()
            .map(|(closure, index, _)| (maker, closure, index))
            .collect(),
        pointee,
        strong: held == Held::Strong,
        made,
        read,
        upgrades,
        called,
        unwrapped,
        unwritten: Vec::new(),
    })
}

/// The `unwrap` of `option`, what the `and_then` ending `and_then` gives,
/// where that's all it's used for but its drops: of a local of its own,
/// set once and lent nowhere mutably, the variable in its place.
fn unwrapped_at<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, uses: &Uses, option: Local) -> Option<Unwrapped> {
    let mut drops = Vec::new();
    let mut unwrap = None;
    for &at in uses.uses(option) {
        if super::drops(body, at, option) {
            drops.push(at);
            continue;
        }
        if unwrap.replace(at).is_some() {
            return None;
        }
    }
    let at = unwrap?;
    let (HandleStep::Unwrap, args, value) = call_at(tcx, body, at)? else {
        return None;
    };
    let owned = value != mir::RETURN_PLACE && uses.def(value) == Some(at);
    (args.len() == 1 && whole(args[0]) == Some(option) && owned && !lent_mut(body, uses, value)).then_some(Unwrapped {
        at,
        value,
        drops,
    })
}

/// What an owner handle's cell holds of what the call returns.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Held {
    /// Its `Weak`, which gives it while what it owns is alive.
    Weak,
    /// Its `Rc` itself, a clone, which gives it once it's set (#577).
    Strong,
}

/// `Rc<OnceCell<Weak<D>>>`'s `D`, or `Rc<OnceCell<Rc<D>>>`'s, whose `Some`
/// is the variable only where `D` never looks like `None`, which
/// `SelfHandles::unboxed` asks.
pub(super) fn once_held<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, Held)> {
    let cell = crate::lower::recognition::rc_pointee(tcx, ty)?;
    let ty::Adt(adt, args) = *cell.kind() else { return None };
    if !crate::lower::recognition::is_std_def(tcx, adt.did(), crate::lower::recognition::StdItem::OnceCell) {
        return None;
    }
    let held = args.type_at(0);
    if let Some(pointee) = weak_pointee(tcx, held) {
        return Some((pointee, Held::Weak));
    }
    Some((crate::lower::recognition::rc_pointee(tcx, held)?, Held::Strong))
}

/// The call at `at`, a block's terminator, of any function: it, its
/// arguments and its destination, a local whole.
pub(super) fn any_call_at<'a, 'tcx>(
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
pub(super) fn reaches(body: &mir::Body<'_>, from: BasicBlock, to: BasicBlock) -> bool {
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
