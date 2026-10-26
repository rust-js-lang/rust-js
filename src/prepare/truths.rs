//! What a `filter` by truth keeps, tested where it's held: its truth
//! (ADR 0635, amended).

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind};

/// `const h = x || undefined; const m = h != null ? a : b;` is `const m = x
/// ? a : b;`, `h` read as `x` in `a`: what a `filter` by truth keeps
/// (ADR 0635), tested next, is there exactly where `x` is truthy, and is
/// `x` there, as a string's truth is exactly a `Some` of one not empty;
/// upstream's `componentName ? `..${componentName}..` : '..'`. Of a
/// module's variable, which another function may set and which is one
/// only once its cell's reads are its name (`plain_statics`), after
/// tidy's `filtered_guards` has read a body's own (ADR 0635, amended):
/// the test is what the statement does first, and `a`, which reads `h`,
/// does nothing, so nothing between can change `x`. `h` is read nowhere
/// else.
pub(super) fn truth_tested(body: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < body.len() {
        let StmtKind::Const(h, made) = &body[i].kind else {
            i += 1;
            continue;
        };
        let ExprKind::Binary(Op::Or, x, none) = &made.kind else {
            i += 1;
            continue;
        };
        let is_h = |e: &Expr| matches!(&e.kind, ExprKind::Var(v) if v == h);
        let read = match &body[i + 1].kind {
            StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Return(Some(e)) => Some(e),
            StmtKind::Assign(target, e) if matches!(target.kind, ExprKind::Var(_)) => Some(e),
            _ => None,
        };
        let folded = match read.map(|e| &e.kind) {
            Some(ExprKind::Cond(test, yes, no))
                if matches!(x.kind, ExprKind::Var(_))
                    && matches!(none.kind, ExprKind::Undefined)
                    && matches!(&test.kind, ExprKind::Binary(Op::LooseNe, t, null)
                        if is_h(t) && matches!(null.kind, ExprKind::Null | ExprKind::Undefined))
                    && !yes.has_effects()
                    && !no.mentions_var(h)
                    && js::mentions_in(&body[i + 2..], h) == 0 =>
            {
                let yes = yes.substitute(&|v| (v == h).then(|| (**x).clone()));
                yes.map(|yes| Expr::cond((**x).clone(), yes, (**no).clone()))
            }
            _ => None,
        };
        let Some(folded) = folded else {
            i += 1;
            continue;
        };
        if let StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Return(Some(e)) | StmtKind::Assign(_, e) =
            &mut body[i + 1].kind
        {
            *e = Expr { span: e.span, ..folded };
        }
        body.remove(i);
    }
}
