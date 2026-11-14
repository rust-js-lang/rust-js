//! What a std struct that's a `SourceIter` gives a collect in place: the
//! step its `as_inner` takes to what it holds, and that dropped (#852).

use super::*;
use crate::lower::recognition::SourceStep;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of std's struct `ty` that's a `SourceIter`, its `as_inner`'s one
    /// step, what it holds that a collect in place reaches the buffer it
    /// reuses through, `inner`, `$someValue(iterator.iter)` of a `Fuse`'s;
    /// and that dropped, `dropInner`, of what has a drop. A collect in
    /// place drops what's left of its source alone as its guard unwinds,
    /// std's `forget_allocation_drop_remaining`, then the adapters with no
    /// guard, a `map`'s function, whose drop's panic leaks what it
    /// collected (#852).
    pub(super) fn source_iter_steps(&mut self, ty: Ty<'tcx>, span: Span) -> R<Vec<js::Prop>> {
        let made_iterator = self.unspelled("iterator");
        let ty::Adt(adt, _) = ty.peel_refs().kind() else {
            return Ok(Vec::new());
        };
        let Some(steps) = crate::lower::recognition::source_iter_step(self.tcx, adt.did()) else {
            return Ok(Vec::new());
        };
        let (mut inner, mut held) = (Expr::var(&made_iterator), ty.peel_refs());
        for step in steps {
            let ty::Adt(of, args) = held.kind() else {
                unreachable!("`as_inner` reads a field of a struct or an `Option`'s `Some`")
            };
            (inner, held) = match step {
                SourceStep::SomeValue => (self.some_value(inner), args.type_at(0)),
                SourceStep::Field(field) => {
                    let field = &of.non_enum_variant().fields[field];
                    let key = crate::lower::bindings::field_key(self.tcx, field);
                    let field_ty = field.ty(self.tcx, args).skip_normalization();
                    (
                        Expr::member(inner, key.text()),
                        self.tcx.erase_and_anonymize_regions(field_ty),
                    )
                }
            };
        }
        let mut drop = Vec::new();
        self.drop_value(inner.clone(), held, span, &mut drop)?;
        let returned = vec![StmtKind::Return(Some(inner)).at(js::Span::NONE)];
        let mut props = vec![js::Prop::Field(
            "inner".into(),
            Expr::arrow(vec![made_iterator.as_str().into()], returned),
        )];
        if !drop.is_empty() {
            props.push(js::Prop::Field(
                "dropInner".into(),
                Expr::arrow(vec![made_iterator.as_str().into()], drop),
            ));
        }
        Ok(props)
    }
}
