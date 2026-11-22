//! `format_args!`'s calls, its parts kept until `Arguments::new` shows
//! them, an assertion's failure, and a write to a writer of the crate's
//! (ADRs 0054, 0058): `mir_std_call`'s first kind, apart. Split from
//! std_calls.rs by concept, as a file is at most 1,500 lines.

use super::*;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// `format_args!`'s parts, kept until `Arguments::new` shows them, an
    /// assertion's failure, and a write to a writer of the crate's (ADRs 0054,
    /// 0058).
    // helper: one of `mir_std_call`'s kinds, each asked in turn, as `std_calls.rs`'s others
    pub(crate) fn mir_fmt_call<'c>(
        &self,
        _state: &State<'_, 'tcx>,
        call: StdCall<'c, 'tcx>,
    ) -> Option<StdLowering<'c, 'a, 'tcx>> {
        let this = self;
        let StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
        } = call;
        let tcx = this.tcx;
        let first_ty = move || generic_args.types().next();
        Some(match known {
            // `format_args!`'s parts, kept until `Arguments::new` shows them.
            Std::FmtDisplay | Std::FmtDebug | Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {
                lower(move |this, _, values, _| {
                    // `{:x}`'s and the like, which `format_value` writes by its spec.
                    let ty = match known {
                        Std::FmtUsize => this.tcx.types.usize,
                        _ => first_ty().expect("a type argument"),
                    };
                    let value = match values.next().expect("an argument") {
                        // A `&dyn Debug`, shown as what it's made of is.
                        Value::DynDebug(from, _, value) if known == Std::FmtDebug => {
                            return Ok(Value::Fmt(known, this.pointee(from), value));
                        }
                        value => this.value_expr(value, span)?,
                    };
                    // As it's shown, a handle the place it's on; a pointer's
                    // `{:p}`, its address, as it is.
                    let (value, ty) = match known {
                        Std::FmtPointer => (value, ty),
                        _ => this.shown_through(value, ty, known),
                    };
                    // One read twice, `v == null ? "None" : ..`, is held first by
                    // tidy, which sees where nothing it does moves (ADR 0377).
                    Ok(Value::Fmt(known, ty, value))
                })
            }
            // `as_str()` of an `Arguments` the body made: its literal's
            // text, `from_str("plain")`'s, or `None` of one made with
            // arguments, `new(..)`'s, as native Rust's (#1091). Of one made
            // elsewhere, a parameter's, nothing says which: refused.
            // Of a literal's given on, a function of its text (#1565), called.
            Std::FmtAsStr => match this.arguments_made(_state.body, _state.at) {
                Some((true, made)) => {
                    let text = rust_js_lower::analysis::mir_reads::arguments_use::holds_text(
                        &this.recognition(),
                        _state.body,
                        made,
                    );
                    lower(move |this, _, values, _| {
                        let value = values.next().expect("the arguments");
                        Ok(match text {
                            true => value,
                            false => Value::Expr(Expr::call(this.value_expr(value, span)?, Vec::new())),
                        })
                    })
                }
                Some((false, _)) => lower(|_, _, _, _| Ok(Value::Expr(Expr::js_undefined()))),
                None => lower(move |this, _, _, _| {
                    Err(this.unsupported(
                        span,
                        "`as_str()` of a `fmt::Arguments` made in another function: whether it was made with arguments, which makes it `None`, isn't known here (#1091)",
                    ))
                }),
            },
            // `panic!("..")`'s, of its payload's kind (ADR 0660, amended).
            Std::PanicFmt => this.panic_fmt_lowering(_state.body, _state.at, span),
            Std::FmtNew => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let template = values.next().expect("a template");
                    let items = values.next().expect("the arguments");
                    let Value::Expr(template) = template else {
                        return Err(this.unsupported(span, "this format string, from its MIR"));
                    };
                    let bytes = expr_bytes(&template)
                        .ok_or_else(|| this.unsupported(span, "this format string, from its MIR"))?;
                    let Value::List(mut items, _) = items else {
                        return Err(this.unsupported(span, "these format arguments, from its MIR"));
                    };
                    let template = rust_js_lower::format_args::decode_template(&bytes)
                        .ok_or_else(|| this.unsupported(span, "this format string"))?;
                    // Written into a writer a piece at a time (#1652).
                    if this.arguments_written(state) {
                        return this.arguments_writes(state, items, &template, span, out);
                    }
                    // As its format string is written: across lines, or
                    // `concat!`'s pieces (ADRs 0066, 0641).
                    let written = rust_js_lower::format_args::written_pieces(tcx, span, template);
                    let template: Vec<_> = written.iter().flat_map(|(pieces, _)| pieces.iter().cloned()).collect();
                    // Where each placeholder's argument is written (ADR 0641, amended).
                    let arguments = rust_js_lower::format_args::placeholder_arguments(tcx, span, &template);
                    let arguments = arguments.as_deref().unwrap_or_default();
                    // Of what may fail, its `Arguments` say so to `format!`'s or
                    // `print!`'s: `fmt_may_fail` of each, by its trait (ADR 0187).
                    let fails = items.iter().any(|item| match *item {
                        Value::Fmt(Std::FmtDisplay, ty, _) => this.fmt_may_fail(this.display_trait(), ty),
                        Value::Fmt(Std::FmtDebug, ty, _) => this.fmt_may_fail(this.debug_trait(), ty),
                        // A number's or a pointer's, unless a type of the crate's.
                        Value::Fmt(_, ty, _) => this.krate.failing.any && ty.peel_refs().is_adt(),
                        _ => false,
                    });
                    // Of what may print as it's formatted, its pieces kept apart,
                    // which `println!` writes through stdout's line as each is
                    // made (#1490).
                    let prints = items.iter().any(|item| match *item {
                        Value::Fmt(Std::FmtDisplay | Std::FmtDebug, ty, _) => this.shown_may_print(ty),
                        _ => false,
                    });
                    let mut parts = this.written_parts(&items, &written, arguments, span)?;
                    // Rust evaluates each argument before it formats any, where
                    // the template formats each where it's written: where that
                    // would run one after formatting that runs code, or after a
                    // piece written apart, each argument that does something
                    // is held first (ADR 0377).
                    if this.formats_out_of_order(&items, &template, fails || prints, span)? {
                        this.flush(state, out)?;
                        for item in items.iter_mut() {
                            if let Value::Fmt(_, _, value) = item
                                && value.has_effects()
                            {
                                *value = this.spill(Base::Temporary, std::mem::replace(value, Expr::taken()), out);
                            }
                        }
                        parts = this.written_parts(&items, &written, arguments, span)?;
                    }
                    // What may fail is written piece by piece, so that what
                    // came before is written: its pieces kept apart (ADR 0187).
                    let joined = rust_js_lower::display::join_written(parts, fails || prints);
                    if prints {
                        return Ok(Value::Printing(joined, fails));
                    }
                    if fails {
                        return Ok(Value::Fmt(Std::FmtNew, output, joined));
                    }
                    joined
                }))
            }),
            // `assert_failed(kind, &left, &right, None or Some(message))`: the
            // two values' `{:?}`, by their types (ADR 0060).
            Std::AssertFailed => lower(move |this, _, values, _| {
                Ok(Value::Expr({
                    let mut list = Vec::new();
                    for (i, value) in values.enumerate() {
                        let value = this.value_expr(value, span)?;
                        list.push(match i {
                            1 | 2 => this.debug_string(value, arg_tys[i].peel_refs(), span)?,
                            _ => value,
                        });
                    }
                    // A `None` message, `undefined`, is left out as preparation
                    // leaves out each trailing one.
                    Expr::call(Expr::helper("$assertFailed"), list)
                }))
            }),
            // A writer of the crate's own: what `write!` and `write_char` give
            // its `write_str` (ADR 0166).
            Std::UserWrite => lower(move |this, state, values, out| {
                this.user_write(state, (def_id, generic_args), values.collect(), span, out)
            }),
            // A writer of bytes of the crate's own: its dictionary's
            // `write_all` or `write_fmt`, given it in a box, as generic code
            // gives it (ADR 0392).
            Std::UserIoWrite => lower(move |this, _, values, _| {
                Ok(Value::Expr({
                    let mut exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    let trait_id = tcx.trait_of_assoc(def_id).expect("`io::Write`'s method");
                    let tr = ty::TraitRef::new_from_args(tcx, trait_id, generic_args);
                    let dictionary = this.dictionary(tr, span)?;
                    let receiver = exprs.remove(0);
                    let receiver = match this.mut_ref_form(tr.self_ty()).is_cell() {
                        true => receiver,
                        false => rust_js_lower::mut_refs::boxed(receiver),
                    };
                    exprs.insert(0, receiver);
                    let call = Expr::call(Expr::member(dictionary, bindings::fn_name(tcx, def_id)), exprs);
                    // std's `write_all` of the crate's own `write`, which may fail
                    // where any of the crate's may: its result a value (ADR 0462).
                    match this.krate.failing.any {
                        true => this.fmt_try(vec![StmtKind::Return(Some(call)).at(js::Span::NONE)]),
                        false => call,
                    }
                }))
            }),
            _ => return None,
        })
    }
}
