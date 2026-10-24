//! `{:?}` of a value: a type's derived `Debug`, a variant's and a tuple's,
//! and the items of a collection.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `{:?}` of a `ty` value (ADR 0060), as Rust shows it: `Some(1)`,
    /// `(1, "a")`, `[1.0, 2.5]`, a call of a `Debug` impl of the crate's own,
    /// derived or not, or `TDebug.fmt(x)` in generic code.
    pub(in crate::lower) fn debug_string(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        self.debug_string_with(value, ty, span, &Pretty::Plain)
    }

    /// `debug_string`, plain or pretty, `{:#?}` (ADR 0137): a part of a
    /// value is shown as the value is.
    pub(in crate::lower) fn debug_string_with(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        span: Span,
        pretty: &Pretty,
    ) -> R<Expr> {
        let (value, ty) = self.shown_through(value, ty);
        // A raw pointer shows its address, which JS hasn't (ADR 0407), and
        // so does a function pointer.
        if self.is_handle_pointer(ty) || matches!(ty.kind(), ty::FnPtr(..)) {
            return self.pointer_shown(value, ty, span);
        }
        // A `dyn Any` shows nothing of its value, as std's `Debug` doesn't;
        // a `TypeId` its hash, which rust-js doesn't keep: its error where
        // it's shown, as a pointer's address is (ADR 0331).
        if self.recognition().is_dyn_any(ty) {
            if value.has_effects() {
                return Ok(Expr::call(Expr::helper("$debugAny"), vec![value]));
            }
            return Ok(Expr::str("Any { .. }"));
        }
        if self.recognition().is_type_id(ty) {
            return Ok(Expr::call(Expr::helper("$typeIdShown"), vec![value]));
        }
        // `Location { file, line, column }`, as std's shows it (ADR 0427).
        if self.recognition().is_location(ty) {
            return self.location_shown(value, true, span);
        }
        // `<disabled>`, or its frames (ADR 0568).
        if self.recognition().is_backtrace(ty) {
            return Ok(self.backtrace_shown(value, true));
        }
        // A `Discriminant` shows its number, as std's does (ADR 0393).
        if self.recognition().is_discriminant(ty) {
            return Ok(join(vec![Expr::str("Discriminant("), value, Expr::str(")")]));
        }
        // A `Weak` shows none of what it points at, `(Weak)`, as std's does.
        if crate::lower::recognition::weak_pointee(self.tcx, ty).is_some() {
            return Ok(Expr::str("(Weak)"));
        }
        // A heap's `PeekMut` shows its top, `PeekMut(8)` (ADR 0333).
        if let Some(item) = self.recognition().peek_mut_of(ty) {
            let top = Expr::index(Expr::member(value, "heap"), Expr::int(0));
            let shown = self.debug_string_with(top, item, span, pretty)?;
            return Ok(join(vec![Expr::str("PeekMut("), shown, Expr::str(")")]));
        }
        // A `MaybeUninit` shows its type's name, `MaybeUninit<u32>` (ADR 0332).
        if self.recognition().uninit_of(ty).is_some() {
            if ty.has_param() {
                return Err(self.unsupported(span, "`{:?}` of a generic `MaybeUninit`"));
            }
            let name = rustc_const_eval::util::type_name(self.tcx, ty);
            return Ok(Expr::str(&name[name.find("MaybeUninit").unwrap_or(0)..]));
        }
        // A write's result: `undefined`, `Ok`, or the error it caught, a
        // `fmt::Error`, which holds nothing (ADR 0187), or an `io::Error`
        // (ADR 0462).
        if self.is_write_result(ty) {
            let ok = Expr::str("Ok(())");
            if matches!(value.kind, js::ExprKind::Undefined) {
                return Ok(ok);
            }
            if !self.is_fmt_result(ty) {
                let mut list = vec![value];
                if pretty.known() != Some(false) {
                    list.push(pretty.options());
                }
                return Ok(Expr::call_helper("$ioResultDebug", list));
            }
            return Ok(Expr::cond(
                Expr::bin(Op::Eq, value, Expr::undefined()),
                ok,
                Expr::str("Err(Error)"),
            ));
        }
        if self.recognition().is_fmt_error(ty) {
            return Ok(Expr::str("Error"));
        }
        if crate::lower::recognition::is_try_from_slice_error(self.tcx, ty) {
            return Ok(Expr::str("TryFromSliceError(())"));
        }
        if crate::lower::recognition::is_from_utf16_error(self.tcx, ty) {
            return Ok(Expr::str("FromUtf16Error { kind: LoneSurrogate }"));
        }
        if crate::lower::recognition::is_decode_utf16_error(self.tcx, ty) {
            return Ok(join(vec![
                Expr::str("DecodeUtf16Error { code: "),
                Expr::call(Expr::var("String"), vec![value]),
                Expr::str(" }"),
            ]));
        }
        // A `SystemTimeError`'s, its `Duration`'s; a time's is its platform's
        // fields, which JS has none of (ADR 0447).
        if let Some(clock) = crate::lower::recognition::clock_type_of(ty) {
            if clock != crate::lower::recognition::ClockType::Error {
                return Err(self.unsupported(span, &format!("`{{:?}}` of a `{ty}`, its platform's fields")));
            }
            if self.writing.options.is_some() || matches!(pretty, Pretty::Given(..)) {
                return Err(self.unsupported(span, "options for a `SystemTimeError`'s `{:?}`"));
            }
            let duration = Expr::call(Expr::helper("$debugDuration"), vec![value]);
            return Ok(join(vec![Expr::str("SystemTimeError("), duration, Expr::str(")")]));
        }
        // A `Duration`'s, in its largest whole unit, `1.5s` (ADR 0188). With
        // options, std rounds and pads it by its own rules: not yet.
        if crate::lower::recognition::is_duration_ty(ty) {
            if self.writing.options.is_some() || matches!(pretty, Pretty::Given(..)) {
                return Err(self.unsupported(span, "options for a `Duration`'s `{:?}`"));
            }
            return Ok(Expr::call(Expr::helper("$debugDuration"), vec![value]));
        }
        // A `Layout`'s, `Layout { size: 8, align: 4 (1 << 2) }`, and a
        // `LayoutError`'s, its name (ADR 0417).
        if let Some(layout) = self.recognition().layout(ty) {
            if self.writing.options.is_some() || matches!(pretty, Pretty::Given(..)) {
                return Err(self.unsupported(span, "options for a `Layout`'s `{:?}`"));
            }
            return Ok(match layout {
                true => Expr::call(Expr::helper("$debugLayout"), vec![value]),
                false => Expr::str("LayoutError"),
            });
        }
        // A part of a `{:?}` given its options (ADR 0058): a leaf applies them.
        if self.writing.options.is_some() && self.is_debug_leaf(ty) {
            let options = self.writing.options.take().expect("checked");
            let shown = self.format_value(
                value,
                (Std::FmtDebug, ty),
                options.spec,
                (options.width.clone(), options.precision.clone()),
                span,
            );
            self.writing.options = Some(options);
            return shown;
        }
        if let Some(shown) = self.formatted(value.clone(), (Std::FmtDebug, ty), pretty) {
            return Ok(shown);
        }
        self.can_apply(ty, pretty, span)?;
        // serde_json's own, `Object {"a": Number(1)}` (ADR 0083).
        if let Some(shown) = self.json_value_debug(value.clone(), ty, pretty) {
            return Ok(shown);
        }
        let std = |item: StdItem| self.is_std_type(ty, item);
        let num = Num::of(ty);
        if self.is_dyn_debug(ty) {
            return Ok(value);
        }
        // An enum with no variants, `Infallible`: no value of it exists, so
        // this never runs, as Rust's `match *self {}` doesn't (ADR 0161).
        if let ty::Adt(adt, _) = ty.kind()
            && adt.is_enum()
            && adt.variants().is_empty()
        {
            return Ok(Expr::str(""));
        }
        // A path's `StripPrefixError`, which holds nothing.
        if self.recognition().is_strip_prefix_error(ty) {
            return Ok(Expr::str("StripPrefixError(())"));
        }
        // An environment's `VarError`: its variant.
        if self.recognition().is_var_error(ty) {
            return Ok(var_error_shown(value, ("NotPresent", "NotUnicode(", ")")));
        }
        // A lock's `TryLockError`, as std shows it: its variant's name, quoted.
        if self.recognition().is_try_lock_error(ty) {
            let blocks = Expr::bin(Op::Eq, value, Expr::str("WouldBlock"));
            return Ok(Expr::cond(
                blocks,
                Expr::str("\"WouldBlock\""),
                Expr::str("\"Poisoned(..)\""),
            ));
        }
        // A channel's errors (ADR 0142), as std shows them: `TryRecvError` is
        // its variant's name already.
        if let Some(error) = self.recognition().channel_error(ty) {
            return Ok(match error {
                ChannelError::Recv => Expr::str("RecvError"),
                ChannelError::TryRecv => value,
                ChannelError::Send => Expr::str("SendError { .. }"),
            });
        }
        let (shown, options) = (self.shown_type(ty), self.options_arg(pretty));
        if let Some(shown) = self.dyn_written(value.clone(), (shown, self.debug_trait()), options, span)? {
            return Ok(shown);
        }
        if num == Some(Num::F64) {
            return Ok(Expr::call(Expr::helper("$debugF64"), vec![value]));
        }
        if num == Some(Num::F32) {
            return Ok(Expr::call(Expr::helper("$debugF32"), vec![value]));
        }
        if num.is_some() || ty.is_bool() {
            return Ok(shown_number(value));
        }
        if ty.is_unit() {
            return Ok(Expr::str("()"));
        }
        if ty.is_char() {
            return Ok(Expr::call(Expr::helper("$debugStr"), vec![value, Expr::str("'")]));
        }
        if self.is_string_like(ty) {
            return Ok(Expr::call(Expr::helper("$debugStr"), vec![value]));
        }
        // What a JS function threw, as JS shows it: `SyntaxError: ..`.
        if self.recognition().is_js_error(ty) {
            return Ok(Expr::call(Expr::var("String"), vec![value]));
        }
        if self.is_json_error(ty) {
            return Ok(Expr::call(Expr::helper("$debugJsonError"), vec![value]));
        }
        // An `io::Error`'s, std's `Custom { kind, error }` or `Kind(..)`, as
        // pretty as it's asked (ADR 0462).
        if self.recognition().is_io_error(ty) {
            let mut list = vec![value];
            if pretty.known() != Some(false) {
                list.push(pretty.options());
            }
            return Ok(Expr::call_helper("$ioErrorDebug", list));
        }
        // Their derived `Debug`s (ADR 0172).
        // Pretty as it's asked, `{:#?}`'s, and its numbers padded as the
        // options say.
        if let Some(owned) = self.recognition().utf8_error(ty) {
            let mut list = vec![value];
            let may_be_pretty = !matches!(pretty, Pretty::Plain);
            if owned || may_be_pretty {
                list.push(Expr::bool(owned));
            }
            if may_be_pretty {
                list.push(pretty.options());
            }
            return Ok(Expr::call(Expr::helper("$debugUtf8Error"), list));
        }
        if let Some(inner) = self.recognition().wrapping_of(ty) {
            return self.debug_string_with(Expr::index(value, Expr::int(0)), inner, span, pretty);
        }
        // A slice's iterator, `Iter([2, 3])`: what's left of it, as its
        // `as_slice()` gives it (ADR 0453).
        if self.is_std_type(ty, StdItem::SliceIter) {
            if pretty.known() != Some(false) {
                return Err(self.unsupported(span, "`{:#?}` of a slice's iterator"));
            }
            let item = self.iterator_item(ty).expect("an iterator's item").peel_refs();
            // Read twice, `it.items.slice(it.at, it.end)`: in place where it
            // reads the same, or is `$iter(v)`, whose rest is `v`; else as an
            // arrow's parameter, which tidy makes a `const`.
            let in_place = value.reads_same() || crate::lower::iterators::fresh_iter_source(&value).is_some();
            let it = match in_place {
                true => value.clone(),
                false => Expr::var("it"),
            };
            let left = self.stepped_left(it, ty, false, span)?;
            let shown = self.debug_string_with(left, Ty::new_slice(self.tcx, item), span, pretty)?;
            let shown = join(vec![Expr::str("Iter("), shown, Expr::str(")")]);
            return Ok(match in_place {
                true => shown,
                false => Expr::call(
                    Expr::arrow(
                        vec!["it".into()],
                        vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                    ),
                    vec![value],
                ),
            });
        }
        // A string's `chars()`, `Chars(['b', 'c'])`: the `char`s it has
        // left, as a slice's iterator shows what it has (#118).
        if self.recognition().is_str_chars(ty) {
            if pretty.known() != Some(false) {
                return Err(self.unsupported(span, "`{:#?}` of a string's `chars()`"));
            }
            let left = Expr::call(
                Expr::member(Expr::var("Array"), "from"),
                vec![Expr::call_helper("$restStr", vec![value])],
            );
            let shown = self.debug_string_with(left, Ty::new_slice(self.tcx, self.tcx.types.char), span, pretty)?;
            return Ok(join(vec![Expr::str("Chars("), shown, Expr::str(")")]));
        }
        // A string's `drain(range)`, `Drain("bc")`: what it has left, as
        // a `&str` (#118).
        if self.recognition().is_string_drain(ty) {
            if pretty.known() != Some(false) {
                return Err(self.unsupported(span, "`{:#?}` of a string's `drain(range)`"));
            }
            let left = Expr::call_helper("$debugStr", vec![Expr::call_helper("$restStr", vec![value])]);
            return Ok(join(vec![Expr::str("Drain("), left, Expr::str(")")]));
        }
        // A path shows its text as a string does (ADR 0173).
        if self.recognition().is_path_like(ty) {
            return Ok(Expr::call(Expr::helper("$debugStr"), vec![value]));
        }
        // A `Cow` shows what it borrows or owns (ADR 0319).
        if let Some((borrowed, _)) = self.recognition().cow_parts(ty) {
            return self.debug_string_with(Expr::member(value, js::positional(0)), borrowed, span, pretty);
        }
        // A parse error is its message (ADR 0063), which says its kind.
        if self.is_parse_error(ty) {
            let ty::Adt(adt, _) = ty.kind() else {
                unreachable!("a struct")
            };
            let name = self.tcx.item_name(adt.did());
            let mut list = vec![value, Expr::str(name.as_str())];
            if pretty.known() != Some(false) {
                list.push(pretty.options());
            }
            return Ok(Expr::call(Expr::helper("$debugParseError"), list));
        }
        if self.is_lang_adt(ty, LangItem::OrderingEnum) {
            let names = ["Less", "Equal", "Greater"];
            if let Some(n) = value.as_int().filter(|n| (-1..=1).contains(n)) {
                return Ok(Expr::str(names[(n + 1) as usize]));
            }
            let names = Expr::array(names.into_iter().map(Expr::str).collect());
            return Ok(Expr::index(names, Expr::bin(Op::Add, value, Expr::int(1))));
        }
        let debug = self.debug_trait();
        // One a bound gives the `Debug` of, `Peekable<Fuse<I>>: Debug`, as
        // rustc resolves it by the bound: its dictionary's (ADR 0543).
        let given = ty.has_param() && self.evidence_for(ty::TraitRef::new(self.tcx, debug, [ty])).is_some();
        if self.is_unknown(ty) || given {
            let tr = ty::TraitRef::new(self.tcx, debug, [ty]);
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            let mut list = vec![value];
            list.extend(self.options_arg(pretty));
            return Ok(Expr::call(Expr::member(dictionary, "fmt"), list));
        }
        // A fieldless enum is its variant's name (ADR 0013), which is what a
        // derived `Debug` shows.
        if let ty::Adt(adt, _) = ty.kind()
            && representation::is_fieldless_enum(*adt)
            && self.is_derived_impl(debug, ty)
        {
            return Ok(self.variant_debug(*adt, value));
        }
        // The crate's own, hand-written or derived.
        if self.has_user_impl(debug, ty) {
            let fmt = self.tcx.associated_item_def_ids(debug)[0];
            let args = self.args_of(debug, ty);
            return self.writer_call(fmt, args, value, pretty, span);
        }
        if self.range_kind(ty).is_some() {
            return self.range_debug(value, ty, span, pretty);
        }
        // `PhantomData<u8>`, its type's name as `type_name` gives it (ADR
        // 0132); of a type parameter, the name its caller gives (ADR 0452).
        if let ty::Adt(_, args) = ty.kind()
            && self.is_lang_adt(ty, LangItem::PhantomData)
        {
            let name = self.type_fact_value(args.type_at(0), crate::lower::recognition::TypeFact::Name, span)?;
            return Ok(join(vec![Expr::str("PhantomData<"), name, Expr::str(">")]));
        }
        match ty.kind() {
            // A constant is known: `Some(1.0)` is `"Some(1.0)"`, with no test.
            _ if let Some(inner) = self.option_of(ty)
                && value.is_constant()
                && !self.boxed_payload(inner) =>
            {
                if matches!(value.kind, js::ExprKind::Undefined | js::ExprKind::Null) {
                    return Ok(Expr::str("None"));
                }
                let shown = self.debug_string_with(value, inner, span, pretty)?;
                let plain = join(vec![Expr::str("Some("), shown.clone(), Expr::str(")")]);
                Ok(self.pretty_or(pretty, plain, "Some(", Expr::array(vec![shown]), ")"))
            }
            _ if let Some(inner) = self.option_of(ty) => {
                let inside = if self.boxed_payload(inner) {
                    self.some_value(Expr::var("value"))
                } else {
                    Expr::var("value")
                };
                let shown = self.debug_string_with(inside, inner, span, pretty)?;
                let plain = join(vec![Expr::str("Some("), shown.clone(), Expr::str(")")]);
                let some = self.pretty_or(pretty, plain, "Some(", Expr::array(vec![shown]), ")");
                let none = Expr::bin(Op::LooseEq, Expr::var("value"), Expr::null());
                let f = Expr::arrow(
                    vec!["value".into()],
                    vec![StmtKind::Return(Some(Expr::cond(none, Expr::str("None"), some))).at(js::Span::NONE)],
                );
                Ok(self.applied(f, value))
            }
            ty::Tuple(tys) => {
                let tys: Vec<Ty<'tcx>> = tys.to_vec();
                let mut parts = vec![Expr::str("(")];
                let mut items = Vec::new();
                for (i, &t) in tys.iter().enumerate() {
                    if i > 0 {
                        parts.push(Expr::str(", "));
                    }
                    let item =
                        self.debug_string_with(Expr::index(Expr::var("tuple"), Expr::int(i as i128)), t, span, pretty)?;
                    parts.push(item.clone());
                    items.push(item);
                }
                if tys.len() == 1 {
                    parts.push(Expr::str(","));
                }
                parts.push(Expr::str(")"));
                let shown = self.pretty_or(pretty, join(parts), "(", Expr::array(items), ")");
                let f = Expr::arrow(
                    vec!["tuple".into()],
                    vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                );
                Ok(self.applied(f, value))
            }
            ty::Array(item, _) | ty::Slice(item) => self.debug_items(value, *item, "[", "]", span, pretty),
            ty::Adt(_, args) if self.is_vec_like(ty) => {
                self.debug_items(value, args.type_at(0), "[", "]", span, pretty)
            }
            ty::Adt(_, args) if self.is_reverse(ty) => {
                let shown = self.debug_string_with(Expr::index(value, Expr::int(0)), args.type_at(0), span, pretty)?;
                let plain = join(vec![Expr::str("Reverse("), shown.clone(), Expr::str(")")]);
                Ok(self.pretty_or(pretty, plain, "Reverse(", Expr::array(vec![shown]), ")"))
            }
            ty::Adt(_, args) if self.shows_inside(ty) => {
                self.debug_string_with(value, args.types().next().expect("what it holds"), span, pretty)
            }
            // An atomic shows what it holds.
            ty::Adt(_, args) if std(StdItem::Atomic) => {
                self.debug_string_with(Expr::held(value), args.type_at(0), span, pretty)
            }
            // `LazyCell(1)`, or `LazyCell(<uninit>)` where it isn't made yet.
            ty::Adt(adt, args) if std(StdItem::LazyCell) => {
                let name = self.tcx.item_name(adt.did());
                let shown = self.debug_string_with(Expr::held(Expr::var("lazy")), args.type_at(0), span, pretty)?;
                let made = Expr::bin(Op::Eq, Expr::member(Expr::var("lazy"), "init"), Expr::undefined());
                let shown = Expr::cond(made, shown, Expr::str("<uninit>"));
                let plain = join(vec![Expr::str(format!("{name}(")), shown.clone(), Expr::str(")")]);
                let shown = self.pretty_or(pretty, plain, &format!("{name}("), Expr::array(vec![shown]), ")");
                let f = Expr::arrow(
                    vec!["lazy".into()],
                    vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                );
                Ok(self.applied(f, value))
            }
            // `OnceCell(1)`, or `OnceCell(<uninit>)` where it holds none.
            ty::Adt(adt, args) if std(StdItem::OnceCell) => {
                let name = self.tcx.item_name(adt.did());
                let item = args.type_at(0);
                let inside = if self.boxed_payload(item) {
                    self.some_value(Expr::var("value"))
                } else {
                    Expr::var("value")
                };
                let shown = self.debug_string_with(inside, item, span, pretty)?;
                let none = Expr::bin(Op::LooseEq, Expr::var("value"), Expr::null());
                let shown = Expr::cond(none, Expr::str("<uninit>"), shown);
                let plain = join(vec![Expr::str(format!("{name}(")), shown.clone(), Expr::str(")")]);
                let shown = self.pretty_or(pretty, plain, &format!("{name}("), Expr::array(vec![shown]), ")");
                let f = Expr::arrow(
                    vec!["value".into()],
                    vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                );
                Ok(self.applied(f, Expr::held(value)))
            }
            // `Mutex { data: 1, poisoned: false, .. }`, or `data: <locked>`
            // while it's held so, as std's shows it (ADR 0328).
            ty::Adt(_, args) if std(StdItem::Mutex) || std(StdItem::RwLock) => {
                let mutex = std(StdItem::Mutex);
                let shown = self.debug_string_with(Expr::var("value"), args.type_at(0), span, pretty)?;
                let show = Expr::arrow(
                    vec!["value".into()],
                    vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                );
                let name = Expr::str(if mutex { "Mutex" } else { "RwLock" });
                Ok(Expr::call(
                    Expr::helper("$showLocked"),
                    vec![value, show, name, Expr::bool(mutex)],
                ))
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) => {
                let name = if std(StdItem::Cell) { "Cell" } else { "RefCell" };
                let shown = match std(StdItem::RefCell) {
                    // What it holds, shown while it's borrowed, or `<borrowed>`
                    // where it's mutably borrowed (ADR 0328).
                    true => {
                        let shown = self.debug_string_with(Expr::var("value"), args.type_at(0), span, pretty)?;
                        let show = Expr::arrow(
                            vec!["value".into()],
                            vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                        );
                        Expr::call(Expr::helper("$showBorrowed"), vec![value, show])
                    }
                    false => self.debug_string_with(Expr::held(value), args.type_at(0), span, pretty)?,
                };
                let plain = join(vec![
                    Expr::str(format!("{name} {{ value: ")),
                    shown.clone(),
                    Expr::str(" }"),
                ]);
                let field = join(vec![Expr::str("value: "), shown]);
                Ok(self.pretty_or(pretty, plain, &format!("{name} {{"), Expr::array(vec![field]), "}"))
            }
            // An iterator over an array, `IntoIter([2, 3])`, of what it has
            // left (ADR 0473).
            ty::Adt(..) if let Some(name) = self.recognition().shows_its_slice(ty) => {
                let item = self.iterator_item(ty).expect("an iterator's item");
                let left = self.stepped_left(Expr::var("it"), ty, false, span)?;
                let items = self.debug_items(left, item, "[", "]", span, pretty)?;
                let plain = join(vec![Expr::str(format!("{name}(")), items.clone(), Expr::str(")")]);
                let shown = self.pretty_or(pretty, plain, &format!("{name}("), Expr::array(vec![items]), ")");
                let show = Expr::arrow(
                    vec!["it".into()],
                    vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                );
                Ok(self.applied(show, value))
            }
            // An explicit `Enumerate`, `Take` or `Skip`, which keeps what it
            // steps and its count (ADR 0542): `Enumerate { iter: .., count: n
            // }`, `Take { iter: .., n: n }`.
            ty::Adt(..)
                if self.explicit_adapter(ty)
                    && let Some((kind, _, inner)) = crate::lower::recognition::counted_adapter(self.tcx, ty) =>
            {
                let (name, kept) = match kind {
                    crate::lower::recognition::Counted::Enumerate => ("Enumerate", "count"),
                    crate::lower::recognition::Counted::Take => ("Take", "n"),
                    crate::lower::recognition::Counted::Skip => ("Skip", "n"),
                };
                let it = Expr::var(&crate::lower::lower_first(name));
                let stepped = Expr::call_helper("$iteratorValue", vec![Expr::member(it.clone(), "iter")]);
                let shown = self.debug_string_with(stepped, inner, span, pretty)?;
                let count = Expr::member(it, kept);
                let plain = join(vec![
                    Expr::str(format!("{name} {{ iter: ")),
                    shown.clone(),
                    Expr::str(format!(", {kept}: ")),
                    count.clone(),
                    Expr::str(" }"),
                ]);
                let fields = vec![
                    join(vec![Expr::str("iter: "), shown]),
                    join(vec![Expr::str(format!("{kept}: ")), count]),
                ];
                let shown = self.pretty_or(pretty, plain, &format!("{name} {{"), Expr::array(fields), "}");
                let show = Expr::arrow(
                    vec![crate::lower::lower_first(name).as_str().into()],
                    vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                );
                Ok(self.applied(show, value))
            }
            // An empty iterator, std's `Empty`, which shows no state.
            ty::Adt(..) if self.recognition().is_iter_empty(ty) => Ok(Expr::str("Empty")),
            // A `Cycle` a struct holds, which keeps what it started with and
            // what it steps (ADR 0544): `Cycle { orig: .., iter: .. }`.
            ty::Adt(..)
                if self.explicit_adapter(ty)
                    && let Some(source) = crate::lower::recognition::cycled_source(self.tcx, ty) =>
            {
                let it = Expr::var("cycle");
                let orig = self.debug_string_with(Expr::member(it.clone(), "orig"), source, span, pretty)?;
                let iter = self.debug_string_with(Expr::member(it, "iter"), source, span, pretty)?;
                let plain = join(vec![
                    Expr::str("Cycle { orig: "),
                    orig.clone(),
                    Expr::str(", iter: "),
                    iter.clone(),
                    Expr::str(" }"),
                ]);
                let fields = vec![
                    join(vec![Expr::str("orig: "), orig]),
                    join(vec![Expr::str("iter: "), iter]),
                ];
                let shown = self.pretty_or(pretty, plain, "Cycle {", Expr::array(fields), "}");
                let show = Expr::arrow(
                    vec!["cycle".into()],
                    vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                );
                Ok(self.applied(show, value))
            }
            ty::Adt(_, args) if self.is_set(ty) => {
                let items = self.in_order_of(value, ty, span)?;
                self.debug_items(items, args.type_at(0), "{", "}", span, pretty)
            }
            ty::Adt(_, args) if self.is_map(ty) => {
                let value = self.in_order_of(value, ty, span)?;
                let (key, item) = (args.type_at(0), args.type_at(1));
                let key = self.debug_string_with(Expr::var("key"), key, span, pretty)?;
                let item = self.debug_string_with(Expr::var("value"), item, span, pretty)?;
                let pair = join(vec![key, Expr::str(": "), item]);
                let f = Expr::arrow(
                    vec![js::Pattern::Array(vec![Some("key".into()), Some("value".into())])],
                    vec![StmtKind::Return(Some(pair)).at(js::Span::NONE)],
                );
                let entries = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![value]);
                let mapped = Expr::call(Expr::member(entries, "map"), vec![f]);
                let shown = Expr::call(Expr::member(mapped.clone(), "join"), vec![Expr::str(", ")]);
                let plain = join(vec![Expr::str("{"), shown, Expr::str("}")]);
                Ok(self.pretty_or(pretty, plain, "{", mapped, "}"))
            }
            ty::Adt(_, args) if std(StdItem::Result) => {
                let inside = || crate::js::result::held(Expr::var("result"));
                let ok = self.debug_string_with(inside(), args.type_at(0), span, pretty)?;
                let err = self.debug_string_with(inside(), args.type_at(1), span, pretty)?;
                let f = Expr::arrow(
                    vec!["result".into()],
                    vec![
                        StmtKind::Return(Some(Expr::cond(
                            result::is(Expr::var("result"), true),
                            self.pretty_or(
                                pretty,
                                join(vec![Expr::str("Ok("), ok.clone(), Expr::str(")")]),
                                "Ok(",
                                Expr::array(vec![ok]),
                                ")",
                            ),
                            self.pretty_or(
                                pretty,
                                join(vec![Expr::str("Err("), err.clone(), Expr::str(")")]),
                                "Err(",
                                Expr::array(vec![err]),
                                ")",
                            ),
                        )))
                        .at(js::Span::NONE),
                    ],
                );
                Ok(self.applied(f, value))
            }
            // Another crate's enum without fields, std's `IntErrorKind` say:
            // its variant's name, which it is (ADR 0013), as a derived `Debug`
            // shows it.
            ty::Adt(adt, _)
                if representation::is_fieldless_enum(*adt)
                    && !adt.did().is_local()
                    && !self.krate.foreign.in_library(adt.did())
                    && self.recognition().derives(std_item(self.tcx, StdItem::Debug), ty) =>
            {
                Ok(self.variant_debug(*adt, value))
            }
            // Another of std's enums whose variants hold what they hold by
            // place, `Bound`'s `Included(3)`, and whose `Debug` is derived:
            // each variant's name and what it holds, as a derived `Debug`
            // shows them, of its value as any enum's is (ADR 0033).
            ty::Adt(adt, args)
                if adt.is_enum()
                    && !representation::is_fieldless_enum(*adt)
                    && !adt.did().is_local()
                    && !self.krate.foreign.in_library(adt.did())
                    && adt.variants().iter().all(|v| v.ctor_kind().is_some())
                    && self.recognition().derives(std_item(self.tcx, StdItem::Debug), ty) =>
            {
                self.tuple_variants_debug(*adt, args, value, span, pretty)
            }
            _ => Err(self.unsupported(span, &format!("`{{:?}}` of a `{ty}`"))),
        }
    }

    /// `{:?}` of another crate's enum whose variants hold what they hold by
    /// place, as a derived `Debug` shows it: `Unbounded`, a fieldless one's
    /// name, the string it is; `Included(3)` of `{ TAG: "Included", _0: 3 }`.
    fn tuple_variants_debug(
        &mut self,
        adt: ty::AdtDef<'tcx>,
        args: ty::GenericArgsRef<'tcx>,
        value: Expr,
        span: Span,
        pretty: &Pretty,
    ) -> R<Expr> {
        let tag = Expr::member(Expr::var("value"), bindings::tag_key(self.tcx, adt.did()));
        let mut shown: Option<Expr> = None;
        for variant in adt.variants().iter().rev() {
            if crate::lower::recognition::never_made(self.tcx, variant) {
                continue;
            }
            let name = variant.name.to_string();
            let js_name = bindings::variant_name(self.tcx, variant);
            let (test, this) = match variant.fields.is_empty() {
                true => (
                    Expr::bin(Op::Eq, Expr::var("value"), Expr::str(&js_name)),
                    Expr::str(&name),
                ),
                false => {
                    let mut parts = Vec::new();
                    for (i, field) in variant.fields.iter().enumerate() {
                        let held =
                            Expr::member(Expr::var("value"), representation::variant_field(self.tcx, variant, i));
                        parts.push(self.debug_string_with(held, self.field_ty(field, args), span, pretty)?);
                    }
                    let mut plain = vec![Expr::str(format!("{name}("))];
                    for (i, part) in parts.iter().enumerate() {
                        if i > 0 {
                            plain.push(Expr::str(", "));
                        }
                        plain.push(part.clone());
                    }
                    plain.push(Expr::str(")"));
                    let this = self.pretty_or(pretty, join(plain), &format!("{name}("), Expr::array(parts), ")");
                    (Expr::bin(Op::Eq, tag.clone(), Expr::str(&js_name)), this)
                }
            };
            shown = Some(match shown {
                None => this,
                Some(rest) => Expr::cond(test, this, rest),
            });
        }
        let shown = shown.ok_or_else(|| self.unsupported(span, "`{:?}` of an enum of no variants"))?;
        let f = Expr::arrow(
            vec!["value".into()],
            vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
        );
        Ok(self.applied(f, value))
    }

    /// A fieldless enum's derived `{:?}`, its variant's Rust name: the value
    /// itself, its name (ADR 0013), but where `rust_js::name` gave one
    /// another, whose Rust name a table of them gives,
    /// `{ "best fit": "BestFit", lookup: "Lookup" }[fit]`; of a tagged
    /// one's tag (ADR 0284).
    fn variant_debug(&self, adt: ty::AdtDef<'tcx>, value: Expr) -> Expr {
        let value = match bindings::declared_tag(self.tcx, adt.did()) {
            Some(key) => Expr::member(value, key),
            None => value,
        };
        let names: Vec<(String, String)> = adt
            .variants()
            .iter()
            .map(|v| (bindings::variant_name(self.tcx, v), v.name.to_string()))
            .collect();
        if names.iter().all(|(js, rust)| js == rust) {
            return value;
        }
        let table = names
            .into_iter()
            .map(|(js, rust)| js::Prop::Field(js, Expr::str(rust)))
            .collect();
        Expr::index(Expr::object(table), value)
    }

    /// A sequence's `{:?}`: `"[" + items.map((item) => ..).join(", ") + "]"`;
    /// of items each their own text, a unit enum's name, or each `join`'s
    /// own, a number's, `items.join(", ")` (ADR 0508).
    fn debug_items(
        &mut self,
        items: Expr,
        item: Ty<'tcx>,
        open: &str,
        close: &str,
        span: Span,
        pretty: &Pretty,
    ) -> R<Expr> {
        let shown = self.debug_string_with(Expr::var("item"), item, span, pretty)?;
        // Each its own text, a unit enum's name; or, `join`'s own, a
        // number's, where it isn't pretty, whose lines are strings.
        let each_its_own = is_var(&shown, "item")
            || (pretty.known() == Some(false)
                && matches!(&shown.kind, js::ExprKind::Stringed(value) if is_var(value, "item")));
        let f = Expr::arrow(
            vec!["item".into()],
            vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
        );
        let items = if self.is_map(item) || open == "{" {
            Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items])
        } else {
            items
        };
        let mapped = match each_its_own {
            true => items,
            false => Expr::call(Expr::member(items, "map"), vec![f]),
        };
        let joined = Expr::call(Expr::member(mapped.clone(), "join"), vec![Expr::str(", ")]);
        let plain = join(vec![Expr::str(open), joined, Expr::str(close)]);
        Ok(self.pretty_or(pretty, plain, open, mapped, close))
    }
}
