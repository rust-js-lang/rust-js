//! A conversion's dictionary: std's `Into` and `From`, of a type known
//! here, its conversion or the crate's `From` it runs. Split from
//! dictionaries.rs by concept, as a file is at most 1500 lines.

use super::super::*;
use crate::lower::generic_facts::Arguments;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `x.into()` of a `T: Into<U>` (ADR 0108), and `W::from(u)` of a `W:
    /// From<U>` (ADR 0477): std's conversion, of a `T` to itself too (ADR
    /// 0387), or the crate's `From`; `None` of another trait.
    pub(super) fn conversion_dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Option<Expr>> {
        // `x.try_into()` of a `U: TryFrom<T>` and `W::try_from(u)` too,
        // std's `Result` of one (#1284).
        // Its direction, `recognition::conversion_by_trait`'s, of its `Self`
        // and its argument (#2166).
        if let Some((target, source, tried)) =
            crate::lower::recognition::conversion_by_trait(self.tcx, tr.def_id, tr.args)
        {
            let method = (self.tcx.associated_items(tr.def_id).in_definition_order())
                .find(|item| item.is_fn())
                .expect("a conversion's method")
                .def_id;
            let from = std_item(self.tcx, if tried { StdItem::TryFrom } else { StdItem::From });
            let from = (self.tcx.associated_items(from).in_definition_order())
                .find(|item| item.is_fn())
                .expect("a conversion's method")
                .def_id;
            let from_args = self.tcx.mk_args(&[target.into(), source.into()]);
            // A `Box<dyn Error>` of an error or a message, std's `From`, as
            // its call makes it, `{ value, impl }` (ADR 0141, #1284).
            let param = self.unspelled("value");
            let value = if !tried && let Some(dyn_error) = self.dyn_error_from(target, source, span)? {
                Expr::arrow(
                    vec![param.clone().into()],
                    vec![
                        StmtKind::Return(Some(Expr::object(vec![
                            Prop::Field(js::HELD.into(), Expr::var(&param)),
                            Prop::Field(js::DICTIONARY.into(), dyn_error),
                        ])))
                        .at(js::Span::NONE),
                    ],
                )
            } else if let Some(known) = self.recognition().classify(method, tr.args)
                && let Some(f) = self.std_fn_value(
                    known,
                    Ty::new_fn_def(self.tcx, method, ty::Binder::dummy(tr.args)),
                    span,
                )?
            {
                f
            } else if let Some(known) = self.recognition().classify(method, tr.args)
                && tried
            {
                // std's try conversion as its call is written where its
                // types are known, of the value the entry's given (#1284).
                let tys = [source];
                let call = crate::lower::calls::Call::given((method, tr.args), &tys, Default::default(), span);
                let mut body = Vec::new();
                let mut given = vec![Expr::var(&param)].into_iter();
                let Some(made) = self.numeric_call(known, call, &mut given, &mut body)? else {
                    return Err(self.no_evidence(span, tr));
                };
                body.push(StmtKind::Return(Some(made)).at(js::Span::NONE));
                Expr::arrow(vec![param.clone().into()], body)
            } else if let Some(value) = self.compiled_conversion(from, from_args, span)? {
                value
            } else if !tried && self.recognition().calls_from_giving_its_argument(from, from_args) {
                // A binding crate's `From` into an untagged enum, the value
                // it's given (ADR 0214, amended): `T: Into<BlobPart>`'s.
                Expr::arrow(
                    vec![param.clone().into()],
                    vec![StmtKind::Return(Some(Expr::var(&param))).at(js::Span::NONE)],
                )
            } else {
                return Err(self.no_evidence(span, tr));
            };
            let name = bindings::fn_name(self.tcx, method);
            return Ok(Some(Expr::object(vec![Prop::Field(name, value)])));
        }
        Ok(None)
    }

    /// Whether `tr`'s dictionary is a conversion's (`conversion_dictionary`),
    /// whose entry takes what it's given as its caller holds it and gives it
    /// what it runs in that one's forms (`compiled_conversion`), as no impl's
    /// dictionary's entry does (`placed_dictionary`).
    pub(super) fn is_conversion(&self, tr: ty::TraitRef<'tcx>) -> bool {
        crate::lower::recognition::conversion_by_trait(self.tcx, tr.def_id, tr.args).is_some()
    }

    /// The function a conversion, `method` (`From::from`, `TryFrom::try_from`)
    /// of `args`, runs where its impl is one rust-js compiled, the crate's
    /// own or a library's (`is_rust_fn`): the impl's method, given its
    /// evidence after the value. The one answer the conversion's
    /// dictionary and `?` (`error_from`) take, so `?` in a library's code
    /// the crate compiles itself converts with the library's `From`,
    /// rand_core's `OsRng::try_fill_bytes` (#1420).
    pub(in crate::lower) fn compiled_conversion(
        &mut self,
        method: DefId,
        args: ty::GenericArgsRef<'tcx>,
        span: Span,
    ) -> R<Option<Expr>> {
        let Some(instance) = self.resolve_instance(method, args)? else {
            return Ok(None);
        };
        if !self.is_rust_fn(instance.def_id()) || self.tcx.trait_of_assoc(instance.def_id()).is_some() {
            return Ok(None);
        }
        // Given and giving back in its generic code's forms, as a direct
        // call of it is: a tuple of iterators held in places of their own,
        // itertools' `multizip` (#2255).
        let called = (instance.def_id(), instance.args);
        self.refuse_placed_within(called, span)?;
        let sig = (self.tcx).instantiate_bound_regions_with_erased(
            self.tcx
                .fn_sig(instance.def_id())
                .instantiate_identity()
                .skip_normalization(),
        );
        let output = (
            instance.def_id(),
            sig.output(),
            self.instantiated(sig.output(), instance.args),
        );
        let callee = self.fn_ref(instance.def_id());
        let param = self.unspelled("value");
        // What it's given crosses into its generic code as a direct call's
        // does, made a JS iterator where it steps one (#2264).
        let mut values = vec![Expr::var(&param)];
        let mut made = Vec::new();
        self.crossing_into(
            (instance.def_id(), instance.args),
            &mut values,
            Arguments::Raw,
            span,
            &mut made,
        )?;
        let given_as_it_is = made.is_empty() && values[0] == Expr::var(&param);
        values.extend(self.evidence_args(instance.def_id(), instance.args, span)?);
        let back = self.form_of(output);
        if values.len() == 1 && back.is_none() && given_as_it_is {
            return Ok(Some(callee));
        }
        let called = Expr::call(callee, values);
        let called = match back {
            Some(form) => self.form_back(form, output, called),
            None => called,
        };
        made.push(StmtKind::Return(Some(called)).at(js::Span::NONE));
        Ok(Some(Expr::arrow(vec![param.clone().into()], made)))
    }
}
