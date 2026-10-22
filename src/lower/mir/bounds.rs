//! What an index the body's ranges show in bounds is (ADRs 0292, 0576):
//! an `Assert` not made, `v[i]` read as JS reads it, a `swap` JS's own.

use super::*;
use crate::lower::combinators::Comb;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether an `Assert` at `span` checks an index THIR's facts know is in
    /// bounds: the index there, `a[i]`, is among them.
    pub(super) fn known_in_bounds(&self, msg: &mir::AssertMessage<'tcx>, span: Span) -> bool {
        matches!(msg, mir::AssertKind::BoundsCheck { .. })
            && (self.thir.exprs.iter_enumerated()).any(|(id, e)| {
                matches!(e.kind, thir::ExprKind::Index { .. })
                    && e.span == span
                    && self.body_facts.in_bounds.contains(&id)
            })
    }

    /// Whether THIR's call of `def_id` at `span` is one whose indexes the
    /// body's ranges show in bounds: a `Vec`'s `v[i]`, `v[0]` after
    /// `v.is_empty()`'s test fails, `swap(i, j)` or `swap_remove(i)`.
    pub(super) fn call_in_bounds(
        &self,
        known: Std,
        def_id: rustc_span::def_id::DefId,
        values: &[Value<'tcx>],
        span: Span,
    ) -> bool {
        let indexed = match known {
            Std::Index => matches!(values, [Value::Expr(_) | Value::Place(_), Value::Expr(_)]),
            Std::Comb(Comb::Swap | Comb::SwapRemove) => true,
            _ => false,
        };
        indexed
            && self
                .thir_call(def_id, span)
                .is_some_and(|(fun, _)| self.body_facts.in_bounds.contains(&fun))
    }

    /// A call `call_in_bounds` knows, unchecked: `items[index]`; `[v[i],
    /// v[j]] = [v[j], v[i]]`, JS's own swap; and of `v.swap_remove(i)`, the
    /// last swapped in, then popped.
    pub(super) fn unchecked_call(
        &mut self,
        known: Std,
        values: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let mut exprs = Vec::new();
        for (value, name) in values.into_iter().zip(["items", "i", "j"]) {
            let e = self.value_expr(value, span)?;
            exprs.push(match known {
                Std::Index => e,
                _ if e.reads_same() => e,
                _ => self.spill(name, e, out),
            });
        }
        let items = exprs.remove(0);
        let at = |index: &Expr| Expr::index(items.clone(), index.clone());
        let (i, j) = match known {
            Std::Index => return Ok(Value::Expr(at(&exprs[0]))),
            Std::Comb(Comb::Swap) => (exprs.remove(0), exprs.remove(0)),
            _ => {
                let last = Expr::bin(Op::Sub, Expr::member(items.clone(), "length"), Expr::int(1));
                (exprs.remove(0), last)
            }
        };
        let swap = StmtKind::Assign(Expr::array(vec![at(&i), at(&j)]), Expr::array(vec![at(&j), at(&i)]));
        out.push(swap.at(self.js_span(span)));
        Ok(Value::Expr(match known {
            Std::Comb(Comb::Swap) => Expr::undefined(),
            _ => Expr::call(Expr::member(items, "pop"), Vec::new()),
        }))
    }
}
