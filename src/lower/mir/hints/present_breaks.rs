//! Where each `let Some(x) = v.get(i) else { break; }` is written
//! (`Written::present_breaks`, rd260).

use super::*;

impl FnCx<'_, '_> {
    /// Each `let Some(x) = v.get(i) else { break; }` the body writes, of
    /// items that take up room (`body_queries::present_item_of_sized`):
    /// where it breaks, `i` is the slice's length or past it.
    pub(super) fn present_breaks(&self) -> Vec<js::Span> {
        // A `break;` alone, the `else` of a `let`, as written.
        let breaks_alone = |block: thir::BlockId| {
            let block = &self.frame.thir[block];
            let only = match (&block.stmts[..], block.expr) {
                ([], Some(e)) => Some(e),
                ([s], None) => match self.frame.thir[*s].kind {
                    thir::StmtKind::Expr { expr, .. } => Some(expr),
                    _ => None,
                },
                _ => None,
            };
            only.is_some_and(|e| {
                matches!(
                    self.frame.thir[body_queries::strip(self.frame.thir, e)].kind,
                    thir::ExprKind::Break { value: None, .. }
                )
            })
        };
        (self.frame.thir.stmts.iter())
            .filter_map(|stmt| match stmt.kind {
                thir::StmtKind::Let {
                    else_block: Some(block),
                    initializer: Some(init),
                    ref pattern,
                    span,
                    ..
                } if breaks_alone(block)
                    && body_queries::present_item_of_sized(self.tcx, self.frame.thir, pattern, init) =>
                {
                    Some(self.js_span(span))
                }
                _ => None,
            })
            .collect()
    }
}
