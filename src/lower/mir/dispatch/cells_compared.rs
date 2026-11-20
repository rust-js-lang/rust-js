//! `x == &mut 1`, `p > q`, `ra.max(rb)`: std's comparisons of `&mut`s to
//! what JS can't change in place, cells, of what they point at, never the
//! handles (ADR 0099). Split from `dispatch.rs` by concept (#1334, #2188).

use super::*;
use crate::lower::recognition::OrderingCall;
use crate::lower::refusal;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A comparison of cells, its value; `None` where it's no comparison
    /// of one, `values` left as they are. One that reads them through
    /// `&self`, `==` and `cmp`, is the pointee's, given what each points at.
    pub(super) fn cells_compared(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (values, arg_tys, nulled): (&mut Vec<Value<'tcx>>, &[Ty<'tcx>], &[bool]),
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let tcx = self.tcx;
        if let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && matches!(
                self.recognition().trait_call(def_id, trait_id),
                Some(TraitCall::Equality { .. } | TraitCall::Ordering)
            )
            && generic_args.types().any(|t| self.is_cell(t))
        {
            // `ra.max(rb)` and `min` take the `&mut`s themselves and give one
            // back, std's `Ord for &mut A`'s: what they point at compared, the
            // handle chosen (#2188). `clamp`'s, whose `PartialOrd` its helper
            // is given would read them as generic code's boxes, is refused.
            let tr = ty::TraitRef::new_from_args(tcx, trait_id, generic_args);
            if let Some((call @ (OrderingCall::Max | OrderingCall::Min | OrderingCall::Clamp), _)) =
                self.recognition().ordering_call(def_id, tr)
            {
                if matches!(call, OrderingCall::Clamp) {
                    let why = "it gives one of them back, which its helper can't";
                    let refused = refusal::Refused::Because("`clamp` of `&mut` references", why);
                    return Err(refusal::refuse(tcx, span, refused));
                }
                let exprs = (std::mem::take(values).into_iter())
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?;
                let call = self.ordering_call(def_id, tr, exprs, span, out)?;
                return (call.map(Value::Expr))
                    .ok_or_else(|| self.unsupported(span, &format!("`{}` of `&mut` references", tcx.item_name(def_id))))
                    .map(Some);
            }
            let pointee = |t: Ty<'tcx>| match *t.kind() {
                ty::Ref(_, inner, ty::Mutability::Mut) if self.is_cell(t) => inner,
                _ => t,
            };
            let peeled = tcx.mk_args_from_iter(generic_args.iter().map(|a| match a.as_type() {
                Some(t) => pointee(t).into(),
                None => a,
            }));
            let mut tys = Vec::new();
            let mut given = Vec::new();
            for (value, &ty) in std::mem::take(values).into_iter().zip(arg_tys) {
                let (value, ty) = match *ty.kind() {
                    ty::Ref(region, inner, m)
                        if let ty::Ref(_, target, _) = *inner.kind()
                            && self.is_cell(inner) =>
                    {
                        let value = match value {
                            Value::Ref(place) => Value::Expr(place),
                            value => Value::Expr(Expr::held(self.value_expr(value, span)?)),
                        };
                        (value, Ty::new_ref(tcx, region, target, m))
                    }
                    _ => (value, ty),
                };
                given.push(value);
                tys.push(ty);
            }
            return (self.mir_call_values_of(state, (def_id, peeled), (given, &tys, nulled), output, span, out))
                .map(Some);
        }
        Ok(None)
    }
}
