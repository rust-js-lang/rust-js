//! Where each item written in a body goes, its hole, filled once every
//! body is lowered: a function a block makes and gives (ADR 0296), one
//! written in a function's body (ADR 0308), and a `const` written there
//! (ADR 0031, amended).

use crate::js::{self, Expr, StmtKind};
use std::collections::HashMap;

/// What fills each item's hole, by its item's index: a function a block
/// makes and gives (ADR 0296), or one written in a function's body (ADR
/// 0308); or a `const` written there (ADR 0031, amended).
#[derive(Default)]
pub(super) struct Holes {
    pub(super) functions: HashMap<u32, js::Function>,
    pub(super) consts: HashMap<u32, js::Const>,
}

/// Each hole in `stmts` filled with its function, and each in that (ADR 0296).
pub(super) fn fill_holes(stmts: &mut Vec<js::Stmt>, holes: &mut Holes) {
    // A function written in a body, a declaration there, before what's left
    // is taken for an expression (ADR 0308), and a `const`, a `const` there
    // (ADR 0031, amended); `each_block_mut` goes on into its body, for those
    // written in it.
    js::each_block_mut(stmts, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            let StmtKind::Expr(e) = &stmt.kind else { continue };
            let js::ExprKind::ItemHole(index) = e.kind else {
                continue;
            };
            // Where it's written, which its comments are before.
            if let Some(function) = holes.functions.remove(&index) {
                stmt.span = function.span;
                stmt.kind = StmtKind::Function(Box::new(function));
            } else if let Some(constant) = holes.consts.remove(&index) {
                stmt.span = constant.span;
                stmt.kind = StmtKind::Const(constant.name, constant.value);
            }
        }
    });
    js::each_expr_mut(stmts, &mut |e| fill_hole(e, holes));
}

pub(super) fn fill_hole(e: &mut Expr, holes: &mut Holes) {
    if let js::ExprKind::ItemHole(index) = e.kind
        && let Some(mut function) = holes.functions.remove(&index)
    {
        fill_holes(&mut function.body, holes);
        e.kind = js::ExprKind::Function(Box::new(function));
    }
}
