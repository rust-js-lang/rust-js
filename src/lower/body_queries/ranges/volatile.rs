//! The variables a raw pointer may change where the walk doesn't see
//! (ADR 0576): never known.

use super::root_var;
use crate::lower::body_queries::strip;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Thir};
use rustc_middle::ty;
use std::collections::HashSet;

/// The variables a raw pointer is made of, which it may change where the
/// walk doesn't see: not a `*const` to what a shared reference refers to,
/// `ptr::eq(&node.children[0], skip)`'s, which nothing may change through it
/// but a cell, whose change is a call the walk sees, `borrow_mut()`.
pub(super) fn volatile(thir: &Thir<'_>) -> HashSet<LocalVarId> {
    (thir.exprs.iter())
        .filter_map(|expr| match expr.kind {
            ExprKind::RawBorrow {
                mutability: ty::Mutability::Not,
                arg,
            } if through_shared(thir, arg) => None,
            ExprKind::RawBorrow { arg, .. } => root_var(thir, arg),
            _ => None,
        })
        .collect()
}

/// Whether a place is reached through a shared reference, `*r` of an `&T`,
/// on the way to its variable.
fn through_shared(thir: &Thir<'_>, mut e: ExprId) -> bool {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Deref { arg } if matches!(thir[arg].ty.kind(), ty::Ref(_, _, ty::Mutability::Not)) => {
                return true;
            }
            ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } | ExprKind::Deref { arg: lhs } => e = lhs,
            _ => return false,
        }
    }
}
