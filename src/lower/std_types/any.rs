//! `TypeId`, `Any` and a `dyn Any`'s downcasts (ADR 0331): a type's
//! `TypeId` is its key, rustc's name for it, and a `dyn Any` the pair of a
//! value and its `Any` dictionary, `{ type_id: () => "i32" }`.

use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::{Std, TypeFact, any_trait, linked_twice};
use crate::lower::std_types::result;
use crate::lower::{FnCx, R};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// What a `TypeId` or a `dyn Any` is asked.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum AnyOp {
    /// `TypeId::of::<T>()`.
    TypeIdOf,
    /// `x.type_id()`: its type's, a `dyn Any`'s its dictionary's.
    TypeIdOfValue,
    /// A `dyn Any`'s `is::<T>()`, `downcast_ref::<T>()` and `downcast_mut::<T>()`.
    Is,
    DowncastRef,
    DowncastMut,
    /// A `Box<dyn Any>`'s `downcast::<T>()`: `Ok` of its value, or `Err` of itself.
    Downcast,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `TypeId`'s or a `dyn Any`'s call: `None` if `known` is another.
    pub(in crate::lower) fn any_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Any(op) = known else {
            return Ok(None);
        };
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        // The type asked of, the last type argument: a `Box`'s allocator is first.
        let of = generic_args.types().last().expect("a type argument");
        if op == AnyOp::TypeIdOf {
            return self.type_fact_value(of, TypeFact::Id, span).map(Some);
        }
        let value = values.next().expect("rustc checked the arguments");
        if op == AnyOp::TypeIdOfValue {
            return self.type_id_of(value, tys[0].peel_refs(), span, out).map(Some);
        }
        let pair = if value.reads_same() {
            value
        } else {
            self.spill("any", value, out)
        };
        let any = tys[0].peel_refs();
        let is = Expr::bin(
            Op::Eq,
            self.type_id_of(pair.clone(), self.pointee_of(any), span, out)?,
            self.type_fact_value(of, TypeFact::Id, span)?,
        );
        let inside = Expr::held(pair.clone());
        Ok(Some(match op {
            AnyOp::Is => is,
            // `Some` boxed where it looks like `None` (ADR 0051).
            AnyOp::DowncastRef => {
                let some = if self.boxed_payload(of) {
                    self.some(inside)
                } else {
                    inside
                };
                Expr::cond(is, some, Expr::undefined())
            }
            // A `&mut` to a number or text is the pair, whose `value` it is
            // (ADR 0074).
            AnyOp::DowncastMut => {
                let found = if self.is_boxable(of) { pair } else { inside };
                Expr::cond(is, found, Expr::undefined())
            }
            // A counted `Rc`'s, itself, as what it points at is (ADR 0520).
            AnyOp::Downcast => {
                let found = match self.counted_rc(tys[0].peel_refs()) {
                    Some(_) => pair.clone(),
                    None => inside,
                };
                Expr::cond(is, result::ok(found), result::err(pair))
            }
            AnyOp::TypeIdOf | AnyOp::TypeIdOfValue => unreachable!("handled above"),
        }))
    }

    /// The `TypeId` of `value`, a `ty`: a `dyn Any`'s its dictionary's, a
    /// `dyn` of a trait under `Any`'s its supertrait's, any other its type's.
    fn type_id_of(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let ty::Dynamic(traits, ..) = ty.kind() else {
            if value.has_effects() {
                out.push(StmtKind::Expr(value).at(self.js_span(span)));
            }
            return self.type_fact_value(ty, TypeFact::Id, span);
        };
        let dictionary = Expr::member(value, js::DICTIONARY);
        let any = match traits.principal_def_id() {
            id if id.is_some() && id == any_trait(self.tcx) => dictionary,
            _ => self
                .any_of_dyn(ty, dictionary)
                .ok_or_else(|| self.unsupported(span, "the `TypeId` of this `dyn`"))?,
        };
        Ok(Expr::call(Expr::member(any, "type_id"), Vec::new()))
    }

    /// `value`, a `ty`, as JS is given it: a `dyn`'s value where its trait is
    /// nothing to JS, `Any`, which a binding takes as any JS value, or a
    /// marker of no methods, a union's, `&dyn IntoNumberOrStr` (ADR 0443),
    /// and of an `Option` of one, of each in a slice, an array or a `Vec`.
    pub(in crate::lower) fn any_given_to_js(&self, value: Expr, ty: Ty<'tcx>) -> Expr {
        let pointee = ty.peel_refs();
        // A pair made here, `{ value: v, impl }`: `v`.
        let made = |value: &Expr| match &value.kind {
            js::ExprKind::Object(props) => match props.as_slice() {
                [Prop::Field(name, inside), _] if name == js::HELD => Some(inside.clone()),
                _ => None,
            },
            _ => None,
        };
        if self.given_as_value(pointee) {
            return made(&value).unwrap_or_else(|| Expr::held(value));
        }
        if let Some(inner) = self.option_of(pointee)
            && self.given_as_value(inner.peel_refs())
        {
            return match value.kind {
                js::ExprKind::Undefined => value,
                _ => made(&value).unwrap_or_else(|| Expr::optional_member(value, js::HELD)),
            };
        }
        let item = match *pointee.kind() {
            ty::Slice(item) | ty::Array(item, _) => Some(item),
            ty::Adt(_, args) if self.is_vec_like(pointee) => args.types().next(),
            _ => None,
        };
        match item {
            Some(item) if self.given_as_value(item.peel_refs()) => {
                if let js::ExprKind::Array(items) = &value.kind {
                    return Expr::array(items.iter().map(|i| self.any_given_to_js(i.clone(), item)).collect());
                }
                let each = Expr::arrow(
                    vec!["item".into()],
                    vec![StmtKind::Return(Some(Expr::held(Expr::var("item")))).at(js::Span::NONE)],
                );
                Expr::call(Expr::member(value, "map"), vec![each])
            }
            _ => value,
        }
    }

    /// What a `Box`, an `Rc` or a reference points at.
    fn pointee_of(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        match ty.kind() {
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => args.type_at(0),
            _ => ty,
        }
    }

    /// The key a `TypeId` is: rustc's name for the type, which each crate
    /// rust-js compiles agrees on; and a type whose name doesn't tell it
    /// apart has rustc's hash of it too, which each agrees on as well: a
    /// function pointer's or a `dyn`'s, and so a closure's, `{{closure}}`,
    /// whose type holds its signature, a function pointer.
    pub(in crate::lower) fn type_id_key(&self, of: Ty<'tcx>) -> String {
        let name = rustc_const_eval::util::type_name(self.tcx, of);
        // A name leaves out lifetimes, which tell a function pointer's or a
        // `dyn`'s types apart, `for<'a> fn(&'a str) -> &'a str` from `..
        // -> &'static str`; and a crate linked at two versions has one path
        // for two types.
        let twice = of.walk().any(|part| {
            part.as_type().is_some_and(|t| match t.kind() {
                ty::Adt(adt, _) => linked_twice(self.tcx, adt.did()),
                ty::FnPtr(..) | ty::Dynamic(..) => true,
                _ => false,
            })
        });
        match twice {
            true => format!("{name}#{:x}", self.tcx.type_id_hash(of).as_u128()),
            false => name,
        }
    }

    /// A `dyn` JS is given the value of, its trait nothing to JS: `Any`, or a
    /// marker of no methods (ADR 0443).
    pub(in crate::lower) fn given_as_value(&self, ty: Ty<'tcx>) -> bool {
        let marker = matches!(ty.kind(), ty::Dynamic(predicates, ..) if predicates
            .principal_def_id()
            .is_some_and(|id| crate::lower::traits::is_marker(self.tcx, self.krate.foreign, id)));
        marker || self.recognition().is_dyn_any(ty)
    }
}
