//! A value only a dictionary knows, a `T`'s or a `dyn`'s, written into a
//! writer's sink a piece at a time (#1652, step 5 of ADR 1135): its
//! formatting dictionary's `fmtInto(f, x, options)`, which writes each
//! piece std's `fmt` of it writes into `f`, as a call of the writer's. The
//! one owner of that entry: what a dictionary of the crate's own impl
//! carries, `$writingInto` of its writer; what a std value's carries, its
//! writes as `sink_writes` says; and the call of it. A dictionary carries
//! one only where `analysis::written_params` says a type reaches a
//! parameter written so, or a `dyn` written so, so no other JS changes.

use super::*;
use crate::lower::temporary_names::Role;

/// The entry's name, on each formatting dictionary that carries it.
pub(in crate::lower) const FMT_INTO: &str = "fmtInto";

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The formatting trait `kind` shows a value by: `Display`, `Debug`, a
    /// radix's, `LowerExp` or `UpperExp`, or `Pointer`.
    fn fmt_trait_of(&self, kind: Std) -> Option<DefId> {
        match kind {
            Std::FmtDisplay => Some(self.display_trait()),
            Std::FmtDebug => Some(self.debug_trait()),
            _ => self.recognition().other_fmt_trait(kind),
        }
    }

    /// Is `trait_id` a formatting trait, whose dictionaries may carry
    /// `fmtInto`?
    fn is_fmt_trait(&self, trait_id: DefId) -> bool {
        trait_id == self.display_trait()
            || trait_id == self.debug_trait()
            || self.radix_trait(trait_id).is_some()
            || [Std::FmtExp(false), Std::FmtExp(true), Std::FmtPointer]
                .into_iter()
                .any(|kind| self.recognition().other_fmt_trait(kind) == Some(trait_id))
    }

    /// What writes `value`, a `kind` of `ty`, into a writer's sink where only
    /// a dictionary knows how, given `options`, a `Formatter`'s: a `T`'s, its
    /// caller's dictionary's `fmtInto`, `TDisplay.fmtInto(f, t, options)`;
    /// a `dyn`'s that's a pair, its pair's, `d.impl.fmtInto(f, d.value,
    /// options)`. `None` of a value something else writes.
    pub(in crate::lower) fn written_by_dictionary(
        &mut self,
        value: Expr,
        (kind, ty): (Std, Ty<'tcx>),
        options: Expr,
        span: Span,
    ) -> R<Option<SinkWrite>> {
        let Some(trait_id) = self.fmt_trait_of(kind) else {
            return Ok(None);
        };
        let ty = ty.peel_refs();
        let (dictionary, value) = if let ty::Dynamic(traits, ..) = ty.kind() {
            // A `dyn Debug`'s, its pair's own dictionary, its type's `Debug`
            // (ADR 0060, amended).
            let debug = kind == Std::FmtDebug && traits.principal_def_id() == Some(self.debug_trait());
            // A pair's `Display`, a `dyn Display`'s or a `dyn Error`'s; the
            // `Debug` of another isn't followed yet (#1652), as std's
            // errors' aren't.
            if !debug && (self.dynamic_trait(ty).is_none() || kind != Std::FmtDisplay) {
                return Ok(None);
            }
            let held = Expr::member(value.clone(), js::DICTIONARY);
            let dictionary = match debug {
                true => held,
                false => {
                    let from = self.dyn_trait_ref(ty, ty).expect("a principal");
                    let to = ty::TraitRef::new(self.tcx, trait_id, [ty]);
                    let Some(dictionary) = self.super_evidence(from, to, held) else {
                        return Ok(None);
                    };
                    dictionary
                }
            };
            if !value.parts_read_same() {
                return Err(self.unsupported(span, &format!("showing a `{ty}` made where it's shown")));
            }
            (dictionary, Expr::held(value))
        } else if self.formatted_by_dictionary(trait_id, ty) {
            let tr = ty::TraitRef::new(self.tcx, trait_id, [ty]);
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            (dictionary, value)
        } else {
            return Ok(None);
        };
        Ok(Some(SinkWrite::Call(
            Expr::member(dictionary, FMT_INTO),
            vec![value, options],
        )))
    }

    /// The `fmtInto` entry of the dictionary of `tr`, an impl of the crate's
    /// own of a formatting trait whose `fmt` entry is in `props`: its writer
    /// called with the sink armed for it, `(f, value, options) =>
    /// $writingInto(f, pointDisplay_fmt, value, options)`. None where
    /// nothing writes its type so; refused where its writer doesn't write
    /// through a sink (#1652).
    pub(in crate::lower) fn impl_fmt_into(
        &mut self,
        tr: ty::TraitRef<'tcx>,
        props: &[js::Prop],
        span: Span,
    ) -> R<Option<js::Prop>> {
        let adt = match tr.self_ty().kind() {
            ty::Adt(adt, _) if adt.did().is_local() => adt.did(),
            _ => return Ok(None),
        };
        if !self.krate.written.of_adt(adt) || !self.is_fmt_trait(tr.def_id) {
            return Ok(None);
        }
        let fmt = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "fmt");
        let writes_through_sink = match self.resolve_self_instance(fmt, tr.args) {
            Ok(Some(instance)) => instance.def_id().is_local() && self.writes_through_sink(instance.def_id()),
            _ => false,
        };
        let entry = (props.iter()).find_map(|prop| match prop {
            js::Prop::Field(name, value) if name == "fmt" => Some(value.clone()),
            _ => None,
        });
        let (Some(entry), true) = (entry, writes_through_sink) else {
            let what = format!(
                "a `{}` written into a writer through generic code or a `dyn` whose writer doesn't write a piece at a time (#1652)",
                tr.self_ty()
            );
            return Err(self.unsupported(span, &what));
        };
        let (sink, value, options) = (self.fresh(Role::F), self.fresh(Role::Value), self.fresh(Role::Options));
        // A generic impl's writer, given its dictionaries after what it's
        // given, `(arg0, arg1) => pairDisplay_fmt(arg0, arg1, TDisplay)`:
        // `$writingInto(f, pairDisplay_fmt, value, options, TDisplay)`.
        let (writer, extra) = match &entry.kind {
            js::ExprKind::Arrow(params, body) => js::passes_on_before(params, body, false)
                .filter(|(callee, _)| matches!(callee.kind, js::ExprKind::Var(_)))
                .map(|(callee, extra)| (callee.clone(), extra.to_vec())),
            _ => None,
        }
        .unwrap_or((entry, Vec::new()));
        let mut args = vec![Expr::var(&sink), writer, Expr::var(&value), Expr::var(&options)];
        args.extend(extra);
        let armed = Expr::call_helper("$writingInto", args);
        Ok(Some(js::Prop::Field(
            FMT_INTO.into(),
            Expr::arrow(
                vec![sink.into(), value.into(), options.into()],
                vec![StmtKind::Return(Some(armed)).at(js::Span::NONE)],
            ),
        )))
    }

    /// The `fmtInto` entry of a std dictionary of `kind` of `ty`, where a
    /// type that reaches a parameter written so is `ty`: its writes as
    /// `sink_writes_given`, or for `Debug` `debug_sink_writes`, makes them,
    /// given the `Formatter`'s options as it runs. Refused where its split
    /// isn't followed yet (#1652).
    pub(in crate::lower) fn std_fmt_into(&mut self, kind: Std, ty: Ty<'tcx>, span: Span) -> R<Option<js::Prop>> {
        if !self.krate.written.of_type(self.tcx, ty) {
            return Ok(None);
        }
        let (sink, value, options) = (self.fresh(Role::F), self.fresh(Role::Value), self.fresh(Role::Options));
        let given = Expr::var(&options);
        let shown = ty.peel_refs();
        let writes = match kind {
            Std::FmtDebug => {
                let pretty = Pretty::When(given.clone()).alternate();
                self.debug_sink_writes(Expr::var(&value), shown, (&given, &pretty), span)?
            }
            _ => self.sink_writes_given(Expr::var(&value), (kind, shown), given, span)?,
        };
        let Some(writes) = writes else {
            return Err(self.refused_piecewise(span, &format!("a `{ty}` written into a writer through generic code")));
        };
        // One std function given just the value and its options is the
        // entry, `fmtInto: $writePadded`.
        let (value_var, options_var) = (Expr::var(&value), Expr::var(&options));
        if let [SinkWrite::Helper(name, args)] = &writes[..]
            && args[..] == [value_var, options_var]
        {
            return Ok(Some(js::Prop::Field(FMT_INTO.into(), Expr::helper(name))));
        }
        let body = (writes.into_iter())
            .map(|write| write.written_into(&Expr::var(&sink), js::Span::NONE))
            .collect();
        Ok(Some(js::Prop::Field(
            FMT_INTO.into(),
            Expr::arrow(vec![sink.into(), value.into(), options.into()], body),
        )))
    }
}
