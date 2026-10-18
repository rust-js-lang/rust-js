//! What tidy is told of a body (`tidy::Hints`) that its THIR says: where
//! the program writes each `if`'s condition, each `&&` and `||`, each `if`
//! of a `continue`, each loop and whether as a `while`, and each `match`
//! that may read a table. A question module: it emits nothing.

use super::*;

/// Where the program writes what tidy reads a body's JS by.
pub(super) struct Written {
    /// Each `match` whose arms are each a variant of its own, with no field,
    /// guard or binding, and its variants' names (ADR 0233).
    pub tables: Vec<(js::Span, Vec<String>)>,
    /// Each `if`'s condition, and a `match` and the guard of an arm of it.
    pub conditions: Vec<(js::Span, js::Span)>,
    /// Each `&&` or `||` the program wrote.
    pub logicals: Vec<js::Span>,
    /// Each `if` of a `continue`, alone or ending what it does.
    pub continues: Vec<js::Span>,
    /// Each loop, whether as a `while`, and its label.
    pub loops: Vec<(js::Span, bool, Option<String>)>,
    /// Each comment in the body (ADR 0444), which a branch it's in keeps
    /// as a statement (`tidy::commented`).
    pub comments: Vec<js::Span>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Where the program writes what tidy reads the body's JS by, of the
    /// body THIR has.
    pub(super) fn written(&self) -> Written {
        // Where each `match` is written whose arms are each a variant of its
        // own, with no field, guard or binding: one that may read a table by
        // what's matched (ADR 0233), as tidy's `table_reads` tells.
        let tables: Vec<(js::Span, Vec<String>)> = (self.thir.exprs.iter())
            .filter_map(|e| {
                let thir::ExprKind::Match { scrutinee, ref arms, .. } = e.kind else {
                    return None;
                };
                let ty::Adt(adt, _) = self.thir[scrutinee].ty.peel_refs().kind() else {
                    return None;
                };
                let each = (arms.iter()).all(|&arm| {
                    let arm = &self.thir[arm];
                    arm.guard.is_none()
                        && matches!(&arm.pattern.kind, thir::PatKind::Variant { subpatterns, .. } if subpatterns.is_empty())
                });
                // Each variant's name too, to know the one an `else` is of.
                let names = adt.variants().iter().map(|v| bindings::variant_name(self.tcx, v)).collect();
                (adt.is_enum() && arms.len() >= 2 && each).then(|| (self.js_span(e.span), names))
            })
            .collect();
        // Where each `if`'s condition is written, of the body THIR has:
        // tests in one are `&&`'s; and a `match` and the guard of an arm of
        // it, whose pattern's test and the guard are.
        let conditions: Vec<(js::Span, js::Span)> = (self.thir.exprs.iter())
            .flat_map(|e| match e.kind {
                thir::ExprKind::If { cond, .. } => {
                    let cond = self.js_span(self.thir[cond].span);
                    vec![(cond, cond)]
                }
                // From `match` to its scrutinee's end, where MIR tests it.
                thir::ExprKind::Match {
                    scrutinee, ref arms, ..
                } => (arms.iter())
                    .filter_map(|&arm| {
                        let guard = self.thir[arm].guard?;
                        let tested = js::Span {
                            lo: self.js_span(e.span).lo,
                            hi: self.js_span(self.thir[scrutinee].span).hi,
                        };
                        Some((tested, self.js_span(self.thir[guard].span)))
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .collect();
        // Where each `&&` or `||` the program wrote is.
        let logicals: Vec<js::Span> = (self.thir.exprs.iter())
            .filter(|e| matches!(e.kind, thir::ExprKind::LogicalOp { .. }))
            .map(|e| self.js_span(e.span))
            .collect();
        // Where each `if` the program wrote of a `continue` tests: one alone,
        // or one that ends what it does.
        let continues: Vec<js::Span> = (self.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If { cond, then, .. } => {
                    let mut then = then;
                    while let thir::ExprKind::Scope { value: inner, .. }
                    | thir::ExprKind::NeverToAny { source: inner } = self.thir[then].kind
                    {
                        then = inner;
                    }
                    let thir::ExprKind::Block { block } = self.thir[then].kind else {
                        return None;
                    };
                    let block = &self.thir[block];
                    let only = match (block.stmts.last(), block.expr) {
                        (_, Some(e)) => Some(e),
                        (Some(s), None) => match self.thir[*s].kind {
                            thir::StmtKind::Expr { expr, .. } => Some(expr),
                            _ => None,
                        },
                        _ => None,
                    }?;
                    let mut only = only;
                    while let thir::ExprKind::Scope { value: inner, .. }
                    | thir::ExprKind::NeverToAny { source: inner } = self.thir[only].kind
                    {
                        only = inner;
                    }
                    matches!(self.thir[only].kind, thir::ExprKind::Continue { .. })
                        .then(|| self.js_span(self.thir[cond].span))
                }
                _ => None,
            })
            .chain(self.let_else_continues())
            .collect();
        // Where each loop is written, and whether as a `while`.
        // Each loop's label the program wrote, `'rows`, by the loop.
        let labels: std::collections::HashMap<thir::ExprId, String> = (self.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::Scope { value, hir_id, .. } => match self.tcx.hir_expect_expr(hir_id).kind {
                    // The loop it scopes, in scopes of its own too.
                    rustc_hir::ExprKind::Loop(_, Some(label), ..) => Some((
                        self.strip(value),
                        label.ident.name.as_str().trim_start_matches('\'').to_string(),
                    )),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let loops: Vec<(js::Span, bool, Option<String>)> = (self.thir.exprs.iter_enumerated())
            .filter_map(|(id, e)| match e.kind {
                thir::ExprKind::Loop { .. } => {
                    let written = e.span.source_callsite();
                    // Where it's written, not the body it's in, which a closure's
                    // span holds too.
                    let whiles = (self.thir.exprs.iter()).any(|w| {
                        w.span.is_desugaring(rustc_span::DesugaringKind::WhileLoop)
                            && w.span.source_callsite().source_equal(written)
                    });
                    Some((self.js_span(written), whiles, labels.get(&id).cloned()))
                }
                _ => None,
            })
            .collect();
        // The body's own, between where its code starts and ends.
        let spans: Vec<js::Span> = (self.thir.exprs.iter())
            .map(|e| self.js_span(e.span))
            .filter(|s| !s.is_none())
            .collect();
        let (lo, hi) = (spans.iter().map(|s| s.lo).min(), spans.iter().map(|s| s.hi).max());
        let comments = (self.krate.sources.output.files.iter())
            .flat_map(|file| &file.comments)
            .filter(|c| lo.is_some_and(|lo| lo <= c.lo) && hi.is_some_and(|hi| c.hi <= hi))
            .map(|c| js::Span { lo: c.lo, hi: c.hi })
            .collect();
        Written {
            tables,
            conditions,
            logicals,
            continues,
            loops,
            comments,
        }
    }

    /// Where each `let … else { continue; }` the program wrote is, which MIR
    /// tests as an `if` of the `continue` in its place.
    fn let_else_continues(&self) -> Vec<js::Span> {
        (self.thir.stmts.iter())
            .filter_map(|stmt| match stmt.kind {
                thir::StmtKind::Let {
                    else_block: Some(block),
                    span,
                    ..
                } => {
                    let block = &self.thir[block];
                    let mut only = match (block.stmts.last(), block.expr) {
                        (_, Some(e)) => e,
                        (Some(s), None) => match self.thir[*s].kind {
                            thir::StmtKind::Expr { expr, .. } => expr,
                            _ => return None,
                        },
                        _ => return None,
                    };
                    while let thir::ExprKind::Scope { value: inner, .. }
                    | thir::ExprKind::NeverToAny { source: inner } = self.thir[only].kind
                    {
                        only = inner;
                    }
                    matches!(self.thir[only].kind, thir::ExprKind::Continue { .. }).then(|| self.js_span(span))
                }
                _ => None,
            })
            .collect()
    }

    /// The names each of whose locals is a `bool`, a shared reference to one
    /// too, which JS reads as it, of a closure what it captures too; and the
    /// names each of whose locals is an integer.
    pub(super) fn typed_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> (std::collections::HashSet<String>, std::collections::HashSet<String>) {
        // A name each of whose locals is a `bool`, and one each of whose
        // locals is an integer.
        let mut booleans = std::collections::HashSet::new();
        let mut others = std::collections::HashSet::new();
        let mut numbers = std::collections::HashSet::new();
        let mut not_numbers = std::collections::HashSet::new();
        for (local, name) in state.locals.names.iter_enumerated().filter(|(_, n)| !n.is_empty()) {
            // A shared reference to one too, `*show_hint`'s, which JS reads as it.
            let ty = mir_body.local_decls[local].ty;
            match ty.is_integral() {
                true => numbers.insert(name.clone()),
                false => not_numbers.insert(name.clone()),
            };
            let bool = match ty.kind() {
                ty::Ref(_, inner, ty::Mutability::Not) => inner.is_bool(),
                _ => ty.is_bool(),
            };
            match bool {
                true => booleans.insert(name.clone()),
                false => others.insert(name.clone()),
            };
        }
        // And a closure's, of what it captures: its parent's.
        let env = mir_body
            .args_iter()
            .next()
            .map(|local| mir_body.local_decls[local].ty.peel_refs());
        if let Some(ty::Closure(_, args)) = env.map(|ty| ty.kind()) {
            for (ty, (capture, _)) in args.as_closure().upvar_tys().iter().zip(&state.captures) {
                if let js::ExprKind::Var(name) = &capture.kind {
                    match ty.peel_refs().is_bool() {
                        true => booleans.insert(name.clone()),
                        false => others.insert(name.clone()),
                    };
                }
            }
        }
        booleans.retain(|name| !others.contains(name));
        numbers.retain(|name| !not_numbers.contains(name));
        (booleans, numbers)
    }
}
