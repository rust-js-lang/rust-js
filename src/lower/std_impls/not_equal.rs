//! `!=`, std's `PartialEq::ne` (#962): what it runs beyond `!(a == b)`,
//! an impl's own `ne`, a type parameter's dictionary's, or std's that asks
//! its parts' `!=`.

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::temporary_names::Role;
use crate::lower::{FnCx, R, Shape};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `a != b` of `ty`s runs more than `!(a == b)`, as std's
    /// `PartialEq::ne` does (#962): an impl's own `ne`, and a type
    /// parameter's, whose dictionary may hold one; or std's own `ne` that
    /// asks its parts' `!=`, of a reference, a `Box`, an `Rc`, an `Arc`, a
    /// `Pin` or a tuple, of one that does. A sequence's `!=` is `!(a ==
    /// b)`, whose `==` asks its items' `!=` (`eq_items`); any other's,
    /// `Option`'s, a derive's, is std's provided `!(a == b)`.
    pub(in crate::lower) fn own_ne(&self, ty: Ty<'tcx>) -> bool {
        self.own_ne_in(ty, &mut Vec::new())
    }

    fn own_ne_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let ty = ty.peel_refs();
        if self.is_unknown(ty) {
            return true;
        }
        if seen.contains(&ty) {
            return false;
        }
        seen.push(ty);
        let own = match ty.kind() {
            ty::Tuple(parts) => parts.iter().any(|part| self.own_ne_in(part, seen)),
            ty::Adt(..) if let Some(imp) = self.partial_eq_impl(ty) => self.writes_ne(imp),
            ty::Adt(..) if let Some(pointee) = crate::lower::recognition::rc_pointee(self.tcx, ty) => {
                self.own_ne_in(pointee, seen)
            }
            ty::Adt(_, args) if ty.is_box() => self.own_ne_in(args.type_at(0), seen),
            // A `Pin`'s target's (`pinned_target`, #1446).
            ty::Adt(..) if let Some(target) = self.recognition().pinned_target(ty) => match target {
                crate::lower::recognition::PinTarget::Pointer(pointer) => self.own_ne_in(pointer, seen),
                crate::lower::recognition::PinTarget::Deref(_, target) => self.own_ne_in(target, seen),
            },
            _ => false,
        };
        seen.pop();
        own
    }

    /// The crate's or a library's impl of `PartialEq<Self>` for `ty`.
    fn partial_eq_impl(&self, ty: Ty<'tcx>) -> Option<DefId> {
        let partial_eq = self.partial_eq_trait();
        let tr = ty::TraitRef::new_from_args(self.tcx, partial_eq, self.args_of(partial_eq, ty));
        self.recognition().user_impl(tr)
    }

    /// Whether `a != b` of a `ty` and a `rhs` runs more than `!(a == b)`, as
    /// `own_ne` asks of one type (#970): of two, the crate's or a library's
    /// impl of `PartialEq<rhs>` for `ty`, where it writes its own `ne`, or a
    /// type parameter's, whose dictionary may hold one; of one, `own_ne`'s.
    pub(in crate::lower) fn own_ne_between(&self, ty: Ty<'tcx>, rhs: Ty<'tcx>) -> bool {
        let (ty, rhs) = (ty.peel_refs(), rhs.peel_refs());
        if ty == rhs {
            return self.own_ne(ty);
        }
        if self.is_unknown(ty) || self.is_unknown(rhs) {
            return true;
        }
        let tr = ty::TraitRef::new(self.tcx, self.partial_eq_trait(), [ty, rhs]);
        self.recognition().user_impl(tr).is_some_and(|imp| self.writes_ne(imp))
    }

    /// `a != b` of a `ty` and a `rhs`, as std's `PartialEq<rhs>::ne` runs
    /// (#970): of two types, a type parameter's dictionary's `ne`,
    /// `TPartialEq.ne(a, b)`, or an impl's own `ne`, where `own_ne_between`
    /// says it's more than `!(a == b)`; of one, `ne_value`'s.
    pub(in crate::lower) fn ne_values(
        &mut self,
        a: Expr,
        b: Expr,
        (ty, rhs): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let (x, lhs) = self.through_refs(a.clone(), ty);
        let (y, other) = self.through_refs(b.clone(), rhs);
        if lhs == other {
            return self.ne_value(a, b, ty, span, out);
        }
        if !self.own_ne_between(lhs, other) {
            return Ok(Expr::unary(
                js::UnaryOp::Not,
                self.eq_values(a, b, (ty, rhs), span, out)?,
            ));
        }
        let tr = ty::TraitRef::new(self.tcx, self.partial_eq_trait(), [lhs, other]);
        if self.is_unknown(lhs) || self.is_unknown(other) {
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            return Ok(Expr::call(Expr::member(dictionary, "ne"), vec![x, y]));
        }
        let ne = crate::lower::recognition::trait_method(self.tcx, self.partial_eq_trait(), "ne");
        self.impl_call(ne, tr.args, vec![x, y], span)
    }

    /// Whether the impl `imp`, a `PartialEq`'s, writes its own `ne`.
    fn writes_ne(&self, imp: DefId) -> bool {
        let ne = crate::lower::recognition::trait_method(self.tcx, self.partial_eq_trait(), "ne");
        self.tcx.impl_item_implementor_ids(imp).contains_key(&ne)
    }

    /// `a != b` of two `ty`s, as std's `PartialEq::ne` runs (#962): of what
    /// `own_ne` says runs more than `!(a == b)`, a type parameter's
    /// dictionary's `ne`, `TPartialEq.ne(a, b)`, an impl's own `ne`, a
    /// tuple's parts' `!=` in turn until one is `true`, and a reference's,
    /// a `Box`'s, a `Pin`'s, an `Rc`'s or an `Arc`'s what they point at's,
    /// an `Rc` of an `Eq` value's after its allocation's, as `RcEqIdent`
    /// specializes it; else `!(a == b)`.
    pub(in crate::lower) fn ne_value(
        &mut self,
        a: Expr,
        b: Expr,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if !self.own_ne(ty) {
            return Ok(Expr::unary(js::UnaryOp::Not, self.eq_value(a, b, ty, span, out)?));
        }
        let (a, _) = self.through_refs(a, ty);
        let (b, ty) = self.through_refs(b, ty);
        let partial_eq = self.partial_eq_trait();
        let tr = ty::TraitRef::new_from_args(self.tcx, partial_eq, self.args_of(partial_eq, ty));
        if self.is_unknown(ty) {
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            return Ok(Expr::call(Expr::member(dictionary, "ne"), vec![a, b]));
        }
        if self.partial_eq_impl(ty).is_some() {
            let ne = crate::lower::recognition::trait_method(self.tcx, partial_eq, "ne");
            return self.impl_call(ne, tr.args, vec![a, b], span);
        }
        match ty.kind() {
            ty::Tuple(_) => {
                let a = if a.reads_same() {
                    a
                } else {
                    self.spill(Role::Left, a, out)
                };
                let b = if b.reads_same() {
                    b
                } else {
                    self.spill(Role::Right, b, out)
                };
                let Shape::Array(parts) = self.shape(ty) else {
                    return Err(self.unsupported(span, &format!("`!=` on `{ty}`")));
                };
                let mut any: Option<Expr> = None;
                for (i, part) in parts.into_iter().enumerate() {
                    let at = |x: &Expr| Expr::index(x.clone(), Expr::int(i as i128));
                    let ne = self.ne_value(at(&a), at(&b), part, span, out)?;
                    any = Some(match any {
                        Some(any) => Expr::bin(Op::Or, any, ne),
                        None => ne,
                    });
                }
                Ok(any.unwrap_or_else(|| Expr::bool(false)))
            }
            ty::Adt(..) if let Some(pointee) = crate::lower::recognition::rc_pointee(self.tcx, ty) => {
                let first = self.rc_eq_first(ty, pointee, span)?;
                let counted = self.counted_rc(ty).is_some();
                let spill = first.is_some();
                let a = if !spill || a.reads_same() {
                    a
                } else {
                    self.spill(Role::Left, a, out)
                };
                let b = if !spill || b.reads_same() {
                    b
                } else {
                    self.spill(Role::Right, b, out)
                };
                let (x, y) = match counted {
                    true => (
                        self.counted_pointee(a.clone(), pointee),
                        self.counted_pointee(b.clone(), pointee),
                    ),
                    false => (a.clone(), b.clone()),
                };
                let ne = self.ne_value(x, y, pointee, span, out)?;
                Ok(match first {
                    Some(asked) => {
                        let same = match asked {
                            Some(asked) => Expr::bin(Op::And, asked, Expr::bin(Op::Eq, a, b)),
                            None => Expr::bin(Op::Eq, a, b),
                        };
                        Expr::bin(Op::And, Expr::unary(js::UnaryOp::Not, same), ne)
                    }
                    None => ne,
                })
            }
            ty::Adt(_, args) if ty.is_box() => self.ne_value(a, b, args.type_at(0), span, out),
            // A `Pin`'s target (`pin_target`, #1446).
            ty::Adt(..) if self.recognition().pinned(ty).is_some() => {
                let (x, target) = self.pin_target(a, ty, span, out)?;
                let (y, _) = self.pin_target(b, ty, span, out)?;
                self.ne_value(x, y, target, span, out)
            }
            _ => Err(self.unsupported(span, &format!("`!=` on `{ty}`"))),
        }
    }

    /// `(a, b) => <a != b>` for a dictionary's `ne`, of two `ty`s, as
    /// `ne_value` has it, or the function itself.
    pub(in crate::lower) fn ne_fn(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let mut body = Vec::new();
        let differ = self.ne_value(Expr::var("a"), Expr::var("b"), ty, span, &mut body)?;
        Ok(self.compare_fn(differ, body))
    }

    /// `(a, b) => <compared>`, `body` its statements first; of a function
    /// of both, as they're given, `$eq` or an impl's `eq`: it.
    pub(in crate::lower) fn compare_fn(&self, compared: Expr, mut body: Vec<Stmt>) -> Expr {
        if body.is_empty()
            && let js::ExprKind::Call(callee, args) = &compared.kind
            && matches!(callee.kind, js::ExprKind::Var(_))
            && matches!(args.as_slice(), [x, y] if matches!((&x.kind, &y.kind), (js::ExprKind::Var(x), js::ExprKind::Var(y)) if x == "a" && y == "b"))
        {
            return (**callee).clone();
        }
        body.push(StmtKind::Return(Some(compared)).at(js::Span::NONE));
        Expr::arrow(vec!["a".into(), "b".into()], body)
    }
}
