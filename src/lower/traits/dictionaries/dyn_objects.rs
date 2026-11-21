//! A `dyn`'s dictionary, of its pair, `{ value, impl }`: each method an
//! arrow of the pair that calls the principal's entry, given the pair and
//! what it's given. Split from dictionaries.rs by concept, as a file is at
//! most 1500 lines.

use super::super::*;
use crate::lower::temporary_names::Base;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn object_dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        let object_param = self.fresh(Base::Named("object"));
        let object = Expr::var(object_param.clone());
        let principal = self
            .dyn_trait_ref(tr.self_ty(), tr.self_ty())
            .ok_or_else(|| self.no_evidence(span, tr))?;
        let mut props = self.supertrait_entries(tr, span)?;
        props.extend(self.item_bound_entries(tr, span)?);
        for item in self.tcx.associated_items(tr.def_id).in_definition_order() {
            // What a `dyn` can't be called with, and what a std trait's
            // dictionary hasn't got, as `lower_dictionary` leaves out.
            if !item.is_fn()
                || self.tcx.generics_require_sized_self(item.def_id)
                || (!self.krate.foreign.compiled_by_rust_js(tr.def_id)
                    && self.tcx.defaultness(item.def_id).has_value()
                    && !in_std_dictionary(self.tcx, item.def_id))
            {
                continue;
            }
            let inputs = self
                .tcx
                .fn_sig(item.def_id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs();
            // Without a `Formatter`, which isn't a JS parameter (ADR 0054), and
            // with a writer's options, where the crate's take them (ADRs 0058, 0137).
            let at = self.formatter_param(item.def_id);
            let writer = at.is_some();
            // Or a type's a library's function shows with them, whose entry
            // takes them, passed on where it does (#1331).
            let options = self.writers_take_options() || self.krate.shown.reached().any();
            let count = inputs.len() - usize::from(writer) + usize::from(options && writer);
            // A `&mut self` method is given a box of the pair, as generic code
            // has a `&mut T` (ADR 0099), and gives the pair, as a call on the
            // `dyn` does: `object.value.impl.scale(object.value, k)`.
            let mutable = matches!(inputs[0].kind(), ty::Ref(_, _, Mutability::Mut));
            let pair = match mutable {
                true => Expr::held(object.clone()),
                false => object.clone(),
            };
            let dictionary = self
                .super_evidence(principal, tr, Expr::member(pair.clone(), js::DICTIONARY))
                .ok_or_else(|| self.no_evidence(span, tr))?;
            let this = match mutable {
                true => pair,
                false => Expr::held(pair),
            };
            let mut params: Vec<js::Pattern> = vec![object_param.clone().into()];
            let mut values = vec![this];
            // Each named as the trait's method names it (#1328).
            let names = crate::lower::signatures::taken_parameter_names(self.tcx, item.def_id, at, options);
            for i in 1..count {
                let named = names.get(i).cloned().flatten().unwrap_or_else(|| format!("arg{i}"));
                let arg = self.fresh(Base::Named(&named));
                params.push(arg.clone().into());
                values.push(Expr::var(arg));
            }
            let name = bindings::fn_name(self.tcx, item.def_id);
            let call = Expr::call(Expr::member(dictionary.clone(), name.clone()), values);
            props.push(Prop::Field(
                name,
                Expr::arrow(params, vec![StmtKind::Return(Some(call)).at(js::Span::NONE)]),
            ));
        }
        props.extend(self.item_drops(tr, false, span)?);
        Ok(Expr::object(props))
    }
}
