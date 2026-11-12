//! A B-tree's own: which keys are std's own nodes and what their calls
//! are given (#926, ADR 0404), its `range` and its `extract_if` (ADR 0325).

use crate::js::{self, Expr, Stmt};
use crate::lower::recognition::{StdItem, std_item, trait_method};
use crate::lower::temporary_names::Role;
use crate::lower::{FnCx, R};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a B-tree of `key`s is std's own nodes, `$ordTreeMap`'s
    /// (#926): one found by its `cmp` whose comparisons can be seen
    /// (`order_seen`), or whose order may not be total (`order_total`,
    /// #936), so which keys a search asks of, and what it finds, is its
    /// nodes' shape, as std's is. One of a quiet and total order is a
    /// sorted array, whose search asks what nothing sees (ADR 0262, no
    /// difference). A type parameter's may be, its caller's class, whose
    /// order `order_seen` says may be seen: its calls are given what std's
    /// nodes ask besides `cmp`, the keys' `eq` and `partial_cmp`, its
    /// entries' sort, the drops of what `append` replaces. One whose `==`
    /// or `!=` can be seen (`eq_seen`) is too, as std's bulk build asks
    /// `!=` of each two in a row, and a range `==` of its bounds (#962).
    /// And one, of `value`s if it's a map's, whose clone may panic partway
    /// and drop what it made (`clone_drops_by_nodes`, #1069), whatever its
    /// keys, as which it drops, and in what order, is its nodes' too.
    pub(in crate::lower) fn tree_of_nodes(&self, key: Ty<'tcx>, value: Option<Ty<'tcx>>) -> bool {
        self.found_by_cmp(key) && (self.order_seen(key) || self.eq_seen(key) || !self.order_total(key))
            || self.clone_drops_by_nodes(key, value)
    }

    /// Whether a clone of a B-tree of `key`s, and of `value`s if it's a
    /// map's, may panic partway and drop what it made: an entry's clone
    /// may panic (`clone_may_panic`) and an entry has a destructor, so
    /// what it drops as the panic unwinds, and in what order, is std's
    /// `clone_subtree`'s frames', node by node (#1069).
    fn clone_drops_by_nodes(&self, key: Ty<'tcx>, value: Option<Ty<'tcx>>) -> bool {
        let may_panic = |ty: Ty<'tcx>| self.needs_clone(ty) && self.clone_may_panic(ty);
        let entry = value.map_or(key, |value| Ty::new_tup(self.tcx, &[key, value]));
        (may_panic(key) || value.is_some_and(may_panic)) && self.drops(entry) != crate::lower::drops::Drops::Nothing
    }

    /// What std's nodes of `key`s ask besides `cmp`: `[eq, ord]`,
    /// `PartialEq::eq` and the keys' `PartialOrd` dictionary, by which a
    /// range's bounds are checked (`s == e`, `s > e`), and `extract_if`'s
    /// end, `<=` or `<`: each operator the impl's own or std's (#926, #937).
    fn tree_order_fns(&mut self, key: Ty<'tcx>, span: Span) -> R<[Expr; 2]> {
        Ok([self.eq_fn(key, span)?, self.operators(key, span)?])
    }

    /// `m.extract_if(f)`, or a B-tree's `extract_if(range, f)`, of the map's
    /// value and type, the range's, and the test's and its type. Closed,
    /// it drops what it holds that has a destructor, `drop`: its test, then
    /// its range, its bounds start then end, as std's `ExtractIf` drops its
    /// `pred` then its `range` where its owner drops it (#945,
    /// `closed_on_drop`).
    pub(in crate::lower) fn map_extract_if_values(
        &mut self,
        (m, receiver): (Expr, Ty<'tcx>),
        range: Option<(Expr, Ty<'tcx>)>,
        (f, f_ty): (Expr, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let set = self.is_set(receiver.peel_refs());
        let mut held = Vec::new();
        let mut hold = |this: &mut Self, name: Role, value: Expr, ty: Ty<'tcx>, out: &mut Vec<Stmt>| -> Expr {
            match ty.needs_drop(this.tcx, this.typing_env) {
                true => {
                    let value = match value.reads_same() {
                        true => value,
                        false => this.spill(name, value, out),
                    };
                    held.push((value.clone(), ty));
                    value
                }
                false => value,
            }
        };
        let f = hold(self, Role::Test, f, f_ty, out);
        let range = range.map(|(range, ty)| (hold(self, Role::Range, range, ty, out), ty));
        let drop = match held.is_empty() {
            true => None,
            false => Some(Expr::arrow(Vec::new(), self.drop_each(held, span)?)),
        };
        let (m, entries) = match range {
            Some(range) => {
                let range = self.tree_range_of((m, receiver), range, (false, false), span, out)?;
                let js::ExprKind::Call(_, list) = &range.kind else {
                    unreachable!("a B-tree's range is `$treeRange`'s")
                };
                (list[0].clone(), range)
            }
            None => (m.clone(), m),
        };
        let handles = !set
            && matches!(receiver.peel_refs().kind(), ty::Adt(_, map) if map.types().nth(1).is_some_and(|value| self.handles_mut(value)));
        let list = vec![m, f, entries, Expr::bool(handles), Expr::bool(set)];
        Ok(Expr::call(
            Expr::helper("$mapExtractIf"),
            Expr::optional_args(list, vec![drop]),
        ))
    }

    /// A B-tree's range of its keys (ADR 0325), of the tree's value and
    /// type, and the range's.
    pub(in crate::lower) fn tree_range_values(
        &mut self,
        (m, receiver): (Expr, Ty<'tcx>),
        (range, bounds): (Expr, Ty<'tcx>),
        mutable: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        self.tree_range_of((m, receiver), (range, bounds), (mutable, true), span, out)
    }

    /// `tree_range_values`, or unchecked, `extract_if`'s range, which std
    /// walks from its start and checks no bounds of (ADR 0344): of a tree
    /// that may be std's nodes, the range it walks, not searched (#926).
    fn tree_range_of(
        &mut self,
        (m, receiver): (Expr, Ty<'tcx>),
        (range, bounds): (Expr, Ty<'tcx>),
        (mutable, checked): (bool, bool),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let receiver = receiver.peel_refs();
        let set = self.is_set(receiver);
        let ty::Adt(adt, map) = receiver.kind() else {
            unreachable!("a B-tree's `range` is of a B-tree")
        };
        let name = self.tcx.item_name(adt.did()).to_string();
        // Its `Bound`s, as `start_bound()` and `end_bound()` give them, of a
        // range of std's or a pair of `Bound`s.
        let key = map.type_at(0);
        let range_bounds = std_item(self.tcx, StdItem::RangeBounds);
        let start_bound = trait_method(self.tcx, range_bounds, "start_bound");
        let bound = self.tcx.instantiate_and_normalize_erasing_regions(
            self.tcx.mk_args(&[bounds.peel_refs().into(), key.into()]),
            self.typing_env,
            self.tcx.fn_sig(start_bound).map_bound(|sig| sig.skip_binder().output()),
        );
        let Some([start, end]) = self.range_bound_values(range, bounds.peel_refs(), bound, span, out)? else {
            return Err(self.unsupported(span, "a B-tree's range of bounds that aren't a range"));
        };
        let cmp = self.cmp_fn(key, false, span)?;
        // `range_mut` of numbers or strings: each value a handle on it, as
        // `iter_mut`'s is (ADR 0152).
        let handles = mutable && self.handles_mut(map.type_at(1));
        let m = match handles && !m.reads_same() {
            true => self.spill(Role::Map, m, out),
            false => m,
        };
        let mut list = vec![m.clone(), cmp, Expr::bool(set), Expr::str(&name), start, end];
        if self.tree_of_nodes(key, (!set).then(|| map.type_at(1))) {
            list.push(Expr::bool(checked));
            list.extend(self.tree_order_fns(key, span)?);
        } else if !checked {
            list.push(Expr::bool(false));
        }
        let range = Expr::call(Expr::helper("$treeRange"), list);
        if !handles {
            return Ok(range);
        }
        Ok(Expr::call(Expr::helper("$mutEntries"), vec![m, range]))
    }
}
