//! Where a comment before part of an expression is before it: a
//! conditional's branch, an array's item, a binary's right operand, an
//! `if`'s test (ADRs 0658, 0708, 0444).

use super::expressions;
use crate::js::{self, ExprKind, Module, StmtKind};

/// Where each conditional's branch is, `c ? a : b`'s `a` and `b`, of those
/// that say, by where it starts: what a comment before one is before (ADR
/// 0658); each of a `concat!`'s pieces past its first, after its `+`
/// (ADR 0641); and a binary's right operand, after its operator, as a
/// comment before it is, between the two or in the closure it's of,
/// `is_some_and(|v| { // c\n v > 3 })` (ADR 0444, amended).
pub(super) fn arm_starts(module: &Module) -> std::collections::HashMap<u32, Arm> {
    let mut starts = std::collections::HashMap::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_expr_mut(&mut stmts, &mut |e| {
            let spans: Vec<(js::Span, &'static str, bool)> = match &e.kind {
                ExprKind::Cond(_, yes, no) => vec![(yes.span, "?", false), (no.span, ":", false)],
                // An array's item, after its `[` or `,`, `HashMap::from([..])`'s
                // pair, as Prettier writes a comment before it (ADR 0708,
                // amended).
                // One placed nowhere, a tuple MIR made, from where its parts
                // are; of an array as it's written, its items one after
                // another, not one rust-js made of a call, `heap.swap(i,
                // j)`'s, each item of the same `heap`.
                ExprKind::Array(items) => {
                    let spans: Vec<js::Span> = (items.iter())
                        .map(|item| match item.span.is_none() {
                            true => written_extent(item).map_or(js::Span::NONE, |(lo, hi)| js::Span { lo, hi }),
                            false => item.span,
                        })
                        .collect();
                    let written = spans.windows(2).all(|pair| pair[0].hi <= pair[1].lo);
                    (spans.into_iter().enumerate())
                        .filter(|_| written)
                        .map(|(i, span)| (span, if i == 0 { "[" } else { "," }, false))
                        .collect()
                }
                // An arrow's value, after its `=>`, as Prettier writes a
                // comment before it (ADR 0883).
                ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => expressions::arrow_value(body)
                    .map(|value| (value.span, "=>", false))
                    .into_iter()
                    .collect(),
                ExprKind::Written(pieces) => pieces.iter().skip(1).map(|piece| (piece.span, "+", false)).collect(),
                // One placed nowhere, an `&&` tidy made of branches, `|| (a
                // && b)`'s, from where its parts are (ADR 0444, amended).
                ExprKind::Binary(op, _, r) => {
                    let span = match r.span.is_none() {
                        true => written_extent(r).map_or(js::Span::NONE, |(lo, hi)| js::Span { lo, hi }),
                        false => r.span,
                    };
                    vec![(span, expressions::operator(*op), true)]
                }
                _ => Vec::new(),
            };
            let arms = (spans.into_iter().filter(|(s, ..)| !s.is_none())).map(|(s, operator, operand)| {
                let arm = Arm {
                    hi: s.hi,
                    operator,
                    operand,
                    opening: false,
                };
                (s.lo, arm)
            });
            // Of an array's first item that starts another array, the outer's
            // item, the first met.
            for (lo, arm) in arms {
                match arm.operator {
                    "[" | "," => _ = starts.entry(lo).or_insert(arm),
                    _ => _ = starts.insert(lo, arm),
                }
            }
        });
        // An `if`'s test, after its `if (`, which a comment between `if`
        // and its first operand is before (`opening`).
        js::each_block_mut(&mut stmts, &mut |block| {
            for stmt in block.iter() {
                if let StmtKind::If(test, ..) = &stmt.kind
                    && let Some((lo, hi)) = written_extent(test)
                {
                    let arm = Arm {
                        hi,
                        operator: "if (",
                        operand: false,
                        opening: true,
                    };
                    starts.entry(lo).or_insert(arm);
                }
            }
        });
    };
    for item in &module.items {
        match item {
            js::Item::Const(c) => of(vec![StmtKind::Expr(c.value.clone()).at(js::Span::NONE)]),
            js::Item::Statements(stmts) => of(stmts.clone()),
            js::Item::Function(f) => of(f.body.clone()),
            js::Item::Namespace(n) => n.methods.iter().for_each(|method| of(method.body.clone())),
        }
    }
    // A function's parameter, after the `(` or the `,` before it, as
    // Prettier writes a comment before one (ADR 0708, amended).
    for item in &module.items {
        let js::Item::Function(f) = item else { continue };
        let params = (f.param_spans.iter().enumerate()).filter(|(_, s)| !s.is_none());
        starts.extend(params.map(|(i, s)| {
            let operator = if i == 0 { "(" } else { "," };
            let arm = Arm {
                hi: s.hi,
                operator,
                operand: false,
                opening: false,
            };
            (s.lo, arm)
        }));
    }
    starts
}

/// Where each `if` that's an `else`'s only statement starts, by where it
/// ends: what's printed `else if` where no comment is before it in the
/// block (ADR 0444, amended).
pub(super) fn else_if_starts(module: &Module) -> std::collections::HashMap<u32, u32> {
    let mut starts = std::collections::HashMap::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_block_mut(&mut stmts, &mut |block| {
            for stmt in block.iter() {
                if let StmtKind::If(_, _, Some(els)) = &stmt.kind
                    && let [only] = els.as_slice()
                    && matches!(only.kind, StmtKind::If(..))
                    && !only.span.is_none()
                {
                    starts.insert(only.span.lo, only.span.hi);
                }
            }
        });
    };
    for item in &module.items {
        match item {
            js::Item::Const(c) => of(vec![StmtKind::Expr(c.value.clone()).at(js::Span::NONE)]),
            js::Item::Statements(stmts) => of(stmts.clone()),
            js::Item::Function(f) => of(f.body.clone()),
            js::Item::Namespace(n) => n.methods.iter().for_each(|method| of(method.body.clone())),
        }
    }
    starts
}

/// Where the Rust of `e` is, from the first of its parts placed to the end
/// of the last: an `||` tidy made of guards is placed nowhere, its
/// operands where they're written. A part placed is all of it: what's in
/// it is within it, or is read where it's written elsewhere, a variable
/// where its pattern binds it, `a` of `if let Some(a) = a && ..`, which
/// isn't where `e` is (ADR 0444, amended).
fn written_extent(e: &js::Expr) -> Option<(u32, u32)> {
    let mut extent: Option<(u32, u32)> = None;
    e.clone().each_mut(&mut |part| {
        if !part.span.is_none() {
            let (lo, hi) = extent.unwrap_or((part.span.lo, part.span.hi));
            extent = Some((lo.min(part.span.lo), hi.max(part.span.hi)));
            *part = js::Expr::undefined();
        }
    });
    extent
}

/// A start `arm_starts` found: where it ends, the operator it's after in
/// the JS, whether it's a binary's operand, and whether it's an `if`'s
/// test, whose comments are only those written after its `if`.
#[derive(Clone, Copy)]
pub(super) struct Arm {
    pub(super) hi: u32,
    pub(super) operator: &'static str,
    pub(super) operand: bool,
    pub(super) opening: bool,
}
