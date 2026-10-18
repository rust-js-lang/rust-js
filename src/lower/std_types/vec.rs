//! A `Vec`'s or a slice's method (ADRs 0025, 0036).

use crate::js;
use crate::js::{Expr, Op, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};
use rustc_middle::ty;

/// Whether `known` reads an item, which `flatten` makes the JS read itself.
pub(in crate::lower) fn reads_flattened(known: Std) -> bool {
    matches!(
        known,
        Std::Method("pop" | "shift") | Std::First | Std::SliceGet | Std::SliceLast
    )
}

/// The JS read `known` is of its arguments' values, `items.shift()` or
/// `items[0]`, whose `undefined` is both no item and a `None` one (ADR 0311).
pub(in crate::lower) fn flattened_read_values(known: Std, values: Vec<Expr>) -> Expr {
    let mut values = values.into_iter();
    let mut value = || values.next().expect("rustc checked the arguments");
    match known {
        Std::Method(method @ ("pop" | "shift")) => Expr::call(Expr::member(value(), method), vec![]),
        Std::First => Expr::index(value(), Expr::int(0)),
        Std::SliceGet => {
            let items = value();
            Expr::index(items, value())
        }
        Std::SliceLast => Expr::read(value(), "at", vec![Expr::int(-1)]),
        _ => unreachable!("`reads_flattened`'s"),
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `Vec`'s or a slice's method (ADRs 0025, 0036): `None` if `known` is another.
    pub(in crate::lower) fn vec_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        boxed: bool,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Call { tys, span, .. } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        let js_span = self.js_span(span);
        Ok(Some(match known {
            // A `Vec`'s capacity, kept where the crate or a library it uses
            // asks it (ADR 0419); else none, `with_capacity` an empty array,
            // and asking for room nothing (ADR 0315).
            Std::Capacity(op) => {
                use crate::lower::recognition::CapacityOp;
                let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
                match (op, self.krate.keeps_capacity) {
                    (CapacityOp::Of, _) => helper("$capacity", vec![arg()]),
                    (CapacityOp::WithCapacity, true) => helper("$withCapacity", vec![arg()]),
                    (CapacityOp::Reserve, true) => helper("$reserve", vec![arg(), arg()]),
                    (CapacityOp::ShrinkTo { fit: true }, true) => helper("$shrinkTo", vec![arg(), Expr::int(0)]),
                    (CapacityOp::ShrinkTo { fit: false }, true) => helper("$shrinkTo", vec![arg(), arg()]),
                    // As ADR 0315's: a new array, or nothing.
                    (_, false) => {
                        let known = match op {
                            CapacityOp::WithCapacity => Std::VecNew,
                            _ => Std::Nothing,
                        };
                        return self.vec_call(known, call, values, boxed, out);
                    }
                }
            }
            // The separator `&T` between each two, or `&[T]`'s items; each
            // item cloned, as `Join`'s are.
            Std::JoinItems => {
                let separator = tys[1].peel_refs();
                let (item, spread) = match separator.kind() {
                    ty::Slice(item) | ty::Array(item, _) => (*item, true),
                    ty::Adt(_, items) if self.is_vec_like(separator) => (items.type_at(0), true),
                    _ => (separator, false),
                };
                let clone = self.clone_arg(item, span)?;
                Expr::call(Expr::helper("$joinWith"), vec![arg(), arg(), Expr::bool(spread), clone])
            }
            Std::Method("pop") if boxed => Expr::call(Expr::helper("$pop"), vec![arg()]),
            Std::First if boxed => {
                let items = arg();
                self.some_at(items, Expr::int(0))
            }
            Std::SliceLast if boxed => {
                let mut items = arg();
                if items.has_effects() {
                    items = self.spill("items", items, out);
                }
                let last = Expr::bin(Op::Sub, Expr::member(items.clone(), "length"), Expr::int(1));
                self.some_at(items, last)
            }
            Std::SliceGet if boxed => {
                let items = arg();
                self.some_at(items, arg())
            }
            Std::First => Expr::index(arg(), Expr::int(0)),
            Std::SliceGet => {
                let items = arg();
                Expr::index(items, arg())
            }
            Std::SliceLast => Expr::read(arg(), "at", vec![Expr::int(-1)]),
            // A copy, unless it's an array just written: `vec![3, 4].into()`.
            // Of what it borrows, `to_vec()`'s, each item a clone too.
            // Of an array made here, itself: a literal's, whose items are
            // made here too, or `$slice(v)`'s, of items that need no clone.
            Std::ToVec => match arg() {
                items if matches!(items.kind, js::ExprKind::Array(_)) => items,
                items
                    if crate::lower::iterators::made_array(&items)
                        && !self.slice_item(tys[0]).is_some_and(|item| self.needs_clone(item)) =>
                {
                    items
                }
                items => match self.slice_item(tys[0]) {
                    Some(item) if tys[0].is_ref() => self.clone_items(items, item, span)?,
                    _ => Expr::call(Expr::member(items, "slice"), vec![]),
                },
            },
            Std::SortBy => {
                let (v, compare) = (arg(), arg());
                Expr::call(Expr::member(v, "sort"), vec![compare])
            }
            // Text is falsy only where it's empty, `!text` (ADR 0266): an empty
            // array is truthy.
            Std::IsEmpty if self.is_string_like(tys[0].peel_refs()) => Expr::unary(js::UnaryOp::Not, arg()),
            Std::IsEmpty => Expr::bin(Op::Eq, Expr::member(arg(), "length"), Expr::num(0)),
            // What it's given does what it does, of a capacity too (ADR 0315).
            Std::VecNew | Std::Nothing => {
                for value in values.by_ref() {
                    if value.has_effects() {
                        out.push(StmtKind::Expr(value).at(js_span));
                    }
                }
                match known {
                    Std::Nothing => Expr::undefined(),
                    _ => Expr::array(vec![]),
                }
            }
            Std::Append => Expr::call(Expr::helper("$append"), vec![arg(), arg()]),
            Std::WriteText => Expr::call(Expr::helper("$writeText"), vec![arg(), arg()]),
            Std::Push => {
                let (v, x) = (arg(), arg());
                Expr::call(Expr::member(v, "push"), vec![x])
            }
            // A range's, which is an object, is its items' (ADR 0129).
            Std::Len if self.range_kind(tys[0].peel_refs()).is_some() => {
                let items = self.iter_source(arg(), tys[0].peel_refs(), span)?;
                Expr::member(items, "length")
            }
            Std::Len => Expr::member(arg(), "length"),
            Std::Index => Expr::call(Expr::helper("$index"), vec![arg(), arg()]),
            // Of items with a destructor, each dropped once it's out, as
            // `truncate` drops them (ADR 0098).
            Std::Clear => match self.items_drop(tys[0], span)? {
                Some(drop) => Expr::call(Expr::helper("$truncate"), vec![arg(), Expr::int(0), drop]),
                None => {
                    out.push(StmtKind::Assign(Expr::member(arg(), "length"), Expr::num(0)).at(js_span));
                    Expr::undefined()
                }
            },
            Std::Retain => {
                let (v, keep) = (arg(), arg());
                Expr::call(Expr::helper("$retain"), self.dropping(vec![v, keep], tys[0], span)?)
            }
            _ => return Ok(None),
        }))
    }
}
