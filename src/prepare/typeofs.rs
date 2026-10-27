//! A host's global asked twice in one `&&`: what an earlier test of it
//! says, a later one needn't (ADR 0964).

use crate::js::{Expr, ExprKind, Op, UnaryOp};

/// Of an `&&` chain, each `typeof p !== "undefined"` after a test of `p`'s
/// `typeof`, `=== "function"` or `!== "undefined"`, which says it already,
/// gone: `js::is_function(js::symbol::new) && js::is_function(SymbolFor)`'s
/// `typeof Symbol === "function" && typeof Symbol.for === "function"`, as
/// React writes it, not with `typeof Symbol !== "undefined"` between. A
/// `typeof` reads a name or a property and runs nothing, so a test gone
/// leaves what the chain does.
pub(super) fn implied(e: &mut Expr) {
    if !matches!(&e.kind, ExprKind::Binary(Op::And, ..)) {
        return;
    }
    let mut parts = Vec::new();
    conjuncts(e.clone(), &mut parts);
    let mut kept: Vec<Expr> = Vec::with_capacity(parts.len());
    for part in parts {
        if let Some(asked) = undefined_test(&part)
            && kept.iter().any(|earlier| says_defined(earlier, asked))
        {
            continue;
        }
        kept.push(part);
    }
    if let Some(chain) = kept.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b)) {
        let span = e.span;
        *e = Expr { span, ..chain };
    }
}

/// `e`'s parts joined by `&&`, in order.
fn conjuncts(e: Expr, parts: &mut Vec<Expr>) {
    match e.kind {
        ExprKind::Binary(Op::And, a, b) => {
            conjuncts(*a, parts);
            conjuncts(*b, parts);
        }
        _ => parts.push(e),
    }
}

/// What `typeof x !== "undefined"` asks of, `x`.
fn undefined_test(e: &Expr) -> Option<&Expr> {
    typeof_test(e, Op::Ne, "undefined")
}

/// Whether `earlier` says `asked` is defined: `typeof asked === "function"`
/// or `typeof asked !== "undefined"`.
fn says_defined(earlier: &Expr, asked: &Expr) -> bool {
    [(Op::Eq, "function"), (Op::Ne, "undefined")]
        .into_iter()
        .any(|(op, kind)| typeof_test(earlier, op, kind).is_some_and(|x| same_read(x, asked)))
}

/// What `typeof x <op> "<kind>"` reads, `x`.
fn typeof_test<'e>(e: &'e Expr, op: Op, kind: &str) -> Option<&'e Expr> {
    let ExprKind::Binary(o, a, b) = &e.kind else {
        return None;
    };
    if *o != op || !matches!(&b.kind, ExprKind::Str(s) if s == kind) {
        return None;
    }
    match &a.kind {
        ExprKind::Unary(UnaryOp::Typeof, x) => Some(x),
        _ => None,
    }
}

/// Whether `a` and `b` read the same name or property path, `Symbol` or
/// `console.log`.
fn same_read(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Var(x), ExprKind::Var(y)) => x == y,
        (ExprKind::Member(x, m), ExprKind::Member(y, n)) => m == n && same_read(x, y),
        _ => false,
    }
}
