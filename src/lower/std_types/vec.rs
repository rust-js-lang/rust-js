//! A `Vec`'s or a slice's method (ADRs 0025, 0036).

use crate::js;
use crate::js::{Expr, Op, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};
use crate::runtime::Helper;
use rustc_middle::thir::{ExprId, ExprKind};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `.flatten()` of a read of items that may be `None`, through a
    /// `copied()` or `cloned()`: the JS read itself, `items.shift()` or
    /// `items[0]`, whose `undefined` is both no item and a `None` one, as
    /// `flatten` makes them one (ADR 0311).
    pub(in crate::lower) fn flattened_read(&mut self, read: ExprId, out: &mut Vec<Stmt>) -> R<Option<Expr>> {
        let ExprKind::Call { fun, ref args, .. } = self.thir[self.strip(read)].kind else {
            return Ok(None);
        };
        let args = args.clone();
        Ok(Some(match self.std_fn(fun) {
            // A copy that's the item itself: of a reference, a number or text.
            Some(Std::OptionCloned)
                if self
                    .option_of(self.thir[self.strip(read)].ty)
                    .and_then(|inner| self.option_of(inner))
                    .is_some_and(|item| item.is_ref() || item.is_primitive() || self.is_string_like(item)) =>
            {
                return self.flattened_read(args[0], out);
            }
            Some(Std::Method(method @ ("pop" | "shift"))) => {
                Expr::call(Expr::member(self.expr(args[0], out)?, method), vec![])
            }
            Some(Std::First) => Expr::index(self.expr(args[0], out)?, Expr::int(0)),
            Some(Std::SliceGet) => {
                let items = self.expr(args[0], out)?;
                Expr::index(items, self.expr(args[1], out)?)
            }
            Some(Std::SliceLast) => Expr::call(Expr::member(self.expr(args[0], out)?, "at"), vec![Expr::int(-1)]),
            _ => return Ok(None),
        }))
    }

    /// A `Vec`'s or a slice's method (ADRs 0025, 0036): `None` if `known` is another.
    pub(in crate::lower) fn vec_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        boxed: bool,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Call { args, span, .. } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        let js_span = self.js_span(span);
        Ok(Some(match known {
            Std::Method("pop") if boxed => {
                self.runtime.insert(Helper::Pop);
                Expr::call(Expr::var("$pop"), vec![arg()])
            }
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
            Std::SliceLast => Expr::call(Expr::member(arg(), "at"), vec![Expr::int(-1)]),
            // A copy, unless it's an array just written: `vec![3, 4].into()`.
            // Of what it borrows, `to_vec()`'s, each item a clone too.
            Std::ToVec => match arg() {
                items if matches!(items.kind, js::ExprKind::Array(_)) => items,
                items => match self.slice_item(self.thir[args[0]].ty) {
                    Some(item) if self.thir[args[0]].ty.is_ref() => self.clone_items(items, item, span)?,
                    _ => Expr::call(Expr::member(items, "slice"), vec![]),
                },
            },
            Std::SortBy => {
                let (v, compare) = (arg(), arg());
                Expr::call(Expr::member(v, "sort"), vec![compare])
            }
            // Text is falsy only where it's empty, `!text` (ADR 0266): an empty
            // array is truthy.
            Std::IsEmpty if self.is_string_like(self.thir[args[0]].ty.peel_refs()) => {
                Expr::unary(js::UnaryOp::Not, arg())
            }
            Std::IsEmpty => Expr::bin(Op::Eq, Expr::member(arg(), "length"), Expr::num(0)),
            Std::VecNew => Expr::array(vec![]),
            Std::Append => {
                self.runtime.insert(Helper::Append);
                Expr::call(Expr::var("$append"), vec![arg(), arg()])
            }
            Std::Push => {
                let (v, x) = (arg(), arg());
                Expr::call(Expr::member(v, "push"), vec![x])
            }
            // `count()` of one that knows where it is (ADR 0071): what it has left,
            // which it then has none of.
            Std::Len if self.is_stepping(args[0]) => {
                self.runtime.insert(Helper::Rest);
                Expr::member(Expr::call(Expr::var("$rest"), vec![arg()]), "length")
            }
            // `count()` of a JS iterator (ADR 0055) takes all of it.
            Std::Len if self.is_lazy_value(args[0]) => {
                let items = self.iter_source(arg(), self.thir[args[0]].ty, span, out)?;
                Expr::member(Expr::call(Expr::member(items, "toArray"), vec![]), "length")
            }
            // A range's, which is an object, is its items' (ADR 0129).
            Std::Len if self.range_kind(self.thir[args[0]].ty.peel_refs()).is_some() => {
                let items = self.iter_source(arg(), self.thir[args[0]].ty.peel_refs(), span, out)?;
                Expr::member(items, "length")
            }
            Std::Len => Expr::member(arg(), "length"),
            Std::Index => self.checked_index(call.fun, args[0], vec![arg(), arg()]),
            Std::Clear => {
                out.push(StmtKind::Assign(Expr::member(arg(), "length"), Expr::num(0)).at(js_span));
                Expr::undefined()
            }
            Std::Retain => {
                self.runtime.insert(Helper::Retain);
                let (v, keep) = (arg(), arg());
                Expr::call(Expr::var("$retain"), vec![v, keep])
            }
            _ => return Ok(None),
        }))
    }
}
