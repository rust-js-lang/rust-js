//! A slice's sorts: which of std's sorts an item's type picks, and the
//! calls of the runtime's that run them (ADRs 0342, 0471).

use rustc_middle::ty;
use rustc_middle::ty::TypeVisitableExt;
use rustc_span::Span;

use super::slice::{SliceOp, SortWith};
use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::recognition::TypeFact;
use crate::lower::{FnCx, R};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What std's unstable sort picks by an item's type, of a type rustc
    /// knows the layout of (ADR 0342): its small sort, by whether it's
    /// `Freeze` and `Copy` and its size, and a Hoare partition of one of more
    /// than 96 bytes. A type parameter's is its caller's (ADR 0471).
    pub(in crate::lower) fn sort_kind(&self, item: ty::Ty<'tcx>, span: Span) -> R<&'static str> {
        if item.has_param() {
            return Err(self.unsupported(
                span,
                "an unstable sort of items that hold a type parameter, whose order std's algorithm picks by their layout",
            ));
        }
        let layout = self
            .layout(item)
            .ok_or_else(|| self.unsupported(span, "an unstable sort of these items"))?;
        let size = layout.size.bytes();
        let freeze = item.is_freeze(self.tcx, self.typing_env);
        Ok(if freeze && self.is_copy(item) && size <= 8 {
            "network"
        } else if freeze && size * 48 <= 4096 {
            if size <= 16 { "general8" } else { "general" }
        } else if size <= 96 {
            "fallback"
        } else {
            "hoare"
        })
    }

    /// What std's stable sort picks by an item's type (#925): its small
    /// sort, `"general8"`, sorting eight at a time, for a `Freeze` one of at
    /// most 16 bytes, `"general"` for a larger `Freeze` one, else
    /// `"fallback"`, insertion. A type parameter's is its caller's,
    /// `TStableSort`.
    pub(in crate::lower) fn stable_sort_kind(&self, item: ty::Ty<'tcx>, span: Span) -> R<&'static str> {
        if item.has_param() {
            return Err(self.unsupported(
                span,
                "a stable sort of items that hold a type parameter, whose comparison can be seen, which std's algorithm picks by their type",
            ));
        }
        let layout = (self.layout(item)).ok_or_else(|| self.unsupported(span, "a stable sort of these items"))?;
        Ok(self.stable_sort_kind_of(item, layout.size.bytes()))
    }

    /// `stable_sort_kind`'s, of an item of `size` bytes.
    fn stable_sort_kind_of(&self, item: ty::Ty<'tcx>, size: u64) -> &'static str {
        match item.is_freeze(self.tcx, self.typing_env) {
            true if size <= 16 => "general8",
            true => "general",
            false => "fallback",
        }
    }

    /// Whether a stable sort is std's own, step for step, not JS's `sort`,
    /// which asks another sequence and, of an order that isn't total, leaves
    /// another (#925, #936): where its calls can be seen, or its order may
    /// not be total. Of a type parameter's items, its caller's type's,
    /// always; of its items' order, `order_seen`'s and `order_total`'s; of a
    /// comparator, `call_seen`'s and `call_total`'s; of a key, its
    /// function's, its order's, or its drop's, which runs after each
    /// comparison. Of `sort`'s, `sort_by`'s or `sort_by_key`'s `generic_args`.
    pub(in crate::lower) fn stable_sort_is_std(&self, with: SortWith, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        let item = generic_args.type_at(0);
        match with {
            // A type parameter's items, its caller's type's, which may be seen.
            _ if self.is_unknown(item) => true,
            // One whose `PartialOrd` writes its own `lt`, which no `Ordering`
            // JS's `sort` is given asks, isn't `order_total`'s, which reads
            // each method an impl writes (#937).
            SortWith::Order => self.order_seen(item) || !self.order_total(item),
            // One by UTF-16 units, JS's own order of strings, asks nothing
            // a program sees (ADR 0945).
            SortWith::By => {
                let compare = generic_args.type_at(1);
                (self.call_seen(compare) || !self.call_total(compare)) && !self.sorts_as_js_strings(compare, item)
            }
            SortWith::Key => {
                let key = generic_args.type_at(1);
                let order = self.order_seen(key) || !self.order_total(key);
                self.call_seen(generic_args.type_at(2)) || order || self.has_drops(key)
            }
        }
    }

    /// `$sortStable(items, isLess, kind, size)`: std's stable sort (#925),
    /// given `is_less` as std's sort of `with` asks it, and what std picks
    /// by the item's type and its size, which sizes its scratch.
    pub(in crate::lower) fn stable_sort_call(
        &mut self,
        with: SortWith,
        values: Vec<Expr>,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let item = generic_args.type_at(0);
        let (kind, size) = self.stable_sort_facts(item, span)?;
        let mut values = values.into_iter();
        let items = values.next().expect("the items");
        let key_ty = (with == SortWith::Key).then(|| generic_args.type_at(1));
        let is_less = self.sort_is_less(with, values.next(), item, key_ty, span, out)?;
        Ok(Expr::call(
            Expr::helper("$sortStable"),
            vec![items, is_less, kind, size],
        ))
    }

    /// What std's stable sort picks by an `item`'s type, and its size, as
    /// `$sortStable` is given them: of a type whose layout is known here,
    /// though it holds a type parameter behind a reference, `(&T, usize)`,
    /// its own; else its caller's, `TStableSort` and `TSize` (#925).
    pub(in crate::lower) fn stable_sort_facts(&mut self, item: ty::Ty<'tcx>, span: Span) -> R<(Expr, Expr)> {
        Ok(match self.tcx.layout_of(self.typing_env.as_query_input(item)) {
            Ok(layout) if item.has_param() => (
                Expr::str(self.stable_sort_kind_of(item, layout.size.bytes())),
                Expr::int(layout.size.bytes().into()),
            ),
            _ => (
                self.type_fact_value(item, TypeFact::StableSort, span)?,
                self.type_fact_value(item, TypeFact::Size, span)?,
            ),
        })
    }

    /// std's `is_less` of a sort of `item`s by `with`, given its comparator
    /// or key: `(a, b) => a < b` by their `PartialOrd::lt`; a comparator's
    /// `compare(a, b) < 0`, of a closure of one expression in place, `(a,
    /// b) => $cmp(b, a) < 0`; a key's `key(a) < key(b)`, by its `lt`.
    fn sort_is_less(
        &mut self,
        with: SortWith,
        by: Option<Expr>,
        item: ty::Ty<'tcx>,
        key_ty: Option<ty::Ty<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut body = Vec::new();
        let (a, b) = (Expr::var("a"), Expr::var("b"));
        let less = match with {
            SortWith::Order => self.operator_value(Op::Lt, a, b, item, span, &mut body)?,
            SortWith::By => {
                let compare = by.expect("rustc checked the arguments");
                if let js::ExprKind::Arrow(params, stmts) = &compare.kind
                    && params.len() == 2
                    && let [
                        Stmt {
                            kind: StmtKind::Return(Some(order)),
                            ..
                        },
                    ] = stmts.as_slice()
                {
                    let less = Expr::bin(Op::Lt, order.clone(), Expr::int(0));
                    let returned = vec![StmtKind::Return(Some(less)).at(js::Span::NONE)];
                    return Ok(Expr::arrow(params.clone(), returned));
                }
                let compare = self.named_once("compare", compare, out);
                Expr::bin(Op::Lt, Expr::call(compare, vec![a, b]), Expr::int(0))
            }
            SortWith::Key => {
                let key = self.named_once("key", by.expect("rustc checked the arguments"), out);
                let key_ty = key_ty.expect("`_by_key` names its key");
                let (a, b) = (Expr::call(key.clone(), vec![a]), Expr::call(key, vec![b]));
                self.operator_value(Op::Lt, a, b, key_ty, span, &mut body)?
            }
        };
        body.push(StmtKind::Return(Some(less)).at(js::Span::NONE));
        Ok(Expr::arrow(vec!["a".into(), "b".into()], body))
    }

    /// `$sortUnstable(items, isLess, kind)`, or `$selectNthUnstable` of an
    /// `index`, a handle on a number or text it gives where `item` is one.
    fn unstable_sort_call(
        &mut self,
        op: SliceOp,
        items: Expr,
        index: Option<Expr>,
        is_less: Expr,
        kind: Expr,
        item: ty::Ty<'tcx>,
    ) -> Expr {
        let select = matches!(op, SliceOp::SelectNth { .. });
        let mut list = vec![items];
        list.extend(index);
        list.extend([is_less, kind]);
        if select && self.handles_mut(item) {
            list.push(Expr::bool(true));
        }
        let name = if select { "$selectNthUnstable" } else { "$sortUnstable" };
        Expr::call(Expr::helper(name), list)
    }

    /// An unstable sort or a selection, `v.sort_unstable_by(f)`, of its
    /// operands as JS: JS's stable sort where ties can't be told apart, else
    /// std's own, picked by the item's type (ADR 0342).
    pub(in crate::lower) fn unstable_sort_values(
        &mut self,
        op: SliceOp,
        with: SortWith,
        values: Vec<Expr>,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let item = generic_args.type_at(0);
        let kind = self.type_fact_value(item, TypeFact::Sort, span)?;
        let mut values = values.into_iter();
        let mut next = || values.next().expect("rustc checked the arguments");
        let items = next();
        let index = matches!(op, SliceOp::SelectNth { .. }).then(&mut next);
        // By UTF-16 units, JS's own order of strings, whose ties are the
        // same string, which no order of them tells apart (ADR 0945).
        if matches!(op, SliceOp::SortUnstable { .. })
            && with == SortWith::By
            && self.sorts_as_js_strings(generic_args.type_at(1), item)
        {
            return Ok(Expr::call(Expr::member(items, "sort"), vec![]));
        }
        let by = (with != SortWith::Order).then(next);
        let key_ty = (with == SortWith::Key).then(|| generic_args.type_at(1));
        let is_less = self.sort_is_less(with, by, item, key_ty, span, out)?;
        Ok(self.unstable_sort_call(op, items, index, is_less, kind, item))
    }
}
