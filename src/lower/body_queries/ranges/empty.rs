//! Which `OnceCell`s a body knows are still empty where it sets them (ADR
//! 0847): one it made, `Rc::new(Payload { io_info: OnceCell::new(), .. })`,
//! that nothing could have set since. Only what has the cell could: the
//! variable that made it, and each made of that one, a clone of its `Rc`
//! in a struct, say. The walk forgets a cell where one of them is used any
//! other way than these: given to a call, captured, read or lent, or
//! dropped where dropping it may run code.

use super::super::strip;
use super::Slice;
use crate::lower::fn_def;
use crate::lower::recognition::{is_pointer_new, is_rc_clone, is_rc_deref, is_std_item, once_method};
use crate::lower::std_types::once::OnceOp;
use rustc_hir::{BindingMode, ByRef};
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, PatKind, StmtKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use std::collections::HashSet;

/// The reads of variables that give nothing their cells: the variable a
/// `get()` or a `set()` is of a cell of, and in a `let` that binds one
/// variable, each variable it's made of (`made_of`), whose cells the one
/// it binds then reaches.
pub(super) fn quiet_reads<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<ExprId> {
    let mut quiet = HashSet::new();
    for stmt in thir.stmts.iter() {
        if let StmtKind::Let {
            ref pattern,
            initializer: Some(init),
            else_block: None,
            ..
        } = stmt.kind
            && matches!(
                pattern.kind,
                PatKind::Binding {
                    mode: BindingMode(ByRef::No, _),
                    subpattern: None,
                    ..
                }
            )
        {
            quiet.extend(made_of(tcx, thir, init).into_iter().map(|(read, _)| read));
        }
    }
    for expr in thir.exprs.iter() {
        if let ExprKind::Call { fun, ref args, .. } = expr.kind
            && let Some((def_id, _)) = fn_def(thir[strip(thir, fun)].ty)
            && matches!(once_method(tcx, def_id), Some(OnceOp::Get | OnceOp::Set))
            && let Some((_, read)) = args.first().and_then(|&cell| cell_place(tcx, thir, cell))
        {
            quiet.insert(read);
        }
    }
    quiet
}

/// The variables `init`, a `let`'s, is made of, each read: itself, `let b
/// = a`, an `Rc` of it cloned, `Rc::clone(&a)`, in a struct, a tuple or an
/// array it makes, or a new `Rc` or `Box` of those.
pub(super) fn made_of<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, init: ExprId) -> Vec<(ExprId, LocalVarId)> {
    let mut found = Vec::new();
    let mut work = vec![init];
    while let Some(e) = work.pop() {
        let e = strip(thir, e);
        match thir[e].kind {
            ExprKind::VarRef { id } => found.push((e, id)),
            ExprKind::Adt(ref adt) => work.extend(adt.fields.iter().map(|f| f.expr)),
            ExprKind::Tuple { ref fields } | ExprKind::Array { ref fields } => work.extend(fields.iter()),
            ExprKind::Call { fun, ref args, .. }
                if let Some((def_id, generic_args)) = fn_def(thir[strip(thir, fun)].ty) =>
            {
                if is_pointer_new(tcx, def_id) {
                    work.extend(args.first());
                } else if is_rc_clone(tcx, def_id, generic_args)
                    && let Some(&rc) = args.first()
                    && let Some(read) = lent(thir, rc)
                    && let ExprKind::VarRef { id } = thir[read].kind
                {
                    found.push((read, id));
                }
            }
            _ => {}
        }
    }
    found
}

/// The variable a shared reference is of, `&kept`, reborrowed or not,
/// `&*&kept`: its read.
fn lent(thir: &Thir<'_>, mut e: ExprId) -> Option<ExprId> {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Borrow {
                borrow_kind: BorrowKind::Shared,
                arg,
            } => e = arg,
            ExprKind::Deref { arg } if matches!(thir[arg].ty.kind(), ty::Ref(..)) => e = arg,
            ExprKind::VarRef { .. } => return Some(e),
            _ => return None,
        }
    }
}

/// The cells `init`, a `let`'s, makes empty, by the fields to each: `[1]`
/// of `Rc::new(Payload { status, io_info: OnceCell::new() })`, `[]` of
/// `OnceCell::new()` itself.
pub(super) fn made_empty<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, init: ExprId) -> Vec<Vec<usize>> {
    let mut found = Vec::new();
    let mut work = vec![(init, Vec::new())];
    while let Some((e, path)) = work.pop() {
        let e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Call { fun, ref args, .. } if let Some((def_id, _)) = fn_def(thir[strip(thir, fun)].ty) => {
                if once_method(tcx, def_id) == Some(OnceOp::New) {
                    found.push(path);
                } else if is_pointer_new(tcx, def_id)
                    && let Some(&value) = args.first()
                {
                    work.push((value, path));
                }
            }
            ExprKind::Adt(ref adt) if adt.adt_def.is_struct() => {
                for field in adt.fields.iter() {
                    let mut to = path.clone();
                    to.push(field.name.as_usize());
                    work.push((field.expr, to));
                }
            }
            _ => {}
        }
    }
    found
}

/// The cell `e`, a `get()`'s or a `set()`'s `&self`, is: the variable it's
/// of and the fields to it, through references, `Box`es and `Rc`s, and
/// that variable's read.
pub(super) fn cell_place<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, mut e: ExprId) -> Option<(Slice, ExprId)> {
    let mut path = Vec::new();
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Borrow {
                borrow_kind: BorrowKind::Shared,
                arg,
            } => e = arg,
            // An `Rc`'s `Deref`, std's, which runs nothing.
            ExprKind::Deref { arg }
                if let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, arg)].kind
                    && let Some((def_id, generic_args)) = fn_def(thir[strip(thir, fun)].ty)
                    && is_rc_deref(tcx, def_id, generic_args)
                    && let [rc] = args[..] =>
            {
                e = rc
            }
            ExprKind::Deref { arg }
                if (matches!(thir[arg].ty.kind(), ty::Ref(..)) || thir[arg].ty.is_box())
                    && !matches!(thir[strip(thir, arg)].kind, ExprKind::Call { .. }) =>
            {
                e = arg
            }
            ExprKind::Field { lhs, name, .. } => {
                path.push(name.as_usize());
                e = lhs;
            }
            ExprKind::VarRef { id } => {
                path.reverse();
                return Some(((path, id), e));
            }
            _ => return None,
        }
    }
}

/// Whether dropping a `ty` may run code that could set a cell: a
/// destructor of the crate's or a binding's, or one rust-js can't see, a
/// `dyn`'s or a type parameter's. std's own run none but their items'.
pub(super) fn drop_runs_code<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    let mut seen = HashSet::new();
    let mut work = vec![ty];
    while let Some(ty) = work.pop() {
        if !ty.needs_drop(tcx, typing_env) || !seen.insert(ty) {
            continue;
        }
        match ty.kind() {
            ty::Adt(adt, args) if is_std_item(tcx, adt.did()) => work.extend(args.types()),
            ty::Adt(adt, args) => {
                if tcx.adt_destructor(adt.did()).is_some() {
                    return true;
                }
                work.extend(adt.all_fields().map(|f| f.ty(tcx, args).skip_normalization()));
            }
            ty::Tuple(items) => work.extend(items.iter()),
            ty::Array(item, _) | ty::Slice(item) => work.push(*item),
            ty::Closure(_, args) => work.extend(args.as_closure().upvar_tys()),
            _ => return true,
        }
    }
    false
}
