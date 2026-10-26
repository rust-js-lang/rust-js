//! The thread-locals and `static mut`s that are their module's `let` or
//! `const` (ADRs 0270, 0096), and the cells that are their function's
//! `let` (ADR 0287).

use super::super::Body;
use super::super::body_queries::strip;
use super::super::fn_def;
use super::super::recognition::{StdItem, is_local_borrow, is_local_with, is_std_def, local_key_access};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, GenericArgsRef, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId, LocalModId};
use std::collections::HashSet;

/// What a `with_borrow` is given to run: a closure made there, or a
/// function, `CURRENT_TASK.with_borrow(Clone::clone)`.
#[derive(Clone, Copy)]
pub(super) enum Given<'tcx> {
    Closure(LocalDefId),
    Fn(DefId, GenericArgsRef<'tcx>),
}

/// What a `with_borrow` is given, `f` of `KEY.with_borrow(f)`.
fn given<'tcx>(thir: &rustc_middle::thir::Thir<'tcx>, f: rustc_middle::thir::ExprId) -> Option<Given<'tcx>> {
    match thir[strip(thir, f)].kind {
        ExprKind::Closure(ref c) => Some(Given::Closure(c.closure_id)),
        _ => fn_def(thir[f].ty).map(|(f, f_args)| Given::Fn(f, f_args)),
    }
}

/// The thread-locals only read and set, by `get`, `set`, `with_borrow`,
/// `with_borrow_mut` and `with`: nothing shares their cell, so each is its
/// module's variable, without a `{ value }`, exported or not, as another
/// module only reads it (`outside_writes`, ADR 0270, amended). What a
/// `with_borrow` runs must be `quiet` of its key, borrowing nothing that
/// may be its cell, as nothing counts its borrows (ADRs 0328, 0580, 0587);
/// and what a `with` runs, lent the cell itself, as one may borrow it
/// there. Where what it's lent is handed on, its module keeps the cell
/// (`plain_statics`).
pub(super) fn plain_thread_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    keys: impl Iterator<Item = LocalDefId>,
    quiet: &dyn Fn(Given<'tcx>, DefId) -> bool,
) -> HashSet<LocalDefId> {
    let mut plain: HashSet<LocalDefId> = keys.collect();
    for body in bodies {
        let thir = &body.thir;
        // Each thread-local read or set where it's read, `KEY` of `KEY.get()`.
        let mut accessed = HashSet::new();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let Some((id, _)) = fn_def(thir[fun].ty)
                && let lends = (is_local_borrow(tcx, id) || is_local_with(tcx, id))
                && (lends || local_key_access(tcx, id).is_some())
                && let Some(&first) = args.first()
                && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
                && (!lends || {
                    matches!((given(thir, args[1]), &thir[strip(thir, arg)].kind),
                        (Some(given), ExprKind::NamedConst { def_id: key, .. })
                            if key.is_local() && quiet(given, *key))
                })
            {
                accessed.insert(strip(thir, arg));
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            if let ExprKind::NamedConst { def_id, .. } = expr.kind
                && let Some(key) = def_id.as_local()
                && !accessed.contains(&id)
            {
                plain.remove(&key);
            }
        }
    }
    plain
}

/// Whether `key` is its module's state whose JS is the value it holds or
/// its `{ value }`, which a write replaces: a `static mut` of the crate's or
/// a library's (ADR 0096, amended), or a thread-local of a `Cell`, or of a
/// `RefCell` of what isn't the object it holds (ADR 0863). A thread-local
/// `RefCell` of an array or an object is that object, changed in place, as
/// any module may change an object it imports.
pub(in crate::lower) fn holds_a_value(tcx: TyCtxt<'_>, key: DefId) -> bool {
    if tcx.is_mutable_static(key) {
        return !tcx.is_foreign_item(key);
    }
    if !matches!(tcx.def_kind(key), rustc_hir::def::DefKind::Const { .. }) {
        return false;
    }
    let ty = tcx.type_of(key).instantiate_identity().skip_normalization();
    let ty::Adt(local, args) = ty.kind() else { return false };
    if !is_std_def(tcx, local.did(), StdItem::LocalKey) {
        return false;
    }
    match args.type_at(0).kind() {
        ty::Adt(cell, _) if is_std_def(tcx, cell.did(), StdItem::Cell) => true,
        ty::Adt(cell, held) if is_std_def(tcx, cell.did(), StdItem::RefCell) => {
            !super::super::drops::holds_itself(tcx, held.type_at(0))
        }
        _ => false,
    }
}

/// `module`'s `static mut`s, of the crate's `consts`: each its module's
/// variable, as a thread-local of a value is (ADR 0096, amended).
pub(in crate::lower) fn module_statics(tcx: TyCtxt<'_>, consts: &[LocalDefId], module: LocalModId) -> Vec<LocalDefId> {
    (consts.iter().copied())
        .filter(|&key| {
            tcx.parent_module_from_def_id(key) == module
                && tcx.is_mutable_static(key.to_def_id())
                && holds_a_value(tcx, key.to_def_id())
        })
        .collect()
}

/// Refuse `key`, which its module exports, where `plain_statics` left it its
/// `{ value }`, as its module needs the box: a JS module importing it would
/// read the box, not what it holds (ADRs 0270, 0096, amended).
pub(in crate::lower) fn refuse_boxed_export(tcx: TyCtxt<'_>, key: DefId) {
    let span = tcx.def_ident_span(key).unwrap_or_else(|| tcx.def_span(key));
    let name = tcx.item_name(key);
    let message = match tcx.is_mutable_static(key) {
        true => format!(
            "rust-js: `{name}` is exported, so a JS module imports it as its module's `export let`, but its module gives on a pointer to it, `&raw mut {name}` or `addr_of!({name})`, a handle on its place, which needs it in a box: read and write it by its name, or through a pointer only where it's taken, `*addr_of_mut!({name}) += 1`, or keep it private to its module and export a `pub fn` that reads it (ADRs 0096, 0270, amended)"
        ),
        false => format!(
            "rust-js: `{name}` is exported, so a JS module imports it as what its cell holds, its module's `export let`, but its module needs the cell itself: it hands on the cell or the `&mut` of what it holds, or borrows it in a `with`, or again within a borrow of it; read and set it with `get`, `set`, `with_borrow` and `with_borrow_mut`, using in their closures what they give, or keep it private to its module and export a `pub fn` that reads it (ADR 0270, amended)"
        ),
    };
    tcx.dcx().span_err(span, message);
}

/// Each use of a thread-local of a value (`holds_a_value`) from outside its
/// module, another of the crate's or a consumer's, that isn't a read of
/// it, `get` or `with_borrow`: a `set`, a `replace`, a `take`, a
/// `with_borrow_mut`, or its cell handed out, `with` or `&KEY`; and of a
/// `static mut`, each write of it, whole, of a field or an item, and each
/// raw pointer taken to it (ADR 0096, amended). Its module's JS is its
/// `export let`, which JS lets only that module set (ADR 0270, amended), so
/// each is refused. A library's thread-local that's its
/// module's variable (`variable`) is read by `with_borrow` only where what
/// that runs is `quiet`, as nothing counts its borrows.
pub(super) fn outside_writes<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    variable: &dyn Fn(DefId) -> bool,
    quiet: &dyn Fn(Given<'tcx>, DefId) -> bool,
) -> Vec<(rustc_span::Span, DefId)> {
    let mut refused = Vec::new();
    for body in bodies {
        let Some(at) = body.def_id.as_local() else { continue };
        let module = tcx.parent_module_from_def_id(at).to_def_id();
        let outside = |key: DefId| match key.as_local() {
            Some(key) => tcx.parent_module_from_def_id(key).to_def_id() != module,
            None => true,
        };
        let thir = &body.thir;
        // Each read of one, `KEY` of `KEY.get()` or `KEY.with_borrow(f)`.
        let mut read = HashSet::new();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let Some((id, _)) = fn_def(thir[fun].ty)
                && local_key_access(tcx, id) == Some(false)
                && tcx.item_name(id).as_str() != "with_borrow_mut"
                && let Some(&first) = args.first()
                && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
            {
                let key = strip(thir, arg);
                let quiet_read = match thir[key].kind {
                    ExprKind::NamedConst { def_id, .. }
                        if !def_id.is_local() && is_local_borrow(tcx, id) && variable(def_id) =>
                    {
                        given(thir, args[1]).is_some_and(|given| quiet(given, def_id))
                    }
                    _ => true,
                };
                if quiet_read {
                    read.insert(key);
                }
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            if let ExprKind::NamedConst { def_id, .. } = expr.kind
                && outside(def_id)
                && !read.contains(&id)
                && holds_a_value(tcx, def_id)
            {
                refused.push((expr.span, def_id));
            }
            // A `static mut`'s, written whole, a field or an item of it, or
            // a pointer taken to it, a handle on its place.
            let written = match expr.kind {
                ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. } => lhs,
                ExprKind::RawBorrow { arg, .. } => arg,
                _ => continue,
            };
            if let Some(def_id) = super::validation::static_of(thir, written)
                && outside(def_id)
                && holds_a_value(tcx, def_id)
            {
                refused.push((expr.span, def_id));
            }
        }
    }
    refused
}
