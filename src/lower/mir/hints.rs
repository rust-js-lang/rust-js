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

    /// Each temporary MIR made, `_5`, by the names a Rust developer's
    /// tooling suggests for it, rust-analyzer's, first to last (ADR 0569):
    /// the method or function that made it, `trailingZeros` of
    /// `a.trailing_zeros()`, `uppercase` of `s.to_uppercase()`, but for one
    /// that names nothing, `unwrap`, `clone`, `new`, `from_str`, and one the
    /// source doesn't call by its name, a macro's, an operator's or a
    /// closure's; then the program's type it holds, `point` of an
    /// `Option<Point>`. One that takes or copies another temporary, `&_5`
    /// as `format_args!` takes what it shows, has that one's names.
    pub(super) fn temporaries_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> std::collections::HashMap<String, Vec<String>> {
        // Each call the source writes by its name, `a.trailing_zeros()`: not
        // one a macro writes, `vec![..]`'s, nor an operator's, `a[i]`'s, nor a
        // closure's, `f(x)`'s `call_mut`.
        let source = self.tcx.sess.source_map();
        let mut called: std::collections::HashMap<Local, rustc_span::def_id::DefId> = std::collections::HashMap::new();
        let mut taken_from: std::collections::HashMap<Local, Local> = std::collections::HashMap::new();
        for data in mir_body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Some(local) = assign.0.as_local()
                    && let Rvalue::Ref(_, _, from) | Rvalue::Use(Operand::Copy(from) | Operand::Move(from), ..) =
                        &assign.1
                    && let Some(from) = from.as_local()
                {
                    taken_from.insert(local, from);
                }
            }
            if let TerminatorKind::Call {
                func,
                destination,
                fn_span,
                ..
            } = &data.terminator().kind
                && destination.projection.is_empty()
                && let Some((def_id, _)) = func.const_fn_def()
                && !fn_span.from_expansion()
                && let Ok(written) = source.span_to_snippet(*fn_span)
                && written.starts_with(self.tcx.item_name(def_id).as_str())
            {
                called.insert(destination.local, def_id);
            }
        }
        (state.locals.names.iter_enumerated())
            .filter(|(_, name)| super::tidy::is_temporary(name))
            .map(|(local, name)| {
                // What it was taken from, as far as that goes.
                let mut at = local;
                for _ in 0..mir_body.local_decls.len() {
                    match taken_from.get(&at) {
                        Some(&from) if from != at && !called.contains_key(&at) => at = from,
                        _ => break,
                    }
                }
                let suggested = [
                    called.get(&at).and_then(|&def_id| self.call_name(def_id)),
                    self.held_type_name(mir_body.local_decls[local].ty),
                ];
                let names = (suggested.into_iter().flatten())
                    .map(|name| variable_name(&crate::lower::camel_case(&name)))
                    .filter(|name| crate::names::js_ident(name) == *name)
                    .collect();
                (name.clone(), names)
            })
            .collect()
    }

    /// What rust-analyzer names a call's result: a method's name without a
    /// conversion's prefix, `uppercase` of `to_uppercase`, and none of a
    /// method that names nothing; a function's without a constructor's
    /// prefix, `config` of `from_config`; and none of a name that's no
    /// noun of what it gives, `new`, `default`, `str`.
    fn call_name(&self, def_id: rustc_span::def_id::DefId) -> Option<String> {
        const USELESS_METHODS: &[&str] = &[
            "to_string",
            "as_str",
            "to_owned",
            "as_ref",
            "clone",
            "cloned",
            "expect",
            "expect_none",
            "unwrap",
            "unwrap_none",
            "unwrap_or",
            "unwrap_or_default",
            "unwrap_or_else",
            "unwrap_unchecked",
            "iter",
            "into_iter",
            "iter_mut",
            "into_future",
            // And the `Err` side of what rust-analyzer's list has, as it names
            // nothing either.
            "unwrap_err",
            "expect_err",
        ];
        const USELESS_NAMES: &[&str] = &[
            "new", "default", "option", "some", "none", "ok", "err", "str", "string", "from", "into",
        ];
        let name = self.tcx.item_name(def_id);
        let name = name.as_str();
        let method = self
            .tcx
            .opt_associated_item(def_id)
            .is_some_and(|item| item.is_method());
        let name = match method {
            true if USELESS_METHODS.contains(&name) => return None,
            true => ["into_", "as_", "to_"]
                .iter()
                .find_map(|p| name.strip_prefix(p))
                .unwrap_or(name),
            false => ["from_", "with_", "into_"]
                .iter()
                .find_map(|p| name.strip_prefix(p))
                .unwrap_or(name),
        };
        let plain = name.starts_with(|c: char| c.is_ascii_lowercase()) && !name.starts_with('_');
        let keyword = rustc_span::symbol::Ident::from_str(name).is_reserved();
        (plain && !keyword && !USELESS_NAMES.contains(&name)).then(|| name.to_string())
    }

    /// The name a value of `ty` is kept by, its type's, `order` of an
    /// `Order`, through a reference and what JS holds as the value it
    /// holds, an `Option`, a `Box`, an `Rc`: of the program's own or a
    /// library's type, `None` of std's. Not a `Result`'s `Ok`, as
    /// rust-analyzer's is: JS holds a `Result` as `{ TAG, _0 }`, which
    /// `order.TAG` would misname.
    fn held_type_name(&self, ty: Ty<'tcx>) -> Option<String> {
        let mut ty = ty.peel_refs();
        loop {
            ty = match ty.kind() {
                _ if let Some(inner) = self.option_of(ty) => inner.peel_refs(),
                ty::Adt(_, args) if ty.is_box() || self.is_std_type(ty, StdItem::Rc) => args.type_at(0).peel_refs(),
                _ => break,
            };
        }
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if crate::lower::recognition::is_std_crate(self.tcx, adt.did()) {
            return None;
        }
        Some(self.tcx.item_name(adt.did()).to_string())
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

/// A type's name as a variable's, lower camel case, as JS names one:
/// `order` of `Order`, `httpServer` of `HTTPServer`, `url` of `URL`.
fn variable_name(type_name: &str) -> String {
    let chars: Vec<char> = type_name.chars().collect();
    // The capitals it starts with, but for one that starts the next word.
    let run = chars.iter().take_while(|c| c.is_ascii_uppercase()).count();
    let lowered = match run > 1 && run < chars.len() {
        true => run - 1,
        false => run,
    };
    (chars.iter().enumerate())
        .map(|(i, c)| match i < lowered.max(1) {
            true => c.to_ascii_lowercase(),
            false => *c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::variable_name;

    // A type's name as a variable's: its first capital lowered, a run of them
    // too, but for the one that starts the next word.
    #[test]
    fn a_type_name_is_a_variable_name() {
        assert_eq!(variable_name("Order"), "order");
        assert_eq!(variable_name("HTTPServer"), "httpServer");
        assert_eq!(variable_name("URL"), "url");
        assert_eq!(variable_name("IOError"), "ioError");
        assert_eq!(variable_name("trailingZeros"), "trailingZeros");
    }
}
