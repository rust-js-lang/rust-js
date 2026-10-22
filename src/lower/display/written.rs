//! What a `Display` writes, and a `Debug` builder's parts assembled: the
//! string of what a writer's calls write.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a call that writes to a formatter, `operation` of `def_id`,
    /// writes, given its arguments but the formatter, `values`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower) fn written_of(
        &mut self,
        operation: WriteCall,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        mut values: Vec<Expr>,
        pretty: &Pretty,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let pretty = pretty.clone();
        Ok(match operation {
            // `write!(f, ..)` is `f.write_fmt(format_args!(..))`, which is a string (ADR 0034).
            WriteCall::Text => values.remove(0),
            // A string, padded and cut as the `Formatter`'s options say, as a
            // `str`'s `Display` is (ADR 0143).
            WriteCall::Pad => {
                let text = values.remove(0);
                let str_ty = self.tcx.types.str_;
                self.formatted(text.clone(), (Std::FmtDisplay, str_ty), &pretty)
                    .unwrap_or(text)
            }
            // A number's digits, `-` before them where it's negative and its
            // prefix where `{:#}` asks; given options, as std writes it,
            // `$padIntegral`, a `+`, zeros past the prefix, padding (ADR 0143).
            WriteCall::PadIntegral => {
                let (non_negative, prefix, digits) = (values.remove(0), values.remove(0), values.remove(0));
                match &pretty {
                    Pretty::When(options) if self.krate.format_options => Expr::call(
                        Expr::helper("$padIntegral"),
                        vec![non_negative, prefix, digits, options.clone()],
                    ),
                    _ => {
                        let sign = Expr::cond(non_negative, Expr::str(""), Expr::str("-"));
                        let prefix = Expr::cond(pretty.alternate(), prefix, Expr::str(""));
                        Expr::bin(Op::Add, Expr::bin(Op::Add, sign, prefix), digits)
                    }
                }
            }
            WriteCall::Display => {
                let ty = generic_args.type_at(0);
                self.display_string_with(values.remove(0), ty, span, &pretty)?
            }
            WriteCall::Debug => {
                let ty = generic_args.type_at(0);
                self.debug_string_with(values.remove(0), ty, span, &pretty)?
            }
            // The crate's impl, given this `Formatter`'s options, or std's of a
            // number, padded by them as it runs (ADR 0185).
            WriteCall::OtherFmt => {
                let ty = generic_args.type_at(0).peel_refs();
                let trait_id = self.tcx.trait_of_assoc(def_id).expect("a trait's `fmt`");
                let std_radix = (!self.has_user_impl(trait_id, ty))
                    .then(|| Num::of(ty).filter(|n| !n.float()).zip(self.radix_trait(trait_id)))
                    .flatten();
                let std_exp = (!self.has_user_impl(trait_id, ty))
                    .then(|| Num::of(ty).zip(self.exp_trait(trait_id)))
                    .flatten();
                // A generic `T`'s: its dictionary's (ADR 0174).
                if self.is_unknown(ty) {
                    let value = values.remove(0);
                    self.other_fmt_dictionary(trait_id, ty, value, &pretty, span)?
                } else if let Some((num, radix)) = std_radix {
                    self.radix_text(values.remove(0), num, radix, &pretty)
                } else if let Some((num, upper)) = std_exp {
                    // A number's `{:e}`, of the precision it's given, padded by
                    // the options it's given.
                    match &pretty {
                        Pretty::When(options) if self.krate.format_options => {
                            let precision = Expr::optional_member(options.clone(), "precision");
                            let text =
                                crate::lower::format_spec::exp_text(values.remove(0), num, upper, Some(precision));
                            Expr::call(
                                Expr::helper("$formatted"),
                                vec![text, options.clone(), Expr::bool(true)],
                            )
                        }
                        _ => crate::lower::format_spec::exp_text(values.remove(0), num, upper, None),
                    }
                } else {
                    if !self.has_user_impl(trait_id, ty) {
                        let path = self.tcx.def_path_str(def_id);
                        return Err(self.unsupported(span, &format!("calling `{path}` of a `{ty}`")));
                    }
                    let args = self.tcx.mk_args(&[self.tcx.erase_and_anonymize_regions(ty).into()]);
                    self.writer_call(def_id, args, values.remove(0), &pretty, span)?
                }
            }
            // More than five fields: arrays of their names and strings.
            WriteCall::StructFields => {
                values.extend(self.writing.given.is_some().then(|| pretty.alternate()));
                Expr::call(Expr::helper("$debugFields"), values)
            }
            WriteCall::TupleFields => {
                let (type_name, items) = (values.remove(0), values.remove(0));
                let items = self.once(items, out);
                let joined = Expr::call(Expr::member(items.clone(), "join"), vec![Expr::str(", ")]);
                let plain = join(vec![type_name.clone(), Expr::str("("), joined, Expr::str(")")]);
                let open = join(vec![type_name, Expr::str("(")]);
                self.pretty_or_parts(&pretty, plain, open, items, Expr::str(")"))
            }
            // A derived `Debug`'s body (ADR 0060): its fields are strings
            // already, each a `&dyn Debug` (`debug_dyn`).
            WriteCall::Struct => {
                let type_name = values.remove(0);
                let mut parts = vec![type_name.clone(), Expr::str(" { ")];
                let mut shown = Vec::new();
                let mut first = true;
                while values.len() >= 2 {
                    let (field, value) = (values.remove(0), values.remove(0));
                    let value = self.once(value, out);
                    if !first {
                        parts.push(Expr::str(", "));
                    }
                    first = false;
                    parts.extend([field.clone(), Expr::str(": "), value.clone()]);
                    shown.push(join(vec![field, Expr::str(": "), value]));
                }
                parts.push(Expr::str(" }"));
                let open = join(vec![type_name, Expr::str(" {")]);
                self.pretty_or_parts(&pretty, join(parts), open, Expr::array(shown), Expr::str("}"))
            }
            WriteCall::Tuple => {
                let type_name = values.remove(0);
                let mut parts = vec![type_name.clone(), Expr::str("(")];
                let mut shown = Vec::new();
                for (i, value) in values.drain(..).enumerate() {
                    let value = self.once(value, out);
                    if i > 0 {
                        parts.push(Expr::str(", "));
                    }
                    parts.push(value.clone());
                    shown.push(value);
                }
                parts.push(Expr::str(")"));
                let open = join(vec![type_name, Expr::str("(")]);
                self.pretty_or_parts(&pretty, join(parts), open, Expr::array(shown), Expr::str(")"))
            }
            // Another writer, given this one's `Formatter`: its options.
            WriteCall::Function => {
                values.extend(self.given_to_writer(Some(def_id)));
                values.extend(self.evidence_args(def_id, generic_args, span)?);
                Expr::call(self.fn_ref(def_id), values)
            }
            WriteCall::Trait => {
                values.extend(self.given_to_writer(Some(def_id)));
                match self.trait_call(def_id, generic_args, values, span, out)? {
                    Some(call) => call,
                    None => return Err(self.unsupported(span, "this call")),
                }
            }
            _ => {
                return Err(crate::lower::refusal::refuse(
                    self.tcx,
                    span,
                    crate::lower::refusal::Refused::Call(def_id),
                ));
            }
        })
    }

    /// What a `Debug` builder of `kind`, a `DebugStruct` say, writes as
    /// its `finish` (ADR 0136): its type's name, then each step, `field`,
    /// `entry` or `entries`, of its arguments but the builder, and their
    /// types, as pretty as this writer is (ADR 0137).
    #[allow(clippy::type_complexity)]
    pub(in crate::lower) fn builder_written(
        &mut self,
        kind: DebugBuilder,
        type_name: Option<Expr>,
        steps: Vec<(BuilderStep, Vec<(Expr, Ty<'tcx>)>, ty::GenericArgsRef<'tcx>)>,
        non_exhaustive: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut parts = Vec::new();
        for (step, values, generic_args) in steps {
            parts.push(self.builder_part(kind, (step, values, generic_args), span, out)?);
        }
        Ok(self.builder_assembled(kind, type_name, parts, non_exhaustive))
    }

    /// What one step of a `Debug` builder of `kind` adds, as it's taken:
    /// a field's or an entry's text, or of `entries`, an array of them.
    #[allow(clippy::type_complexity)]
    pub(in crate::lower) fn builder_part(
        &mut self,
        kind: DebugBuilder,
        (step, values, generic_args): (BuilderStep, Vec<(Expr, Ty<'tcx>)>, ty::GenericArgsRef<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<BuilderPart> {
        let pretty = self.writer_pretty();
        // A `&dyn Debug` is the string it shows already (ADR 0060).
        let debug_of = |this: &mut Self, (value, ty): (Expr, Ty<'tcx>)| -> R<Expr> {
            match this.is_dyn_debug(ty) {
                true => Ok(value),
                false => this.debug_string(value, ty, span),
            }
        };
        let mut values = values.into_iter();
        Ok(match (kind, step) {
            (DebugBuilder::Struct, BuilderStep::Field) => {
                let (field, value) = (values.next().expect("a name"), values.next().expect("a value"));
                let shown = debug_of(self, value)?;
                BuilderPart::One(join(vec![field.0, Expr::str(": "), shown]))
            }
            (DebugBuilder::Tuple, BuilderStep::Field)
            | (DebugBuilder::List | DebugBuilder::Set, BuilderStep::Entry) => {
                let value = values.next().expect("a value");
                BuilderPart::One(debug_of(self, value)?)
            }
            (DebugBuilder::Map, BuilderStep::Entry) => {
                let (key, value) = (values.next().expect("a key"), values.next().expect("a value"));
                let key = debug_of(self, key)?;
                let value = debug_of(self, value)?;
                BuilderPart::One(join(vec![key, Expr::str(": "), value]))
            }
            (DebugBuilder::List | DebugBuilder::Set | DebugBuilder::Map, BuilderStep::Entries) => {
                let items = values.next().expect("the entries").0;
                let tys: Vec<Ty<'tcx>> = generic_args.types().collect();
                let iterable = *tys.last().expect("the entries' type");
                let items = self.iterable_items(items, iterable, span, out)?;
                let shown = if kind == DebugBuilder::Map {
                    let pair = Expr::var("entry");
                    let key = self.debug_string_with(Expr::index(pair.clone(), Expr::int(0)), tys[0], span, &pretty)?;
                    let value = self.debug_string_with(Expr::index(pair, Expr::int(1)), tys[1], span, &pretty)?;
                    join(vec![key, Expr::str(": "), value])
                } else {
                    self.debug_string_with(Expr::var("entry"), tys[0], span, &pretty)?
                };
                let each = Expr::arrow(
                    vec!["entry".into()],
                    vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                );
                BuilderPart::Many(Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items, each]))
            }
            _ => return Err(self.unsupported(span, "this step of a `Debug` builder")),
        })
    }

    /// What a `Debug` builder of `kind` writes of its `parts`, as its
    /// `finish`: its type's name, then the parts, as pretty as this writer
    /// is (ADR 0137). Of one held in a variable, its parts are an array, as
    /// many as its steps took, none perhaps (ADR 0532).
    pub(in crate::lower) fn builder_assembled(
        &mut self,
        kind: DebugBuilder,
        type_name: Option<Expr>,
        parts: Vec<BuilderPart>,
        non_exhaustive: bool,
    ) -> Expr {
        let pretty = self.writer_pretty();
        use BuilderPart as Part;
        // The parts, comma separated: in place when each is one string, else
        // an array of them joined, as `entries` may give none.
        let separated = |parts: Vec<Part>, extra: Option<&str>| -> Expr {
            let mut parts = parts;
            if let Some(extra) = extra {
                parts.push(Part::One(Expr::str(extra)));
            }
            if parts.iter().all(|p| matches!(p, Part::One(_))) {
                let mut pieces = Vec::new();
                for (i, part) in parts.into_iter().enumerate() {
                    if i > 0 {
                        pieces.push(Expr::str(", "));
                    }
                    if let Part::One(e) = part {
                        pieces.push(e);
                    }
                }
                return join(pieces);
            }
            let mut arrays: Vec<Expr> = Vec::new();
            let mut ones: Vec<Expr> = Vec::new();
            for part in parts {
                match part {
                    Part::One(e) => ones.push(e),
                    Part::Many(e) => {
                        if !ones.is_empty() {
                            arrays.push(Expr::array(std::mem::take(&mut ones)));
                        }
                        arrays.push(e);
                    }
                }
            }
            if !ones.is_empty() {
                arrays.push(Expr::array(ones));
            }
            let mut arrays = arrays.into_iter();
            let first = arrays.next().expect("a part");
            let all = arrays.fold(first, |all, next| Expr::call(Expr::member(all, "concat"), vec![next]));
            Expr::call(Expr::member(all, "join"), vec![Expr::str(", ")])
        };
        // Pretty: each part on a line of its own, an array of them, as `entries`
        // may give none (ADR 0137).
        let array = {
            let mut arrays: Vec<Expr> = Vec::new();
            let mut ones: Vec<Expr> = Vec::new();
            for part in parts.clone() {
                match part {
                    Part::One(e) => ones.push(e),
                    Part::Many(e) => {
                        if !ones.is_empty() {
                            arrays.push(Expr::array(std::mem::take(&mut ones)));
                        }
                        arrays.push(e);
                    }
                }
            }
            if !ones.is_empty() || arrays.is_empty() {
                arrays.push(Expr::array(ones));
            }
            let mut arrays = arrays.into_iter();
            let first = arrays.next().expect("a part");
            arrays.fold(first, |all, next| Expr::call(Expr::member(all, "concat"), vec![next]))
        };
        let pretty_open = match kind {
            DebugBuilder::Struct => type_name.clone().map(|name| join(vec![name, Expr::str(" {")])),
            DebugBuilder::Tuple => type_name.clone().map(|name| join(vec![name, Expr::str("(")])),
            DebugBuilder::List => Some(Expr::str("[")),
            _ => Some(Expr::str("{")),
        };
        let pretty_close = match kind {
            DebugBuilder::Tuple => ")",
            DebugBuilder::List => "]",
            _ => "}",
        };
        let has_parts = !parts.is_empty();
        let rest = non_exhaustive.then_some("..");
        let empty = parts.is_empty() && !non_exhaustive;
        let written = match kind {
            DebugBuilder::Struct | DebugBuilder::Tuple if empty => type_name.expect("a type's name"),
            DebugBuilder::Struct => {
                let inside = separated(parts, rest);
                join(vec![
                    type_name.expect("a type's name"),
                    Expr::str(" { "),
                    inside,
                    Expr::str(" }"),
                ])
            }
            DebugBuilder::Tuple => {
                let inside = separated(parts, rest);
                join(vec![
                    type_name.expect("a type's name"),
                    Expr::str("("),
                    inside,
                    Expr::str(")"),
                ])
            }
            _ => {
                let (open, close) = if kind == DebugBuilder::List {
                    ("[", "]")
                } else {
                    ("{", "}")
                };
                if empty {
                    Expr::str(format!("{open}{close}"))
                } else {
                    let inside = separated(parts, rest);
                    join(vec![Expr::str(open), inside, Expr::str(close)])
                }
            }
        };
        match (&pretty, has_parts, pretty_open) {
            (Pretty::Plain, _, _) | (_, false, _) | (_, _, None) => written,
            (_, true, Some(open)) => {
                let mut list = vec![open, array, Expr::str(pretty_close)];
                if non_exhaustive {
                    list.push(Expr::bool(true));
                }
                let pretty_form = Expr::call(Expr::helper("$pretty"), list);
                pretty.choose(pretty_form, written)
            }
        }
    }

    /// `{}` of a `ty` value: the string itself, `String(x)`, `$displayF64(x)`,
    /// a hand-written `fmt`'s string, or `TDisplay.fmt(x)` in generic code.
    pub(in crate::lower) fn display_string(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        self.display_string_with(value, ty, span, &Pretty::Plain)
    }

    /// `display_string`, of a `{:#}` or not, which only the crate's own
    /// `fmt` can tell (ADR 0137).
    pub(in crate::lower) fn display_string_with(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        span: Span,
        pretty: &Pretty,
    ) -> R<Expr> {
        let (value, ty) = self.shown_through(value, ty);
        let ty = self.shown_type(ty);
        if let Some(shown) = self.formatted(value.clone(), (Std::FmtDisplay, ty), pretty) {
            return Ok(shown);
        }
        self.can_apply(ty, pretty, span)?;
        // A UTF-8 error's message, a `FromUtf8Error`'s its `Utf8Error`'s (ADR 0172).
        if let Some(owned) = self.recognition().utf8_error(ty) {
            let error = if owned { Expr::member(value, "error") } else { value };
            return Ok(Expr::call(Expr::helper("$utf8ErrorMessage"), vec![error]));
        }
        if let Some((borrowed, _)) = self.recognition().cow_parts(ty)
            && borrowed.is_str()
        {
            return Ok(Expr::member(value, "_0"));
        }
        // A path's `display()`, its text (ADR 0173).
        if self.recognition().is_path_like(ty) {
            return Ok(value);
        }
        if crate::lower::recognition::is_try_from_slice_error(self.tcx, ty) {
            return Ok(Expr::str("could not convert slice to array"));
        }
        // std's clock's (ADR 0447): a `SystemTimeError`'s message; a time's
        // has none.
        if let Some(clock) = crate::lower::recognition::clock_type_of(ty) {
            return match clock {
                crate::lower::recognition::ClockType::Error => {
                    Ok(Expr::str("second time provided was later than self"))
                }
                _ => Err(self.unsupported(span, &format!("the `Display` of a `{ty}`, which std has none of"))),
            };
        }
        if crate::lower::recognition::is_from_utf16_error(self.tcx, ty) {
            return Ok(Expr::str("invalid utf-16: lone surrogate found"));
        }
        // A `char`'s case mapping, the `char`s it hasn't given: their text,
        // all of an array's, the rest of one stepped (ADR 0070).
        if crate::lower::recognition::is_case_mapping(self.tcx, ty) {
            return Ok(match crate::lower::iterators::made_array(&value) {
                true => Expr::call(Expr::member(value, "join"), vec![Expr::str("")]),
                false => Expr::call(Expr::helper("$restStr"), vec![value]),
            });
        }
        // An escape is its text, and `decode_utf16`'s error its code (ADR 0327).
        if crate::lower::recognition::is_text_escape(self.tcx, ty) {
            return Ok(value);
        }
        // A byte's escape, the bytes it has left (ADR 0558).
        if crate::lower::recognition::is_byte_escape(self.tcx, ty) {
            return Ok(Expr::call(Expr::helper("$restBytes"), vec![value]));
        }
        if crate::lower::recognition::is_decode_utf16_error(self.tcx, ty) {
            let hex = Expr::call(Expr::member(value, "toString"), vec![Expr::int(16)]);
            return Ok(join(vec![Expr::str("unpaired surrogate found: "), hex]));
        }
        // `format_args!`'s text (ADR 0034), which its `Display` writes as it
        // is, whatever width the `Formatter` has, as strum's derive asks.
        if self.is_lang_adt(ty, LangItem::FormatArguments) {
            return Ok(value);
        }
        // A `Wrapping` shows its number, as both its `Display` and its
        // `Debug` do (ADR 0175).
        if let Some(inner) = self.recognition().wrapping_of(ty) {
            return self.display_string_with(Expr::index(value, Expr::int(0)), inner, span, pretty);
        }
        if self.is_string_like(ty) || self.is_parse_error(ty) {
            return Ok(value);
        }
        // A path's `StripPrefixError`, as std shows it.
        if self.recognition().is_strip_prefix_error(ty) {
            return Ok(Expr::str("prefix not found"));
        }
        // An environment's `VarError`, as std shows it.
        if self.recognition().is_var_error(ty) {
            let shown = (
                "environment variable not found",
                "environment variable was not valid unicode: ",
                "",
            );
            return Ok(var_error_shown(value, shown));
        }
        // A lock's `TryLockError`, as std shows it.
        if self.recognition().is_try_lock_error(ty) {
            let blocks = Expr::bin(Op::Eq, value, Expr::str("WouldBlock"));
            return Ok(Expr::cond(
                blocks,
                Expr::str("try_lock failed because the operation would block"),
                Expr::str("poisoned lock: another task failed inside"),
            ));
        }
        if self.is_json_error(ty) {
            return Ok(Expr::call(Expr::helper("$displayJsonError"), vec![value]));
        }
        // An `io::Error`'s, as std shows it, and an `ErrorKind`'s words (ADR 0462).
        if self.recognition().is_io_error(ty) {
            return Ok(Expr::call_helper("$ioErrorDisplay", vec![value]));
        }
        if self.recognition().is_io_error_kind(ty) {
            return Ok(Expr::call_helper("$ioKindText", vec![value]));
        }
        // `file:line:column` (ADR 0427).
        if self.recognition().is_location(ty) {
            return self.location_shown(value, false, span);
        }
        // `disabled backtrace`, or its frames (ADR 0568).
        if self.recognition().is_backtrace(ty) {
            return Ok(self.backtrace_shown(value, false));
        }
        // A `Value`'s JSON, or a `Number`'s (ADR 0083).
        if let Some(shown) = self.json_value_display(value.clone(), ty, false) {
            return Ok(shown);
        }
        if ty.is_bool() || Num::of(ty).is_some_and(|n| !n.float()) {
            return Ok(shown_number(value));
        }
        if Num::of(ty) == Some(Num::F64) {
            return Ok(Expr::call(Expr::helper("$displayF64"), vec![value]));
        }
        if Num::of(ty) == Some(Num::F32) {
            return Ok(Expr::call(Expr::helper("$displayF32"), vec![value]));
        }
        let display = self.display_trait();
        // A `dyn ToString`'s text, what `to_string()` of it gives, is its
        // dictionary's `to_string` of its value (ADR 0141).
        if self.dynamic_trait(ty) == self.to_string_trait() && self.to_string_trait().is_some() {
            if !value.parts_read_same() {
                return Err(self.unsupported(span, &format!("showing a `{ty}` made where it's shown")));
            }
            let dictionary = Expr::member(value.clone(), js::DICTIONARY);
            return Ok(Expr::call(
                Expr::member(dictionary, "to_string"),
                vec![Expr::held(value)],
            ));
        }
        let options = self.options_arg(pretty);
        if let Some(shown) = self.dyn_written(value.clone(), (ty, display), options, span)? {
            return Ok(shown);
        }
        if self.is_unknown(ty) {
            let tr = ty::TraitRef::new(self.tcx, display, [ty]);
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            let mut list = vec![value];
            list.extend(self.options_arg(pretty));
            return Ok(Expr::call(Expr::member(dictionary, "fmt"), list));
        }
        if self.has_user_impl(display, ty) {
            let fmt = self.tcx.associated_item_def_ids(display)[0];
            let args = self.tcx.mk_args(&[self.tcx.erase_and_anonymize_regions(ty).into()]);
            return self.writer_call(fmt, args, value, pretty, span);
        }
        Err(self.unsupported(span, &format!("`{{}}` of a `{ty}`")))
    }
}
