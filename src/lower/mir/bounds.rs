//! What an index the body's ranges show in bounds is (ADRs 0292, 0576):
//! an `Assert` not made, `v[i]` read as JS reads it, and its place
//! through `index_mut` unchecked, a `swap` JS's own.

use super::*;
use crate::lower::combinators::Comb;
use rustc_hir::attrs::lang_items::LangItem;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether an `Assert` at `span` checks an index THIR's facts know is in
    /// bounds: the index there, `a[i]`, is among them.
    pub(super) fn known_in_bounds(&self, msg: &mir::AssertMessage<'tcx>, span: Span) -> bool {
        matches!(msg, mir::AssertKind::BoundsCheck { .. })
            && (self.frame.thir.exprs.iter_enumerated()).any(|(id, e)| {
                matches!(e.kind, thir::ExprKind::Index { .. })
                    && e.span == span
                    && self.frame.facts.in_bounds.contains(&id)
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
        indexed && self.indexes_in_bounds(def_id, span)
    }

    /// Whether THIR's call of `def_id` at `span` has its indexes in bounds,
    /// as the body's ranges show: of `&v[a..b]`, its range (ADR 0806).
    pub(super) fn indexes_in_bounds(&self, def_id: rustc_span::def_id::DefId, span: Span) -> bool {
        (self.thir_call(def_id, span)).is_some_and(|(fun, _)| self.frame.facts.in_bounds.contains(&fun))
    }

    /// Whether a call is `v[i]` through `index_mut`, of a value JS can't
    /// change in place: the item's place, not its value (ADR 0099).
    pub(super) fn index_mut_place(&self, known: Std, def_id: rustc_span::def_id::DefId, output: ty::Ty<'tcx>) -> bool {
        known == Std::Index
            && (self.tcx.trait_of_assoc(def_id)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::IndexMut))
            && matches!(output.kind(), ty::Ref(_, item, _) if self.is_cell_pointee(*item))
    }

    /// An index's call: unchecked where `call_in_bounds` knows it,
    /// `items[index]`; `[v[i], v[j]] = [v[j], v[i]]`, JS's own swap; and of
    /// `v.swap_remove(i)`, the last swapped in, then popped. Of
    /// `index_mut_place`'s, the item's place, checked first as `$index`
    /// checks it where it isn't known.
    pub(super) fn index_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        known: Std,
        (def_id, output, items_ty): (rustc_span::def_id::DefId, ty::Ty<'tcx>, ty::Ty<'tcx>),
        values: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        if self.index_mut_place(known, def_id, output) {
            let checked = !self.indexes_in_bounds(def_id, span);
            let mut exprs = (values.into_iter())
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let index = exprs.pop().expect("an index");
            let items = exprs.pop().expect("the items");
            self.flush(state, out)?;
            let items = if items.reads_same() {
                items
            } else {
                self.spill("items", items, out)
            };
            // A variable that reads the same wherever it's read, a parameter,
            // read where it's set too.
            let index = if index.is_constant()
                || matches!(&index.kind, js::ExprKind::Var(v) if self.locals.mir.alike.contains(v))
            {
                index
            } else {
                self.spill("index", index, out)
            };
            if checked {
                let check = self.index_checked(items.clone(), index.clone(), items_ty);
                out.push(StmtKind::Expr(check).at(self.js_span(span)));
            }
            return Ok(Value::Ref(Expr::index(items, index)));
        }
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
