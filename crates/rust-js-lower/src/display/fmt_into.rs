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
use crate::generic_facts::Form;
use crate::temporary_names::Role;

/// The entry's name, on each formatting dictionary that carries it.
pub(crate) const FMT_INTO: &str = "fmtInto";

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The formatting trait `kind` shows a value by: `Display`, `Debug`, a
    /// radix's, `LowerExp` or `UpperExp`, or `Pointer`.
    pub(crate) fn fmt_trait_of(&self, kind: Std) -> Option<DefId> {
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
    /// options)`; a library's type's whose contract lists it, its
    /// dictionary's, `Dep.hexyDebug.fmtInto(f, h, options)`, as its library
    /// compiled its writer. `None` of a value something else writes.
    pub(crate) fn written_by_dictionary(
        &mut self,
        value: Expr,
        (kind, ty): (Std, Ty<'tcx>),
        options: Expr,
        span: Span,
    ) -> R<Option<SinkWrite>> {
        let Some(trait_id) = self.fmt_trait_of(kind) else {
            return Ok(None);
        };
        let ty = self.shown_ref_type(kind, ty);
        let (dictionary, value) = if let ty::Dynamic(..) = ty.kind() {
            // A pair's, by its dictionary of the trait or of the one it
            // extends: a `dyn Debug`'s own, its type's `Debug` (ADR 0060,
            // amended); a `dyn Display`'s or a `dyn Error`'s supertrait's, its
            // `Display`, or a `dyn Error`'s `Debug`, which each of std's
            // errors' dictionaries writes as std's `fmt` does, a piece at a
            // time (#2250).
            if self.dynamic_trait(ty).is_none() {
                return Ok(None);
            }
            let held = Expr::member(value.clone(), js::DICTIONARY);
            let from = self.dyn_trait_ref(ty, ty).expect("a principal");
            let to = ty::TraitRef::new(self.tcx, trait_id, [ty]);
            let Some(dictionary) = self.super_evidence(from, to, held) else {
                return Ok(None);
            };
            if !value.parts_read_same() {
                return Err(self.unsupported(span, &format!("showing a `{ty}` made where it's shown")));
            }
            (dictionary, Expr::held(value))
        } else if self.formatted_by_dictionary(trait_id, ty) {
            let tr = ty::TraitRef::new(self.tcx, trait_id, [ty]);
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            (dictionary, value)
        } else if self.recognition().formatted_by(trait_id, ty).is_some_and(|adt| {
            // Not std's, whose compiled writer is armed itself
            // (`writes_into_a_writer`), its dictionary made here of it. Of
            // the type its impl is of, `impl Display for &dep::ByRef`'s
            // `ByRef`, where `ty` kept a wrapper of its own (#1798).
            self.krate.foreign.lists_fmt_into(adt) && crate::recognition::std_crate(self.tcx, adt.krate).is_none()
        }) {
            let tr = ty::TraitRef::new(self.tcx, trait_id, [ty]);
            // Given as its dictionary takes its `Self`, as a type parameter
            // given it holds it: a `&mut` to an object, `impl Display for
            // &mut dep::ByRef`'s, generic code's box, as a `dyn` of it is
            // given (#1824).
            let value = match self.in_generic_box(ty) {
                true => {
                    let declared = ty::GenericArgs::identity_for_item(self.tcx, trait_id).type_at(0);
                    self.form_given(Form::Box, (trait_id, declared, ty), value, span)?
                }
                false => value,
            };
            (self.dictionary(tr, span)?, value)
        } else {
            return Ok(None);
        };
        // An armed sink's pieces, its `fmtInto`'s; an unarmed one's text,
        // its `fmt`'s (`$writeBy`).
        Ok(Some(SinkWrite::Helper("$writeBy", vec![dictionary, value, options])))
    }

    /// The `fmtInto` entry of the dictionary of `tr`, an impl of the crate's
    /// own of a formatting trait whose `fmt` entry is in `props`: its writer
    /// called with the sink armed for it, `(f, value, options) =>
    /// $writingInto(f, pointDisplay_fmt, value, options)`. None where
    /// nothing writes its type so; refused where its writer doesn't write
    /// through a sink (#1652).
    pub(crate) fn impl_fmt_into(
        &mut self,
        tr: ty::TraitRef<'tcx>,
        props: &[js::Prop],
        span: Span,
    ) -> R<Option<js::Prop>> {
        // Of the crate's type, or a library's whose impl the crate compiles
        // its own of, which its contract lists (`listed_writers_compiled_here`),
        // or of `@rust-js/std`'s build, a std type whose writer it compiles
        // and frames, `fuseDebug`'s (#1712).
        if !self.is_fmt_trait(tr.def_id) {
            return Ok(None);
        }
        let fmt = crate::recognition::trait_method(self.tcx, tr.def_id, "fmt");
        let instance = self.resolve_self_instance(fmt, tr.args).ok().flatten();
        // The type its impl is of, `impl Display for &ByRef`'s `ByRef`
        // (#1798).
        let of_impl = instance.and_then(|instance| self.tcx.impl_of_assoc(instance.def_id()));
        let Some(adt) = of_impl.and_then(|imp| crate::recognition::formatted_adt(self.tcx, imp)) else {
            return Ok(None);
        };
        let compiled_std = instance.is_some_and(|instance| {
            crate::recognition::is_compiled_std(self.tcx, instance.def_id())
                && self.writes_through_sink(instance.def_id())
        });
        let listed = !adt.is_local() && !compiled_std && self.krate.foreign.lists_fmt_into(adt);
        if !adt.is_local() && !listed && !compiled_std {
            return Ok(None);
        }
        // And of a type whose writer may print as it formats, so where a
        // generic writer writes it, a print lands between its pieces, as
        // Rust's line has it, not before its text (#1608, #1712).
        let printing = self.krate.prints.framed_generic && self.krate.prints.types.contains(&adt);
        if !(listed || printing || self.krate.written.of_adt(adt, tr.def_id)) {
            return Ok(None);
        }
        let (writes_through_sink, takes_options) = match instance {
            Some(instance) => (
                (instance.def_id().is_local() || listed || compiled_std) && self.writes_through_sink(instance.def_id()),
                // As the dictionary's own `fmt` entry gives them, of a
                // writer this build compiles (`traits::impls`).
                self.compiled_writer_takes_options(instance.def_id()),
            ),
            None => (false, false),
        };
        let entry = (props.iter()).find_map(|prop| match prop {
            js::Prop::Field(name, value) if name == "fmt" => Some(value.clone()),
            _ => None,
        });
        // Reached only as a field of a type whose writer never shows it,
        // which framing leaves, nothing writes so (`given_adt`).
        if !writes_through_sink && !listed && !self.krate.written.given_adt(adt, tr.def_id) {
            return Ok(None);
        }
        let (Some(entry), true) = (entry, writes_through_sink) else {
            // Of a value a library makes a `dyn` of alone: none, which its
            // contract says (#1797).
            if !(listed || printing) && self.krate.written.best_effort_adt(adt, tr.def_id) {
                self.dependencies.borrow_mut().fmt_into_left = true;
                return Ok(None);
            }
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
        // Its options only where it takes them, as its `fmt` entry calls it:
        // a generic impl's dictionaries come next, `pairDisplay_fmt(pair,
        // TDisplay)` (#1739).
        let mut args = vec![Expr::var(&sink), writer, Expr::var(&value)];
        let mut params: Vec<js::Pattern> = vec![sink.into(), value.into()];
        if takes_options {
            args.push(Expr::var(&options));
            params.push(options.into());
        }
        args.extend(extra);
        let armed = Expr::call_helper("$writingInto", args);
        Ok(Some(js::Prop::Field(
            FMT_INTO.into(),
            Expr::arrow(params, vec![StmtKind::Return(Some(armed)).at(js::Span::NONE)]),
        )))
    }

    /// The writes of a std value of `shown`, `value`, by `kind`, given the
    /// `Formatter`'s `options` as it runs, `{:#?}` by them too: as
    /// `sink_writes_given`, or for `Debug` `debug_sink_writes`, makes them.
    /// `None` where its split isn't followed.
    fn std_writes_by_options(
        &mut self,
        kind: Std,
        shown: Ty<'tcx>,
        (value, options): (&js::Name, &js::Name),
        span: Span,
    ) -> R<Option<Vec<SinkWrite>>> {
        let given = Expr::var(options.clone());
        match kind {
            Std::FmtDebug => {
                let pretty = Pretty::When(given.clone()).alternate();
                self.debug_sink_writes(Expr::var(value.clone()), shown, (&given, &pretty), span)
            }
            _ => self.sink_writes_given(Expr::var(value.clone()), (kind, shown), given, span),
        }
    }

    /// A std formatting trait's `fmt` of `ty` taken as a `fn` pointer,
    /// `self.format(f, fmt::Display::fmt)`, where a writer is shown into a
    /// writer: the function that writes into the sink it claims, by the
    /// `Formatter`'s options as it runs, `(value, options) => { const f =
    /// $sink(); $writeInteger(f, value, options); return f.done(); }`, so a
    /// framed writer that calls it arms it (#1652); called plainly, its
    /// text. `None` where nothing is framed, or of the crate's own type,
    /// whose writer is framed itself, of a type only a dictionary knows, or
    /// where its split isn't followed.
    pub(crate) fn std_fmt_fn_value(
        &mut self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        span: Span,
    ) -> R<Option<Expr>> {
        if self.krate.prints.framed.is_empty() || !crate::recognition::is_fmt_method(self.tcx, def_id) {
            return Ok(None);
        }
        let Some(ty) = args.types().next() else {
            return Ok(None);
        };
        let trait_id = self.tcx.trait_of_assoc(def_id).expect("a formatting trait's `fmt`");
        let shown = self.recognition().refs_shown_by(trait_id, ty);
        // Not the crate's own type's, whose writer is framed itself, its
        // `&ByRef`'s too (#1798). What only a dictionary knows, itertools'
        // `Format`'s `I::Item`'s, is written by it, `$writeBy`: its text
        // where nothing's armed (#1712).
        if (self.recognition().formatted_by(trait_id, shown)).is_some_and(|adt| adt.is_local()) {
            return Ok(None);
        }
        let kind = match (self.radix_trait(trait_id), self.exp_trait(trait_id)) {
            (Some(radix), _) => Std::FmtRadix(radix),
            (_, Some(upper)) => Std::FmtExp(upper),
            _ if crate::recognition::is_std_def(self.tcx, trait_id, StdItem::Debug) => Std::FmtDebug,
            _ if crate::recognition::is_std_def(self.tcx, trait_id, StdItem::Display) => Std::FmtDisplay,
            _ => return Ok(None),
        };
        let (sink, value, options) = (self.fresh(Role::F), self.fresh(Role::Value), self.fresh(Role::Options));
        let Some(writes) = self.std_writes_by_options(kind, shown, (&value, &options), span)? else {
            return Ok(None);
        };
        let f = Expr::var(&sink);
        let mut body = vec![StmtKind::Const(sink, Expr::call_helper("$sink", Vec::new())).at(js::Span::NONE)];
        body.extend(writes.into_iter().map(|write| write.written_into(&f, js::Span::NONE)));
        body.push(StmtKind::Return(Some(Expr::call(Expr::member(f, "done"), Vec::new()))).at(js::Span::NONE));
        Ok(Some(Expr::arrow(vec![value.into(), options.into()], body)))
    }

    /// The `fmtInto` entry of a std dictionary of `kind` of `ty`, where a
    /// type that reaches a parameter written so is `ty`: its writes as
    /// `sink_writes_given`, or for `Debug` `debug_sink_writes`, makes them,
    /// given the `Formatter`'s options as it runs. Refused where its split
    /// isn't followed yet (#1652).
    pub(crate) fn std_fmt_into(&mut self, kind: Std, ty: Ty<'tcx>, span: Span) -> R<Option<js::Prop>> {
        let written = self.krate.written.of_type(self.tcx, ty);
        // Of one holding a type whose writer may print, its pieces where
        // they're followed, so a print lands between them (#1608, #1712).
        let printing = self.krate.prints.holds_printing(ty);
        if !written && !printing {
            return Ok(None);
        }
        let (sink, value, options) = (self.fresh(Role::F), self.fresh(Role::Value), self.fresh(Role::Options));
        let Some(writes) = self.std_writes_by_options(kind, ty.peel_refs(), (&value, &options), span)? else {
            if !written {
                return Ok(None);
            }
            // Of a value a library makes a `dyn` of alone: none, which its
            // contract says (#1797).
            if !printing && self.krate.written.best_effort(self.tcx, ty) {
                self.dependencies.borrow_mut().fmt_into_left = true;
                return Ok(None);
            }
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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The refusal of `what`, written by a writer that writes through a
    /// sink a `write!` into a writer may arm, where its split isn't
    /// followed yet (#1652).
    pub fn refused_in_a_writer(&self, what: &str, span: Span) -> crate::refusal::Refusal {
        self.refused_piecewise(span, &format!("{what} in a writer shown into a writer by `write!`"))
    }

    /// The refusal of `what`, written into a writer, whose split into
    /// pieces isn't followed yet: `what`, and why, std writes it a piece at
    /// a time, naming #1652.
    pub fn refused_piecewise(&self, span: Span, what: &str) -> crate::refusal::Refusal {
        crate::refusal::refuse(
            self.tcx,
            span,
            crate::refusal::Refused::Because(
                what,
                "std writes it a piece at a time, each a call of the writer's, which isn't followed for it (#1652)",
            ),
        )
    }
}
