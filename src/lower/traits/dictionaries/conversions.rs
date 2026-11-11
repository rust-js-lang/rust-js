//! A conversion's dictionary: std's `Into` and `From`, of a type known
//! here, its conversion or the crate's `From` it runs. Split from
//! dictionaries.rs by concept, as a file is at most 1500 lines.

use super::super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `x.into()` of a `T: Into<U>` (ADR 0108), and `W::from(u)` of a `W:
    /// From<U>` (ADR 0477): std's conversion, of a `T` to itself too (ADR
    /// 0387), or the crate's `From`; `None` of another trait.
    pub(super) fn conversion_dictionary(
        &mut self,
        tr: ty::TraitRef<'tcx>,
        ty: Ty<'tcx>,
        span: Span,
    ) -> R<Option<Expr>> {
        let into = is_std_def(self.tcx, tr.def_id, StdItem::Into);
        if into || is_std_def(self.tcx, tr.def_id, StdItem::From) {
            let method = self.tcx.associated_item_def_ids(tr.def_id)[0];
            let (target, source) = match into {
                true => (tr.args.type_at(1), ty),
                false => (ty, tr.args.type_at(1)),
            };
            let from = std_item(self.tcx, StdItem::From);
            let from = self.tcx.associated_item_def_ids(from)[0];
            let from_args = self.tcx.mk_args(&[target.into(), source.into()]);
            let value = if let Some(known) = self.recognition().classify(method, tr.args)
                && let Some(f) = self.std_fn_value(
                    known,
                    Ty::new_fn_def(self.tcx, method, ty::Binder::dummy(tr.args)),
                    span,
                )? {
                f
            } else if let Some(instance) = self.resolve_instance(from, from_args)?
                && self.is_rust_fn(instance.def_id())
                && self.tcx.trait_of_assoc(instance.def_id()).is_none()
            {
                let callee = self.fn_ref(instance.def_id());
                let mut values = vec![Expr::var("value")];
                values.extend(self.evidence_args(instance.def_id(), instance.args, span)?);
                match values.len() {
                    1 => callee,
                    _ => Expr::arrow(
                        vec!["value".into()],
                        vec![StmtKind::Return(Some(Expr::call(callee, values))).at(js::Span::NONE)],
                    ),
                }
            } else {
                return Err(self.no_evidence(span, tr));
            };
            let name = if into { "into" } else { "from" };
            return Ok(Some(Expr::object(vec![Prop::Field(name.into(), value)])));
        }
        Ok(None)
    }
}
