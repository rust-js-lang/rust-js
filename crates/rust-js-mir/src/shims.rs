//! How a function made a value is called (#2335), which says where one that
//! tracks its caller is called from: rustc calls it through a shim, and the
//! shim's location is the one it's given. Through a `Fn` trait, the trait's
//! method's, `core/src/ops/function.rs`'s `call`, `call_mut` or
//! `call_once`; as a `fn` pointer, the function's own definition's.
//! `.map(Option::unwrap)` panics natively at `FnMut::call_mut`'s line, as
//! `Iterator::map`'s `F: FnMut` calls it so.
//!
//! Found in the body, of each use of the value: a call's argument, as the
//! callee's MIR calls its parameter where it says, else by its parameter's
//! bound, the most of `Fn`, `FnMut` and `FnOnce` it has (#2522); a cast
//! to a `fn` pointer; a local or a field the value is put in, by its uses;
//! a shared reference to it, `&f` or a promoted `&Option::unwrap`, called
//! through `Fn::call` as a `&dyn Fn*` or an `F: Fn*` argument, as `&F`'s
//! impls call it; and a box of it, `Box::new(f)`, read through or made a
//! `Box<dyn FnOnce>`. Where its uses disagree, or one is none of these,
//! it's untold, which a binary refuses (`Called::Untold`).

use rust_js_lower::analysis::mir_reads::referred::referred_constant;
use rustc_abi::FieldIdx;
use rustc_data_structures::either::Either;
use rustc_hir::def_id::DefId;
use rustc_index::IndexSlice;
use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext as Read, PlaceContext, Visitor};
use rustc_middle::mir::{
    self, Body, Const, ConstOperand, Local, Location, Operand, ProjectionElem, Promoted, Rvalue, StatementKind,
    TerminatorKind,
};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;

/// How a function value is called.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Through {
    Fn,
    FnMut,
    FnOnce,
    Pointer,
}

impl Through {
    fn of(kind: ty::ClosureKind) -> Through {
        match kind {
            ty::ClosureKind::Fn => Through::Fn,
            ty::ClosureKind::FnMut => Through::FnMut,
            ty::ClosureKind::FnOnce => Through::FnOnce,
        }
    }

    /// What its shim is: a `Fn` trait's method, `call_once` past
    /// `FnOnce`'s `Output`, or the function itself, a `fn` pointer's shim
    /// of it.
    pub(crate) fn shim(self, tcx: TyCtxt<'_>, function: DefId) -> Option<DefId> {
        let kind = match self {
            Through::Fn => ty::ClosureKind::Fn,
            Through::FnMut => ty::ClosureKind::FnMut,
            Through::FnOnce => ty::ClosureKind::FnOnce,
            Through::Pointer => return Some(function),
        };
        let tr = tcx.fn_trait_kind_to_def_id(kind)?;
        tcx.associated_items(tr)
            .in_definition_order()
            .find(|item| item.is_fn())
            .map(|item| item.def_id)
    }
}

/// What the body says of how a function value is called.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Called {
    /// One way, said by every use of it that says one.
    Through(Through),
    /// Never: no use of it reads it, a borrow no one reads, `let _r = &f`
    /// in a branch never taken.
    Never,
    /// Not told: a use none of these rules reads, or two that disagree, a
    /// local used two ways, a value stored in a struct passed whole,
    /// captured, returned. A binary refuses it (#2335, Tu's decision).
    Untold,
}

/// How a use of a function value says it's called: a way, nothing, as a
/// borrow no one reads, or what can't be told.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Way {
    Called(Through),
    Unused,
    Unknown,
    /// Returned by the body it's made in, whose callers say (`returned_through`,
    /// #2530); of a callee's MIR, unknown.
    Returned,
}

/// What an operand holds of the function value: the value; a shared
/// reference to it, `&f` or a promoted `&Option::unwrap`, which every `Fn*`
/// impl of `&F` calls through `F::call`; or a box of it, `Box::new(f)`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Held {
    Value,
    Reference,
    Boxed,
}

/// How `function`, made a value at `span` in `body`, is called: the one way
/// every use of it that says anything says. `promoted` is the body's
/// promoted constants, of which a `&f` MIR promotes is one.
pub(crate) fn through<'tcx>(
    tcx: TyCtxt<'tcx>,
    (body, promoted): (&Body<'tcx>, &IndexSlice<Promoted, Body<'tcx>>),
    (function, span): (DefId, Span),
    (bodies, library): (&[&Body<'tcx>], bool),
) -> Called {
    let mut found = Found {
        value: Value {
            function,
            span,
            promoted,
        },
        at: Vec::new(),
    };
    found.visit_body(body);
    // Made where no use is seen: a use none of these rules reads.
    if found.at.is_empty() {
        return Called::Untold;
    }
    let value = found.value;
    let ways = found
        .at
        .iter()
        .map(|&(at, held)| use_through(tcx, body, at, &mut |op| value.held(op) == Some(held), held, 0));
    let way = match joined(ways) {
        Way::Returned => returned_through(tcx, (bodies, library), body.source.def_id()),
        way => way,
    };
    match way {
        Way::Called(way) => Called::Through(way),
        Way::Unused => Called::Never,
        Way::Unknown | Way::Returned => Called::Untold,
    }
}

/// The one way of those that say one: unused where none says anything,
/// unknown where one can't be told or two disagree.
fn joined(ways: impl Iterator<Item = Way>) -> Way {
    let mut one = Way::Unused;
    for way in ways {
        one = match (one, way) {
            (_, Way::Unknown) => return Way::Unknown,
            (one, Way::Unused) => one,
            (Way::Unused, way) => way,
            (Way::Called(a), Way::Called(b)) if a == b => one,
            (Way::Returned, Way::Returned) => one,
            _ => return Way::Unknown,
        };
    }
    one
}

/// The function value made at `span`, and the body's promoted constants.
#[derive(Clone, Copy)]
struct Value<'a, 'tcx> {
    function: DefId,
    span: Span,
    promoted: &'a IndexSlice<Promoted, Body<'tcx>>,
}

impl<'tcx> Value<'_, 'tcx> {
    /// What `op` holds of the value: it, the constant; or a reference to
    /// it, a promoted constant that refers to it.
    fn held(&self, op: &Operand<'tcx>) -> Option<Held> {
        let Operand::Constant(c) = op else {
            return None;
        };
        if self.is_it(c) {
            return Some(Held::Value);
        }
        let Const::Unevaluated(uv, _) = c.const_ else {
            return None;
        };
        let referred = referred_constant(self.promoted.get(uv.promoted?)?)?;
        self.is_it(&referred).then_some(Held::Reference)
    }

    fn is_it(&self, c: &ConstOperand<'tcx>) -> bool {
        c.span == self.span && matches!(*c.const_.ty().kind(), ty::FnDef(id, _) if id == self.function)
    }
}

/// Where the function's value is an operand, and what the operand holds
/// of it.
struct Found<'a, 'tcx> {
    value: Value<'a, 'tcx>,
    at: Vec<(Location, Held)>,
}

impl<'tcx> Visitor<'tcx> for Found<'_, 'tcx> {
    fn visit_operand(&mut self, op: &Operand<'tcx>, location: Location) {
        if let Some(held) = self.value.held(op) {
            self.at.push((location, held));
        }
    }
}

/// Whether `place` is `local`'s part `part`: whole, or its field.
fn is_place(place: mir::Place<'_>, local: Local, part: &[ProjectionElem<Local, Ty<'_>>]) -> bool {
    place.local == local
        && place.projection.len() == part.len()
        && place.projection.iter().zip(part).all(|(a, b)| match (a, b) {
            (ProjectionElem::Field(a, _), ProjectionElem::Field(b, _)) => a == *b,
            (ProjectionElem::Deref, ProjectionElem::Deref) => true,
            _ => false,
        })
}

/// How the operand `is_it` says, holding `held` of the value, used at `at`,
/// is called: a call's argument, a cast, or put in a local, by the local's
/// uses. `depth` bounds a value put from local to local.
fn use_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    at: Location,
    is_it: &mut dyn FnMut(&Operand<'tcx>) -> bool,
    held: Held,
    depth: usize,
) -> Way {
    let known = |way: Option<Through>| way.map_or(Way::Unknown, Way::Called);
    match body.stmt_at(at) {
        Either::Left(statement) => {
            let StatementKind::Assign(assign) = &statement.kind else {
                return Way::Unknown;
            };
            let (place, rvalue) = &**assign;
            match rvalue {
                Rvalue::Cast(mir::CastKind::PointerCoercion(PointerCoercion::ReifyFnPointer(_), _), op, _)
                    if is_it(op) && held == Held::Value =>
                {
                    Way::Called(Through::Pointer)
                }
                Rvalue::Cast(mir::CastKind::PointerCoercion(PointerCoercion::Unsize, _), op, target) if is_it(op) => {
                    known(dyn_through(tcx, held, *target))
                }
                Rvalue::Use(op, _) if is_it(op) && depth < 4 => match place.as_local() {
                    Some(local) => stored_through(tcx, body, (local, None), held, depth + 1),
                    None => Way::Unknown,
                },
                // Stored in a tuple's or a struct's field: that field's uses.
                Rvalue::Aggregate(kind, fields)
                    if depth < 4 && matches!(**kind, mir::AggregateKind::Tuple | mir::AggregateKind::Adt(..)) =>
                {
                    match (fields.iter_enumerated().find(|(_, op)| is_it(op)), place.as_local()) {
                        (Some((field, _)), Some(local)) => {
                            stored_through(tcx, body, (local, Some(field)), held, depth + 1)
                        }
                        _ => Way::Unknown,
                    }
                }
                // Captured by a closure std's MIR made in place, `map_fold`'s
                // of `Cloned::fold`: as the closure's body calls that capture
                // (#2522).
                Rvalue::Aggregate(kind, fields) if held == Held::Value => match **kind {
                    mir::AggregateKind::Closure(closure, _) => match fields.iter_enumerated().find(|(_, op)| is_it(op))
                    {
                        Some((field, _)) => known(capture_through(tcx, closure, field)),
                        None => Way::Unknown,
                    },
                    _ => Way::Unknown,
                },
                _ => Way::Unknown,
            }
        }
        Either::Right(terminator) => {
            let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &terminator.kind
            else {
                return Way::Unknown;
            };
            // Called itself, `f(x)` of a local `f`: a call of the function,
            // as written, the call's own location, and through no shim.
            if held == Held::Value && is_it(func) && !args.iter().any(|arg| is_it(&arg.node)) {
                return Way::Unused;
            }
            let Some(at) = args.iter().position(|arg| is_it(&arg.node)) else {
                return Way::Unknown;
            };
            let callee = func.ty(body, tcx);
            // A `Fn` trait's own method called of it, `Fn::call(&f, ..)` of
            // what a call returned, by that trait; of a reference, `F::call`,
            // as `&F`'s impls call it (#2530).
            if at == 0
                && let ty::FnDef(called, _) = *callee.kind()
                && let Some(kind) = tcx
                    .trait_of_assoc(called)
                    .and_then(|tr| tcx.fn_trait_kind_from_def_id(tr))
            {
                return match held {
                    Held::Value => Way::Called(Through::of(kind)),
                    Held::Reference => Way::Called(Through::Fn),
                    Held::Boxed => Way::Unknown,
                };
            }
            match held {
                // `Box::new(f)`: the box's uses.
                Held::Value if is_box_new(tcx, callee) && depth < 4 => match destination.as_local() {
                    Some(boxed) => stored_through(tcx, body, (boxed, None), Held::Boxed, depth + 1),
                    None => Way::Unknown,
                },
                Held::Value => known(argument_through(tcx, callee, at, depth)),
                // A reference given as an `F: Fn*`, which `&F`'s impls call
                // through `F::call` whatever `F`'s bound (#2355).
                Held::Reference => known(argument_through(tcx, callee, at, depth).map(|_| Through::Fn)),
                Held::Boxed => Way::Unknown,
            }
        }
    }
}

/// Whether `callee` is `Box::new`.
fn is_box_new<'tcx>(tcx: TyCtxt<'tcx>, callee: Ty<'tcx>) -> bool {
    use rust_js_lower::recognition::{StdItem, is_std_def};
    matches!(*callee.kind(), ty::FnDef(id, _) if is_std_def(tcx, id, StdItem::BoxNew))
}

/// How the value held in `local`, or in its field `field`, as `held` says,
/// is called, by each use of it: moved or copied whole; of the value,
/// borrowed whole, a reference to it; of a box, read through, `*b`, the
/// value; unused where it has none, a borrow in a branch never taken,
/// `let _r = &f`. Returned, it's called where this body doesn't say.
fn stored_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    (local, field): (Local, Option<FieldIdx>),
    held: Held,
    depth: usize,
) -> Way {
    if local == mir::RETURN_PLACE && held == Held::Value && field.is_none() {
        return Way::Returned;
    }
    if local == mir::RETURN_PLACE || depth > 4 {
        return Way::Unknown;
    }
    let part: Vec<ProjectionElem<Local, Ty<'tcx>>> = match field {
        Some(field) => vec![ProjectionElem::Field(field, tcx.types.unit)],
        None => Vec::new(),
    };
    let mut uses = Uses {
        local,
        field,
        held,
        at: Vec::new(),
        other: false,
    };
    uses.visit_body(body);
    if uses.other {
        return Way::Unknown;
    }
    joined(uses.at.into_iter().map(|(at, used)| match (used, held) {
        // `&f`, put in a local: a reference to the value, by that local's uses.
        (Used::Borrowed, Held::Value) => match body.stmt_at(at).left().map(|statement| &statement.kind) {
            Some(StatementKind::Assign(assign)) => match assign.0.as_local() {
                Some(reference) => stored_through(tcx, body, (reference, None), Held::Reference, depth + 1),
                None => Way::Unknown,
            },
            _ => Way::Unknown,
        },
        // `&*r`, a reference reborrowed: the reference's, by that local's uses.
        (Used::Reborrowed, Held::Reference) => match body.stmt_at(at).left().map(|statement| &statement.kind) {
            Some(StatementKind::Assign(assign)) => match assign.0.as_local() {
                Some(reference) => stored_through(tcx, body, (reference, None), Held::Reference, depth + 1),
                None => Way::Unknown,
            },
            _ => Way::Unknown,
        },
        (Used::Borrowed, Held::Reference | Held::Boxed) | (Used::Reborrowed, _) => Way::Unknown,
        // What holds it, given whole to a call: as the callee's MIR uses
        // that field of its parameter, which no bound says (#2522).
        (Used::HolderMoved, held) => match (body.stmt_at(at).right().map(|terminator| &terminator.kind), field) {
            (Some(TerminatorKind::Call { func, args, .. }), Some(field)) if depth < 4 => {
                let Some(at) = args
                    .iter()
                    .position(|arg| arg.node.place().is_some_and(|p| p.as_local() == Some(local)))
                else {
                    return Way::Unknown;
                };
                match *func.ty(body, tcx).kind() {
                    ty::FnDef(callee, _) if tcx.is_mir_available(callee) => {
                        let callee = tcx.optimized_mir(callee);
                        let param = Local::from_usize(at + 1);
                        if param.as_usize() > callee.arg_count {
                            return Way::Unknown;
                        }
                        not_returned(stored_through(tcx, callee, (param, Some(field)), held, depth + 1))
                    }
                    _ => Way::Unknown,
                }
            }
            _ => Way::Unknown,
        },
        (Used::Read, held) => use_through(
            tcx,
            body,
            at,
            &mut |op| op.place().is_some_and(|p| is_place(p, local, &part)),
            held,
            depth,
        ),
        // `*b`, of a box or a reference: the value.
        (Used::Opened, _) => {
            let opened: Vec<ProjectionElem<Local, Ty<'tcx>>> =
                part.iter().copied().chain([ProjectionElem::Deref]).collect();
            use_through(
                tcx,
                body,
                at,
                &mut |op| op.place().is_some_and(|p| is_place(p, local, &opened)),
                Held::Value,
                depth,
            )
        }
    }))
}

/// How a local holding the value is used: read whole, borrowed whole, or,
/// a box of it or a reference to it, read through, `*b`; a reference,
/// reborrowed, `&*r`.
#[derive(Clone, Copy)]
enum Used {
    Read,
    Borrowed,
    Opened,
    Reborrowed,
    /// The whole of what holds it in a field, moved: a call's argument,
    /// `Map::fold(map, ..)` of a `Map` std's MIR made in place, which the
    /// callee's MIR reads of that field (#2522).
    HolderMoved,
}

/// Each use of a local, whole, or of its field `field`, a value stored in a
/// tuple or a struct: where, and how. Any other, a part of it, a write, or
/// the whole of one that holds the value in a field, which takes it
/// elsewhere, makes `other`; another field's use is none of the value's.
/// Of a box, a read through it, and its drop, are its too; of a reference,
/// a read through it and a reborrow.
struct Uses {
    local: Local,
    field: Option<FieldIdx>,
    held: Held,
    at: Vec<(Location, Used)>,
    other: bool,
}

impl<'tcx> Visitor<'tcx> for Uses {
    fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, location: Location) {
        if place.local != self.local {
            return;
        }
        // Past the field, if any: nothing, the part held; `*`, through a box.
        let rest = match (self.field, place.projection.as_slice()) {
            (None, rest) => Some(rest),
            (Some(field), [ProjectionElem::Field(f, _), rest @ ..]) if *f == field => Some(rest),
            (Some(_), [ProjectionElem::Field(..), ..]) => return,
            (Some(_), _) => None,
        };
        let whole = rest.is_some_and(|rest| rest.is_empty());
        if self.field.is_some()
            && place.projection.is_empty()
            && matches!(context, PlaceContext::NonMutatingUse(Read::Move))
        {
            self.at.push((location, Used::HolderMoved));
            return;
        }
        let opened = self.held != Held::Value && matches!(rest, Some([ProjectionElem::Deref]));
        match context {
            PlaceContext::NonMutatingUse(Read::Copy | Read::Move) if whole => self.at.push((location, Used::Read)),
            PlaceContext::NonMutatingUse(Read::Copy | Read::Move) if opened => self.at.push((location, Used::Opened)),
            PlaceContext::NonMutatingUse(Read::SharedBorrow) if whole => self.at.push((location, Used::Borrowed)),
            PlaceContext::NonMutatingUse(Read::SharedBorrow) if opened && self.held == Held::Reference => {
                self.at.push((location, Used::Reborrowed))
            }
            // Its drop, or a box's, which calls nothing of the value's: a
            // callee's own unwind of a parameter it moves on, `f` of
            // `x.map(f)` (#2522).
            PlaceContext::MutatingUse(MutatingUseContext::Drop)
                if whole && matches!(self.held, Held::Boxed | Held::Value) => {}
            // Its `let`, written, by a statement or a call, `Box::new(f)`'s,
            // and a mention of it, `let _ = f`, that reads nothing.
            PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call)
            | PlaceContext::NonMutatingUse(Read::Inspect)
            | PlaceContext::NonUse(_) => {}
            _ => self.other = true,
        }
    }
}

/// Of a call of `callee`, how its argument `at` is called: as the
/// callee's own MIR calls its parameter, where it has MIR that says, an
/// `F: FnMut` moved into `Option::map`, which calls it through
/// `FnOnce::call_once`, natively `function.rs:250:5` (#2522); else by its
/// parameter's bounds (`called_through`), the most it allows, of one that
/// stores it, `Iterator::map`'s in its `Map`, or of a `Fn` trait's own
/// method, which has none.
fn argument_through<'tcx>(tcx: TyCtxt<'tcx>, callee: Ty<'tcx>, at: usize, depth: usize) -> Option<Through> {
    let ty::FnDef(callee, _) = *callee.kind() else {
        return None;
    };
    if depth < 4 && tcx.is_mir_available(callee) {
        let body = tcx.optimized_mir(callee);
        let param = Local::from_usize(at + 1);
        if param.as_usize() <= body.arg_count
            && let Way::Called(way) = not_returned(stored_through(tcx, body, (param, None), Held::Value, depth + 1))
        {
            return Some(way);
        }
    }
    rust_js_lower::generic_facts::called_through(tcx, callee, at).map(Through::of)
}

/// The way an unsize names, where it names one whatever comes after, as
/// native Rust's measured: a `&dyn Fn*` of a reference is called through
/// `Fn::call`, as `&F`'s impls call it; a `Box<dyn FnOnce>` of a box,
/// through `FnOnce::call_once`, its one impl. A `Box<dyn Fn>` or a
/// `Box<dyn FnMut>` is called as its own calls say, `F: FnMut`'s
/// `call_mut`, and a `&mut dyn` as its call is made: none here.
fn dyn_through<'tcx>(tcx: TyCtxt<'tcx>, held: Held, target: Ty<'tcx>) -> Option<Through> {
    let pointee = match held {
        Held::Reference => target
            .builtin_deref(true)
            .filter(|_| matches!(target.kind(), ty::Ref(_, _, ty::Mutability::Not)))?,
        Held::Boxed => target.boxed_ty()?,
        Held::Value => return None,
    };
    let ty::Dynamic(predicates, ..) = pointee.kind() else {
        return None;
    };
    let kind = tcx.fn_trait_kind_from_def_id(predicates.principal_def_id()?)?;
    match (held, kind) {
        (Held::Reference, _) => Some(Through::Fn),
        (Held::Boxed, ty::ClosureKind::FnOnce) => Some(Through::FnOnce),
        _ => None,
    }
}

/// How `closure`'s body calls its capture `field`, a function value: each
/// call of a `Fn` trait's method whose receiver is that capture, through a
/// borrow of it or itself, `<F as FnMut>::call_mut(&mut (*self).1, ..)`,
/// by that trait, or `&mut F`'s and `&F`'s impls', which call `F`'s
/// `call_mut` and `call`. `None` where it's used otherwise, or two ways.
fn capture_through(tcx: TyCtxt<'_>, closure: DefId, field: FieldIdx) -> Option<Through> {
    if !tcx.is_mir_available(closure) {
        return None;
    }
    let body = tcx.optimized_mir(closure);
    let self_local = Local::from_usize(1);
    let is_capture = |place: mir::Place<'_>| {
        place.local == self_local
            && matches!(place.projection.as_slice(),
                [ProjectionElem::Field(f, _)] | [ProjectionElem::Deref, ProjectionElem::Field(f, _)] if *f == field)
    };
    // The locals a borrow or a move of the capture is put in.
    let mut holders: Vec<Local> = Vec::new();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let held = match &assign.1 {
                Rvalue::Ref(_, _, place) | Rvalue::Use(Operand::Move(place) | Operand::Copy(place), _) => {
                    is_capture(*place)
                }
                _ => false,
            };
            if held && let Some(local) = assign.0.as_local() {
                holders.push(local);
            }
        }
    }
    let mut ways: Vec<Through> = Vec::new();
    for data in body.basic_blocks.iter() {
        let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
            continue;
        };
        let Some(receiver) = args.first().and_then(|arg| arg.node.place()) else {
            continue;
        };
        if !(is_capture(receiver) || receiver.as_local().is_some_and(|local| holders.contains(&local))) {
            continue;
        }
        let ty::FnDef(called, called_args) = *func.ty(body, tcx).kind() else {
            return None;
        };
        let kind = tcx
            .trait_of_assoc(called)
            .and_then(|tr| tcx.fn_trait_kind_from_def_id(tr))?;
        ways.push(match *called_args.skip_binder().type_at(0).kind() {
            ty::Ref(_, _, ty::Mutability::Mut) => Through::FnMut,
            ty::Ref(_, _, ty::Mutability::Not) => Through::Fn,
            _ => Through::of(kind),
        });
    }
    match ways.as_slice() {
        [first, rest @ ..] if rest.iter().all(|way| way == first) => Some(*first),
        _ => None,
    }
}

/// Of a callee's own MIR, what it returns isn't the body being lowered's
/// return: unknown.
fn not_returned(way: Way) -> Way {
    match way {
        Way::Returned => Way::Unknown,
        way => way,
    }
}

/// How a function value `owner` returns is called: as each of the
/// crate's calls of `owner`, in `bodies`, the MIR the emitter reads, uses
/// what it returns, where every mention of `owner` is such a call (#2530).
/// Never of one no call names, which nothing calls. Unknown of a closure,
/// and of one a caller rust-js
/// doesn't compile may call: JS's or, of a library, another crate's
/// (`library::seen_outside`, #2015, #2403), as #2335's rule has it.
fn returned_through<'tcx>(tcx: TyCtxt<'tcx>, (bodies, library): (&[&Body<'tcx>], bool), owner: DefId) -> Way {
    if !owner.is_local()
        || !matches!(
            tcx.def_kind(owner),
            rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
        )
        || rust_js_lower::library::seen_outside(tcx, library, owner)
    {
        return Way::Unknown;
    }
    let names = |ty: Ty<'tcx>| matches!(*ty.peel_refs().kind(), ty::FnDef(id, _) if id == owner);
    let mut ways: Vec<Way> = Vec::new();
    for &caller in bodies {
        let mut mentions = Mentions {
            names: &names,
            at: Vec::new(),
        };
        mentions.visit_body(caller);
        for at in mentions.at {
            let way = match caller.stmt_at(at).right().map(|terminator| &terminator.kind) {
                Some(TerminatorKind::Call {
                    func: Operand::Constant(c),
                    destination,
                    ..
                }) if names(c.ty()) => match destination.as_local() {
                    Some(made) => not_returned(stored_through(tcx, caller, (made, None), Held::Value, 1)),
                    None => Way::Unknown,
                },
                _ => Way::Unknown,
            };
            ways.push(way);
        }
    }
    joined(ways.into_iter())
}

/// Each place a body names a function `names` says, as a constant.
struct Mentions<'a, 'tcx> {
    names: &'a dyn Fn(Ty<'tcx>) -> bool,
    at: Vec<Location>,
}

impl<'tcx> Visitor<'tcx> for Mentions<'_, 'tcx> {
    fn visit_const_operand(&mut self, constant: &ConstOperand<'tcx>, location: Location) {
        if (self.names)(constant.ty()) {
            self.at.push(location);
        }
    }
}
