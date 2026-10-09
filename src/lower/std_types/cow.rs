//! `Cow` (ADR 0319): its enum, `{ TAG, _0 }` (ADR 0033), what it borrowed
//! or owns, made owned by a clone of what it borrowed.

use rustc_middle::thir::ExprId;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use crate::js::{Expr, Op, Stmt, StmtKind};
use crate::lower::bindings::{tag_key, variant_name};
use crate::lower::{FnCx, R};

/// A `Cow`'s own methods.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum CowOp {
    /// `Deref`: what it borrows or owns, one JS value.
    Deref,
    IntoOwned,
    ToMut,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `into_owned()`: what it owns, or a clone of what it borrowed; and
    /// `to_mut()`: made owned so, the `&mut` to what it owns.
    pub(in crate::lower) fn cow_call(
        &mut self,
        op: CowOp,
        args: &[ExprId],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let cow_ty = self.thir[args[0]].ty.peel_refs();
        let Some((_, owned)) = self.recognition().cow_parts(cow_ty) else {
            return Err(self.unsupported(span, &format!("a `{cow_ty}`, of a type with its own `ToOwned`")));
        };
        let ty::Adt(adt, _) = *cow_ty.kind() else {
            unreachable!("a `Cow`'s method is given a `Cow`")
        };
        let mut variants = adt.variants().iter();
        let (borrowed_variant, owned_variant) =
            (variants.next().expect("`Borrowed`"), variants.next().expect("`Owned`"));
        // `to_mut()` of text or a number, which JS never changes in place, is
        // a handle, `$cowMut` (`item_handles`).
        let cow = self.expr(args[0], out)?;
        if op == CowOp::Deref || op == CowOp::IntoOwned && !self.needs_clone(owned) {
            return Ok(Expr::member(cow, "_0"));
        }
        let cow = if cow.reads_same() {
            cow
        } else {
            self.spill("cow", cow, out)
        };
        let tag = Expr::member(cow.clone(), tag_key(self.tcx, adt.did()));
        let borrowed = Expr::bin(Op::Eq, tag.clone(), Expr::str(variant_name(self.tcx, borrowed_variant)));
        let value = Expr::member(cow.clone(), "_0");
        let clone = self.owned_clone(value.clone(), owned, span, out)?;
        if op == CowOp::IntoOwned {
            return Ok(Expr::cond(borrowed, clone, value));
        }
        let js_span = self.js_span(span);
        let made_owned = vec![
            StmtKind::Assign(tag, Expr::str(variant_name(self.tcx, owned_variant))).at(js_span),
            StmtKind::Assign(value.clone(), clone).at(js_span),
        ];
        out.push(StmtKind::If(borrowed, made_owned, None).at(js_span));
        Ok(value)
    }

    /// What `to_owned()` makes of what a `Cow` borrowed: a slice's is a new
    /// array, which its `Vec` may change; another's its `Clone`'s.
    fn owned_clone(&mut self, value: Expr, owned: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        match owned.kind() {
            ty::Adt(_, args) if self.is_vec_like(owned) => self.clone_items(value, args.type_at(0), span),
            _ => self.clone_value(value, owned, span, out),
        }
    }
}
