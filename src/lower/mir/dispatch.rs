//! A call from MIR: what it calls and its arguments' values, handed to
//! `calls.rs`, `std_calls.rs` or `iter.rs`, and a std call of its values.

use super::*;

/// One kind of std call's lowering, `std_calls`': its value, or `None`, a
/// call it doesn't know, of which it's taken no argument and written nothing.
type StdHandler<'a, 'tcx> = for<'s, 'c> fn(
    &mut FnCx<'a, 'tcx>,
    &mut State<'s, 'tcx>,
    std_calls::StdCall<'c, 'tcx>,
    &mut std::vec::IntoIter<Value<'tcx>>,
    &mut Vec<Stmt>,
) -> R<Option<Value<'tcx>>>;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of a function pointer, which may be JS's (ADR 0330): called
    /// with its arguments given to JS.
    fn mir_fn_pointer_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let ty::FnPtr(sig, _) = func.ty(&state.body.local_decls, self.tcx).kind() else {
            return Err(self.unsupported(span, "calling a function value, from its MIR"));
        };
        // Rest arguments as a value, its last parameter, spread (ADR 0667):
        // the pointer is a variadic binding's JS function.
        let rest = (sig.skip_binder().inputs().last()).is_some_and(|&ty| bindings::is_rest_value(self.tcx, ty));
        let mut operands: Vec<&Operand<'tcx>> = vec![func];
        operands.extend(args.iter().map(|a| &a.node));
        let values = self.take_operands(state, &operands, out)?;
        let mut exprs = values
            .into_iter()
            .map(|v| self.value_expr(v, span))
            .collect::<R<Vec<_>>>()?;
        let callee = exprs.remove(0);
        if rest {
            crate::lower::calls::spread_rest(&mut exprs);
        }
        crate::lower::calls::given_to_js(&mut exprs);
        Ok(Value::Expr(Expr::call(callee, exprs)))
    }

    pub(super) fn mir_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        destination: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let decls = &state.body.local_decls;
        // A writer's `Debug` builder's step, or a write to its `Formatter`.
        if let Some(value) = self.mir_writer_call(state, func, args, destination, span, out)? {
            return Ok(value);
        }
        let Some((def_id, generic_args)) = fn_def(func.ty(decls, self.tcx)) else {
            return self.mir_fn_pointer_call(state, func, args, span, out);
        };
        let (def_id, generic_args) = self.recognition().call_it_is(def_id, generic_args);
        if let Some(defined) = self.mir_defined(state.body, def_id, args, span)? {
            return Ok(Value::Expr(defined));
        }
        self.check_text_pointer(state.body, (def_id, generic_args), destination, span)?;
        // `..Default::default()` of a struct whose `Default` is derived, an
        // update's base: each default the update reads, none it replaces that
        // does nothing, a component's `children: C` among them, which needs no
        // dictionary then (`update_default`).
        if args.is_empty()
            && let Some(fields) = self.default_base_at(span)?
        {
            // Each field read where it's read, `{ size, tags: [] }`, of one
            // held whole; else the object of them.
            if destination.projection.is_empty() {
                let values = fields.into_iter().map(|(_, value)| value).collect();
                state.defaults.insert(destination.local, values);
                return Ok(Value::Expr(Expr::undefined()));
            }
            let props = fields
                .into_iter()
                .map(|(name, value)| Prop::Field(name, value))
                .collect();
            return Ok(Value::Expr(Expr::object(props)));
        }
        // A function a variable holds, `js::import!(f).await`'s: called as
        // that variable, which may be all that holds it, not as its import.
        // One a JS function, or the crate's that JS calls as it is
        // (`called_as_held`).
        let called_as_js = match bindings::is_binding(self.tcx, def_id) {
            true => {
                !crate::lower::bindings::is_method(self.tcx, def_id)
                    && matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::Call(_))
            }
            false => self.called_as_held(def_id, args.len()),
        };
        if let Operand::Copy(_) | Operand::Move(_) = func
            && called_as_js
        {
            let mut operands: Vec<&Operand<'tcx>> = vec![func];
            operands.extend(args.iter().map(|a| &a.node));
            let values = self.take_operands(state, &operands, out)?;
            let mut exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let callee = exprs.remove(0);
            crate::lower::calls::given_to_js(&mut exprs);
            return Ok(Value::Expr(Expr::call(callee, exprs)));
        }
        // `js::import!(f)`: what `f` names imported, not its value.
        if bindings::is_binding(self.tcx, def_id)
            && let bindings::JsForm::Import { module } = bindings::js_form(self.tcx, def_id)
        {
            let Some((item, _)) = args.first().and_then(|a| fn_def(a.node.ty(decls, self.tcx))) else {
                return Err(self.unsupported(span, "importing what isn't a function"));
            };
            return Ok(Value::Expr(self.dynamic_import_of(item, module, span)?));
        }
        let arg_tys: Vec<Ty<'tcx>> = args.iter().map(|a| a.node.ty(decls, self.tcx)).collect();
        let output = destination.ty(decls, self.tcx).ty;
        let operands: Vec<&Operand<'tcx>> = args.iter().map(|a| &a.node).collect();
        let known = self.recognition().classify(def_id, generic_args);
        // A read of items that may be `None` whose `Option` only `.flatten()`
        // reads: the JS read itself, which `flatten` gives as it is (ADR 0311).
        if let Some(known) = known
            && crate::lower::std_types::vec::reads_flattened(known)
            && self.flattened_by(state, destination)
        {
            let values = self.take_operands(state, &operands, out)?;
            let values = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            state.flattened.insert(destination.local);
            return Ok(Value::Expr(crate::lower::std_types::vec::flattened_read_values(
                known, values,
            )));
        }
        if self.reads_through(state, known, &operands, destination) {
            return Ok(self.take_operands(state, &operands, out)?.remove(0));
        }
        self.mir_call_operands(state, (def_id, generic_args), (&operands, &arg_tys), output, span, out)
    }

    /// Whether a call's value is its one argument, as it's read: `flatten()`
    /// of a read that's flattened, and a copy that's the item itself, which
    /// is then flattened too (ADR 0311); and `unwrap()` of a throwing call's
    /// `Result` made where it's read, of each branch, each call itself (ADR
    /// 0747).
    fn reads_through(
        &self,
        state: &mut State<'_, 'tcx>,
        known: Option<Std>,
        operands: &[&Operand<'tcx>],
        destination: Place<'tcx>,
    ) -> bool {
        let [Operand::Move(read)] = operands else {
            return false;
        };
        if !read.projection.is_empty() {
            return false;
        }
        match known {
            Some(Std::OptionFlatten | Std::OptionCloned) if state.flattened.contains(&read.local) => {
                state.flattened.insert(destination.local);
                true
            }
            Some(Std::UnwrapOk) => self.unwrapped_where_made(state, read.local),
            _ => false,
        }
    }

    /// Whether what a call makes, `destination`, is read only by `.flatten()`,
    /// or by a copy that's the item itself, `.copied()` of a reference, a
    /// number or text, which only `.flatten()` reads.
    fn flattened_by(&self, state: &State<'_, 'tcx>, destination: Place<'tcx>) -> bool {
        let decls = &state.body.local_decls;
        if !destination.projection.is_empty() || state.locals.reads[destination.local] != 1 {
            return false;
        }
        state.body.basic_blocks.iter().any(|data| {
            let TerminatorKind::Call {
                func,
                args,
                destination: made,
                ..
            } = &data.terminator().kind
            else {
                return false;
            };
            let reads = matches!(&args[..], [arg]
                if matches!(&arg.node, Operand::Move(read) if *read == Place::from(destination.local)));
            let known = fn_def(func.ty(decls, self.tcx))
                .and_then(|(def_id, generic_args)| self.recognition().classify(def_id, generic_args));
            let item_itself = || {
                self.option_of(made.ty(decls, self.tcx).ty)
                    .and_then(|inner| self.option_of(inner))
                    .is_some_and(|item| item.is_ref() || item.is_primitive() || self.is_string_like(item))
            };
            reads
                && match known {
                    Some(Std::OptionFlatten) => true,
                    Some(Std::OptionCloned) => item_itself() && self.flattened_by(state, *made),
                    _ => false,
                }
        })
    }

    /// Whether `local` holds a throwing JS call's `Result` that's unwrapped
    /// where it's made, of each branch that gives it, `if c { f() } else {
    /// g() }.unwrap()` (ADR 0747): its one read an `unwrap()`, and each
    /// write of it a call that throws (`call_throws`), which goes on to
    /// that read past nothing that runs. Each call is then the call itself,
    /// its throw JS's, and the `unwrap()` its value, as of one call
    /// unwrapped (ADR 0643).
    pub(super) fn unwrapped_where_made(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        let body = state.body;
        let decls = &body.local_decls;
        if state.locals.reads[local] != 1 {
            return false;
        }
        let known = |func: &Operand<'tcx>| {
            fn_def(func.ty(decls, self.tcx))
                .and_then(|(def_id, generic_args)| self.recognition().classify(def_id, generic_args))
        };
        let unwrap = body.basic_blocks.iter_enumerated().find_map(|(block, data)| {
            let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                return None;
            };
            let reads = matches!(&args[..], [arg]
                if matches!(&arg.node, Operand::Move(read) if *read == Place::from(local)));
            (reads && known(func) == Some(Std::UnwrapOk)).then_some(block)
        });
        let Some(unwrap) = unwrap else { return false };
        // From `block` on to the `unwrap()`, nothing runs: no statement does
        // anything, and each block goes on, or drops what drops nothing.
        let reaches = |mut block: BasicBlock| {
            for _ in 0..body.basic_blocks.len() {
                let data = &body.basic_blocks[block];
                let idle = |s: &mir::Statement<'_>| {
                    matches!(
                        s.kind,
                        StatementKind::StorageLive(_)
                            | StatementKind::StorageDead(_)
                            | StatementKind::FakeRead(_)
                            | StatementKind::AscribeUserType(..)
                            | StatementKind::PlaceMention(_)
                            | StatementKind::Nop
                    )
                };
                if !data.statements.iter().all(idle) {
                    return false;
                }
                if block == unwrap {
                    return true;
                }
                match &data.terminator().kind {
                    TerminatorKind::Goto { target } => block = *target,
                    TerminatorKind::Drop { place, target, .. }
                        if self.drops(place.ty(decls, self.tcx).ty) == crate::lower::drops::Drops::Nothing =>
                    {
                        block = *target
                    }
                    _ => return false,
                }
            }
            false
        };
        let mut made = false;
        for data in body.basic_blocks.iter() {
            if (data.statements.iter())
                .any(|s| matches!(&s.kind, StatementKind::Assign(assign) if assign.0.local == local))
            {
                return false;
            }
            if let TerminatorKind::Call {
                func,
                destination,
                target,
                ..
            } = &data.terminator().kind
                && destination.local == local
            {
                let throws = destination.projection.is_empty()
                    && fn_def(func.ty(decls, self.tcx)).is_some_and(|(def_id, generic_args)| {
                        matches!(
                            crate::lower::analysis::call_throws(
                                &self.recognition(),
                                self.krate.throwing,
                                def_id,
                                generic_args
                            ),
                            crate::lower::recognition::Catching::Result
                        )
                    });
                if !throws || !target.is_some_and(reaches) {
                    return false;
                }
                made = true;
            }
        }
        made
    }

    /// `.await` of what's given, `into_future(x)`'s (`awaits.rs`): of an
    /// `IntoFuture` of the crate's, of what its `into_future` makes (ADR
    /// 0571).
    pub(super) fn mir_await(
        &mut self,
        state: &mut State<'_, 'tcx>,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        destination: mir::Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let operands: Vec<&Operand<'tcx>> = args.iter().map(|a| &a.node).collect();
        let mut ty = operands[0].ty(&state.body.local_decls, self.tcx);
        let mut awaited = self.take_operands(state, &operands, out)?.remove(0);
        if self.recognition().own_into_future(ty)
            && let Some((def_id, generic_args)) = fn_def(func.ty(&state.body.local_decls, self.tcx))
        {
            let sig = self
                .tcx
                .fn_sig(def_id)
                .instantiate(self.tcx, generic_args)
                .skip_normalization();
            let made = self.tcx.instantiate_bound_regions_with_erased(sig).output();
            let made = self
                .tcx
                .normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(made));
            awaited = self.mir_call_values(
                state,
                (def_id, generic_args),
                (vec![awaited], &[ty], &[]),
                made,
                span,
                out,
            )?;
            ty = made;
        }
        // A pinned `&mut` to a future, `pin!(f).await`, is the promise it
        // points at (ADR 0329): the place it's to, or its cell's `value`.
        let pointer = self.recognition().pinned(ty).unwrap_or(ty);
        let awaited = self.value_expr(awaited, span)?;
        let awaited = Expr::await_(self.through_refs(awaited, pointer).0);
        // What it gives, out of the box `$settled` put it in, where it may
        // be a thenable (#172).
        let output = destination.ty(&state.body.local_decls, self.tcx).ty;
        Ok(Value::Expr(match self.may_be_thenable(output) {
            true => Expr::call_helper("$unsettled", vec![awaited]),
            false => awaited,
        }))
    }

    /// Any other function as a value, `.map(str::len)`: the arrow that calls
    /// it, its call lowered as one from MIR is, of its parameters.
    pub(super) fn mir_called_value(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        span: Span,
    ) -> R<Expr> {
        let tcx = self.tcx;
        let sig = tcx.fn_sig(def_id).instantiate(tcx, args).skip_normalization();
        let sig = tcx.instantiate_bound_regions_with_erased(sig);
        let mut inputs: Vec<Ty<'tcx>> = sig.inputs().to_vec();
        let params: Vec<String> = match inputs.len() {
            // Named by its type: `(s) => ..` of a string.
            1 => vec![self.fresh(self.parameter_name(inputs[0].peel_refs()))],
            n => (0..n)
                .map(|i| self.fresh(["a", "b", "c", "d", "e", "f"].get(i).copied().unwrap_or("arg")))
                .collect(),
        };
        let mut values: Vec<Value> = params.iter().map(|p| Value::Expr(Expr::var(p))).collect();
        // A C-variadic one's, given on what it's given past the rest, `(a,
        // ...args) => f(a, ...args)` (ADR 0428): C's numbers and pointers,
        // passed as they are.
        let variadic = crate::lower::recognition::is_c_variadic(tcx, def_id).then(|| self.fresh("args"));
        if let Some(rest) = &variadic {
            values.push(Value::Expr(Expr::spread(Expr::var(rest))));
            inputs.push(tcx.types.i32);
        }
        // What's made for the call is the arrow's, not what's around it: the
        // boxes it gives a `&mut` too, copied back in it once the call returns,
        // as a call's are after it (ADR 0099).
        let mut body = Vec::new();
        let (value, copy_backs) = self.in_fresh_evaluation(state, |cx, state| {
            let called = cx.mir_call_values(
                state,
                (def_id, args),
                (values, &inputs, &[]),
                sig.output(),
                span,
                &mut body,
            );
            let value = called.and_then(|value| cx.value_expr(value, span));
            let flushed = cx.flush(state, &mut body);
            value.and_then(|value| flushed.map(|()| value))
        });
        let mut value = value?;
        if !copy_backs.is_empty() {
            value = self.spill("value", value, &mut body);
            for (place, boxed, object) in copy_backs {
                let js_span = self.js_span(span);
                body.push(self.copied_back(place, &boxed, object, js_span));
            }
        }
        if body.is_empty()
            && let Some(callee) = crate::lower::called_by_name(&params, &value)
        {
            return Ok(callee);
        }
        body.push(StmtKind::Return(Some(value)).at(self.js_span(span)));
        let mut patterns: Vec<js::Pattern> = params.into_iter().map(Into::into).collect();
        patterns.extend(variadic.map(js::Pattern::Rest));
        Ok(Expr::arrow(patterns, body))
    }

    /// A call of what `operands` give, each read, an `==` of an `Option`
    /// strict where it can be (`strictly_equal`).
    fn mir_call_operands(
        &mut self,
        state: &mut State<'_, 'tcx>,
        callee: (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (operands, arg_tys): (&[&Operand<'tcx>], &[Ty<'tcx>]),
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        if let Some(given) = self.mir_thrown_given(state, callee, operands, output, span, out)? {
            return Ok(given);
        }
        let strict = self.strictly_equal(state, callee.0, operands);
        // Of a binding with a nullable parameter, each `Option` that's
        // `null` or a value already, which it takes as it is (ADR 0275).
        let nulled: Vec<bool> = match bindings::nullable_params(self.tcx, callee.0).is_empty() {
            true => Vec::new(),
            false => operands.iter().map(|o| self.nulled(state, o)).collect(),
        };
        let values = self.take_operands(state, operands, out)?;
        match self.mir_call_values(state, callee, (values, arg_tys, &nulled), output, span, out)? {
            Value::Expr(e) if strict => Ok(Value::Expr(js::strict_equality(e))),
            value => Ok(value),
        }
    }

    /// What's given an unwind's payload's `&JsError`, the closure of
    /// `and_then`, `map` or `is_some_and` of its downcast, a local
    /// `OptionWrites::thrown` says is a `Some` always (ADR 0710): called
    /// with it, as Rust calls it with a `Some`'s value, `x.stack` (ADR 0710,
    /// amended). `None` of any other call.
    fn mir_thrown_given(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        operands: &[&Operand<'tcx>],
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        use crate::lower::combinators::Comb;
        let known = self.recognition().classify(def_id, generic_args);
        let Some(known @ (Std::Comb(Comb::AndThen | Comb::IsSomeAnd) | Std::OptionMap)) = known else {
            return Ok(None);
        };
        let [Operand::Copy(subject) | Operand::Move(subject), _] = operands[..] else {
            return Ok(None);
        };
        if !subject.projection.is_empty() || !state.options.thrown.contains_key(&subject.local) {
            return Ok(None);
        }
        let values = self.take_operands(state, operands, out)?;
        let exprs = (values.into_iter())
            .map(|v| self.value_expr(v, span))
            .collect::<R<Vec<_>>>()?;
        let [error, f]: [Expr; 2] = exprs.try_into().ok().expect("an `Option` and a function");
        let given = self.call_with(f, vec![error], "then", out);
        let boxed = known == Std::OptionMap && self.option_of(output).is_some_and(|item| self.boxed_payload(item));
        Ok(Some(Value::Expr(if boxed { self.some(given) } else { given })))
    }

    /// Whether an `==` of an `Option` is `===`: of one side only ever
    /// `Some(..)`, `p == Some(s)`, never `None`, so either answers the same
    /// where `==` is written, of a payload that's its `Some` (`eq_value`),
    /// and `===` is what a person writes (ADR 0030).
    fn strictly_equal(
        &self,
        state: &State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        operands: &[&Operand<'tcx>],
    ) -> bool {
        self.tcx.trait_of_assoc(def_id).is_some_and(|trait_id| {
            matches!(
                self.recognition().trait_call(def_id, trait_id),
                Some(TraitCall::Equality { .. })
            )
        }) && operands.iter().any(|operand| {
            operand
                .place()
                .is_some_and(|place| place.projection.is_empty() && state.options.made_some.contains(&place.local))
        })
    }

    /// A call, `mir_call_values_of`'s; of std's, each function or value it's
    /// given by value with a destructor, dropped where std drops it (ADRs
    /// 0469, 0470, 0490): as it returns, or where it's left unused, by its
    /// subject's variant, both held in variables first.
    fn mir_call_values(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (mut values, arg_tys, nulled): (Vec<Value<'tcx>>, &[Ty<'tcx>], &[bool]),
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let given = match self.is_std_call(def_id, generic_args)? {
            true => {
                let known = self.known_trait_call(def_id, generic_args);
                self.given_unused(known, (arg_tys, output), span)?
            }
            false => {
                self.check_called_once(def_id, arg_tys, span)?;
                Vec::new()
            }
        };
        if given.is_empty() {
            return self.mir_call_values_of(
                state,
                (def_id, generic_args),
                (values, arg_tys, nulled),
                output,
                span,
                out,
            );
        }
        let subject = match given.iter().find_map(|&(_, when)| when) {
            Some(when) => Some(self.held_value(&mut values, 0, when.subject(), span, out)?),
            None => None,
        };
        let mut held = Vec::new();
        for (at, when) in given {
            let f = self.held_value(&mut values, at, "f", span, out)?;
            held.push((when, f, arg_tys[at]));
        }
        let value = self.mir_call_values_of(
            state,
            (def_id, generic_args),
            (values, arg_tys, nulled),
            output,
            span,
            out,
        )?;
        // What it gives, before what's dropped after it.
        let value = match value {
            Value::Expr(e) => Value::Expr(self.spill("value", e, out)),
            value => value,
        };
        self.dropped_given(subject, held, span, out)?;
        Ok(value)
    }

    /// Whether `def_id` is std's, as it's called with `generic_args`: not
    /// the crate's, a binding, a call of a function value, nor a trait's
    /// that resolves to the crate's impl.
    fn is_std_call(&self, def_id: rustc_span::def_id::DefId, generic_args: ty::GenericArgsRef<'tcx>) -> R<bool> {
        let tcx = self.tcx;
        if self.is_rust_fn(def_id) || bindings::is_binding(tcx, def_id) {
            return Ok(false);
        }
        if let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && (tcx.fn_trait_kind_from_def_id(trait_id).is_some() || self.is_rust_trait(trait_id))
        {
            return Ok(false);
        }
        Ok(!self.resolves_to_crate(def_id, generic_args)?)
    }

    fn mir_call_values_of(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (values, arg_tys, nulled): (Vec<Value<'tcx>>, &[Ty<'tcx>], &[bool]),
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let arg_tys = arg_tys.to_vec();
        let tcx = self.tcx;
        // What's refused of a call: the crate's collection or `Borrow` where
        // std's code takes JS's (ADRs 0160, 0167).
        self.check_call_takes(def_id, generic_args, span)?;
        // serde_json's `Value` and what makes one (ADR 0083).
        if let Some(operation) = self.recognition().json_call(def_id, generic_args) {
            let exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            return Ok(Value::Expr(self.json_call_values(operation, exprs, span, out)?));
        }
        // `x == &mut 1`, `p > q`, of `&mut`s to what JS can't change in place:
        // of what they point at (ADR 0099), not the handles.
        if let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && matches!(
                self.recognition().trait_call(def_id, trait_id),
                Some(TraitCall::Equality { .. } | TraitCall::Ordering)
            )
            && generic_args.types().any(|t| self.is_cell(t))
        {
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
            for (value, &ty) in values.into_iter().zip(&arg_tys) {
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
            return self.mir_call_values_of(state, (def_id, peeled), (given, &tys, nulled), output, span, out);
        }
        // A constructor called, `f(Shape::Rect)`'s: the arrow making what it
        // makes (ADR 0125), applied.
        if matches!(tcx.def_kind(def_id), DefKind::Ctor(_, rustc_hir::def::CtorKind::Fn)) {
            let made = self.constructor_value(def_id, generic_args, span)?;
            let exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            return Ok(Value::Expr(crate::lower::calls::apply(made, exprs)));
        }
        if self.of_never_failing(def_id, &arg_tys) {
            return self.never_failing_call(state, def_id, (values, &arg_tys), span, out);
        }
        // `x?`: `Try::branch(x)`, kept as `x` (`Value::Branch`), and what it
        // returns, `FromResidual::from_residual(r)` (ADR 0052).
        if tcx.is_lang_item(def_id, LangItem::TryTraitBranch) {
            let ty = arg_tys[0];
            // Of one that may be an `Err`: its error thrown on, `fmt_check`
            // (ADR 0187), and what's left never breaks.
            if self.is_write_result(ty) && self.krate.any_failing {
                return self.tried_write(state, values, ty, span, out);
            }
            if !self.is_write_result(ty) && self.option_of(ty).is_none() && !self.is_std_type(ty, StdItem::Result) {
                return Err(self.unsupported(span, &format!("`?` on a `{ty}`, from its MIR")));
            }
            let tried = self.value_expr(values.into_iter().next().expect("the value tried"), span)?;
            let tried = match tried.reads_same() {
                true => tried,
                false => {
                    self.flush(state, out)?;
                    self.spill(
                        if self.option_of(ty).is_some() {
                            "value"
                        } else {
                            "result"
                        },
                        tried,
                        out,
                    )
                }
            };
            return Ok(Value::Branch(tried, ty));
        }
        if tcx.is_lang_item(def_id, LangItem::TryTraitFromResidual) {
            let ty = arg_tys[0];
            if self.option_of(ty).is_none() && !self.is_std_type(ty, StdItem::Result) {
                return Err(self.unsupported(span, &format!("`?` returning a `{ty}`, from its MIR")));
            }
            let residual = self.value_expr(values.into_iter().next().expect("the residual"), span)?;
            if self.is_write_result(output) {
                return Ok(Value::Expr(self.thrown_residual(residual, output)));
            }
            let convert = self.tried_conversion(ty, Some(output), span)?;
            let converted = convert.map(|conversion| conversion.of(crate::js::result::held(residual.clone())));
            return Ok(Value::Expr(self.tried_returned(residual, ty, converted)));
        }
        // `x.into()` is the `From::from(x)` it calls, the crate's own (ADR 0052).
        let (def_id, generic_args) = self
            .resolve_into(def_id, generic_args)
            .unwrap_or((def_id, generic_args));
        // Calling a closure, `Fn::call(&f, (a, b))`: in JS, `f(a, b)`, and an
        // `async` one's, `AsyncFn::async_call`, which gives its promise.
        if let Some(fn_trait) = tcx.trait_of_assoc(def_id)
            && (tcx.fn_trait_kind_from_def_id(fn_trait).is_some()
                || tcx.async_fn_trait_kind_from_def_id(fn_trait).is_some())
        {
            return self.mir_closure_call(values, &arg_tys, None, span);
        }
        // `a += b` of a number or a string, through its `&mut`: the place
        // assigned, as the operator's is.
        if let Some(value) = self.mir_assign_call(state, (def_id, generic_args), (&values, &arg_tys), span, out)? {
            return Ok(value);
        }
        // A std iterator's method: a JS iterator's, lazy as Rust's. A `&mut`
        // to one is it, an object its helpers step in place. Not one the
        // crate's impl has, in a copied default as its impl calls it (ADR
        // 0492).
        if tcx.trait_of_assoc(def_id).is_some() && !self.resolves_to_crate(def_id, generic_args)? {
            let mut exprs = (values.clone().into_iter())
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            // `v.iter().map(f).next_back()`: the map its temporary holds, which
            // nothing else reads, there, `next_back` stepping its source from
            // its end; tidy drops the temporary nothing reads then.
            if let Some(js::ExprKind::Var(name)) = exprs.first().map(|e| &e.kind)
                && let Some((local, _)) = state.locals.names.iter_enumerated().find(|(_, n)| *n == name)
                && state.locals.reads[local] == 1
                && let Some(map) = state.maps.get(&local)
            {
                exprs[0] = map.clone();
            }
            let mut made = Vec::new();
            if let Some(call) = self.mir_iter_call(def_id, &arg_tys, exprs, output, span, &mut made)? {
                // What it holds first, `last()`'s items: after what's made before.
                if !made.is_empty() {
                    self.flush(state, out)?;
                    out.extend(made);
                }
                return Ok(Value::Expr(call));
            }
        }
        // A `From` into an untagged enum, or `into()` to one, which the
        // crate's `from` it resolves to would be: the value (ADR 0214).
        if values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) {
            return Ok(values.into_iter().next().expect("one"));
        }
        // A trait's method: its impl's, a dictionary's, or what rust-js
        // writes itself, `==` of a struct say (ADRs 0049, 0052).
        let known = self.known_trait_call(def_id, generic_args);
        let rust_impl = tcx.trait_of_assoc(def_id).is_some() && self.resolves_to_crate(def_id, generic_args)?;
        // A `&mut` to a number is boxed for the crate's own trait or impl;
        // std's, `write!` to a `String` say, takes its place below.
        let gives_ref = values.iter().any(|v| matches!(v, Value::Ref(_)));
        // Not one that's a binding, a default method's `get ruleId` called
        // through a generic's bound: the binding, below.
        // Nor `BinaryHeap::from(items)`, which std's own lowering heaps; nor
        // std's that's the same JS value, a `Box`'s `as_ref()` (ADR 0331);
        // nor std's `clone_from`, `*dest = source.clone()` written in place,
        // where the crate's impl isn't known to have its own; nor a `fmt` method on a held `Formatter` (ADR 0461).
        if !matches!(known, Some(Std::Any(_) | Std::Same | Std::FmtTo(_)))
            && (rust_impl || known != Some(Std::CloneFrom))
            && !bindings::is_binding(tcx, def_id)
            && let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && (rust_impl
                || (crate::lower::recognition::operational(tcx, self.krate.foreign, trait_id)
                    && (!gives_ref
                        || self.is_rust_trait(trait_id)
                        // One known by its bounds, its dictionary's: `W::write_char(w, c)`
                        // of a generic writer, `T::Buffer`'s `as_mut()` (ADRs 0180, 0533).
                        || (generic_args.types().next()).is_some_and(|t| known_by_bounds(t.peel_refs())))))
        {
            let exprs = self.boxed_args(state, (def_id, generic_args), values.clone(), span, out)?;
            if let Some(call) = self.trait_call(def_id, generic_args, exprs, span, out)? {
                // A generic `&mut T`'s box, the crate's `deref_mut`'s, of an
                // object `T` here: what's in it.
                let call = self.generic_result_values(self.callee(def_id, generic_args), call, span)?;
                return Ok(Value::Expr(call));
            }
        }
        // `cmp::max(a, b)` of what isn't a number:
        // `Ord::max(a, b)`'s (ADR 0136).
        if known.is_none()
            && let Some(max) = crate::lower::recognition::cmp_extreme(tcx, def_id)
        {
            let method = trait_method(tcx, std_item(tcx, StdItem::Ord), if max { "max" } else { "min" });
            let exprs = (values.iter().cloned())
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            if let Some(call) = self.trait_call(method, generic_args, exprs, span, out)? {
                return Ok(Value::Expr(call));
            }
        }
        // `Box<dyn Error>` from an error or a message, by std's `From`, or the
        // `Into` it gives (ADR 0141): `{ value, impl }`.
        if let Some((to, from)) = self.converted(def_id, generic_args)
            && let Some(dictionary) = self.dyn_error_from(to, from, span)?
            && let [value] = &values[..]
        {
            let value = self.value_expr(value.clone(), span)?;
            return Ok(Value::Expr(Expr::object(vec![
                Prop::Field(js::HELD.into(), value),
                Prop::Field(js::DICTIONARY.into(), dictionary),
            ])));
        }
        // A prop `jsx!` isn't given: none, which JSX leaves out (ADR 0213).
        if bindings::is_omitted(tcx, def_id) {
            return Ok(Value::Expr(Expr::undefined()));
        }
        // A binding: the JS function, method, property or operator it names
        // (ADRs 0019, 0020), given its arguments; the registry's symbol's read too (ADR 0634).
        if bindings::is_binding(tcx, def_id) || crate::lower::recognition::is_symbol_for_read(tcx, def_id) {
            return self.mir_binding_call((def_id, generic_args), (values, &arg_tys, nulled), span, out);
        }
        // A function giving each variant its own name gives what it's given,
        // `section.as_str()` is `section` (ADR 0264).
        if values.len() == 1 && self.gives_own_name(def_id) {
            return Ok(values.into_iter().next().expect("one"));
        }
        // The crate's own function, given its dictionaries (ADR 0049).
        if self.is_rust_fn(def_id) && tcx.trait_of_assoc(def_id).is_none() && !bindings::is_binding(tcx, def_id) {
            return self.mir_crate_call(state, (def_id, generic_args), values, &arg_tys, span, out);
        }
        if let Some(known) = known {
            let call = (known, def_id, generic_args);
            return self.mir_std_value(state, call, (&arg_tys, values, output), span, out);
        }
        Err(refuse(self.tcx, span, Refused::Call(def_id)))
    }

    /// `values[at]` held in a variable, which it's left as.
    fn held_value(
        &mut self,
        values: &mut [Value<'tcx>],
        at: usize,
        base: &str,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let value = std::mem::replace(&mut values[at], Value::Expr(Expr::undefined()));
        let value = self.value_expr(value, span)?;
        let value = match value.kind {
            js::ExprKind::Var(_) => value,
            _ => self.spill(base, value, out),
        };
        values[at] = Value::Expr(value.clone());
        Ok(value)
    }

    /// A std call, `mir_std_call`'s, its value as its `output` is held,
    /// `std_value_as`'s.
    fn mir_std_value(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (known, def_id, generic_args): (Std, rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (arg_tys, values, output): (&[Ty<'tcx>], Vec<Value<'tcx>>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let value = self.mir_std_call(state, (known, def_id), generic_args, arg_tys, values, output, span, out)?;
        Ok(self.std_value_as(value, output, arg_tys))
    }

    /// A std call's value as its `output` is held. A std iterator
    /// `mir_std_call` makes an array of, `bytes()`'s: a JS iterator, which
    /// MIR steps; not one that's the string it shows, an escape's, nor a `T`
    /// it gives, `unwrap_or`'s, its value. A write's result a `Result`'s
    /// method makes of a value's, `map` of an `io::Result<usize>`: `undefined`
    /// or its error, `$writeOf(..)` (ADR 0462).
    fn std_value_as(&mut self, value: Value<'tcx>, output: Ty<'tcx>, arg_tys: &[Ty<'tcx>]) -> Value<'tcx> {
        // std's own iterator, of whatever items, `option::IntoIter<I::Item>`.
        let std_iterator = matches!(output.kind(), ty::Adt(adt, _) if crate::lower::recognition::is_std_crate(self.tcx, adt.did()))
            && self.iterator_item(output).is_some();
        match value {
            Value::Expr(e)
                if (self.implements_iterator(output) || std_iterator)
                    && !matches!(output.kind(), ty::Param(_))
                    && !output.is_box()
                    && self.range_kind(output).is_none()
                    && !self.is_user_iterator(output)
                    && !crate::lower::recognition::is_text_escape(self.tcx, output) =>
            {
                Value::Expr(self.std_iterator(e, output))
            }
            Value::Expr(e)
                if self.is_write_result(output)
                    && arg_tys
                        .first()
                        .is_some_and(|&from| self.recognition().is_result(from) && !self.is_write_result(from)) =>
            {
                Value::Expr(Expr::call_helper("$writeOf", vec![e]))
            }
            value => value,
        }
    }

    /// A std function of its arguments' values, lowered once they're values
    /// (`std_values`).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower) fn std_by_values(
        &mut self,
        known: Std,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        arg_tys: &[Ty<'tcx>],
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let boxed = self.option_of(output).is_some_and(|inner| self.boxed_payload(inner));
        let call = crate::lower::calls::Call {
            def_id,
            generic_args,
            tys: arg_tys,
            span,
        };
        self.std_values(known, call, values, boxed, output, out)
    }

    /// A `Vec` the call of `def_id` may shorten, where capacity's kept (ADR
    /// 0419): what it has kept first, as Rust's shortening leaves its
    /// capacity, `$keepCapacity(v)`.
    fn keep_capacity(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        values: &mut [Value<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        if self.krate.keeps_capacity
            && let Some(at) = crate::lower::recognition::shortens_vec(self.tcx, def_id)
            && let Some(vector) = values.get(at).cloned()
        {
            let vector = self.value_expr(vector, span)?;
            self.flush(state, out)?;
            let vector = match vector.reads_same() {
                true => vector,
                false => self.spill("items", vector, out),
            };
            let kept = Expr::call(Expr::helper("$keepCapacity"), vec![vector.clone()]);
            out.push(StmtKind::Expr(kept).at(self.js_span(span)));
            values[at] = Value::Expr(vector);
        }
        Ok(())
    }

    /// A std function rust-js knows, given what MIR made of its arguments.
    #[allow(clippy::too_many_arguments)]
    fn mir_std_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (known, def_id): (Std, rustc_span::def_id::DefId),
        generic_args: ty::GenericArgsRef<'tcx>,
        arg_tys: &[Ty<'tcx>],
        mut values: Vec<Value<'tcx>>,
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let first_ty = || generic_args.types().next();
        let tcx = self.tcx;
        // `format!`, `print!` and `to_string()` of what may fail: in a `try`, a
        // `fmt::Error` std's panic (ADR 0187).
        let fails = match known {
            Std::Format | Std::Print { .. } => matches!(values.first(), Some(Value::Fmt(Std::FmtNew, ..))),
            Std::ToString => first_ty().is_some_and(|ty| self.fmt_may_fail(self.display_trait(), ty)),
            _ => false,
        };
        if fails {
            let exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let value = match known {
                Std::ToString => self.std_by_values(known, def_id, generic_args, arg_tys, exprs, output, span, out)?,
                _ => exprs.into_iter().next().expect("the text"),
            };
            if matches!(known, Std::Print { .. }) {
                self.flush(state, out)?;
            }
            return Ok(Value::Expr(self.failing_values(known, Vec::new(), value, span, out)));
        }
        // One whose `Some` is boxed where it looks like `None` (ADR 0051).
        if self.refuses_boxed_option(known, output) {
            return Err(self.unsupported(span, "this call, for an `Option` of what could look like `None`"));
        }
        // A value with a destructor taken, refused; a chain that owns its items
        // is `mir_iter_call`'s, so none here does.
        self.check_takes_drops(known, def_id, arg_tys, span)?;
        self.keep_capacity(state, def_id, &mut values, span, out)?;
        // An index's: unchecked where the body's ranges show it in bounds,
        // and through `index_mut`, of a value JS can't change in place, the
        // item's place.
        if self.call_in_bounds(known, def_id, &values, span) || self.index_mut_place(known, def_id, output) {
            return self.index_call(state, known, (def_id, output, arg_tys[0]), values, span, out);
        }
        // `m.entry(k).or_insert(v)`: the value put in if the key's missing,
        // `$orInsert`, which is the value, an object; of a number,
        // `*m.entry(k).or_insert(0) += 1`, a handle on the key's value,
        // `$mutGet`'s (ADR 0152). The entry is `$entry(m, k)`, `{ TAG, _0: [m, k] }`.
        if let Std::Map(op @ (MapOp::OrInsert | MapOp::OrInsertWith { .. } | MapOp::OrDefault)) = known {
            return self.or_insert_values(state, op, values, (generic_args, output), span, out);
        }
        // `&mut *s` of a `String`, `as_mut_str()`: a `&mut str` to all of
        // it, the same place the `&mut String` is; a pin's `as_mut()`, its.
        if matches!(known, Std::Same | Std::Pin(crate::lower::std_types::pin::PinOp::Mut))
            && self.is_cell(self.recognition().pinned(output).unwrap_or(output))
            && let [Value::Ref(place)] = values.as_slice()
        {
            return Ok(Value::Ref(place.clone()));
        }
        // A `&mut` it made to an item JS can't change in place: a handle on
        // each (ADR 0152).
        if let Some(cell) = self.makes_items_of(output, generic_args, arg_tys) {
            let exprs = values
                .iter()
                .cloned()
                .map(|v| match v {
                    // A `Box<[T; N]>`'s, through a `&mut`: the array in its place.
                    Value::Ref(place) if known == Std::Pointee => Ok(place),
                    v => self.value_expr(v, span),
                })
                .collect::<R<Vec<_>>>()?;
            let receiver = arg_tys.first().map_or(tcx.types.unit, |t| t.peel_refs());
            if let Some(handles) = self.item_handles_of(known, exprs, receiver, span, out)? {
                return Ok(Value::Expr(match self.implements_iterator(output) {
                    true => self.std_iterator(handles, output),
                    false => handles,
                }));
            }
            // `o.as_mut()` of an `Option` of what a `&mut` to is a cell, a
            // number or a `Box`: a handle on its place where it's `Some`; of
            // a generic `T`'s, on what its `Some` holds (ADR 0051).
            if known == Std::Pointee
                && let Some(item) = self.option_of(receiver)
                && let [Value::Ref(place)] = values.as_slice()
                && fixed_place(place)
            {
                let none = Expr::bin(js::Op::LooseEq, place.clone(), Expr::null());
                let held = self.boxed_payload(item).then(|| self.some_value(place.clone()));
                let some = self.value_expr(Value::Ref(held.unwrap_or_else(|| place.clone())), span)?;
                return Ok(Value::Expr(Expr::cond(none, Expr::undefined(), some)));
            }
            if !known.gives_its_cell() {
                let path = tcx.def_path_str(def_id);
                return Err(self.unsupported(span, &format!("a `{cell}` from `{path}` used as a value, from its MIR")));
            }
        }
        let mut values = values.into_iter();
        // Each kind in turn, which says `None` to what it doesn't know.
        let call = std_calls::StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
        };
        // Each writes apart, the body's once it takes the call: one that says
        // `None` changed nothing written, and took and made nothing (ADR 0713).
        let handlers: [(&str, StdHandler<'a, 'tcx>); 7] = [
            ("mir_fmt_call", Self::mir_fmt_call),
            ("mir_text_call", Self::mir_text_call),
            ("mir_map_call", Self::mir_map_call),
            ("mir_vec_call", Self::mir_vec_call),
            ("mir_iter_std_call", Self::mir_iter_std_call),
            ("mir_place_call", Self::mir_place_call),
            ("mir_value_call", Self::mir_value_call),
        ];
        for (name, handler) in handlers {
            let (args, pending, mut written) = (values.len(), state.pending.len(), Vec::new());
            if let Some(value) = handler(self, state, call, &mut values, &mut written)? {
                out.extend(written);
                return Ok(value);
            }
            let unchanged = written.is_empty() && (values.len(), state.pending.len()) == (args, pending);
            assert!(unchanged, "`{name}` declined a call it changed");
        }
        Ok(Value::Expr(match known {
            // Panics end what runs: what's made before is made first.
            Std::Panic | Std::PanicFmt | Std::PanicDisplay { .. } | Std::BeginPanic => {
                self.flush(state, out)?;
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.std_by_values(known, def_id, generic_args, arg_tys, exprs, output, span, out)?
            }
            _ if lowered_from_values(known) => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.std_by_values(known, def_id, generic_args, arg_tys, exprs, output, span, out)?
            }
            _ => {
                let name = self.tcx.def_path_str(def_id);
                return Err(self.unsupported(span, &format!("`{name}`, from its MIR")));
            }
        }))
    }
}
