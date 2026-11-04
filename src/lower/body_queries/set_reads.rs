//! A cell read back right after it's set (rd331e): `self.surface.set(Some(..));
//! create(self.surface.get().unwrap(), ..)`, whose `unwrap` can't fail, as
//! no code runs between the set and the read that could take it out.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::thir::{self, AdtExprBase, ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use std::collections::HashSet;

use super::strip;
use crate::lower::recognition::{CellMethod, cell_own_method};

/// Each `unwrap` or `expect` of a `Cell`'s `get` that is what the statement
/// after a `cell.set(Some(..))` of the same place runs first: its
/// `fn_span`. What runs before it in that statement only reads places,
/// borrows, literals and constants, so no code, another `&` to the cell
/// say, can set it between: what it reads is the `Some` just set, and its
/// `unwrap` is the value (rd331e). A `get`'s cell is of a `Copy` value,
/// so its set runs no drop of the old one after it stores.
pub(super) fn set_then_unwrapped<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<Span> {
    let mut found = HashSet::new();
    for block in thir.blocks.iter() {
        for (k, &stmt) in block.stmts.iter().enumerate() {
            let Some(cell) = set_some(tcx, thir, stmt) else {
                continue;
            };
            let next = match block.stmts.get(k + 1) {
                Some(&next) => statement_value(thir, next),
                None => block.expr,
            };
            let Some(First::Call {
                call,
                consumer: Some(unwrap),
            }) = next.map(|next| first_call(thir, next, None))
            else {
                continue;
            };
            if let Some(read) = cell_get(tcx, thir, call)
                && same_place(thir, cell, read)
                && let ExprKind::Call { fun, fn_span, .. } = thir[unwrap].kind
                && is_option_unwrap(tcx, thir[fun].ty)
            {
                found.insert(fn_span);
            }
        }
    }
    found
}

/// The place `stmt` sets a `Some` of, `cell.set(Some(..));`'s `cell`, or
/// `cell.replace(Some(..));`'s.
fn set_some<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, stmt: thir::StmtId) -> Option<ExprId> {
    let thir::StmtKind::Expr { expr, .. } = thir[stmt].kind else {
        return None;
    };
    let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, expr)].kind else {
        return None;
    };
    let ty::FnDef(def_id, _) = *thir[fun].ty.kind() else {
        return None;
    };
    if cell_own_method(tcx, def_id) != Some(CellMethod::Writes) {
        return None;
    }
    let [cell, value] = args[..] else { return None };
    let ExprKind::Adt(ref adt) = thir[strip(thir, value)].kind else {
        return None;
    };
    let some = tcx.is_lang_item(adt.adt_def.variant(adt.variant_index).def_id, LangItem::OptionSome);
    some.then(|| borrowed(thir, cell)).flatten()
}

/// What `stmt` evaluates: an expression statement's expression, or a
/// `let`'s initializer, its pattern bound after; `None` of a `let .. else`.
fn statement_value(thir: &Thir<'_>, stmt: thir::StmtId) -> Option<ExprId> {
    match thir[stmt].kind {
        thir::StmtKind::Expr { expr, .. } => Some(expr),
        thir::StmtKind::Let {
            initializer,
            else_block: None,
            ..
        } => initializer,
        thir::StmtKind::Let { .. } => None,
    }
}

/// The place `&place` borrows, of a shared borrow.
fn borrowed(thir: &Thir<'_>, e: ExprId) -> Option<ExprId> {
    match thir[strip(thir, e)].kind {
        ExprKind::Borrow {
            borrow_kind: rustc_middle::mir::BorrowKind::Shared,
            arg,
        } => Some(arg),
        _ => None,
    }
}

/// The place `call`, a `Cell`'s `get`, reads, `cell.get()`'s `cell`.
fn cell_get<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, call: ExprId) -> Option<ExprId> {
    let ExprKind::Call { fun, ref args, .. } = thir[call].kind else {
        return None;
    };
    let ty::FnDef(def_id, _) = *thir[fun].ty.kind() else {
        return None;
    };
    match (cell_own_method(tcx, def_id), &args[..]) {
        (Some(CellMethod::Get), &[cell]) => borrowed(thir, cell),
        _ => None,
    }
}

/// Is `fun`'s type an `Option`'s own `unwrap` or `expect`?
pub(in crate::lower) fn is_option_unwrap<'tcx>(tcx: TyCtxt<'tcx>, fun: ty::Ty<'tcx>) -> bool {
    let ty::FnDef(def_id, _) = *fun.kind() else {
        return false;
    };
    let option = (tcx.inherent_impl_of_assoc(def_id)).is_some_and(|imp| {
        matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
            ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option))
    });
    option && matches!(tcx.item_name(def_id).as_str(), "unwrap" | "expect")
}

/// Are `a` and `b` one place, of the same variable by the same fields and
/// dereferences, `(*self).surface`?
fn same_place(thir: &Thir<'_>, a: ExprId, b: ExprId) -> bool {
    match (&thir[strip(thir, a)].kind, &thir[strip(thir, b)].kind) {
        (
            &ExprKind::Field {
                lhs: a,
                variant_index: va,
                name: na,
            },
            &ExprKind::Field {
                lhs: b,
                variant_index: vb,
                name: nb,
            },
        ) => va == vb && na == nb && same_place(thir, a, b),
        (&ExprKind::Deref { arg: a }, &ExprKind::Deref { arg: b }) => same_place(thir, a, b),
        (ExprKind::VarRef { id: a }, ExprKind::VarRef { id: b }) => a == b,
        (ExprKind::UpvarRef { var_hir_id: a, .. }, ExprKind::UpvarRef { var_hir_id: b, .. }) => a == b,
        _ => false,
    }
}

/// What an expression runs first, as it's evaluated left to right, inner
/// before outer.
enum First {
    /// No code: places, borrows, literals and constants alone.
    Inert,
    /// A call, after only what's `Inert`, and the call it's an argument
    /// of, if it's one.
    Call { call: ExprId, consumer: Option<ExprId> },
    /// What this doesn't read, which may run code.
    Unknown,
}

/// The call `e` runs first, `consumer` the call `e` is an argument of.
fn first_call(thir: &Thir<'_>, e: ExprId, consumer: Option<ExprId>) -> First {
    let e = strip(thir, e);
    match thir[e].kind {
        ExprKind::Call { fun, ref args, .. } => {
            // What's called first, `(self.pick)(..)`'s place, then each
            // argument in order.
            for (i, &operand) in std::iter::once(&fun).chain(args.iter()).enumerate() {
                match first_call(thir, operand, (i > 0).then_some(e)) {
                    First::Inert => {}
                    found => return found,
                }
            }
            First::Call { call: e, consumer }
        }
        ExprKind::Borrow { arg, .. }
        | ExprKind::Deref { arg }
        | ExprKind::Field { lhs: arg, .. }
        | ExprKind::PointerCoercion { source: arg, .. }
        | ExprKind::Cast { source: arg } => first_call(thir, arg, None),
        ExprKind::Adt(ref adt) if matches!(adt.base, AdtExprBase::None) => {
            for field in adt.fields.iter() {
                match first_call(thir, field.expr, None) {
                    First::Inert => {}
                    found => return found,
                }
            }
            First::Inert
        }
        // A block that only binds, `_ = ..`'s `{ let _ = ..; }`.
        ExprKind::Block { block } => {
            let block = &thir[block];
            for &stmt in block.stmts.iter() {
                let found = match statement_value(thir, stmt) {
                    Some(value) => first_call(thir, value, None),
                    None if matches!(thir[stmt].kind, thir::StmtKind::Let { initializer: None, .. }) => First::Inert,
                    None => First::Unknown,
                };
                match found {
                    First::Inert => {}
                    found => return found,
                }
            }
            match block.expr {
                Some(value) => first_call(thir, value, None),
                None => First::Inert,
            }
        }
        ExprKind::VarRef { .. }
        | ExprKind::UpvarRef { .. }
        | ExprKind::Literal { .. }
        | ExprKind::NonHirLiteral { .. }
        | ExprKind::ZstLiteral { .. }
        | ExprKind::NamedConst { .. }
        | ExprKind::ConstParam { .. }
        | ExprKind::StaticRef { .. } => First::Inert,
        _ => First::Unknown,
    }
}
