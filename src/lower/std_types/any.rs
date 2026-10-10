//! `TypeId`, `Any` and a `dyn Any`'s downcasts (ADR 0331): a type's
//! `TypeId` is its key, rustc's name for it, and a `dyn Any` the pair of a
//! value and its `Any` dictionary, `{ type_id: () => "i32" }`.

use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::{Std, TypeFact, any_trait, linked_twice};
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
        let inside = Expr::member(pair.clone(), "value");
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
            AnyOp::Downcast => Expr::cond(is, Self::ok(inside), Self::err(pair)),
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
        let dictionary = Expr::member(value, "impl");
        let any = match traits.principal_def_id() {
            id if id.is_some() && id == any_trait(self.tcx) => dictionary,
            _ => self
                .any_of_dyn(ty, dictionary)
                .ok_or_else(|| self.unsupported(span, "the `TypeId` of this `dyn`"))?,
        };
        Ok(Expr::call(Expr::member(any, "type_id"), Vec::new()))
    }

    /// `value`, a `ty`, as JS is given it: a `dyn Any`'s value, which a
    /// binding takes as any JS value, of each in a slice, an array or a
    /// `Vec` of them too.
    pub(in crate::lower) fn any_given_to_js(&self, value: Expr, ty: Ty<'tcx>) -> Expr {
        let pointee = ty.peel_refs();
        if self.recognition().is_dyn_any(pointee) {
            // A pair made here, `{ value: v, impl }`: `v`.
            if let js::ExprKind::Object(props) = &value.kind
                && let [Prop::Field(name, inside), _] = props.as_slice()
                && name == "value"
            {
                return inside.clone();
            }
            return Expr::member(value, "value");
        }
        let item = match *pointee.kind() {
            ty::Slice(item) | ty::Array(item, _) => Some(item),
            ty::Adt(_, args) if self.is_vec_like(pointee) => args.types().next(),
            _ => None,
        };
        match item {
            Some(item) if self.recognition().is_dyn_any(item.peel_refs()) => {
                if let js::ExprKind::Array(items) = &value.kind {
                    return Expr::array(items.iter().map(|i| self.any_given_to_js(i.clone(), item)).collect());
                }
                let each = Expr::arrow(
                    vec!["item".into()],
                    vec![StmtKind::Return(Some(Expr::member(Expr::var("item"), "value"))).at(js::Span::NONE)],
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
    /// rust-js compiles agrees on. Not a closure's, which share a name; and a
    /// type whose name doesn't tell it apart has its hash too.
    pub(in crate::lower) fn type_id_key(&self, of: Ty<'tcx>, span: Span) -> R<String> {
        let unnamed = of.walk().any(|part| {
            part.as_type().is_some_and(|t| {
                matches!(
                    t.kind(),
                    ty::Closure(..) | ty::CoroutineClosure(..) | ty::Coroutine(..) | ty::CoroutineWitness(..)
                )
            })
        });
        if unnamed {
            return Err(self.unsupported(span, "the `TypeId` of a closure's type"));
        }
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
        Ok(match twice {
            true => format!("{name}#{:x}", self.tcx.type_id_hash(of).as_u128()),
            false => name,
        })
    }
}
