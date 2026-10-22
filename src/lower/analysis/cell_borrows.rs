//! What each of the crate's bodies may borrow, of its cells (ADR 0580): a
//! `RefCell`, a `Mutex` or an `RwLock`, by the thread-local it's held in
//! or by its type, or anything, of code rust-js can't see. A borrow asks
//! whether its cell is borrowed only of one it may be, so a guard held
//! while the crate's own functions run is checked and not counted where
//! none of them borrows its cell (ADR 0328): `TIMER_QUEUE`'s while
//! `TASK_QUEUE`'s is taken, or the heap's `push` runs.

use crate::lower::Body;
use crate::lower::bindings::{JsForm, is_binding, js_form};
use crate::lower::body_queries::strip;
use crate::lower::fn_def;
use crate::lower::recognition::{Recognition, Std, StdItem};
use rustc_middle::thir::{ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// A cell a borrow is of: a thread-local's, by its key, or any of a type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(in crate::lower) enum Cell<'tcx> {
    Key(DefId),
    Type(Ty<'tcx>),
}

/// A borrow of a cell: which, and whether mutably. A shared borrow beside
/// a shared one asks only whether it's mutably borrowed, which neither
/// changes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(in crate::lower) struct Borrow<'tcx> {
    pub(in crate::lower) cell: Cell<'tcx>,
    pub(in crate::lower) mutable: bool,
}

/// What running a body may borrow: its cells, and the values of its type
/// parameters' it may drop, whose destructors are its caller's to say.
#[derive(Clone, PartialEq, Default, Debug)]
struct Summary<'tcx> {
    cells: HashSet<Borrow<'tcx>>,
    drops: HashSet<Ty<'tcx>>,
}

/// What each body of the crate may borrow, its calls' and its closures'
/// with it; `None`, anything.
#[derive(Default)]
pub(in crate::lower) struct CellBorrows<'tcx> {
    of: HashMap<DefId, Option<Summary<'tcx>>>,
}

/// What a body does itself: what it borrows and drops, the bodies of the
/// crate's it runs, of their generic arguments, and whether it may run what
/// rust-js can't see.
#[derive(Default)]
struct Direct<'tcx> {
    own: Summary<'tcx>,
    runs: Vec<(DefId, Option<ty::GenericArgsRef<'tcx>>)>,
    any: bool,
}

impl<'tcx> CellBorrows<'tcx> {
    /// Each body's, until what each runs holds; `drops` says whether
    /// dropping a type runs a destructor (ADR 0320).
    pub(in crate::lower) fn of_crate(
        tcx: TyCtxt<'tcx>,
        recognition: &Recognition<'_, 'tcx>,
        bodies: &[&Body<'tcx>],
        drops: &dyn Fn(Ty<'tcx>) -> bool,
    ) -> Self {
        let direct: HashMap<DefId, Direct<'tcx>> = (bodies.iter())
            .filter(|body| body.root().is_some())
            .map(|body| {
                let recognition = Recognition {
                    typing_env: ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(body.def_id)),
                    ..*recognition
                };
                (body.def_id, direct(&recognition, &body.thir, drops))
            })
            .collect();
        let mut of: HashMap<DefId, Option<Summary<'tcx>>> = (direct.iter())
            .map(|(&id, d)| (id, (!d.any).then(|| d.own.clone())))
            .collect();
        loop {
            let mut changed = false;
            for (&id, d) in &direct {
                let Some(mut next) = of[&id].clone() else { continue };
                let any = (d.runs.iter()).any(|&(run, args)| match of.get(&run) {
                    Some(Some(theirs)) => match instantiated(tcx, theirs, args, drops) {
                        Some(theirs) => {
                            next.cells.extend(theirs.cells);
                            next.drops.extend(theirs.drops);
                            false
                        }
                        None => true,
                    },
                    // What isn't the crate's own, or may borrow anything.
                    _ => true,
                });
                let next = (!any).then_some(next);
                if next != of[&id] {
                    of.insert(id, next);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        CellBorrows { of }
    }

    /// Whether running `body`, a closure's, may ask of `held`, a borrow.
    pub(in crate::lower) fn body_asks(&self, tcx: TyCtxt<'tcx>, body: DefId, held: Borrow<'tcx>) -> bool {
        match self.of.get(&body) {
            Some(Some(summary)) => summary.asks(tcx, held),
            _ => true,
        }
    }

    /// Whether calling `def_id`, of `args`, may ask of `held`: what a body of
    /// the crate's it runs may, or std's borrow of a cell, its receiver;
    /// anything else, which may run code of the crate's, may. `drops` says
    /// whether dropping a type runs a destructor.
    pub(in crate::lower) fn call_asks(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
        receiver: Option<(&Thir<'tcx>, ExprId)>,
        held: Borrow<'tcx>,
        drops: &dyn Fn(Ty<'tcx>) -> bool,
    ) -> bool {
        let tcx = recognition.tcx;
        match called(recognition, def_id, args, receiver) {
            Called::Nothing => false,
            Called::Cell(theirs) => asks(tcx, theirs, held),
            Called::Body(body, args) => match self.of.get(&body) {
                Some(Some(theirs)) => {
                    instantiated(tcx, theirs, Some(args), drops).is_none_or(|theirs| theirs.asks(tcx, held))
                }
                _ => true,
            },
            Called::Any => true,
        }
    }
}

impl<'tcx> Summary<'tcx> {
    /// Whether it may ask of `held`: one of its borrows may, or a drop its
    /// caller says may run anything.
    fn asks(&self, tcx: TyCtxt<'tcx>, held: Borrow<'tcx>) -> bool {
        !self.drops.is_empty() || self.cells.iter().any(|&theirs| asks(tcx, theirs, held))
    }
}

/// A body's summary as a call of `args` runs it: its type parameters'
/// cells and drops of the call's types; `None` where a drop known now runs
/// a destructor, which may run anything.
fn instantiated<'tcx>(
    tcx: TyCtxt<'tcx>,
    summary: &Summary<'tcx>,
    args: Option<ty::GenericArgsRef<'tcx>>,
    drops: &dyn Fn(Ty<'tcx>) -> bool,
) -> Option<Summary<'tcx>> {
    let of = |ty: Ty<'tcx>| match args {
        Some(args) if ty.has_param() => tcx.erase_and_anonymize_regions(
            ty::EarlyBinder::bind(tcx, ty)
                .instantiate(tcx, args)
                .skip_normalization(),
        ),
        _ => ty,
    };
    let cells = (summary.cells.iter())
        .map(|&borrow| Borrow {
            cell: match borrow.cell {
                Cell::Type(ty) => Cell::Type(of(ty)),
                key => key,
            },
            ..borrow
        })
        .collect();
    let mut kept = HashSet::new();
    for &ty in &summary.drops {
        let ty = of(ty);
        match ty.has_param() {
            true => _ = kept.insert(ty),
            false if drops(ty) => return None,
            false => {}
        }
    }
    Some(Summary { cells, drops: kept })
}

/// The borrow `call`, a borrow's call, is: `c.borrow_mut()`'s of `c`, a
/// thread-local's by its key.
pub(in crate::lower) fn borrowed<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    thir: &Thir<'tcx>,
    call: ExprId,
) -> Option<Borrow<'tcx>> {
    let ExprKind::Call { fun, ref args, .. } = thir[call].kind else {
        return None;
    };
    let (id, generic_args) = fn_def(thir[fun].ty)?;
    match called(recognition, id, generic_args, args.first().map(|&a| (thir, a))) {
        Called::Cell(cell) => Some(cell),
        _ => None,
    }
}

/// What a call runs, as it borrows a cell.
enum Called<'tcx> {
    /// Nothing that borrows: std's, a binding that makes a value or reads
    /// one.
    Nothing,
    /// A borrow of a cell, std's.
    Cell(Borrow<'tcx>),
    /// A body of the crate's, of these generic arguments.
    Body(DefId, ty::GenericArgsRef<'tcx>),
    /// What rust-js can't see.
    Any,
}

/// Whether a binding runs no code of the crate's: what makes a JSX element
/// or an object, or casts a value, a component given called when React
/// renders it; and a read of a property, `node.sortIndex`, `in` or
/// `instanceof`, but by a JS getter that calls back into Rust.
fn binding_runs_nothing(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_binding(tcx, id)
        && matches!(
            js_form(tcx, id),
            JsForm::Jsx(_)
                | JsForm::Prop(_)
                | JsForm::Object(_)
                | JsForm::This
                | JsForm::Get(_)
                | JsForm::GetIndex
                | JsForm::In
                | JsForm::InstanceOf(_)
                | JsForm::Truthy
        )
}

/// What calling `def_id`, of `args`, runs; `receiver` its first argument,
/// the cell a borrow is of.
fn called<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    receiver: Option<(&Thir<'tcx>, ExprId)>,
) -> Called<'tcx> {
    let tcx = recognition.tcx;
    // A trait's binding, of a type parameter its bound reads (ADR 0307).
    if !tcx.is_closure_like(def_id) && binding_runs_nothing(tcx, def_id) {
        return Called::Nothing;
    }
    let Ok(Some(instance)) = crate::lower::items::resolve_instance_in(tcx, recognition.typing_env, def_id, args) else {
        return Called::Any;
    };
    let ty::InstanceKind::Item(id) = instance.def else {
        return Called::Any;
    };
    // A closure of the crate's, called by its `Fn` trait's method.
    if tcx.is_closure_like(id) {
        return match id.is_local() {
            true => Called::Body(id, instance.args),
            false => Called::Any,
        };
    }
    // A borrow, of the cell its receiver is: a thread-local's by its key.
    // A thread-local's own, but its `with_borrow`, may change it.
    let refcell = receiver.is_some_and(|(thir, r)| recognition.is_std_type(thir[r].ty.peel_refs(), StdItem::RefCell));
    let mutable = match recognition.classify(id, instance.args) {
        Some(Std::Borrow { mutable, .. } | Std::TryBorrow { mutable, .. } | Std::LocalBorrow { mutable }) => {
            Some(mutable)
        }
        // A `RefCell`'s own write borrows it mutably while it runs. A
        // `swap`'s other cell is lent, a cell whose borrows are counted.
        Some(Std::CellReplace | Std::CellTake | Std::CellSwap | Std::CellReplaceWith) if refcell => Some(true),
        _ if recognition.asks_borrows(id) => Some(true),
        _ => None,
    };
    if let Some(mutable) = mutable {
        let Some((thir, receiver)) = receiver else {
            return Called::Any;
        };
        let held = thir[receiver].ty.peel_refs();
        let cell = match thir[strip(thir, receiver)].kind {
            ExprKind::Borrow { arg, .. }
                if let ExprKind::NamedConst { def_id: key, .. } = thir[strip(thir, arg)].kind =>
            {
                Cell::Key(key)
            }
            _ => match held.kind() {
                ty::Adt(_, key_args) if recognition.is_std_type(held, StdItem::LocalKey) => {
                    match key_args.types().next() {
                        Some(cell) => Cell::Type(tcx.erase_and_anonymize_regions(cell)),
                        None => return Called::Any,
                    }
                }
                _ if recognition.is_borrowed_cell(held) => Cell::Type(tcx.erase_and_anonymize_regions(held)),
                _ => return Called::Any,
            },
        };
        return Called::Cell(Borrow { cell, mutable });
    }
    if binding_runs_nothing(tcx, id) {
        return Called::Nothing;
    }
    match id.is_local() && !is_binding(tcx, id) {
        true => Called::Body(id, instance.args),
        false if recognition.is_std(id) => Called::Nothing,
        false => Called::Any,
    }
}

/// What a body borrows and runs, by its own expressions: each call's, each
/// closure it makes, which whatever it's given to may run, and each value
/// it may drop, whose destructor runs.
fn direct<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    thir: &Thir<'tcx>,
    drops: &dyn Fn(Ty<'tcx>) -> bool,
) -> Direct<'tcx> {
    let tcx = recognition.tcx;
    let mut found = Direct::default();
    for e in thir.exprs.iter() {
        match e.kind {
            ExprKind::Call { fun, ref args, .. } => match fn_def(thir[fun].ty) {
                Some((id, generic_args)) => {
                    match called(recognition, id, generic_args, args.first().map(|&a| (thir, a))) {
                        Called::Nothing => {}
                        Called::Cell(cell) => _ = found.own.cells.insert(cell),
                        Called::Body(body, args) => found.runs.push((body, Some(args))),
                        Called::Any => found.any = true,
                    }
                }
                None => found.any = true,
            },
            ExprKind::Closure(ref closure) => found.runs.push((closure.closure_id.to_def_id(), None)),
            ExprKind::InlineAsm(_) | ExprKind::Yield { .. } | ExprKind::Become { .. } => found.any = true,
            _ => {}
        }
        // A value whose drop runs a destructor, which may run anything; of a
        // type parameter's, its caller's to say. A guard's releases its
        // borrow.
        let ty = tcx.erase_and_anonymize_regions(e.ty);
        if recognition.is_guard(ty) || !ty.needs_drop(tcx, recognition.typing_env) {
            continue;
        }
        match ty.has_param() {
            true => _ = found.own.drops.insert(ty),
            false if drops(ty) => found.any = true,
            false => {}
        }
    }
    found
}

/// Whether a borrow asks of one `held`: of a cell that may be its, one of
/// them mutable.
fn asks<'tcx>(tcx: TyCtxt<'tcx>, theirs: Borrow<'tcx>, held: Borrow<'tcx>) -> bool {
    (theirs.mutable || held.mutable) && overlap(tcx, theirs.cell, held.cell)
}

/// Whether two cells may be one: a thread-local's and another's of its key,
/// or of its cell's type; two types alike.
fn overlap<'tcx>(tcx: TyCtxt<'tcx>, a: Cell<'tcx>, b: Cell<'tcx>) -> bool {
    let held = |key: DefId| match tcx.type_of(key).instantiate_identity().skip_normalization().kind() {
        ty::Adt(_, args) => args.types().next().map(|cell| tcx.erase_and_anonymize_regions(cell)),
        _ => None,
    };
    let alike = |x: Ty<'tcx>, y: Ty<'tcx>| x == y || x.has_param() || y.has_param();
    match (a, b) {
        (Cell::Key(x), Cell::Key(y)) => x == y,
        (Cell::Key(key), Cell::Type(ty)) | (Cell::Type(ty), Cell::Key(key)) => {
            held(key).is_none_or(|cell| alike(cell, ty))
        }
        (Cell::Type(x), Cell::Type(y)) => alike(x, y),
    }
}
