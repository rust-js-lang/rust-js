//! A closure's handle on itself (ADR 0843, amended, the user's rd96
//! decision): the `Rc` of a `RefCell` of the `Weak` its own `Rc` was made
//! of, as the Rust book teaches a value that refers to itself,
//!
//! ```ignore
//! let cell = Rc::new(RefCell::new(None));
//! let f: Rc<dyn Fn()> = Rc::new({
//!     let cell = Rc::clone(&cell);
//!     move || {
//!         let f = cell.borrow().as_ref().and_then(Weak::upgrade).unwrap();
//!         ..
//!     }
//! });
//! *cell.borrow_mut() = Some(Rc::downgrade(&f));
//! ```
//!
//! The closure runs only borrowed from an `Rc` of it, so its own `Rc` has
//! a strong count of one at least while it runs, and the cell, set before
//! anything can call it and never again, holds the `Weak` of it: each
//! upgrade gives that `Rc`, and asks no count. Where nothing else counts
//! it, the closure is the JS function itself, `const f = () => { .. f ..
//! }`: the cell is no JS, its making, its clone, its setting, its drop, and
//! the closure's upgrade is its own name.
//!
//! Read of each body's MIR, as borrowck reads it, the one the body is
//! lowered from, so what's found is what's lowered: the cell's local, all
//! it's made of and all made of it, where each is one of the steps above,
//! nothing between the closure's `Rc` and the cell's setting, and in the
//! closure, the cell captured read only by the chain that upgrades it.

use crate::lower::recognition::{HandleStep, handle_step, weak_pointee};
use crate::lower::{Body, fn_def};
use rustc_middle::mir::{
    self, AggregateKind, BasicBlock, Local, Location, Operand, Place, PlaceElem, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// A closure's handle on itself, of its maker and of the closure.
pub(crate) struct SelfHandle<'tcx> {
    /// The body that makes the closure, its cell and its `Rc`.
    pub(crate) maker: DefId,
    /// The closure.
    pub(crate) closure: DefId,
    /// The maker's local of the closure's `Rc`: what the closure's own
    /// name is.
    pub(crate) handle: Local,
    /// Which of the closure's captures is the cell's clone.
    pub(crate) capture: usize,
    /// What its `Rc` points at, the `Weak`'s: the handle is no JS only
    /// where nothing counts it.
    pointee: Ty<'tcx>,
    /// What the maker writes of the cell: nothing, at each.
    made: HashSet<Location>,
    /// What the closure writes of reading it: nothing, at each.
    read: HashSet<Location>,
    /// Each block whose call gives the closure's own `Rc`, the chain's
    /// `unwrap`, and the local it gives it to: the closure's name.
    pub(crate) own: HashMap<BasicBlock, Local>,
    /// Where each `Weak::upgrade` of the chain is written, which asks no
    /// count.
    upgrades: Vec<Span>,
}

/// Each closure's handle on itself the crate makes.
#[derive(Default)]
pub(crate) struct SelfHandles<'tcx> {
    handles: Vec<SelfHandle<'tcx>>,
}

impl<'tcx> SelfHandles<'tcx> {
    /// What `bodies` make, read of their MIR.
    pub(crate) fn new(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> SelfHandles<'tcx> {
        let of: HashMap<DefId, &mir::Body<'tcx>> = (bodies.iter())
            .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
            .collect();
        let mut handles = Vec::new();
        for (&maker, &body) in &of {
            let cells: Vec<Local> = (body.local_decls.iter_enumerated())
                .filter(|(_, decl)| user_variable(decl) && held_weak(tcx, decl.ty).is_some())
                .map(|(local, _)| local)
                .collect();
            if cells.is_empty() {
                continue;
            }
            let uses = Uses::of(body);
            for cell in cells {
                if let Some(mut handle) = made(tcx, (maker, body, &uses), cell)
                    && let Some(&closure) = of.get(&handle.closure)
                    && let Some((read, own, upgrades)) = read(tcx, closure, handle.capture)
                {
                    handle.read = read;
                    handle.own = own;
                    handle.upgrades = upgrades;
                    handles.push(handle);
                }
            }
        }
        SelfHandles { handles }
    }

    /// Does the `Weak::upgrade` written at `span`, in `body`, ask no count:
    /// one of a handle's chain, which always gives its `Rc` (`counted_rcs`)?
    pub(crate) fn asks_no_count(&self, body: DefId, span: Span) -> bool {
        (self.handles.iter()).any(|h| h.closure == body && h.upgrades.contains(&span))
    }

    /// Keep those `counts` doesn't count, which are no JS; a counted one is
    /// lowered as the program wrote it.
    pub(crate) fn uncounted(&mut self, counts: impl Fn(Ty<'tcx>) -> bool) {
        self.handles.retain(|h| !counts(h.pointee));
    }

    /// The handle `body` makes of `closure`, if any.
    pub(crate) fn made_of(&self, body: DefId, closure: DefId) -> Option<&SelfHandle<'tcx>> {
        (self.handles.iter()).find(|h| h.maker == body && h.closure == closure)
    }

    /// The handle `closure` reads itself by, if any.
    pub(crate) fn read_by(&self, closure: DefId) -> Option<&SelfHandle<'tcx>> {
        self.handles.iter().find(|h| h.closure == closure)
    }

    /// What `body` writes nothing of: its handles' cells' making and
    /// setting, and its reads of its own.
    pub(crate) fn unlowered(&self, body: DefId) -> HashSet<Location> {
        let mut at = HashSet::new();
        for h in &self.handles {
            if h.maker == body {
                at.extend(&h.made);
            }
            if h.closure == body {
                at.extend(&h.read);
            }
        }
        at
    }
}

/// Where each local is assigned whole, and where it's read or written
/// through, of `body`: what says nothing, a storage marker, a fake read or
/// a type's ascription, neither.
struct Uses {
    defs: HashMap<Local, Vec<Location>>,
    uses: HashMap<Local, Vec<Location>>,
}

impl Uses {
    fn of(body: &mir::Body<'_>) -> Uses {
        let mut uses = Uses {
            defs: HashMap::new(),
            uses: HashMap::new(),
        };
        let used = |local: Local, at: Location, uses: &mut Uses| {
            let list = uses.uses.entry(local).or_default();
            if !list.contains(&at) {
                list.push(at);
            }
        };
        for (block, data) in body.basic_blocks.iter_enumerated() {
            for (index, statement) in data.statements.iter().enumerate() {
                let at = Location {
                    block,
                    statement_index: index,
                };
                match &statement.kind {
                    StatementKind::Assign(assign) => {
                        let (place, rvalue) = &**assign;
                        match place.projection.is_empty() {
                            true => uses.defs.entry(place.local).or_default().push(at),
                            false => used(place.local, at, &mut uses),
                        }
                        for local in rvalue_locals(rvalue) {
                            used(local, at, &mut uses);
                        }
                    }
                    kind if inert(kind) => {}
                    _ => {
                        let mut locals = Vec::new();
                        mir::visit::Visitor::visit_statement(&mut Locals(&mut locals), statement, at);
                        for local in locals {
                            used(local, at, &mut uses);
                        }
                    }
                }
            }
            let at = body.terminator_loc(block);
            let terminator = data.terminator();
            if let TerminatorKind::Call { destination, .. } = &terminator.kind
                && destination.projection.is_empty()
            {
                uses.defs.entry(destination.local).or_default().push(at);
            }
            let mut locals = Vec::new();
            mir::visit::Visitor::visit_terminator(&mut Locals(&mut locals), terminator, at);
            for local in locals {
                // A call's destination is its def, not a use.
                if matches!(&terminator.kind, TerminatorKind::Call { destination, .. }
                    if destination.projection.is_empty() && destination.local == local)
                {
                    continue;
                }
                used(local, at, &mut uses);
            }
        }
        uses
    }

    fn uses(&self, local: Local) -> &[Location] {
        self.uses.get(&local).map_or(&[], Vec::as_slice)
    }

    /// Its one def, where it has one.
    fn def(&self, local: Local) -> Option<Location> {
        match self.defs.get(&local).map(Vec::as_slice) {
            Some(&[at]) => Some(at),
            _ => None,
        }
    }
}

/// Each local a statement or a terminator mentions, but storage markers.
struct Locals<'a>(&'a mut Vec<Local>);

impl<'tcx> mir::visit::Visitor<'tcx> for Locals<'_> {
    fn visit_local(&mut self, local: Local, context: mir::visit::PlaceContext, _: Location) {
        if !matches!(context, mir::visit::PlaceContext::NonUse(_)) {
            self.0.push(local);
        }
    }
}

/// What says nothing that runs, of a statement.
fn inert(kind: &StatementKind<'_>) -> bool {
    matches!(
        kind,
        StatementKind::FakeRead(_)
            | StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::PlaceMention(_)
            | StatementKind::AscribeUserType(..)
            | StatementKind::Coverage(_)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop
            | StatementKind::BackwardIncompatibleDropHint { .. }
    )
}

/// Each local `rvalue` reads.
fn rvalue_locals(rvalue: &Rvalue<'_>) -> Vec<Local> {
    let mut locals = Vec::new();
    mir::visit::Visitor::visit_rvalue(&mut Locals(&mut locals), rvalue, Location::START);
    locals
}

/// The local an operand moves or copies whole.
fn whole(operand: &Operand<'_>) -> Option<Local> {
    operand.place().filter(|p| p.projection.is_empty()).map(|p| p.local)
}

/// The call at `at`, a block's terminator: its step, its arguments and its
/// destination, a local whole.
fn call_at<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &'a mir::Body<'tcx>,
    at: Location,
) -> Option<(HandleStep, Vec<&'a Operand<'tcx>>, Local)> {
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
    let (id, generics) = fn_def(func.ty(body, tcx))?;
    if !destination.projection.is_empty() {
        return None;
    }
    let step = handle_step(tcx, id, generics)?;
    Some((step, args.iter().map(|a| &a.node).collect(), destination.local))
}

/// The statement at `at`, an assignment of a local whole.
fn assign_at<'a, 'tcx>(body: &'a mir::Body<'tcx>, at: Location) -> Option<(&'a Place<'tcx>, &'a Rvalue<'tcx>)> {
    let data = &body.basic_blocks[at.block];
    match &data.statements.get(at.statement_index)?.kind {
        StatementKind::Assign(assign) => Some((&assign.0, &assign.1)),
        _ => None,
    }
}

/// Whether the terminator at `at` drops a place of `local`.
fn drops(body: &mir::Body<'_>, at: Location, local: Local) -> bool {
    let data = &body.basic_blocks[at.block];
    at.statement_index == data.statements.len()
        && matches!(&data.terminator().kind, TerminatorKind::Drop { place, .. } if place.local == local)
}

/// The handle `cell`, of `body`, makes, where it's one: everything but
/// what reads it in the closure, which `read` finds.
fn made<'tcx>(
    tcx: TyCtxt<'tcx>,
    (maker, body, uses): (DefId, &mir::Body<'tcx>, &Uses),
    cell: Local,
) -> Option<SelfHandle<'tcx>> {
    // An `Rc<RefCell<Option<Weak<D>>>>`.
    let pointee = held_weak(tcx, body.local_decls[cell].ty)?;
    let mut made = HashSet::new();
    // What it's made of: `Rc::new(RefCell::new(None))`.
    let at = uses.def(cell)?;
    let (HandleStep::RcNew, args, _) = call_at(tcx, body, at)? else {
        return None;
    };
    made.insert(at);
    let inner = whole(args.first()?)?;
    let at = uses.def(inner)?;
    let (HandleStep::CellNew, args, _) = call_at(tcx, body, at)? else {
        return None;
    };
    made.insert(at);
    let none = whole(args.first()?)?;
    let at = uses.def(none)?;
    let (_, Rvalue::Aggregate(kind, fields)) = assign_at(body, at)? else {
        return None;
    };
    let AggregateKind::Adt(adt, variant, ..) = **kind else {
        return None;
    };
    if !tcx.is_lang_item(adt, rustc_hir::attrs::lang_items::LangItem::Option)
        || !fields.is_empty()
        || variant.as_u32() != 0
    {
        return None;
    }
    made.insert(at);
    for local in [inner, none] {
        if !only_made(body, uses, local, &mut made) {
            return None;
        }
    }
    // All made of it, each a step of cloning it for the closure or setting
    // it, until no more is: each local's one def what puts it in.
    let mut slice: Vec<Local> = vec![cell];
    let mut capture: Option<(DefId, usize, Local)> = None;
    let mut written: Vec<(Location, Local)> = Vec::new();
    let mut i = 0;
    while i < slice.len() {
        let local = slice[i];
        i += 1;
        for &at in uses.uses(local) {
            if let Some((place, rvalue)) = assign_at(body, at) {
                // `(*p) = move value`: the cell set.
                if place.local == local && place.projection.as_slice() == [PlaceElem::Deref] {
                    let Rvalue::Use(operand, ..) = rvalue else { return None };
                    written.push((at, whole(operand)?));
                    made.insert(at);
                    continue;
                }
                // The closure made of its clone.
                if let Rvalue::Aggregate(kind, fields) = rvalue
                    && let AggregateKind::Closure(closure, _) = **kind
                {
                    let index = fields.iter().position(|f| whole(f) == Some(local))?;
                    if capture.is_some() || fields.iter().filter(|f| rvalue_reads(f, local)).count() != 1 {
                        return None;
                    }
                    capture = Some((closure, index, place.local));
                    continue;
                }
                if !place.projection.is_empty() {
                    return None;
                }
                made.insert(at);
                if !slice.contains(&place.local) {
                    slice.push(place.local);
                }
                continue;
            }
            if drops(body, at, local) {
                made.insert(at);
                continue;
            }
            let (step, args, destination) = call_at(tcx, body, at)?;
            if !matches!(
                step,
                HandleStep::RcClone | HandleStep::RcDeref | HandleStep::BorrowMut | HandleStep::GuardDerefMut
            ) || args.first().and_then(|a| whole(a)) != Some(local)
                || args.len() != 1
            {
                return None;
            }
            made.insert(at);
            if !slice.contains(&destination) {
                slice.push(destination);
            }
        }
    }
    // Made once, of a closure that captures its clone.
    let (closure, index, made_closure) = capture?;
    // What it's set to, in each place MIR sets it, one value:
    // `Some(Rc::downgrade(&handle))`, of the closure's `Rc`.
    let value = written.first()?.1;
    if written.iter().any(|&(_, v)| v != value) {
        return None;
    }
    let mut set = vec![value];
    let mut handle = None;
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
        let (_, rvalue) = assign_at(body, at)?;
        match rvalue {
            Rvalue::Aggregate(kind, fields)
                if matches!(**kind, AggregateKind::Adt(adt, variant, ..)
                    if tcx.is_lang_item(adt, rustc_hir::attrs::lang_items::LangItem::Option) && variant.as_u32() == 1) =>
            {
                set.push(whole(fields.iter().next()?)?);
            }
            Rvalue::Cast(_, operand, _) => set.push(whole(operand)?),
            // `&handle`, or a reborrow of a reference to it.
            Rvalue::Ref(_, _, place) => match place.projection.as_slice() {
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
    // A variable the program named, which its closure reads by its name.
    let handle = handle.filter(|&h| user_variable(&body.local_decls[h]))?;
    // The handle's `Rc`: `Rc::new` of the closure, made an `Rc<dyn ..>`.
    let mut of = handle;
    // What's moved into it, whose drop after is MIR's of nothing.
    let mut moved = vec![made_closure];
    loop {
        let at = uses.def(of)?;
        if let Some((HandleStep::RcNew, args, _)) = call_at(tcx, body, at) {
            if whole(args.first()?)? != made_closure {
                return None;
            }
            break;
        }
        match assign_at(body, at)? {
            (_, Rvalue::Cast(mir::CastKind::PointerCoercion(..), operand, _)) => {
                of = whole(operand)?;
                moved.push(of);
            }
            _ => return None,
        }
    }
    // Nothing runs between the handle's making and the cell's setting
    // that may call the closure: from the handle's def on, only the steps
    // of setting it, to where it's set.
    let first_set = written
        .iter()
        .map(|&(at, _)| at)
        .find(|at| !body.basic_blocks[at.block].is_cleanup)?;
    if !set_next(body, (uses.def(handle)?, first_set), &made, &moved) {
        return None;
    }
    Some(SelfHandle {
        maker,
        closure,
        handle,
        capture: index,
        pointee,
        made,
        read: HashSet::new(),
        own: HashMap::new(),
        upgrades: Vec::new(),
    })
}

/// Whether each use of `local` is one of `made`, or a drop of it where it
/// unwinds, which then is too.
fn only_made(body: &mir::Body<'_>, uses: &Uses, local: Local, made: &mut HashSet<Location>) -> bool {
    for &at in uses.uses(local) {
        if drops(body, at, local) {
            made.insert(at);
        } else if !made.contains(&at) {
            return false;
        }
    }
    true
}

/// Whether `operand` reads `local`.
fn rvalue_reads(operand: &Operand<'_>, local: Local) -> bool {
    operand.place().is_some_and(|p| p.local == local)
}

/// Whether `decl` is a variable the program named: none of a body whose
/// locals' kinds rustc cleared, a constant's evaluated or another crate's.
fn user_variable(decl: &mir::LocalDecl<'_>) -> bool {
    matches!(decl.local_info, mir::ClearCrossCrate::Set(_)) && decl.is_user_variable()
}

/// `Rc<RefCell<Option<Weak<D>>>>`'s `D`.
fn held_weak<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let cell = crate::lower::recognition::rc_pointee(tcx, ty)?;
    let ty::Adt(adt, args) = *cell.kind() else { return None };
    if !crate::lower::recognition::is_std_def(tcx, adt.did(), crate::lower::recognition::StdItem::RefCell) {
        return None;
    }
    let ty::Adt(option, args) = *args.type_at(0).kind() else {
        return None;
    };
    if !tcx.is_lang_item(option.did(), rustc_hir::attrs::lang_items::LangItem::Option) {
        return None;
    }
    weak_pointee(tcx, args.type_at(0))
}

/// Whether from `from`, the handle's def, MIR goes on to `to`, the cell's
/// setting, by `made`'s steps alone, and what runs nothing: a storage
/// marker, or the drop of what was `moved` into the handle.
fn set_next(body: &mir::Body<'_>, (from, to): (Location, Location), made: &HashSet<Location>, moved: &[Local]) -> bool {
    let (mut block, mut index) = (from.block, from.statement_index + 1);
    // Each block once at most: past that, it's gone round.
    for _ in 0..=body.basic_blocks.len() {
        let data = &body.basic_blocks[block];
        while index < data.statements.len() {
            let at = Location {
                block,
                statement_index: index,
            };
            if at == to {
                return true;
            }
            if !made.contains(&at) && !inert(&data.statements[index].kind) {
                return false;
            }
            index += 1;
        }
        let at = body.terminator_loc(block);
        block = match &data.terminator().kind {
            TerminatorKind::Goto { target } => *target,
            TerminatorKind::Call {
                target: Some(target), ..
            }
            | TerminatorKind::Drop { target, .. }
                if made.contains(&at) =>
            {
                *target
            }
            TerminatorKind::Drop { place, target, .. }
                if place.projection.is_empty() && moved.contains(&place.local) =>
            {
                *target
            }
            _ => return false,
        };
        index = 0;
    }
    false
}

/// What the closure of `body` writes nothing of in reading its cell, its
/// capture `capture`: each read of it, `cell.borrow().as_ref()
/// .and_then(Weak::upgrade).unwrap()`, and each guard's drop; the blocks
/// whose `unwrap` gives its own `Rc`, to the local each gives it to; and
/// where each `Weak::upgrade` is written. None where it reads the cell
/// otherwise.
#[allow(clippy::type_complexity)]
fn read<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    capture: usize,
) -> Option<(HashSet<Location>, HashMap<BasicBlock, Local>, Vec<Span>)> {
    let env = Local::from_usize(1);
    let uses = Uses::of(body);
    let mut read = HashSet::new();
    let mut own = HashMap::new();
    let mut upgrades = Vec::new();
    let mut slice = Vec::new();
    // Each read of the capture: `&(*env).capture`, or `&env.capture` of a
    // closure called once.
    let of_capture = |place: &Place<'_>| match place.projection.as_slice() {
        [PlaceElem::Deref, PlaceElem::Field(field, _), rest @ ..] | [PlaceElem::Field(field, _), rest @ ..] => {
            (field.as_usize() == capture).then_some(rest.is_empty())
        }
        _ => None,
    };
    for &at in uses.uses(env) {
        let mut reads_it = Vec::new();
        if let Some((place, rvalue)) = assign_at(body, at) {
            if place.local == env && of_capture(place).is_some() {
                return None;
            }
            mir::visit::Visitor::visit_rvalue(&mut Places(env, &mut reads_it), rvalue, at);
            if reads_it.iter().any(|p| of_capture(p).is_some()) {
                match rvalue {
                    Rvalue::Ref(_, mir::BorrowKind::Shared, p)
                        if of_capture(p) == Some(true) && place.projection.is_empty() =>
                    {
                        read.insert(at);
                        slice.push(place.local);
                    }
                    _ => return None,
                }
            }
            continue;
        }
        let data = &body.basic_blocks[at.block];
        let mut places = Vec::new();
        match data.statements.get(at.statement_index) {
            Some(statement) => mir::visit::Visitor::visit_statement(&mut Places(env, &mut places), statement, at),
            None => mir::visit::Visitor::visit_terminator(&mut Places(env, &mut places), data.terminator(), at),
        }
        if places.iter().any(|p| of_capture(p).is_some()) {
            return None;
        }
    }
    if slice.is_empty() {
        return None;
    }
    let mut i = 0;
    while i < slice.len() {
        let local = slice[i];
        i += 1;
        uses.def(local)?;
        for &at in uses.uses(local) {
            if let Some((place, _)) = assign_at(body, at) {
                if !place.projection.is_empty() {
                    return None;
                }
                read.insert(at);
                slice.push(place.local);
                continue;
            }
            if drops(body, at, local) {
                read.insert(at);
                continue;
            }
            let (step, args, destination) = call_at(tcx, body, at)?;
            if args.first().and_then(|a| whole(a)) != Some(local) {
                return None;
            }
            match (step, &args[..]) {
                (HandleStep::RcDeref | HandleStep::Borrow | HandleStep::GuardDeref | HandleStep::AsRef, [_]) => {
                    slice.push(destination);
                }
                (HandleStep::AndThen, [_, Operand::Constant(upgrade)])
                    if fn_def(upgrade.ty())
                        .is_some_and(|(id, generics)| handle_step(tcx, id, generics) == Some(HandleStep::Upgrade))
                        && !upgrade.span.from_expansion() =>
                {
                    upgrades.push(upgrade.span);
                    slice.push(destination);
                }
                // Its own `Rc`, to a local of its own, set once.
                (HandleStep::Unwrap, [_]) if destination != mir::RETURN_PLACE && uses.def(destination) == Some(at) => {
                    own.insert(at.block, destination);
                }
                _ => return None,
            }
            read.insert(at);
        }
    }
    (!own.is_empty() && !upgrades.is_empty()).then_some((read, own, upgrades))
}

/// Each place of `local` a statement or a terminator mentions.
struct Places<'a, 'tcx>(Local, &'a mut Vec<Place<'tcx>>);

impl<'tcx> mir::visit::Visitor<'tcx> for Places<'_, 'tcx> {
    fn visit_place(&mut self, place: &Place<'tcx>, _: mir::visit::PlaceContext, _: Location) {
        if place.local == self.0 {
            self.1.push(*place);
        }
    }
}
