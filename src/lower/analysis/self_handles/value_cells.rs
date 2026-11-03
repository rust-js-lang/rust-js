//! Callbacks' handle on the value the call they're made in gives (ADR
//! 0317, amended, rd621): React's Flight client's DEV `then` makes the
//! callbacks that settle a native promise in its executor, and they write
//! on that promise, which only the executor's call gives, so the program
//! holds it in a cell it sets right after:
//!
//! ```ignore
//! let wrapper: Rc<Cell<Option<Promise<T>>>> = Rc::default();
//! let wrapper_promise = js::promise::new(|res, rej| {
//!     let resolved = Rc::clone(&wrapper);
//!     resolve = Some(Rc::new(move |value| {
//!         js::set(resolved.get().unwrap(), "_debugInfo", ..);
//!         res(value);
//!     }));
//! });
//! wrapper.set(Some(wrapper_promise));
//! ```
//!
//! Each reader, a closure the executor makes, is only moved into a place
//! a `&mut` the executor holds lends, of a local of its maker's: the
//! executor neither calls it nor gives it to anything, and its maker reads
//! that local only after the cell's set, which is next after the call. So
//! no reader runs before the cell holds the call's value, a JS value
//! nothing changes in place, and each `get().unwrap()` of it gives that
//! value: the variable it's assigned to, which closes over it as upstream's
//! does,
//!
//! ```js
//! const wrapperPromise = new Promise((res, rej) => {
//!   resolve = (value) => {
//!     wrapperPromise._debugInfo = ..;
//!     res(value);
//!   };
//! });
//! ```
//!
//! its making, its borrow, its clones, its setting and its drops no JS, and
//! each read the variable. A read before the set can't happen, so neither
//! Rust's panic of it nor JS's of a `const` not yet made is reached.

use super::owners::{any_call_at, reaches};
use super::*;

/// Each reader an executor makes, and which of its captures is the cell's
/// clone.
type Readers = Vec<(DefId, usize)>;

/// `Rc<Cell<Option<T>>>`'s `Cell<Option<T>>`, of a `T` that's a JS value
/// nothing changes in place: a shared reference, or a JS object's handle
/// that's `Copy`, a `Promise`.
pub(super) fn value_cell<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let cell = crate::lower::recognition::rc_pointee(tcx, ty)?;
    let ty::Adt(adt, args) = *cell.kind() else { return None };
    if !crate::lower::recognition::is_std_def(tcx, adt.did(), crate::lower::recognition::StdItem::Cell) {
        return None;
    }
    let ty::Adt(option, args) = *args.type_at(0).kind() else {
        return None;
    };
    if !tcx.is_lang_item(option.did(), rustc_hir::attrs::lang_items::LangItem::Option) {
        return None;
    }
    let value = args.type_at(0);
    let frozen = match *value.kind() {
        ty::Ref(_, _, mir::Mutability::Not) => true,
        ty::Adt(adt, args) => {
            crate::lower::representation::marks_js_object(tcx, adt, args)
                && tcx.type_is_copy_modulo_regions(ty::TypingEnv::fully_monomorphized(), value)
        }
        _ => false,
    };
    frozen.then_some(cell)
}

/// The handle `cell`, of `body`, a value cell, makes, where it's one, its
/// readers read of `of`: made empty, lent to the one closure, the
/// executor, given to the call whose value it's set to next, and read by
/// the closures that executor makes and moves into its maker's locals.
pub(super) fn value_made<'tcx>(
    tcx: TyCtxt<'tcx>,
    (maker, body, uses): (DefId, &mir::Body<'tcx>, &Uses),
    cell: Local,
    of: &HashMap<DefId, &mir::Body<'tcx>>,
) -> Option<OwnerHandle<'tcx>> {
    let pointee = value_cell(tcx, body.local_decls[cell].ty)?;
    let mut made = HashSet::new();
    // What it's made of: `Rc::default()`, or `Rc::new(Cell::new(None))`.
    let at = uses.def(cell)?;
    made.insert(at);
    match call_at(tcx, body, at)? {
        (HandleStep::RcDefault, args, _) if args.is_empty() => {}
        (HandleStep::RcNew, args, _) => {
            let inner = whole(args.first()?)?;
            let at = uses.def(inner)?;
            let (HandleStep::ValueCellNew, args, _) = call_at(tcx, body, at)? else {
                return None;
            };
            made.insert(at);
            let none = whole(args.first()?)?;
            let at = uses.def(none)?;
            let (_, Rvalue::Aggregate(kind, fields)) = assign_at(body, at)? else {
                return None;
            };
            if !matches!(**kind, AggregateKind::Adt(adt, variant, ..)
                if tcx.is_lang_item(adt, rustc_hir::attrs::lang_items::LangItem::Option) && variant.as_u32() == 0)
                || !fields.is_empty()
            {
                return None;
            }
            made.insert(at);
            for local in [inner, none] {
                if !only_made(body, uses, local, &mut made) {
                    return None;
                }
            }
        }
        _ => return None,
    }
    // All made of it: the executor's borrow, and its one setting.
    let mut slice: Vec<Local> = vec![cell];
    let mut lent: Option<(DefId, usize, Local, Location)> = None;
    let mut sets: Vec<(Location, Local, Local)> = Vec::new();
    let mut i = 0;
    while i < slice.len() {
        let local = slice[i];
        i += 1;
        for &at in uses.uses(local) {
            if drops(body, at, local) {
                made.insert(at);
                continue;
            }
            if let Some((place, rvalue)) = assign_at(body, at) {
                match rvalue {
                    // `&cell`, or a reborrow of one.
                    Rvalue::Ref(_, mir::BorrowKind::Shared, p)
                        if p.local == local
                            && matches!(p.projection.as_slice(), [] | [PlaceElem::Deref])
                            && place.projection.is_empty() =>
                    {
                        made.insert(at);
                        slice.push(place.local);
                    }
                    // The executor, which captures a borrow of it.
                    Rvalue::Aggregate(kind, fields) if local != cell => {
                        let AggregateKind::Closure(closure, _) = **kind else {
                            return None;
                        };
                        let index = fields.iter().position(|f| whole(f) == Some(local))?;
                        if lent.is_some()
                            || fields.iter().filter(|f| rvalue_reads(f, local)).count() != 1
                            || !place.projection.is_empty()
                        {
                            return None;
                        }
                        lent = Some((closure, index, place.local, at));
                    }
                    _ => return None,
                }
                continue;
            }
            let (step, args, destination) = call_at(tcx, body, at)?;
            if args.first().and_then(|a| whole(a)) != Some(local) {
                return None;
            }
            match (step, &args[..]) {
                (HandleStep::RcDeref, [_]) => slice.push(destination),
                (HandleStep::ValueCellSet, [_, value]) => sets.push((at, whole(value)?, destination)),
                _ => return None,
            }
            made.insert(at);
        }
    }
    let (executor, capture, made_executor, made_at) = lent?;
    // Set once, of the call's value: `cell.set(Some(value))`.
    let [(set_at, value, result)] = sets[..] else {
        return None;
    };
    if body.basic_blocks[set_at.block].is_cleanup || !only_made(body, uses, result, &mut made) {
        return None;
    }
    let at = uses.def(value)?;
    let (_, Rvalue::Aggregate(kind, fields)) = assign_at(body, at)? else {
        return None;
    };
    if !matches!(**kind, AggregateKind::Adt(adt, variant, ..)
        if tcx.is_lang_item(adt, rustc_hir::attrs::lang_items::LangItem::Option) && variant.as_u32() == 1)
    {
        return None;
    }
    let [Operand::Copy(given) | Operand::Move(given)] = fields.iter().collect::<Vec<_>>()[..] else {
        return None;
    };
    made.insert(at);
    if !given.projection.is_empty() || !only_made(body, uses, value, &mut made) {
        return None;
    }
    // A variable the program named, as it is or copied, made by the call
    // alone, never changed.
    let mut handle = given.local;
    while !user_variable(&body.local_decls[handle]) {
        let at = uses.def(handle)?;
        // Copied, or a shared reference reborrowed, the same value.
        let from = match assign_at(body, at)?.1 {
            Rvalue::Use(Operand::Copy(from) | Operand::Move(from), ..) if from.projection.is_empty() => from,
            Rvalue::Ref(_, mir::BorrowKind::Shared, from)
                if from.projection.as_slice() == [PlaceElem::Deref]
                    && body.local_decls[from.local].ty.ref_mutability() == Some(mir::Mutability::Not) =>
            {
                from
            }
            _ => return None,
        };
        made.insert(at);
        if !only_made(body, uses, handle, &mut made) {
            return None;
        }
        handle = from.local;
    }
    if lent_mut(body, uses, handle) {
        return None;
    }
    let call = uses.def(handle)?;
    let (_, args, _) = any_call_at(tcx, body, call)?;
    // The executor, given to that call alone, as it's made.
    let mut given_at = None;
    for &at in uses.uses(made_executor) {
        if !drops(body, at, made_executor) && given_at.replace(at).is_some() {
            return None;
        }
    }
    if given_at != Some(call)
        || args.iter().filter(|a| rvalue_reads(a, made_executor)).count() != 1
        || !args
            .iter()
            .any(|a| matches!(a, Operand::Move(_)) && whole(a) == Some(made_executor))
    {
        return None;
    }
    // Each place it moves a reader into, a `&mut` of a local of the maker's.
    let (_, Rvalue::Aggregate(_, captured)) = assign_at(body, made_at)? else {
        return None;
    };
    let lends_local = |field: usize| {
        let Some(lent) = captured.raw.get(field).and_then(whole) else {
            return false;
        };
        matches!(uses.def(lent).and_then(|at| assign_at(body, at)),
            Some((_, Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, p))) if p.projection.is_empty())
    };
    let (clones, readers) = made_in(tcx, of.get(&executor)?, capture, &lends_local)?;
    // Nothing runs between the call and the cell's setting but its steps,
    // so no reader can read it before; nor is it set again.
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
    if !set_next(body, (after, set_at), &made, &[]) || reaches(body, set_at.block, set_at.block) {
        return None;
    }
    let mut read = HashMap::new();
    read.insert(executor, (clones, HashMap::new()));
    let mut closures = vec![(maker, executor, capture)];
    for (reader, index) in readers {
        let (at, own, _) = super::read(tcx, of.get(&reader)?, (index, Chain::Value))?;
        read.insert(reader, (at, own));
        closures.push((executor, reader, index));
    }
    Some(OwnerHandle {
        maker,
        handle,
        declared: None,
        closures,
        pointee,
        strong: false,
        made,
        read,
        upgrades: Vec::new(),
        called: None,
        unwrapped: Vec::new(),
    })
}

/// Of the executor, `body`, whose capture `capture` borrows the cell: its
/// clones of it, no JS, and each reader it makes of one, with which of its
/// captures that is, each moved, as it is, in an `Rc` or a `Some`, into
/// the place one of its captures, a `&mut` of its maker's local
/// (`lends_local`), lends, and nothing else of that capture used.
fn made_in<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    capture: usize,
    lends_local: &dyn Fn(usize) -> bool,
) -> Option<(HashSet<Location>, Readers)> {
    let env = Local::from_usize(1);
    let uses = Uses::of(body);
    // A capture's field, and what's past it.
    let field_of = |place: &Place<'tcx>| match place.projection.as_slice() {
        [PlaceElem::Deref, PlaceElem::Field(field, _), rest @ ..] | [PlaceElem::Field(field, _), rest @ ..] => {
            Some((field.as_usize(), rest.to_vec()))
        }
        _ => None,
    };
    let mut clones = HashSet::new();
    let mut lent = Vec::new();
    // Each place of the executor's captures, by where.
    let mut stores: HashMap<usize, Vec<Location>> = HashMap::new();
    for &at in uses.uses(env) {
        let data = &body.basic_blocks[at.block];
        let mut places = Vec::new();
        match data.statements.get(at.statement_index) {
            Some(statement) => mir::visit::Visitor::visit_statement(&mut Places(env, &mut places), statement, at),
            None => mir::visit::Visitor::visit_terminator(&mut Places(env, &mut places), data.terminator(), at),
        }
        for place in places {
            let (field, rest) = field_of(&place)?;
            if field == capture {
                // `&*cell`, or the borrow itself, to clone.
                let lends = matches!(assign_at(body, at), Some((to, Rvalue::Ref(_, mir::BorrowKind::Shared, p)
                    | Rvalue::Use(Operand::Copy(p), ..))) if *p == place && to.projection.is_empty()
                    && matches!(rest.as_slice(), [] | [PlaceElem::Deref]));
                if !lends {
                    return None;
                }
                clones.insert(at);
                lent.push(assign_at(body, at)?.0.local);
            } else {
                stores.entry(field).or_default().push(at);
            }
        }
    }
    // Each clone, captured by one reader.
    let mut made: Vec<(DefId, usize, Local)> = Vec::new();
    let mut i = 0;
    while i < lent.len() {
        let borrow = lent[i];
        i += 1;
        for &at in uses.uses(borrow) {
            if drops(body, at, borrow) {
                clones.insert(at);
                continue;
            }
            // A reborrow of it.
            if let Some((to, Rvalue::Ref(_, mir::BorrowKind::Shared, p))) = assign_at(body, at)
                && p.local == borrow
                && p.projection.as_slice() == [PlaceElem::Deref]
                && to.projection.is_empty()
            {
                clones.insert(at);
                lent.push(to.local);
                continue;
            }
            let (HandleStep::RcClone, args, clone) = call_at(tcx, body, at)? else {
                return None;
            };
            if args.len() != 1 || whole(args[0]) != Some(borrow) {
                return None;
            }
            clones.insert(at);
            // Captured by one reader, as it is or through a tuple, `let
            // (chunk, resolved) = (Rc::clone(self), Rc::clone(&wrapper))`.
            let mut held = clone;
            let mut captured = None;
            while captured.is_none() {
                let mut next = None;
                for &at in uses.uses(held) {
                    if drops(body, at, held) {
                        clones.insert(at);
                        continue;
                    }
                    if next.replace(at).is_some() {
                        return None;
                    }
                }
                let at = next?;
                let (place, Rvalue::Aggregate(kind, fields)) = assign_at(body, at)? else {
                    return None;
                };
                let index = fields
                    .iter()
                    .position(|f| matches!(f, Operand::Move(_)) && whole(f) == Some(held))?;
                if fields.iter().filter(|f| rvalue_reads(f, held)).count() != 1 || !place.projection.is_empty() {
                    return None;
                }
                match **kind {
                    AggregateKind::Closure(reader, _) => captured = Some((reader, index, place.local)),
                    AggregateKind::Tuple => {
                        let tuple = place.local;
                        let mut out = None;
                        for &at in uses.uses(tuple) {
                            if let Some((to, Rvalue::Use(Operand::Move(p), ..))) = assign_at(body, at)
                                && p.local == tuple
                                && matches!(p.projection.as_slice(), [PlaceElem::Field(f, _)] if f.as_usize() == index)
                                && (!to.projection.is_empty() || out.replace((at, to.local)).is_some())
                            {
                                return None;
                            }
                        }
                        let (at, to) = out?;
                        clones.insert(at);
                        held = to;
                    }
                    _ => return None,
                }
            }
            made.push(captured?);
        }
    }
    if made.is_empty() {
        return None;
    }
    // Each reader moved, made an `Rc`, coerced and put in a `Some`, into a
    // capture's place, `*resolve = ..`.
    let mut stored: HashMap<usize, Location> = HashMap::new();
    for &(_, _, mut value) in &made {
        loop {
            uses.def(value)?;
            // What unwinds only finishes the store, then drops it.
            let mut next = None;
            for &at in uses.uses(value) {
                if !drops(body, at, value) && !body.basic_blocks[at.block].is_cleanup && next.replace(at).is_some() {
                    return None;
                }
            }
            let at = next?;
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
                if !moved {
                    return None;
                }
                if place.local == env {
                    let (field, rest) = field_of(place)?;
                    if rest.as_slice() != [PlaceElem::Deref] || stored.insert(field, at).is_some() {
                        return None;
                    }
                    break;
                }
                if !place.projection.is_empty() {
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
    // A capture stored into is a `&mut` of the maker's local, and only
    // stored into, its value before dropped.
    for (&field, &at) in &stored {
        if !lends_local(field) {
            return None;
        }
        let only = stores.get(&field).is_some_and(|all| {
            all.iter().all(|&other| {
                other == at
                    || (body.basic_blocks[other.block].is_cleanup
                        && matches!(assign_at(body, other), Some((p, _)) if Some(p) == assign_at(body, at).map(|(p, _)| p)))
                    || matches!(&body.basic_blocks[other.block].terminator().kind,
                        TerminatorKind::Drop { place, .. } if other.statement_index == body.basic_blocks[other.block].statements.len()
                            && place.local == env && field_of(place).is_some_and(|(f, rest)| f == field && rest.as_slice() == [PlaceElem::Deref]))
            })
        });
        if !only {
            return None;
        }
    }
    Some((
        clones,
        made.into_iter().map(|(reader, index, _)| (reader, index)).collect(),
    ))
}
