//! `PartialOrd` and `Ord` (ADR 0057): a comparison is an `Ordering`, -1, 0
//! or 1 (ADR 0036), and `partial_cmp`'s `None` is `undefined`. JS's own `<`
//! for what it orders as Rust does, `$cmp`, a hand-written `cmp`, or the
//! parts compared in turn: `$cmp(a.x, b.x) || $cmp(a.y, b.y)`, since `Equal`
//! is the one that's falsy.

use super::bindings;
use super::recognition::{OrderingCall, StdItem, std_item, trait_method};
use super::representation::{Num, variant_field};
use super::{FnCx, R, Shape};
use crate::js::{self, Expr, Op, Stmt, StmtKind};
use rustc_abi::VariantIdx;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;
use rustc_span::{DUMMY_SP, Span};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn ord_trait(&self) -> DefId {
        std_item(self.tcx, StdItem::Ord)
    }

    pub(super) fn partial_ord_trait(&self) -> DefId {
        self.tcx.require_lang_item(LangItem::PartialOrd, DUMMY_SP)
    }

    /// Does JS's `<` order `ty` as Rust does? Numbers, `bool`s (`false <
    /// true`), and `Ordering`s, which are numbers. Not strings and `char`s,
    /// which `<` orders by UTF-16 units (ADR 0183).
    pub(super) fn is_primitive_ord(&self, ty: Ty<'tcx>) -> bool {
        // A `&mut` to a number is a cell, an object (ADR 0099).
        if self.has_cell_layer(ty) {
            return false;
        }
        let ty = ty.peel_refs();
        Num::of(ty).is_some() || ty.is_bool() || self.is_lang_adt(ty, LangItem::OrderingEnum)
    }

    /// Strings and `char`s, which `$cmp` orders by code point (ADR 0183).
    pub(super) fn is_text_ord(&self, ty: Ty<'tcx>) -> bool {
        !self.has_cell_layer(ty) && self.is_string_like(ty.peel_refs())
    }

    /// `$cmp(a, b)`, of strings or `char`s: their `Ordering` by code point.
    pub(super) fn text_order(&mut self, a: Expr, b: Expr) -> Expr {
        Expr::call(Expr::helper("$cmp"), vec![a, b])
    }

    /// `a < b` and the rest, of strings or `char`s, by code point, as Rust
    /// orders them (ADR 0183): JS's own operator where one is a literal whose
    /// characters are all below U+D800, which it orders the same, `c >=
    /// "a"`; else `$cmp(a, b) < 0`.
    pub(super) fn text_compare(&mut self, op: Op, a: Expr, b: Expr) -> Expr {
        let exact = |e: &Expr| matches!(&e.kind, js::ExprKind::Str(s) if s.chars().all(|c| (c as u32) < 0xd800));
        if exact(&a) || exact(&b) {
            return Expr::bin(op, a, b);
        }
        Expr::bin(op, Expr::call(Expr::helper("$cmp"), vec![a, b]), Expr::int(0))
    }

    /// Is `ty` `Ord`, so that its `partial_cmp` is never `None`?
    fn is_total(&self, ty: Ty<'tcx>) -> bool {
        let tr = ty::TraitRef::new(self.tcx, self.ord_trait(), [self.tcx.erase_and_anonymize_regions(ty)]);
        matches!(
            self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(_) | ImplSource::Param(_) | ImplSource::Builtin(..))
        )
    }

    /// `a.cmp(&b)`, or `a.partial_cmp(&b)` if `partial`, of `ty` values: an
    /// `Ordering`, and for `partial_cmp`, `undefined` when there's none.
    pub(super) fn cmp_value(
        &mut self,
        a: Expr,
        b: Expr,
        ty: Ty<'tcx>,
        partial: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let (a, _) = self.through_refs(a, ty);
        let (b, ty) = self.through_refs(b, ty);
        // `f64`'s `NaN` isn't ordered at all.
        if partial && Num::of(ty).is_some_and(Num::float) {
            return Ok(Expr::call(Expr::helper("$partialCmp"), vec![a, b]));
        }
        // A path's components in turn (ADR 0401).
        if self.recognition().is_path(ty) {
            return Ok(Expr::call(Expr::helper("$pathCmp"), vec![a, b]));
        }
        if self.is_primitive_ord(ty) || self.is_text_ord(ty) {
            return Ok(Expr::call(Expr::helper("$cmp"), vec![a, b]));
        }
        // A raw pointer, by its address (ADR 0562).
        if let Some((pointee, _)) = super::pointers::raw_pointee(self.tcx, ty)
            && self.is_handle_pointer(ty)
        {
            let size = self.pointee_size(pointee, span)?;
            return Ok(Expr::call(Expr::helper("$ptrCmp"), vec![a, b, size]));
        }
        let (ord, partial_ord) = (self.ord_trait(), self.partial_ord_trait());
        if self.is_unknown(ty) {
            let total = ty::TraitRef::new(self.tcx, ord, [ty]);
            if let Some(dictionary) = self.evidence_for(total) {
                return Ok(Expr::call(Expr::member(dictionary, "cmp"), vec![a, b]));
            }
            let tr = ty::TraitRef::new_from_args(self.tcx, partial_ord, self.args_of(partial_ord, ty));
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            return Ok(Expr::call(Expr::member(dictionary, "partial_cmp"), vec![a, b]));
        }
        // A hand-written one: `cmp`, or `partial_cmp` if that's what's asked
        // for, or all there is.
        let own_ord = self.has_user_impl(ord, ty);
        let own_partial = self.has_user_impl(partial_ord, ty);
        if own_ord && (!partial || !own_partial) {
            let cmp = trait_method(self.tcx, ord, "cmp");
            return self.impl_call(cmp, self.args_of(ord, ty), vec![a, b], span);
        }
        if own_partial {
            let cmp = trait_method(self.tcx, partial_ord, "partial_cmp");
            return self.impl_call(cmp, self.args_of(partial_ord, ty), vec![a, b], span);
        }
        // Derived. Each is read more than once below.
        let a = if a.reads_same() { a } else { self.spill("left", a, out) };
        let b = if b.reads_same() { b } else { self.spill("right", b, out) };
        match ty.kind() {
            _ if let Some(inner) = self.option_of(ty) => {
                // `None` is less than any `Some`.
                let (x, y) = if self.boxed_payload(inner) {
                    (self.some_value(a.clone()), self.some_value(b.clone()))
                } else {
                    (a.clone(), b.clone())
                };
                let some = self.cmp_value(x, y, inner, partial, span, out)?;
                let none = |x: &Expr| Expr::bin(Op::LooseEq, x.clone(), Expr::null());
                Ok(Expr::cond(
                    none(&a),
                    Expr::cond(none(&b), Expr::int(0), Expr::int(-1)),
                    Expr::cond(none(&b), Expr::int(1), some),
                ))
            }
            // A counted `Rc`'s `value` (ADR 0320).
            ty::Adt(_, args) if let Some(pointee) = self.counted_rc(ty) => {
                let (a, b) = (self.counted_pointee(a, pointee), self.counted_pointee(b, pointee));
                self.cmp_value(a, b, args.type_at(0), partial, span, out)
            }
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) || self.recognition().pinned(ty).is_some() => {
                self.cmp_value(a, b, args.type_at(0), partial, span, out)
            }
            // A `Cow`: what it borrows (ADR 0400).
            ty::Adt(..) if let Some((x, borrowed)) = self.cow_deref(&a, ty) => {
                let y = self.cow_deref(&b, ty).expect("a `Cow`").0;
                self.cmp_value(x, y, borrowed, partial, span, out)
            }
            // `Reverse(x)`: `x`s the other way round, as its impl has it.
            ty::Adt(_, args) if self.is_reverse(ty) => {
                let inside = |x: Expr| Expr::index(x, Expr::int(0));
                self.cmp_value(inside(b), inside(a), args.type_at(0), partial, span, out)
            }
            // `Wrapping(x)`: its number's, as its derive has it (ADR 0175).
            ty::Adt(_, args) if self.recognition().wrapping_of(ty).is_some() => {
                let inside = |x: Expr| Expr::index(x, Expr::int(0));
                self.cmp_value(inside(a), inside(b), args.type_at(0), partial, span, out)
            }
            ty::Adt(_, args) if self.is_vec_like(ty) => self.cmp_items(a, b, args.type_at(0), partial, span),
            ty::Array(item, _) | ty::Slice(item) => self.cmp_items(a, b, *item, partial, span),
            // A fieldless enum: by its discriminants, as the derive compares
            // them, `Low = 1` before `High = 2` however they're declared.
            ty::Adt(adt, _) if super::representation::is_string_enum(self.tcx, *adt) => {
                let mut variants = super::discriminants(self.tcx, *adt);
                variants.sort_by_key(|&(_, value)| value);
                let names = variants.into_iter().map(|(name, _)| Expr::str(name)).collect();
                Ok(Expr::call(Expr::helper("$cmpIn"), vec![Expr::array(names), a, b]))
            }
            // One with fields, the crate's, tagged: by its variant, as the
            // derive orders them, then, of the same, its fields in turn.
            ty::Adt(adt, args)
                if adt.is_enum() && adt.did().is_local() && !bindings::is_untagged(self.tcx, adt.did()) =>
            {
                let mut variants: Vec<(VariantIdx, u128)> =
                    adt.discriminants(self.tcx).map(|(v, d)| (v, d.val)).collect();
                variants.sort_by_key(|&(_, value)| value);
                // Where each is among them, `a.TAG === "Rect" ? 1 : ..`.
                let place = |cx: &mut Self, x: &Expr| {
                    let rest = &variants[..variants.len() - 1];
                    (rest.iter().enumerate().rev()).fold(
                        Expr::int((variants.len() - 1) as i128),
                        |others, (i, &(v, _))| {
                            Expr::cond(cx.variant_test(x.clone(), ty, *adt, v), Expr::int(i as i128), others)
                        },
                    )
                };
                let (at_a, at_b) = (place(self, &a), place(self, &b));
                let order = Expr::call(Expr::helper("$cmp"), vec![at_a, at_b]);
                // Of the same variant, its fields, each in turn.
                let mut same = Expr::int(0);
                for &(v, _) in variants.iter().rev() {
                    let def = adt.variant(v);
                    if def.fields.is_empty() {
                        continue;
                    }
                    let mut orders = Vec::new();
                    let mut total = true;
                    for (i, field) in def.fields.iter().enumerate() {
                        let t = self.field_ty(field, args);
                        total &= !partial || self.is_total(t);
                        let name = variant_field(self.tcx, def, i);
                        orders.push(self.cmp_value(
                            Expr::member(a.clone(), &name),
                            Expr::member(b.clone(), &name),
                            t,
                            partial,
                            span,
                            out,
                        )?);
                    }
                    let fields = match total {
                        true => orders
                            .into_iter()
                            .reduce(|all, next| Expr::bin(Op::Or, all, next))
                            .expect("a field"),
                        false => Expr::call(Expr::helper("$thenCmp"), orders),
                    };
                    same = Expr::cond(self.variant_test(a.clone(), ty, *adt, v), fields, same);
                }
                Ok(Expr::bin(Op::Or, order, same))
            }
            ty::Adt(adt, _) if adt.is_enum() => Err(self.unsupported(span, &format!("comparing `{ty}`s"))),
            // Another crate's struct orders as its impl says, which may not be
            // field by field; but a derive's is, as the crate's own (ADR 0555).
            ty::Adt(adt, _)
                if !adt.did().is_local()
                    && !self.recognition().derives(if partial { partial_ord } else { ord }, ty) =>
            {
                Err(self.unsupported(span, &format!("comparing `{ty}`s")))
            }
            // A type of one value, `()` or a struct of no fields, `struct
            // Zst;`: `Equal` to itself, as its derive has it.
            _ if self.one_value(ty) && !a.has_effects() && !b.has_effects() => Ok(Expr::int(0)),
            _ => {
                let parts: Vec<(Expr, Expr, Ty<'tcx>)> = match self.shape(ty) {
                    Shape::Object(fields) => fields
                        .into_iter()
                        .map(|(name, t)| (Expr::member(a.clone(), name.clone()), Expr::member(b.clone(), name), t))
                        .collect(),
                    Shape::Array(tys) => tys
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| {
                            let at = |x: &Expr| Expr::index(x.clone(), Expr::int(i as i128));
                            (at(&a), at(&b), t)
                        })
                        .collect(),
                    Shape::Other => return Err(self.unsupported(span, &format!("comparing `{ty}`s"))),
                };
                // Each part in turn, until one isn't `Equal`. An unordered
                // one (`undefined`) is falsy too, so where one can be, the
                // parts go through `$thenCmp`, which stops at it.
                let total = !partial || parts.iter().all(|&(_, _, t)| self.is_total(t));
                let mut orders = Vec::new();
                for (x, y, t) in parts {
                    orders.push(self.cmp_value(x, y, t, partial, span, out)?);
                }
                if orders.is_empty() {
                    return Ok(Expr::int(0));
                }
                if total {
                    return Ok(orders
                        .into_iter()
                        .reduce(|all, next| Expr::bin(Op::Or, all, next))
                        .expect("some parts"));
                }
                Ok(Expr::call(Expr::helper("$thenCmp"), orders))
            }
        }
    }

    /// Whether `ty` has one value, `()` or a struct of no fields, which is
    /// `Equal` to itself.
    fn one_value(&self, ty: Ty<'tcx>) -> bool {
        ty.is_unit()
            || matches!(ty.kind(), ty::Adt(adt, _) if adt.is_struct() && adt.non_enum_variant().fields.is_empty())
    }

    /// Two sequences, item by item, then by length: `$cmpItems(a, b, $cmp)`.
    fn cmp_items(&mut self, a: Expr, b: Expr, item: Ty<'tcx>, partial: bool, span: Span) -> R<Expr> {
        let compare = self.cmp_fn(item, partial, span)?;
        Ok(Expr::call(Expr::helper("$cmpItems"), vec![a, b, compare]))
    }

    /// `(a, b) => <their Ordering>`, or the function itself: `$cmp`,
    /// `wordOrd_cmp`, or a dictionary's, `TOrd.cmp`, which needs no `this`.
    pub(super) fn cmp_fn(&mut self, ty: Ty<'tcx>, partial: bool, span: Span) -> R<Expr> {
        let mut body = Vec::new();
        let order = self.cmp_value(Expr::var("a"), Expr::var("b"), ty, partial, span, &mut body)?;
        if body.is_empty()
            && let js::ExprKind::Call(callee, args) = &order.kind
            && callee.reads_same()
            && matches!(args.as_slice(), [x, y] if is_var(x, "a") && is_var(y, "b"))
        {
            return Ok((**callee).clone());
        }
        body.push(StmtKind::Return(Some(order)).at(js::Span::NONE));
        Ok(Expr::arrow(vec!["a".into(), "b".into()], body))
    }

    /// `a < b` and the rest, `a.cmp(&b)`, `a.partial_cmp(&b)`, `a.max(b)`
    /// and `a.min(b)`. `None` for numbers, which std's operators already are.
    pub(super) fn ordering_call(
        &mut self,
        id: DefId,
        tr: ty::TraitRef<'tcx>,
        values: Vec<Expr>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let ty = tr.self_ty();
        let Some((call, partial)) = self.recognition().ordering_call(id, tr) else {
            return Ok(None);
        };
        let operator = match call {
            OrderingCall::Lt => Some(Op::Lt),
            OrderingCall::Le => Some(Op::Le),
            OrderingCall::Gt => Some(Op::Gt),
            OrderingCall::Ge => Some(Op::Ge),
            _ => None,
        };
        let [a, b]: [Expr; 2] = values
            .try_into()
            .map_err(|_| self.unsupported(span, "this comparison"))?;
        // JS's own `<` orders numbers and `bool`s as Rust does, and strings
        // where one is a literal below U+D800 (ADR 0183).
        if let Some(op) = operator
            && self.is_primitive_ord(ty)
        {
            return Ok(Some(Expr::bin(op, a, b)));
        }
        if let Some(op) = operator
            && self.is_text_ord(ty)
        {
            return Ok(Some(self.text_compare(op, a, b)));
        }
        // `max` and `min` return one of them, so each is read twice.
        let (a, b) = if matches!(call, OrderingCall::Max | OrderingCall::Min) {
            let a = if a.reads_same() { a } else { self.spill("left", a, out) };
            let b = if b.reads_same() { b } else { self.spill("right", b, out) };
            (a, b)
        } else {
            (a, b)
        };
        // `partial_cmp`'s `undefined` makes every one of these false, as `None` does.
        let order = self.cmp_value(a.clone(), b.clone(), ty, partial, span, out)?;
        let zero = || Expr::int(0);
        Ok(Some(match call {
            _ if let Some(op) = operator => Expr::bin(op, order, zero()),
            // The second when they're equal, as Rust's `max` does.
            OrderingCall::Max => Expr::cond(Expr::bin(Op::Gt, order, zero()), a, b),
            OrderingCall::Min => Expr::cond(Expr::bin(Op::Gt, order, zero()), b, a),
            _ => order,
        }))
    }
}

fn is_var(e: &Expr, name: &str) -> bool {
    matches!(&e.kind, js::ExprKind::Var(n) if n == name)
}
