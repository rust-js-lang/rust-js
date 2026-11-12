//! A call from MIR: what it calls and its arguments' values, handed to
//! `calls.rs`, `dispatch/known_calls.rs` or `iter.rs`.

use super::*;
use crate::lower::temporary_names::{Base, Role};

/// The forms a function value's arrow takes: generic code's, as generic
/// code that calls it as an `F` gives them (#975), or its callers' (#1324).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Forms {
    Generic,
    Callers,
}

mod known_calls;

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
        // Of a pointer the crate makes of a binding's function, what its
        // parameter takes (ADR 0443, amended).
        let fn_ptr = func.ty(&state.body.local_decls, self.tcx);
        let pointers = self.krate.cell_borrows.pointers();
        let bindings: Vec<rustc_span::def_id::DefId> = (pointers.may_be(self.tcx, fn_ptr).into_iter().flatten())
            .map(|&(f, _)| f)
            .filter(|&f| bindings::is_binding(self.tcx, f))
            .collect();
        if rest {
            crate::lower::calls::spread_rest(&mut exprs);
        }
        // One that may only be Rust functions, the crate's or a library's,
        // which never count their arguments, is called as they are,
        // `dispatcher.useState()` (ADR 0907); one that may be a binding's,
        // or come from JS, is given what JS is (ADR 0330, amended).
        // A with_this function, a `Box<dyn Fn>`, is never one, so it's
        // called through what holds it, `t.g()`, as JS calls it (#404).
        let inputs = sig.skip_binder().inputs();
        let absent = |i, arg: &Expr| crate::lower::calls::absent(self.tcx, inputs, &bindings, i, arg);
        // One that may read `js::arguments()` is given every argument (rd332).
        let counts = (pointers.may_be(self.tcx, fn_ptr).into_iter().flatten()).any(|&(f, _)| self.reads_arguments(f));
        match pointers.rust_only(self.tcx, fn_ptr) {
            _ if counts => crate::lower::calls::given_to_js(&mut exprs),
            true => crate::lower::calls::own_absent(&mut exprs, absent),
            false => crate::lower::calls::given_to_js_but(&mut exprs, absent),
        }
        // One that may be a binding's JS function, called through what holds
        // it, is called with no `this`, which a host's own may refuse (#427).
        let call = Value::Expr(Expr::call(callee, exprs));
        self.called_unbound(state, (call, Called::Pointer(fn_ptr)))
    }

    /// A method std's impl for a `&mut W` passes on to `W`'s
    /// (`passed_through_mut_ref`): `W`'s, given the `&mut W` its receiver
    /// is of, `*self` (#1309). Another call as it is.
    fn passed_through<'b>(
        &self,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        args: &'b [rustc_span::Spanned<Operand<'tcx>>],
    ) -> (
        ty::GenericArgsRef<'tcx>,
        std::borrow::Cow<'b, [rustc_span::Spanned<Operand<'tcx>>]>,
    ) {
        let passed = (self.recognition())
            .passed_through_mut_ref(def_id, generic_args)
            .zip(args.first().and_then(|receiver| receiver.node.place()));
        let Some((inner, receiver)) = passed else {
            return (generic_args, std::borrow::Cow::Borrowed(args));
        };
        let deref = rustc_span::Spanned {
            node: Operand::Copy(self.tcx.mk_place_deref(receiver)),
            span: args[0].span,
        };
        (inner, std::iter::once(deref).chain(args[1..].iter().cloned()).collect())
    }

    /// A method std's impl for a `Box<I>` passes on to `I`'s
    /// (`passed_through_box`): `I`'s arguments, its receiver's type made
    /// `I`'s of the box's (#1278). A `Box<I>` or a `&Box<I>` is `I`'s value;
    /// a `&mut Box<I>` of an iterator is what a `&mut I` is (#1314), given as
    /// it is, where that's the iterator itself; `None` where it's a cell, and
    /// of another call.
    fn passed_through_box(
        &mut self,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        arg_tys: &mut [Ty<'tcx>],
    ) -> Option<ty::GenericArgsRef<'tcx>> {
        let args = self.recognition().passed_through_box(def_id, generic_args)?;
        let receiver_ty = arg_tys.first_mut()?;
        let (boxed, mutability) = match *receiver_ty.kind() {
            ty::Ref(_, boxed, mutability) => (boxed, Some(mutability)),
            _ => (*receiver_ty, None),
        };
        let inner = boxed.boxed_ty()?;
        if mutability == Some(ty::Mutability::Mut) && !self.mut_ref_form(inner).read_is_itself() {
            return None;
        }
        *receiver_ty = match mutability {
            Some(mutability) => Ty::new_ref(self.tcx, self.tcx.lifetimes.re_erased, inner, mutability),
            None => inner,
        };
        Some(args)
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
        let (generic_args, args) = self.passed_through((def_id, generic_args), args);
        let args = &args[..];
        if let Some(defined) = self.mir_defined(state, def_id, args, span, out)? {
            return Ok(Value::Expr(defined));
        }
        self.check_call(state.body, (def_id, generic_args), args, destination, span)?;
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
        // that variable, which may be all that holds it (`called_as_held`).
        let called_as_js = self.called_as_held(def_id, args.len());
        if let Operand::Copy(_) | Operand::Move(_) = func
            && called_as_js
        {
            let mut operands: Vec<&Operand<'tcx>> = vec![func];
            operands.extend(args.iter().map(|a| &a.node));
            let values = self.take_operands(state, &operands, out)?;
            let exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            // Its write's result caught as a call's by name is (#1042).
            let called = crate::lower::calls::called_given_to_js(exprs);
            return Ok(Value::Expr(self.fmt_result_value(def_id, generic_args, called)));
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
        let tys: Vec<Ty<'tcx>> = args.iter().map(|a| a.node.ty(decls, self.tcx)).collect();
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
            state.flattened.locals.insert(destination.local);
            return Ok(Value::Expr(crate::lower::std_types::vec::flattened_read_values(
                known, values,
            )));
        }
        if self.reads_through(state, known, &operands, destination) {
            return Ok(self.take_operands(state, &operands, out)?.remove(0));
        }
        self.mir_call_operands(state, (def_id, generic_args), (&operands, &tys), destination, span, out)
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
            Some(Std::OptionFlatten | Std::OptionCloned) if state.flattened.locals.contains(&read.local) => {
                state.flattened.locals.insert(destination.local);
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
    /// it, its call lowered as one from MIR is, of its parameters, which
    /// take `forms`.
    pub(super) fn mir_called_value(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        span: Span,
        forms: Forms,
    ) -> R<Expr> {
        let tcx = self.tcx;
        let sig = tcx.fn_sig(def_id).instantiate(tcx, args).skip_normalization();
        let sig = tcx.instantiate_bound_regions_with_erased(sig);
        let mut inputs: Vec<Ty<'tcx>> = sig.inputs().to_vec();
        let params: Vec<String> = match inputs.len() {
            // Named by its type: `(s) => ..` of a string.
            1 => vec![self.fresh(self.parameter_name(inputs[0].peel_refs()))],
            n => (0..n)
                .map(|i| {
                    self.fresh(
                        [Role::A, Role::B, Role::C, Role::D, Role::E, Role::F]
                            .get(i)
                            .copied()
                            .unwrap_or(Role::Arg),
                    )
                })
                .collect(),
        };
        // Given in generic code's forms where it's asked them, as generic
        // code that calls it as an `F` gives them: a raw pointer of its type
        // parameter its handle, a `&mut` of one its box, read back as the
        // call takes it, `x.value` of a slice's or an object's (ADRs 0454,
        // 1114). A library's free function's alone: a trait's method's value
        // is its impl's, of the instance's own types, and std's are as their
        // calls are.
        let generic_forms = forms == Forms::Generic
            && tcx.trait_of_assoc(def_id).is_none()
            && self.krate.foreign.item(def_id).is_some();
        let declared = tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let mut values: Vec<Value> = (params.iter().enumerate())
            .map(|(i, p)| {
                let given = (declared.inputs().get(i))
                    .filter(|_| generic_forms)
                    .map(|&of| (def_id, of, self.instantiated(of, args)));
                Value::Expr(
                    match given.and_then(|given| self.form_of(given).map(|form| (form, given))) {
                        Some((form, given)) => self.form_back(form, given, Expr::var(p)),
                        None => Expr::var(p),
                    },
                )
            })
            .collect();
        // A C-variadic one's, given on what it's given past the rest, `(a,
        // ...args) => f(a, ...args)` (ADR 0428): C's numbers and pointers,
        // passed as they are.
        let variadic = crate::lower::recognition::is_c_variadic(tcx, def_id).then(|| self.fresh(Role::Args));
        if let Some(rest) = &variadic {
            values.push(Value::Expr(Expr::spread(Expr::var(rest))));
            inputs.push(tcx.types.i32);
        }
        // What's made for the call is the arrow's, not what's around it. It
        // copies nothing back: its arguments are its parameters' values,
        // never a place, so a box it's given is given on as it is (ADR 0099,
        // amended, #1109).
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
        let value = value?;
        assert!(
            copy_backs.is_empty(),
            "a function value's arrow gives its call its parameters' values, never a place to box and copy back"
        );
        // What it gives back in generic code's form too, a slice's raw
        // pointer `$ptrTo(r)`.
        let output = (def_id, declared.output(), self.instantiated(declared.output(), args));
        // What it gives back in generic code's form too, an object's `&mut`
        // `$cellOf(r)`.
        let value = match self.form_of(output).filter(|_| generic_forms) {
            Some(form) => self.form_given(form, output, value),
            None => value,
        };
        body.push(StmtKind::Return(Some(value)).at(self.js_span(span)));
        let mut patterns: Vec<js::Pattern> = params.into_iter().map(Into::into).collect();
        patterns.extend(variadic.map(js::Pattern::Rest));
        let arrow = Expr::arrow(patterns, body);
        let taker = crate::lower::passed_on::Taker::FnValue;
        Ok(crate::lower::passed_on::passed_on(&arrow, taker, &[]).unwrap_or(arrow))
    }

    /// A call of what `operands` give, each read, an `==` of an `Option`
    /// strict where it can be (`strictly_equal`).
    fn mir_call_operands(
        &mut self,
        state: &mut State<'_, 'tcx>,
        callee: (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (operands, arg_tys): (&[&Operand<'tcx>], &[Ty<'tcx>]),
        destination: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let output = destination.ty(&state.body.local_decls, self.tcx).ty;
        self.namespace_call(state, callee, operands.first().copied(), destination);
        if let Some(given) = self.mir_thrown_given(state, callee, operands, output, span, out)? {
            return Ok(given);
        }
        if let Some(equal) = self.nested_equality(state, callee, operands, span, out)? {
            return Ok(Value::Expr(equal));
        }
        if let Some(value) = self.held_any_of(state, callee.0, operands, span, out)? {
            return Ok(Value::Expr(value));
        }
        if let Some(tested) = self.null_or_value_tested(state, callee, operands, span, out)? {
            return Ok(Value::Expr(tested));
        }
        // A map that's the option itself, `lane.map(EventPriority::from)`:
        // the option alone, its function never named, so one only it names
        // is written nowhere (ADR 0584, amended).
        if self.recognition().classify(callee.0, callee.1) == Some(crate::lower::Std::OptionMap)
            && let [option, _] = operands
            && let [_, f] = arg_tys
            && self.maps_to_itself(*f, (callee.1.type_at(0), callee.1.type_at(1)))
        {
            return Ok(self.take_operands(state, &[option], out)?.remove(0));
        }
        let strict = self.strictly_equal(state, callee.0, operands, span);
        // Of a binding with a nullable parameter, or of what a `Cell` is
        // written, which a nullable field's stores so, each `Option` that's
        // `null` or a value already, which it takes as it is (ADRs 0275, 0772).
        let writes_a_cell = matches!(
            self.recognition().classify(callee.0, callee.1),
            Some(Std::CellSet | Std::CellReplace)
        );
        // An optional nullable cell field's, given what another such holds,
        // stores it as it is, `undefined` where it's absent (ADR 0810,
        // amended).
        let optional = writes_a_cell
            && (operands.first())
                .and_then(|cell| self.borrowed(state, cell))
                .and_then(|cell| self.place_field(state, cell))
                .is_some_and(|field| bindings::is_optional(self.tcx, field));
        let nulled: Vec<bool> = match writes_a_cell || !null_params(self.tcx, callee.0).is_empty() {
            false => Vec::new(),
            true => (operands.iter())
                .map(|o| match self.stored_given(state, o) {
                    crate::lower::aggregates::Given::Nulled => true,
                    crate::lower::aggregates::Given::Optional => optional,
                    _ => false,
                })
                .collect(),
        };
        // A binding's method looked up on a JS value of any shape before its
        // other arguments run, where Rust runs them first: what they take
        // that does something, a getter's read say, made before it (#188).
        if crate::lower::binding_calls::method_may_be_a_getters(self.tcx, callee.0)
            && operands.iter().skip(1).any(|operand| self.acts_pending(state, operand))
        {
            self.flush(state, out)?;
        }
        // A call of the crate's function with an own nullable parameter:
        // what it gives each, which says whether every call gives it `null`
        // or a value (`analysis/nulled_params.rs`, ADR 0861, amended).
        // A parameter of the caller's own given on is so only where that one
        // is, each of its calls giving it so (rd624).
        if self.krate.nulled_params.may_have(callee.0) {
            let caller = (self.item, (state.body.source.def_id(), state.at));
            let given_on: Vec<Option<usize>> = operands.iter().map(|o| self.given_on_param(state, o)).collect();
            let declared: Vec<bool> = (nulled.iter().zip(&given_on))
                .map(|(&n, on)| n || on.is_some())
                .collect();
            self.krate.nulled_params.record(caller, callee.0, &declared, &given_on);
        }
        self.record_bound(state, callee, operands);
        let mut values = self.take_operands(state, operands, out)?;
        // Of another crate's, one its crate's calls each give `null` or a
        // value, which its body tests `!== null`: what may be `undefined`
        // given it `?? null`, as its contract says.
        if !callee.0.is_local() && self.krate.foreign.has_nulled_params(callee.0) {
            use crate::lower::representation::{NoneForm, NonePlace, none_form};
            for (i, value) in values.iter_mut().enumerate() {
                let form = none_form(self.tcx, NonePlace::ForeignParam(self.krate.foreign, callee.0, i));
                if form == NoneForm::Null && nulled.get(i) != Some(&true) {
                    let given = self.value_expr(std::mem::replace(value, Value::Expr(Expr::undefined())), span)?;
                    *value = Value::Expr(form.storage().written(given, false));
                }
            }
        }
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
        if !subject.projection.is_empty() || !state.facts.options().thrown.contains_key(&subject.local) {
            return Ok(None);
        }
        let values = self.take_operands(state, operands, out)?;
        let exprs = (values.into_iter())
            .map(|v| self.value_expr(v, span))
            .collect::<R<Vec<_>>>()?;
        let [error, f]: [Expr; 2] = exprs.try_into().ok().expect("an `Option` and a function");
        let given = self.call_with(f, vec![error], Role::Then, out);
        let boxed = known == Std::OptionMap && self.option_of(output).is_some_and(|item| self.boxed_payload(item));
        Ok(Some(Value::Expr(if boxed { self.some(given) } else { given })))
    }

    /// `a == b` and `a != b` of two `Option`s of `Option`s each held as its
    /// payload (`OptionWrites::nested`, ADR 1017), whose three kinds of
    /// value, `undefined`, `null` and the payload's, are each one of them:
    /// `a === b` of a payload `===` compares as its `==` does, `instance
    /// !== previousInstance`; else `a === undefined || b === undefined ? a
    /// === b : <a == b of their payloads' `Option`s>`. `None` of any other
    /// call.
    fn nested_equality(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        operands: &[&Operand<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let nested = |operand: &Operand<'tcx>| {
            operand
                .place()
                .and_then(|place| place.as_local())
                .is_some_and(|local| state.facts.options().nested.contains(&local))
        };
        // `flatten()` of one `mir_nested` says may be, or of a nested field:
        // what it holds, its `None` `null` as the field's is, `entry.value`
        // (#470).
        if let [x] = operands
            && self.holds_nested_operand(state, x)
            && self.recognition().classify(def_id, generic_args) == Some(Std::OptionFlatten)
        {
            let place = x.place().expect("a place");
            return Ok(Some(match place.projection.is_empty() {
                true => self.nested_operand(state, x, span, out)?,
                false => self.mir_place(state, place, span, out)?,
            }));
        }
        // `is_some()` and `is_none()` of one, through a reference: a test of
        // its `None`, `last !== undefined` (ADR 1017, amended), `null` of a
        // value never `null` (rd645).
        if let [x] = operands
            && nested(x)
            && let Some(known @ (Std::IsSome | Std::IsNone)) = self.recognition().classify(def_id, generic_args)
        {
            let none = self.held_none(x.ty(state.body, self.tcx));
            let x = self.nested_operand(state, x, span, out)?;
            let op = if known == Std::IsSome { Op::Ne } else { Op::Eq };
            return Ok(Some(Expr::bin(op, x, none)));
        }
        let negate = match self
            .tcx
            .trait_of_assoc(def_id)
            .and_then(|t| self.recognition().trait_call(def_id, t))
        {
            Some(TraitCall::Equality { negate }) => negate,
            _ => return Ok(None),
        };
        let [a, b] = operands else { return Ok(None) };
        // Of one with a constant `Some(None)`, `null`: `last === null` (ADR
        // 1017, amended).
        let constant = match (nested(a), nested(b)) {
            (true, false) if self.nested_some_none(state.body, b) => Some(0),
            (false, true) if self.nested_some_none(state.body, a) => Some(1),
            _ => None,
        };
        if let Some(at) = constant {
            let value = (self.take_operands(state, operands, out)?.into_iter().nth(at)).expect("two operands");
            let x = self.value_expr(value, span)?;
            let op = if negate { Op::Ne } else { Op::Eq };
            return Ok(Some(Expr::bin(op, x, Expr::null())));
        }
        if !nested(a) || !nested(b) {
            return Ok(None);
        }
        let ty = a.ty(state.body, self.tcx).peel_refs();
        let payload = self.nested_payload(ty).expect("an `Option` of an `Option`");
        let mut values = self.take_operands(state, operands, out)?.into_iter();
        let mut next = |cx: &mut Self, out: &mut Vec<Stmt>| -> R<Expr> {
            let value = cx.value_expr(values.next().expect("two operands"), span)?;
            Ok(if value.reads_same() {
                value
            } else {
                cx.spill(Role::Option, value, out)
            })
        };
        let (a, b) = (next(self, out)?, next(self, out)?);
        // A `Box` and an uncounted `Rc` are what they point at, as `==`
        // compares them (ADR 0023).
        let mut held = payload;
        while let ty::Adt(_, args) = held.kind()
            && (held.is_box() || self.is_rc(held) && self.counted_rc(held).is_none())
        {
            held = args.type_at(0);
        }
        let equal = if self.js_eq(held).is_some() || self.eq_by_address(held) {
            Expr::bin(Op::Eq, a, b)
        } else {
            let held = Ty::new_option(self.tcx, payload);
            let none = Expr::bin(
                Op::Or,
                Expr::bin(Op::Eq, a.clone(), Expr::undefined()),
                Expr::bin(Op::Eq, b.clone(), Expr::undefined()),
            );
            let both = self.eq_value(a.clone(), b.clone(), held, span, out)?;
            Expr::cond(none, Expr::bin(Op::Eq, a, b), both)
        };
        Ok(Some(match negate {
            true => equal.negated_test(),
            false => equal,
        }))
    }

    /// The payload `operand` holds, through a reference, read once.
    fn nested_operand(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operand: &Operand<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let value = (self.take_operands(state, &[operand], out)?.into_iter().next()).expect("one operand");
        self.value_expr(value, span)
    }

    /// Whether an `==` of an `Option` is `===`: of one side only ever
    /// `Some(..)`, `p == Some(s)`, never `None`, so either answers the same
    /// where `==` is written, of a payload that's its `Some` (`eq_value`),
    /// and `===` is what a person writes (ADR 0030); and one of a value
    /// read as the registry's symbol, itself, which only `===` tells from
    /// what turns into one (ADR 0771).
    fn strictly_equal(
        &self,
        state: &State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        operands: &[&Operand<'tcx>],
        span: Span,
    ) -> bool {
        self.tcx.trait_of_assoc(def_id).is_some_and(|trait_id| {
            matches!(
                self.recognition().trait_call(def_id, trait_id),
                Some(TraitCall::Equality { .. })
            )
        }) && (operands.iter().any(|operand| {
            operand.place().is_some_and(|place| {
                place.projection.is_empty() && state.facts.options().made_some.contains(&place.local)
            })
        }) || self.frame.facts.registered_equalities.contains(&span))
    }

    /// A call, `mir_call_values_given`'s, and of what makes std's struct
    /// where it's no iterator, the JS iterator it is here (ADR 0917).
    fn mir_call_values(
        &mut self,
        state: &mut State<'_, 'tcx>,
        callee: (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        given: (Vec<Value<'tcx>>, &[Ty<'tcx>], &[bool]),
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let made = self.mir_call_values_given(state, callee, given, output, span, out)?;
        Ok(match made {
            // Std's struct a std function made where it's no iterator,
            // `StepBy::new`'s: the JS iterator it is here, where it's one
            // (ADR 0917).
            Value::Expr(made)
                if crate::lower::recognition::makes_no_iterator(self.tcx, callee.0)
                    && crate::lower::recognition::std_struct_steps_in(self.tcx, self.typing_env, output) =>
            {
                let steps = self.std_iterator_steps(output, span)?;
                Value::Expr(Expr::call_helper("$stdIterator", vec![made, steps]))
            }
            made => made,
        })
    }

    /// A call, `mir_call_values_of`'s; of std's, each function or value it's
    /// given by value with a destructor, dropped where std drops it (ADRs
    /// 0469, 0470, 0490): as it returns, or where it's left unused, by its
    /// subject's variant, both held in variables first.
    fn mir_call_values_given(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (mut values, arg_tys, nulled): (Vec<Value<'tcx>>, &[Ty<'tcx>], &[bool]),
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        // Std's, or one lowered as std's iterator method, though it resolves
        // to a body compiled into `@rust-js/std`, a `Rev`'s `rfold` the
        // build that rewrites adapters steps as `$foldBack` (#754).
        let std = self.is_std_call(def_id, generic_args)?
            || (self.tcx.trait_of_assoc(def_id).is_some()
                && self.std_iterator_call(def_id, generic_args, values.first(), span)?);
        let given = match std {
            true => {
                let known = self.recognition().classify(def_id, generic_args);
                self.given_unused(known, (arg_tys, output), Some(def_id), span)?
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
            Some(when) => Some(self.held_value(&mut values, 0, when.subject().into(), span, out)?),
            None => None,
        };
        let mut held = Vec::new();
        // Not a function a by-value consumer moves into its fold, which
        // `$consume` drops before what's left (`consumed_function`, #734).
        let method = self.recognition().iterator_method(def_id);
        for (at, when) in given {
            let consumed = at + 1 == arg_tys.len() && self.consumed_function(method, arg_tys);
            let f = self.held_value(&mut values, at, Role::F.into(), span, out)?;
            if !(when.is_none() && consumed) {
                held.push((when, f, arg_tys[at]));
            }
        }
        let mut call = Vec::new();
        let value = self.mir_call_values_of(
            state,
            (def_id, generic_args),
            (values, arg_tys, nulled),
            output,
            span,
            &mut call,
        )?;
        // What it gives, before what's dropped after it, and as a panic in
        // the call unwinds (#719).
        let (value, unwound) = match value {
            Value::Expr(e) => (
                Value::Expr(self.spill(Role::Value, e, &mut call)),
                self.dropped_unwinding(&held, span)?,
            ),
            value => (value, Vec::new()),
        };
        self.unwinding(call, unwound, self.js_span(span), out);
        self.dropped_given(subject, held, span, out)?;
        Ok(value)
    }

    /// Whether a trait method's call of `def_id` at `span`, of
    /// `generic_args`, is of the crate's impl or a compiled std body's: not
    /// a `rev()`, an `enumerate()`, a `map()` or an `inspect()` made in
    /// place, which is what the recognizers make of it (ADR 0871).
    fn calls_rust_impl(
        &self,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
    ) -> R<bool> {
        Ok(self.tcx.trait_of_assoc(def_id).is_some()
            && self.resolves_to_crate(def_id, generic_args)?
            && !self.adapter_in_place(def_id, span))
    }

    /// Whether a trait method's call of `def_id` at `span`, given
    /// `receiver`, is a std iterator's, a JS iterator's method: not one the
    /// crate's impl has, in a copied default as its impl calls it (ADR
    /// 0492), nor a compiled std body's, but an adapter stepped where it's
    /// made (ADR 0871), as its maker marked it.
    fn std_iterator_call(
        &self,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        receiver: Option<&Value<'tcx>>,
        span: Span,
    ) -> R<bool> {
        let receiver = receiver.and_then(|receiver| match receiver {
            Value::Expr(e) | Value::Place(e) | Value::Ref(e) => Some(e),
            _ => None,
        });
        Ok(!self.resolves_to_crate(def_id, generic_args)?
            || self.adapter_in_place(def_id, span)
            || self.steps_adapter_in_place(def_id, generic_args, receiver))
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
            && (tcx.fn_trait_kind_from_def_id(trait_id).is_some() || self.krate.foreign.compiled_by_rust_js(trait_id))
        {
            return Ok(false);
        }
        Ok(!self.resolves_to_crate(def_id, generic_args)?)
    }

    /// Whether `?` of a `ty`, or what it returns of one, is written as it
    /// is, `Value::Branch`'s: recognition's `tried_as_written`, which the
    /// bodies `@rust-js/std` compiles leave to it too.
    fn tried_as_written(&self, ty: Ty<'tcx>) -> bool {
        crate::lower::recognition::tried_as_written(self.tcx, ty)
    }

    /// A serde_json call (ADR 0083), its `Result`, where it gives one, as
    /// the body uses it (`result_use`, #1545).
    fn mir_json_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operation: crate::lower::recognition::JsonCall<'tcx>,
        values: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let exprs = values
            .into_iter()
            .map(|v| self.value_expr(v, span))
            .collect::<R<Vec<_>>>()?;
        let used = match state.called_into {
            Some(into) => super::result_use::result_use(&self.recognition(), state.body, into),
            None => super::result_use::ResultUse::Inspected,
        };
        Ok(Value::Expr(self.json_call_values(operation, exprs, used, span, out)?))
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
        let mut arg_tys = arg_tys.to_vec();
        let tcx = self.tcx;
        // A method std's impl for a `Box<I>` passes on to `I`'s: `I`'s (#1278).
        if let Some(args) = self.passed_through_box((def_id, generic_args), &mut arg_tys) {
            return self.mir_call_values_of(state, (def_id, args), (values, &arg_tys, nulled), output, span, out);
        }
        // What's refused of a call: the crate's collection or `Borrow` where
        // std's code takes JS's (ADRs 0160, 0167).
        self.check_call_takes(def_id, generic_args, span)?;
        // serde_json's `Value` and what makes one (ADR 0083).
        if let Some(operation) = self.recognition().json_call(def_id, generic_args) {
            return self.mir_json_call(state, operation, values, span, out);
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
        // returns, `FromResidual::from_residual(r)` (ADR 0052), of an
        // `Option`, a `Result` or a `fmt::Result`; of another `Try`, std's
        // generic code's, its dictionary's, as any trait's (ADR 0913).
        if tcx.is_lang_item(def_id, LangItem::TryTraitBranch) && self.tried_as_written(arg_tys[0]) {
            let ty = arg_tys[0];
            // Of one that may be an `Err`: its error thrown on, `fmt_check`
            // (ADR 0187), and what's left never breaks.
            if self.is_write_result(ty) && self.krate.failing.any {
                return self.tried_write(state, values, ty, span, out);
            }
            let tried = self.value_expr(values.into_iter().next().expect("the value tried"), span)?;
            let tried = self.read_once_tried(state, tried, ty, out)?;
            return Ok(Value::Branch(tried, ty));
        }
        if tcx.is_lang_item(def_id, LangItem::TryTraitFromResidual) && self.tried_as_written(arg_tys[0]) {
            let ty = arg_tys[0];
            let residual = self.value_expr(values.into_iter().next().expect("the residual"), span)?;
            if self.is_write_result(output) {
                return Ok(Value::Expr(self.thrown_residual(residual, (ty, output), span)?));
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
            return self.mir_closure_call(values, &arg_tys, None, span, Some((state, output)));
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
        if tcx.trait_of_assoc(def_id).is_some() && self.std_iterator_call(def_id, generic_args, values.first(), span)? {
            let mut exprs = (values.clone().into_iter())
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            // `v.iter().map(f).next_back()`: the map its temporary holds, which
            // nothing else reads, there, `next_back` stepping its source from
            // its end; tidy drops the temporary nothing reads then.
            if let Some(js::ExprKind::Var(name)) = exprs.first().map(|e| &e.kind)
                && let Some((local, _)) = state.locals.names.iter_enumerated().find(|(_, n)| *n == name)
                && state.locals.reads[local] == 1
                && let Some(map) = state.iter_locals.maps.get(&local)
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
        // What a function gives what it's given of is that (ADRs 0264, 0584,
        // amended): `section.as_str()`, `EventPriority::from(lane)`.
        if values.len() == 1 && self.gives_its_argument(self.callee(def_id, generic_args).0) {
            return Ok(values.into_iter().next().expect("one"));
        }
        // A trait's method: its impl's, a dictionary's, or what rust-js
        // writes itself, `==` of a struct say (ADRs 0049, 0052).
        let known = self.recognition().classify(def_id, generic_args);
        let rust_impl = self.calls_rust_impl(def_id, generic_args, span)?;
        // A `&mut` to a number is boxed for the crate's own trait or impl;
        // std's, `write!` to a `String` say, takes its place below.
        let gives_ref = self.gives_a_mut((def_id, generic_args), &values, &arg_tys)?;
        // Not a binding's, a default method's `get ruleId` called through a
        // generic's bound, or a `Duplex`'s `as_ref()` (`is_binding_of`): below.
        // Nor `BinaryHeap::from(items)`, which std's own lowering heaps; nor std's own written in place
        // (`Std::never_through_a_trait_call`); nor std's `clone_from`, `*dest = source.clone()` written in place,
        // where the crate's impl isn't known to have its own.
        if !known.is_some_and(Std::never_through_a_trait_call)
            && (rust_impl || self.std_trait_call(known, def_id))
            && !self.is_binding_of(def_id, generic_args)
            && let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && (rust_impl
                || (crate::lower::recognition::operational(tcx, self.krate.foreign, trait_id)
                    && (!gives_ref
                        || self.krate.foreign.compiled_by_rust_js(trait_id)
                        // One known by its bounds, its dictionary's: `W::write_char(w, c)`
                        // of a generic writer, `T::Buffer`'s `as_mut()` (ADRs 0180, 0533).
                        || (generic_args.types().next()).is_some_and(|t| known_by_a_dictionary(t.peel_refs())))))
        {
            let exprs = self.boxed_args(state, (def_id, generic_args), (values.clone(), &arg_tys), span, out)?;
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
        if let Some(binding) = self.binding_called(def_id, generic_args) {
            return self.mir_binding_call(binding, (values, &arg_tys, nulled), span, out);
        }
        // The crate's own function, given its dictionaries (ADR 0049).
        if self.is_rust_fn(def_id) && tcx.trait_of_assoc(def_id).is_none() && !bindings::is_binding(tcx, def_id) {
            return self.mir_crate_call(state, (def_id, generic_args), values, &arg_tys, span, out);
        }
        if let Some(known) = known {
            let call = (known, def_id, generic_args);
            return self.mir_std_value(state, call, ((&arg_tys, nulled), values, output), span, out);
        }
        Err(refuse(self.tcx, span, Refused::Call(def_id)))
    }

    /// `values[at]` held in a variable, which it's left as.
    fn held_value(
        &mut self,
        values: &mut [Value<'tcx>],
        at: usize,
        base: Base<'_>,
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
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The binding a call of `def_id` is, given `args`: itself, or the
    /// registry's symbol's read (ADR 0634); or, of a std trait's method, the
    /// binding a binding's impl of it declares, an index signature's
    /// `Index::index`, `options[i]` (ADR 1011).
    fn binding_called(
        &self,
        def_id: rustc_span::def_id::DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<(rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>)> {
        let tcx = self.tcx;
        if bindings::is_binding(tcx, def_id) || crate::lower::recognition::is_symbol_for_read(tcx, def_id) {
            return Some((def_id, args));
        }
        self.impl_binding(def_id, args)
    }

    /// Whether a call of `def_id`, given `args`, is a binding's: a binding,
    /// a binding trait's default method, or a std trait's method whose impl
    /// is a binding's (`impl_binding`), not a trait call through a
    /// dictionary (rd588).
    fn is_binding_of(&self, def_id: rustc_span::def_id::DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        bindings::is_binding(self.tcx, def_id) || self.impl_binding(def_id, args).is_some()
    }

    /// The binding a binding's impl of a std trait declares, which a call
    /// of the trait's method `def_id`, given `args`, is: an index
    /// signature's `Index::index` (ADR 1011), a `Duplex`'s `as_ref()`
    /// (rd588).
    fn impl_binding(
        &self,
        def_id: rustc_span::def_id::DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<(rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>)> {
        let tcx = self.tcx;
        tcx.trait_of_assoc(def_id)?;
        let instance = crate::lower::items::resolve_instance_in(tcx, self.typing_env, def_id, args).ok()??;
        match instance.def {
            ty::InstanceKind::Item(item) if bindings::is_binding(tcx, item) => Some((item, instance.args)),
            _ => None,
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether std's impl of `def_id`, `known` as it's recognised, may go by
    /// the trait call: not `clone_from`, written in place where the crate's
    /// impl isn't known to have its own; nor a formatting trait's `fmt`,
    /// whose dictionary's entry gives the value's text, no `Formatter`, so
    /// what no recogniser takes is refused, never that call (#1477).
    fn std_trait_call(&self, known: Option<Std>, def_id: rustc_span::def_id::DefId) -> bool {
        known != Some(Std::CloneFrom) && !crate::lower::recognition::is_fmt_method(self.tcx, def_id)
    }
}
