//! How a function made a value is called (#2335), which says where one that
//! tracks its caller is called from: rustc calls it through a shim, and the
//! shim's location is the one it's given. Through a `Fn` trait, the trait's
//! method's, `core/src/ops/function.rs`'s `call`, `call_mut` or
//! `call_once`; as a `fn` pointer, the function's own definition's.
//! `.map(Option::unwrap)` panics natively at `FnMut::call_mut`'s line, as
//! `Iterator::map`'s `F: FnMut` calls it so.
//!
//! Found in the body, of each use of the value: a call's argument, by its
//! parameter's bound, the most of `Fn`, `FnMut` and `FnOnce` it has; a cast
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
    function: DefId,
    span: Span,
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
    match joined(ways) {
        Way::Called(way) => Called::Through(way),
        Way::Unused => Called::Never,
        Way::Unknown => Called::Untold,
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
            match held {
                // `Box::new(f)`: the box's uses.
                Held::Value if is_box_new(tcx, callee) && depth < 4 => match destination.as_local() {
                    Some(boxed) => stored_through(tcx, body, (boxed, None), Held::Boxed, depth + 1),
                    None => Way::Unknown,
                },
                Held::Value => known(argument_through(tcx, callee, at)),
                // A reference given as an `F: Fn*`, which `&F`'s impls call
                // through `F::call` whatever `F`'s bound (#2355).
                Held::Reference => known(argument_through(tcx, callee, at).map(|_| Through::Fn)),
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
        let opened = self.held != Held::Value && matches!(rest, Some([ProjectionElem::Deref]));
        match context {
            PlaceContext::NonMutatingUse(Read::Copy | Read::Move) if whole => self.at.push((location, Used::Read)),
            PlaceContext::NonMutatingUse(Read::Copy | Read::Move) if opened => self.at.push((location, Used::Opened)),
            PlaceContext::NonMutatingUse(Read::SharedBorrow) if whole => self.at.push((location, Used::Borrowed)),
            PlaceContext::NonMutatingUse(Read::SharedBorrow) if opened && self.held == Held::Reference => {
                self.at.push((location, Used::Reborrowed))
            }
            // A box's drop, which calls nothing of the value's.
            PlaceContext::MutatingUse(MutatingUseContext::Drop) if whole && self.held == Held::Boxed => {}
            // Its `let`, written, by a statement or a call, `Box::new(f)`'s,
            // and a mention of it, `let _ = f`, that reads nothing.
            PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call)
            | PlaceContext::NonMutatingUse(Read::Inspect)
            | PlaceContext::NonUse(_) => {}
            _ => self.other = true,
        }
    }
}

/// Of a call of `callee`, how its argument `at` is called: by its
/// parameter's bounds (`called_through`).
fn argument_through<'tcx>(tcx: TyCtxt<'tcx>, callee: Ty<'tcx>, at: usize) -> Option<Through> {
    let ty::FnDef(callee, _) = *callee.kind() else {
        return None;
    };
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
