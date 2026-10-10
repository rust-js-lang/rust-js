//! Iterators from MIR (ADR 0364): each a JS iterator, whose helpers are
//! lazy as Rust's adapters are, so what each closure does happens in the
//! order Rust's does, item by item. A range is its `{ start, end }` until
//! it's iterated, and an iterator of the crate's its impl's `next`, made a
//! JS iterator where one's wanted. What's taken by a `&mut`, `any` of
//! `&mut it`, steps it through `$lent`, which a helper that stops early
//! closes in its place, as Rust leaves the iterator to go on.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Mutability, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use crate::js::{self, Expr, Op, StmtKind};
use crate::lower::recognition::{StdItem, is_std_def, std_item};
use crate::lower::representation::Num;
use crate::lower::std_types::range::RangeKind;
use crate::lower::{FnCx, R};
use crate::runtime::Helper;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of an `Iterator`'s or an `IntoIterator`'s method, of a std
    /// iterator, from MIR: `None` if it's another.
    pub(in crate::lower) fn mir_iter_call(
        &mut self,
        def_id: DefId,
        arg_tys: &[Ty<'tcx>],
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        let Some(trait_id) = tcx.trait_of_assoc(def_id) else {
            return Ok(None);
        };
        let name = tcx.item_name(def_id);
        let name = name.as_str();
        let receiver_ty = arg_tys.first().copied().unwrap_or(tcx.types.unit);
        let iter_ty = receiver_ty.peel_refs();
        // `x.into_iter()`: a range stays one, an iterator is itself, and a
        // collection's items are a JS iterator of them.
        if is_std_def(tcx, trait_id, StdItem::IntoIterator) && name == "into_iter" {
            let value = values.into_iter().next().expect("the receiver");
            if self.range_kind(iter_ty).is_some() || self.implements_iterator(receiver_ty) {
                return Ok(Some(value));
            }
            // `for x in &mut v` of numbers: handles on its items, as
            // `iter_mut()`'s are, `mir_std_call`'s (ADR 0099).
            let handles = matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut))
                && self.iterator_item(output).is_some_and(|item| self.is_cell(item));
            if handles {
                return Ok(None);
            }
            if iter_ty.is_array() || iter_ty.is_slice() || self.is_vec_like(iter_ty) {
                return Ok(Some(self.std_iterator(value, output)));
            }
            return Ok(None);
        }
        if !is_std_def(tcx, trait_id, StdItem::Iterator) || self.is_user_iterator(iter_ty) {
            return Ok(None);
        }
        let mut values = values.into_iter();
        let receiver = values.next().expect("the receiver");
        // `next(&mut it)`: a range's start moved, or the JS iterator stepped.
        if name == "next" {
            let item = self.option_of(output).expect("`next` gives an `Option`");
            return Ok(Some(match self.range_kind(iter_ty) {
                Some(RangeKind::Exclusive) => self.helper(Helper::RangeNext, "$rangeNext", vec![receiver]),
                Some(RangeKind::From) => self.helper(Helper::RangeFromNext, "$rangeFromNext", vec![receiver]),
                Some(RangeKind::Inclusive) => {
                    self.helper(Helper::RangeInclusiveNext, "$rangeInclusiveNext", vec![receiver])
                }
                Some(_) => return Ok(None),
                None if self.boxed_payload(item) => self.helper(Helper::NextSome, "$nextSome", vec![receiver]),
                None => self.helper(Helper::Next, "$next", vec![receiver]),
            }));
        }
        // Taken by a `&mut`, it's stepped through what can't close it.
        let lent = matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut));
        let it = self.as_js_iterator(receiver, iter_ty, span)?;
        let it = match lent {
            true => self.lent_iterator(it),
            false => it,
        };
        let item = self.iterator_item(iter_ty);
        // An item with a destructor a method may leave undropped, `filter`'s
        // or `count`'s: what std drops, refused till it's dropped here.
        let keeps_each = matches!(
            name,
            "map"
                | "for_each"
                | "fold"
                | "collect"
                | "any"
                | "all"
                | "position"
                | "enumerate"
                | "by_ref"
                | "fuse"
                | "sum"
                | "product"
                | "count"
        );
        if !keeps_each && item.is_some_and(|item| self.drops(item) != crate::lower::drops::Drops::Nothing) {
            return Err(self.unsupported(span, &format!("`{name}` of items with a destructor, from its MIR")));
        }
        let mut arg = || values.next().expect("rustc checked the arguments");
        let method = |name: &str, it: Expr, args: Vec<Expr>| Expr::call(Expr::member(it, name), args);
        Ok(Some(match name {
            "map" => method("map", it, vec![self.unary_callback(arg())]),
            "filter" => method("filter", it, vec![self.unary_callback(arg())]),
            "flat_map" => method("flatMap", it, vec![self.unary_callback(arg())]),
            "take" => method("take", it, vec![arg()]),
            "skip" => method("drop", it, vec![arg()]),
            "for_each" => method("forEach", it, vec![self.unary_callback(arg())]),
            "any" => method("some", it, vec![self.unary_callback(arg())]),
            "all" => method("every", it, vec![self.unary_callback(arg())]),
            "find" => {
                let found = method("find", it, vec![self.unary_callback(arg())]);
                match item.is_some_and(|item| self.boxed_payload(item)) {
                    true => return Err(self.unsupported(span, "`find` of what looks like `None`, from its MIR")),
                    false => found,
                }
            }
            "position" => {
                let found = self.unary_callback(arg());
                self.helper(Helper::Position, "$position", vec![it, found])
            }
            "enumerate" => {
                let (x, i) = (self.fresh("item"), self.fresh("i"));
                let pair = Expr::array(vec![Expr::var(&i), Expr::var(&x)]);
                let f = Expr::arrow(
                    vec![x.into(), i.into()],
                    vec![StmtKind::Return(Some(pair)).at(js::Span::NONE)],
                );
                method("map", it, vec![f])
            }
            "fold" => {
                let init = arg();
                let f = self.binary_callback(arg());
                method("reduce", it, vec![f, init])
            }
            // Each item counted, and dropped as `count` drops it.
            "count" => {
                let n = self.fresh("n");
                let x = self.fresh("item");
                let mut body = Vec::new();
                if let Some(item) = item {
                    self.drop_value(Expr::var(&x), item, span, &mut body)?;
                }
                let mut params = vec![n.clone().into()];
                if !body.is_empty() {
                    params.push(x.into());
                }
                body.push(StmtKind::Return(Some(Expr::bin(Op::Add, Expr::var(&n), Expr::int(1)))).at(js::Span::NONE));
                method("reduce", it, vec![Expr::arrow(params, body), Expr::int(0)])
            }
            "sum" | "product" => {
                let num = Num::of(output)
                    .ok_or_else(|| self.unsupported(span, &format!("`{name}` of a `{output}`, from its MIR")))?;
                let (op, start) = match name {
                    "sum" => (Op::Add, 0),
                    _ => (Op::Mul, 1),
                };
                let (a, b) = (self.fresh("a"), self.fresh("b"));
                let total = num.wrap(Expr::bin(op, Expr::var(&a), Expr::var(&b)));
                let f = Expr::arrow(
                    vec![a.into(), b.into()],
                    vec![StmtKind::Return(Some(total)).at(js::Span::NONE)],
                );
                method("reduce", it, vec![f, num.literal(start)])
            }
            "copied" => it,
            "cloned" => match item {
                Some(item) if self.is_copy(item.peel_refs()) => it,
                Some(item) => {
                    let x = self.fresh("item");
                    let mut body = Vec::new();
                    let copy = self.clone_value(Expr::var(&x), item.peel_refs(), span, &mut body)?;
                    body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
                    method("map", it, vec![Expr::arrow(vec![x.into()], body)])
                }
                None => return Ok(None),
            },
            // Of all the items, in order, as THIR's of an array.
            "max" | "min" => {
                let items = method("toArray", it, Vec::new());
                self.extreme_of(name == "max", items, item, span)?
            }
            "last" => {
                let items = method("toArray", it, Vec::new());
                let mut made = Vec::new();
                let last = self.last_of(items, item, &mut made);
                if !made.is_empty() {
                    return Err(self.unsupported(span, "`last` of what looks like `None`, from its MIR"));
                }
                last
            }
            "by_ref" => it,
            "fuse" => it,
            "collect" => {
                if self.is_vec_like(output) {
                    method("toArray", it, Vec::new())
                } else if self.is_lang_adt(output, LangItem::String) {
                    method("join", method("toArray", it, Vec::new()), vec![Expr::str("")])
                } else {
                    return Err(self.unsupported(span, &format!("`collect` into a `{output}`, from its MIR")));
                }
            }
            _ => return Ok(None),
        }))
    }

    /// `value`, an iterator of type `ty`, as a JS iterator: a range stepped
    /// by its `next`, the crate's by its impl's, a JS one as it is.
    pub(super) fn as_js_iterator(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        match self.range_kind(ty) {
            Some(RangeKind::Exclusive) => {
                self.runtime.insert(Helper::RangeNext);
                Ok(self.helper(Helper::Iterator, "$iterator", vec![value, Expr::var("$rangeNext")]))
            }
            Some(RangeKind::From) => {
                self.runtime.insert(Helper::RangeFromNext);
                Ok(self.helper(Helper::Iterator, "$iterator", vec![value, Expr::var("$rangeFromNext")]))
            }
            Some(RangeKind::Inclusive) => {
                self.runtime.insert(Helper::RangeInclusiveNext);
                Ok(self.helper(
                    Helper::Iterator,
                    "$iterator",
                    vec![value, Expr::var("$rangeInclusiveNext")],
                ))
            }
            Some(_) => Err(self.unsupported(span, "iterating this range, from its MIR")),
            None if self.is_user_iterator(ty) => {
                let trait_id = std_item(self.tcx, StdItem::Iterator);
                self.user_iterator(value, ty, trait_id, "next", span)
            }
            None => Ok(value),
        }
    }

    /// A std iterator of type `ty` made of `value`, the array or the text
    /// THIR's lowering makes of it, as MIR steps it: over an array, a
    /// `$iter`, which knows where it is, so a clone of it does (ADR 0181);
    /// another, the JS iterator of `value`.
    pub(super) fn std_iterator(&mut self, value: Expr, ty: Ty<'tcx>) -> Expr {
        match self.array_source(ty) {
            Some(_) => match from_iterator(value) {
                Ok(items) | Err(items) if !is_js_iterator(&items) => self.stepped_items(items),
                Ok(items) | Err(items) => items,
            },
            None if is_js_iterator(&value) => value,
            None => self.js_iterator(value),
        }
    }

    /// Whether `ty` is an iterator: one whose `Item` is a type.
    pub(in crate::lower) fn implements_iterator(&self, ty: Ty<'tcx>) -> bool {
        self.iterator_item(ty)
            .is_some_and(|item| !matches!(item.kind(), ty::Alias(..)))
            || matches!(ty.kind(), ty::Param(_))
    }

    /// A function JS's helpers give an item, and its index too, as one
    /// that takes the item alone: an arrow of one parameter as it is,
    /// anything else called with the item alone.
    fn unary_callback(&mut self, f: Expr) -> Expr {
        match &f.kind {
            js::ExprKind::Arrow(params, _) if params.len() <= 1 => f,
            _ => {
                let x = self.fresh("item");
                let call = Expr::call(f, vec![Expr::var(&x)]);
                Expr::arrow(vec![x.into()], vec![StmtKind::Return(Some(call)).at(js::Span::NONE)])
            }
        }
    }

    /// As `unary_callback`, of a function of two: `reduce`'s, given an index too.
    fn binary_callback(&mut self, f: Expr) -> Expr {
        match &f.kind {
            js::ExprKind::Arrow(params, _) if params.len() <= 2 => f,
            _ => {
                let (a, b) = (self.fresh("a"), self.fresh("b"));
                let call = Expr::call(f, vec![Expr::var(&a), Expr::var(&b)]);
                Expr::arrow(
                    vec![a.into(), b.into()],
                    vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
                )
            }
        }
    }

    fn helper(&mut self, helper: Helper, name: &str, args: Vec<Expr>) -> Expr {
        self.runtime.insert(helper);
        Expr::call(Expr::var(name), args)
    }
}

/// Whether `e` is a JS iterator as it's made: `Iterator.from(..)`, a
/// `$iter`, or a collection's `values()`, `keys()` or `entries()`.
fn is_js_iterator(e: &Expr) -> bool {
    let js::ExprKind::Call(callee, _) = &e.kind else {
        return false;
    };
    match &callee.kind {
        js::ExprKind::Var(_) => crate::lower::iterators::is_stepped_items(e),
        js::ExprKind::Member(object, name) => {
            matches!(name.as_str(), "values" | "keys" | "entries")
                || (name == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator"))
        }
        _ => false,
    }
}

/// What `Iterator.from(x)` makes a JS iterator of, `x`: `Ok` of it, or
/// `Err` of `e` itself, of anything else.
fn from_iterator(e: Expr) -> Result<Expr, Expr> {
    match e.kind {
        js::ExprKind::Call(callee, mut args)
            if args.len() == 1
                && matches!(&callee.kind, js::ExprKind::Member(object, name)
                    if name == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator")) =>
        {
            Ok(args.pop().expect("one"))
        }
        kind => Err(Expr { kind, span: e.span }),
    }
}
