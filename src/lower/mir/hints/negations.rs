//! Where each `!( .. )` the program wrote of an `&&` or an `||` is
//! (`Written::negations`, ADR 0879, amended).

use super::*;

impl FnCx<'_, '_> {
    /// Each `!( .. )` of an `&&` or `||`: where `!(a && b)`'s `a && b` is,
    /// and whether it's an `&&`. An `if` of a `bool` whose `else` is
    /// `false`, `!(if let Some(n) = x { n > 3 } else { false })`, is its
    /// test's `&&`; one whose `then` is `true`, its `||` (rd240).
    pub(super) fn negations(&self) -> Vec<(js::Span, bool)> {
        let thir = self.frame.thir;
        // Whether `e` is the `bool` `value` written, in a block of it alone.
        let literal = |mut e: thir::ExprId, value: bool| loop {
            e = body_queries::strip(thir, e);
            match &thir[e].kind {
                thir::ExprKind::Block { block } if thir[*block].stmts.is_empty() => match thir[*block].expr {
                    Some(inner) => e = inner,
                    None => return false,
                },
                thir::ExprKind::Literal { lit, neg: false } => {
                    return matches!(lit.node, rustc_ast::LitKind::Bool(b) if b == value);
                }
                _ => return false,
            }
        };
        (thir.exprs.iter())
            .filter_map(|e| {
                let thir::ExprKind::Unary { op: UnOp::Not, arg } = e.kind else {
                    return None;
                };
                let arg = &thir[body_queries::strip(thir, arg)];
                let and = match arg.kind {
                    thir::ExprKind::LogicalOp { op, .. } => matches!(op, thir::LogicalOp::And),
                    thir::ExprKind::If {
                        else_opt: Some(otherwise),
                        ..
                    } if literal(otherwise, false) => true,
                    thir::ExprKind::If {
                        then,
                        else_opt: Some(_),
                        ..
                    } if literal(then, true) => false,
                    _ => return None,
                };
                Some((self.js_span(arg.span), and))
            })
            .collect()
    }
}
