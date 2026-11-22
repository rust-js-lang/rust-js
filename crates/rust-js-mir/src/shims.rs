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
//! to a `fn` pointer; a `dyn` of one of them, a reference's or a box's; and
//! a local the value is put in, by the uses of the local. Where its uses
//! disagree, or one is none of these, it's not found, and said so.

use rustc_abi::FieldIdx;
use rustc_data_structures::either::Either;
use rustc_hir::def_id::DefId;
use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext as Read, PlaceContext, Visitor};
use rustc_middle::mir::{self, Body, Local, Location, Operand, ProjectionElem, Rvalue, StatementKind, TerminatorKind};
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

/// How a use of a function value says it's called: a way, nothing, as a
/// borrow no one reads, or what can't be told.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Way {
    Called(Through),
    Unused,
    Unknown,
}

/// How `function`, made a value at `span` in `body`, is called, where every
/// use of it that says anything says one way.
pub(crate) fn through<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>, function: DefId, span: Span) -> Option<Through> {
    let mut found = Found {
        function,
        span,
        at: Vec::new(),
    };
    found.visit_body(body);
    let ways = found
        .at
        .iter()
        .map(|&at| use_through(tcx, body, at, &mut |op| is_value(op, function, span), 0));
    match joined(ways) {
        Way::Called(way) => Some(way),
        Way::Unused | Way::Unknown => None,
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

fn is_value(op: &Operand<'_>, function: DefId, span: Span) -> bool {
    matches!(op, Operand::Constant(c) if c.span == span && matches!(*c.const_.ty().kind(), ty::FnDef(id, _) if id == function))
}

/// Where the function's value is an operand.
struct Found {
    function: DefId,
    span: Span,
    at: Vec<Location>,
}

impl<'tcx> Visitor<'tcx> for Found {
    fn visit_operand(&mut self, op: &Operand<'tcx>, location: Location) {
        if is_value(op, self.function, self.span) {
            self.at.push(location);
        }
    }
}

/// Whether `place` is `local` whole, or its field `field`.
fn is_place(place: mir::Place<'_>, local: Local, field: Option<FieldIdx>) -> bool {
    place.local == local
        && match (field, place.projection.as_slice()) {
            (None, []) => true,
            (Some(field), [ProjectionElem::Field(f, _)]) => *f == field,
            _ => false,
        }
}

/// How the operand `is_it` says, used at `at`, is called: a call's argument,
/// a cast, or put in a local, by the local's uses. `depth` bounds a value
/// put from local to local.
fn use_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    at: Location,
    is_it: &mut dyn FnMut(&Operand<'tcx>) -> bool,
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
                    if is_it(op) =>
                {
                    Way::Called(Through::Pointer)
                }
                Rvalue::Cast(mir::CastKind::PointerCoercion(PointerCoercion::Unsize, _), op, target) if is_it(op) => {
                    known(dyn_through(tcx, *target))
                }
                Rvalue::Use(op, _) if is_it(op) && depth < 4 => match place.as_local() {
                    Some(local) => local_through(tcx, body, local, depth + 1),
                    None => Way::Unknown,
                },
                // Stored in a tuple's or a struct's field: that field's uses.
                Rvalue::Aggregate(kind, fields)
                    if depth < 4 && matches!(**kind, mir::AggregateKind::Tuple | mir::AggregateKind::Adt(..)) =>
                {
                    match (fields.iter_enumerated().find(|(_, op)| is_it(op)), place.as_local()) {
                        (Some((field, _)), Some(local)) => stored_through(tcx, body, (local, Some(field)), depth + 1),
                        _ => Way::Unknown,
                    }
                }
                _ => Way::Unknown,
            }
        }
        Either::Right(terminator) => {
            let TerminatorKind::Call { func, args, .. } = &terminator.kind else {
                return Way::Unknown;
            };
            match args.iter().position(|arg| is_it(&arg.node)) {
                Some(at) => known(argument_through(tcx, func.ty(body, tcx), at)),
                None => Way::Unknown,
            }
        }
    }
}

/// How the value in `local` is called, by each of its uses: moved or copied
/// whole, or borrowed whole, a reference to a `dyn` of it; unused where it
/// has none, a borrow in a branch never taken, `let _r = &f`.
fn local_through<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>, local: Local, depth: usize) -> Way {
    stored_through(tcx, body, (local, None), depth)
}

/// How the value in `local`, or in its field `field`, is called, by each
/// use of it: `let f = (Option::unwrap,); v.map(f.0)`, as of a local.
fn stored_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    (local, field): (Local, Option<FieldIdx>),
    depth: usize,
) -> Way {
    let mut uses = Uses {
        local,
        field,
        at: Vec::new(),
        other: false,
    };
    uses.visit_body(body);
    if uses.other || depth > 4 {
        return Way::Unknown;
    }
    joined(uses.at.into_iter().map(|(at, borrowed)| match borrowed {
        // `&f`, put in a local and made a `dyn`: the reference's local's.
        true => match body.stmt_at(at).left().map(|statement| &statement.kind) {
            Some(StatementKind::Assign(assign)) => match assign.0.as_local() {
                Some(reference) => local_through(tcx, body, reference, depth + 1),
                None => Way::Unknown,
            },
            _ => Way::Unknown,
        },
        false => use_through(
            tcx,
            body,
            at,
            &mut |op| op.place().is_some_and(|p| is_place(p, local, field)),
            depth,
        ),
    }))
}

/// Each use of a local, whole, or of its field `field`, a value stored in a
/// tuple or a struct: where, and whether it's borrowed there. Any other, a
/// part of it, a write, or the whole of one that holds the value in a
/// field, which takes it elsewhere, makes `other`; another field's use is
/// none of the value's.
struct Uses {
    local: Local,
    field: Option<FieldIdx>,
    at: Vec<(Location, bool)>,
    other: bool,
}

impl<'tcx> Visitor<'tcx> for Uses {
    fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, location: Location) {
        if place.local != self.local {
            return;
        }
        let ours = match (self.field, place.projection.as_slice()) {
            (None, []) => true,
            (Some(field), [ProjectionElem::Field(f, _)]) if *f == field => true,
            (Some(field), [ProjectionElem::Field(f, _), ..]) if *f != field => return,
            _ => false,
        };
        match context {
            PlaceContext::NonMutatingUse(Read::Copy | Read::Move) if ours => self.at.push((location, false)),
            PlaceContext::NonMutatingUse(Read::SharedBorrow) if ours => self.at.push((location, true)),
            // Its `let`, written, and a mention of it, `let _ = f`, that reads nothing.
            PlaceContext::MutatingUse(MutatingUseContext::Store)
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

/// A `dyn Fn`'s, a reference's or a box's of one: its trait's.
fn dyn_through<'tcx>(tcx: TyCtxt<'tcx>, target: Ty<'tcx>) -> Option<Through> {
    let pointee = target.builtin_deref(true).or_else(|| target.boxed_ty())?;
    let ty::Dynamic(predicates, ..) = pointee.kind() else {
        return None;
    };
    tcx.fn_trait_kind_from_def_id(predicates.principal_def_id()?)
        .map(Through::of)
}
