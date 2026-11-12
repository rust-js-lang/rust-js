//! A write of a place a raw pointer is taken to or into, an object written
//! whole again (#1484): over what it holds, `$assign(p, v)`, each object
//! inside it a pointer is into written over first and put back in the
//! value; made, its first; or refused. Which writes and how is
//! `pointers/rewritten.rs`'s. Split from `assign.rs` by concept (#1334).

use super::*;
use crate::lower::pointers::{Rewritten, within_items};
use crate::lower::temporary_names::Role;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value` written to `place` where `rewritten` says how: written, none;
    /// the value back where it's written as any other write is.
    pub(super) fn rewritten_write(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        value: Value<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let rewritten = if within_items(self.tcx, state.body, &place) {
            state.facts.rewritten(state.location()).cloned()
        } else {
            None
        };
        let Some((how, kept)) = rewritten else {
            return Ok(Some(value));
        };
        if how == Rewritten::Made {
            return Ok(Some(value));
        }
        let value = self.value_expr(value, span)?;
        self.flush(state, out)?;
        let js_span = self.js_span(span);
        let target = self.mir_place(state, place, span, out)?;
        // Where it may hold none, its value read once, as both ways read it.
        let value = match how == Rewritten::Tested && !value.reads_same() {
            true => self.spill(Role::Value, value, out),
            false => value,
        };
        let mut written = Vec::new();
        // Each object inside it a pointer is into, deepest first: written
        // over by what the value holds there, and put back in the value, so
        // the write over the whole keeps it, `$assign(s.f, value.f); value.f
        // = s.f;`.
        let value_in_place = match kept.is_empty() {
            true => value.clone(),
            false => {
                // Held as any value read more than once is, which #890's
                // namer names.
                let held = match value.reads_same() {
                    true => value.clone(),
                    false => self.spill(Role::Value, value.clone(), out),
                };
                for inner in kept {
                    let inner_js = self.mir_place(state, inner, span, &mut written)?;
                    let steps = inner.projection.len() - place.projection.len();
                    let indexed = !(place.projection.iter()).all(|elem| matches!(elem, PlaceElem::Field(..)));
                    let Some(of_value) = path_from(&target, &inner_js, steps, indexed, held.clone()) else {
                        return Err(refuse(
                            self.tcx,
                            span,
                            Refused::Memory(
                                "a place whose address is taken inside what's assigned whole, reached by no field JS names (#1504)",
                            ),
                        ));
                    };
                    written.push(
                        StmtKind::Expr(Expr::call_helper("$assign", vec![inner_js.clone(), of_value.clone()]))
                            .at(js_span),
                    );
                    written.push(StmtKind::Assign(of_value, inner_js).at(js_span));
                }
                held
            }
        };
        written.push(StmtKind::Expr(Expr::call_helper("$assign", vec![target.clone(), value_in_place])).at(js_span));
        match how {
            // `if (m === undefined) m = v; else $assign(m, v);`
            Rewritten::Tested => out.push(
                StmtKind::If(
                    Expr::bin(Op::Eq, target.clone(), Expr::undefined()),
                    vec![StmtKind::Assign(target, value).at(js_span)],
                    Some(written),
                )
                .at(js_span),
            ),
            _ => out.extend(written),
        }
        Ok(None)
    }
}

/// What `inner`, a place reached from `outer` by `steps` fields,
/// `outer.f.g`, is of `value`: `value.f.g`. Its last `steps` are each a
/// key; what they're of is `outer`, the same path, or, of an `indexed`
/// `outer`, `a[i]`, an item read again, its index a temporary of its own.
/// None where it's reached otherwise.
fn path_from(outer: &Expr, inner: &Expr, steps: usize, indexed: bool, value: Expr) -> Option<Expr> {
    if steps == 0 {
        return (indexed || js::same_path(outer, inner)).then_some(value);
    }
    match &inner.kind {
        js::ExprKind::Member(of, key) => Some(Expr::member(path_from(outer, of, steps - 1, indexed, value)?, key)),
        js::ExprKind::Index(of, at) => Some(Expr::index(
            path_from(outer, of, steps - 1, indexed, value)?,
            (**at).clone(),
        )),
        _ => None,
    }
}
