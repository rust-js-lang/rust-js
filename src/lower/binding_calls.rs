//! A call of a binding, of its arguments' values (ADR 0364): JS's
//! function, method, property or operator it names (ADRs 0019, 0020).

use super::bindings::{JsForm, is_binding, is_method, is_optional_param, is_rest_value, is_variadic, js_form};
use super::calls::{given_to_js, spread_rest};
use super::recognition::StdItem;
use super::{FnCx, R};
use crate::js;
use crate::js::{Expr, Op, StmtKind, UnaryOp};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `def_id`'s call, given `args`, each as JS is given it: `fun_span`,
    /// where its name is written, and `named`, rustc's span of the path
    /// that names it, which says where it's read (ADR 0633).
    pub(in crate::lower) fn binding_values(
        &mut self,
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        mut args: Vec<Expr>,
        (fun_span, named): (js::Span, Span),
        span: Span,
        out: &mut Vec<js::Stmt>,
    ) -> R<Expr> {
        if self.tcx.is_foreign_item(def_id) {
            let refused = super::bindings::extern_fn_refused(self.tcx, def_id);
            return Err(self.tcx.dcx().span_err(span, refused));
        }
        // A type parameter's value as any value is `js::any_of`'s; of a
        // type that's known, its own conversion's, the one way (ADR 0909).
        if super::recognition::is_any_of(self.tcx, def_id) && !ty::TypeVisitableExt::has_param(&generic_args.type_at(0))
        {
            return Err(self.tcx.dcx().span_err(
                span,
                "`js::any_of` is of a type parameter's value, not of a value of a type that's known: \
                 `js::unknown_of(value)` of one that's always there, `value.map(js::unknown_of)` of an `Option`",
            ));
        }
        self.functions_themselves(def_id, generic_args, &mut args, span)?;
        let mut this = is_method(self.tcx, def_id).then(|| args.remove(0));
        // Its last argument is JS's rest arguments (ADRs 0221, 0640): a
        // tuple, `impl js::Rest<T>`'s or an emitter's event's, its items,
        // `()` none; a slice, a `Vec` or a typed array of them, spread, or
        // what an array written out holds.
        if is_variadic(self.tcx, def_id) {
            let sig = self.tcx.fn_sig(def_id);
            let collection = |ty: ty::Ty<'tcx>| {
                let ty = ty.peel_refs();
                ty.is_slice() || self.is_std_type(ty, StdItem::Vec)
            };
            // A slice declared is the one way's old form: a rest parameter
            // is `impl js::Rest<T>`, given a tuple or a slice.
            let declared = (sig
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs()
                .last())
            .copied();
            if declared.is_some_and(collection) {
                return Err(self.tcx.dcx().span_err(
                    span,
                    "a `#[rust_js::variadic]` binding's rest parameter is `impl js::Rest<T>`, \
                     given a tuple of its arguments or a slice or a `Vec` to spread, not a slice",
                ));
            }
            let last = (sig.instantiate(self.tcx, generic_args))
                .skip_normalization()
                .skip_binder()
                .inputs()
                .last()
                .copied();
            let last = last.map(|ty| {
                self.tcx
                    .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
                    .unwrap_or(ty)
            });
            let tuple = last.is_some_and(|ty| matches!(ty.kind(), ty::Tuple(_)));
            // Rest arguments as a value, `&dyn js::Rest<T>`, spread (ADR 0667).
            let rest = last.is_some_and(|ty| is_rest_value(self.tcx, ty));
            // A JS object the js crate makes rest arguments of, a typed
            // array, which is iterable: spread (ADR 0718).
            let object = last.is_some_and(|ty| self.is_js_object(ty.peel_refs()));
            if !last.is_some_and(collection) && !tuple && !rest && !object {
                return Err(self.unsupported(
                    span,
                    "a `#[rust_js::variadic]` binding whose last parameter isn't rest arguments: a tuple, a slice, a `Vec`, a typed array or a `&dyn js::Rest<T>`",
                ));
            }
            spread_rest(&mut args);
        }
        // `function::bind` of a binding's function, its first arguments
        // bound: of its method, its receiver bound first and its `this` the
        // same, the method itself, `performance.measure.bind(performance,
        // name, options)`; and where each of its parameters is bound, a
        // trailing `None` of an optional one left out, as its call leaves it
        // (ADRs 0907, 0443, amended).
        if super::recognition::is_function_bind(self.tcx, def_id)
            && let Some(&ty::FnDef(f, _)) = generic_args.types().next().map(|ty| ty.kind())
            && is_binding(self.tcx, f)
        {
            let mut first = 0;
            if is_method(self.tcx, f)
                && let JsForm::Call(name) = js_form(self.tcx, f)
                && !name.contains('#')
                && let [this_arg, receiver, ..] = args.as_slice()
                && js::same_path(this_arg, receiver)
                && receiver.reads_same()
            {
                let receiver = args.remove(1);
                this = Some(Expr::member(receiver, name).binding());
                first = 1;
            }
            let params = self.tcx.fn_sig(f).skip_binder().skip_binder().inputs().len();
            if first + args.len() - 1 == params {
                while args.len() > 1
                    && args
                        .last()
                        .is_some_and(|last| matches!(last.kind, js::ExprKind::Undefined))
                    && is_optional_param(self.tcx, f, first + args.len() - 2)
                {
                    args.pop();
                }
            }
        }
        // What skips what's falsy is given `test && value` for `test ?
        // value : undefined`: `false` is skipped as `undefined` is, and
        // the test read as one, `(error || ready) && "b"`.
        if super::bindings::skips_falsy(self.tcx, def_id) {
            for arg in &mut args {
                if let js::ExprKind::Cond(test, value, none) = &arg.kind
                    && matches!(none.kind, js::ExprKind::Undefined)
                {
                    if let js::ExprKind::Var(name) = &test.kind {
                        self.note_tested(name);
                    }
                    *arg = Expr::bin(Op::And, test.tested(), (**value).clone());
                } else if let js::ExprKind::Var(name) = &arg.kind {
                    self.note_falsy(name);
                }
            }
        }
        // A trailing `None` of an optional parameter, TypeScript's `x?`, is no
        // argument, as a person leaves one out (ADR 0443): of a variadic
        // binding too, before a rest of none, `console.timeLog()` (ADR
        // 0640). A rest's argument is no optional parameter's, nor a spread,
        // so a `None` before one stays.
        let offset = usize::from(this.is_some());
        while args
            .last()
            .is_some_and(|last| matches!(last.kind, js::ExprKind::Undefined))
            && is_optional_param(self.tcx, def_id, args.len() - 1 + offset)
        {
            args.pop();
        }
        given_to_js(&mut args);
        // A keyed read of a value that may look like `None`, an `Option`
        // itself: `Some(None)` of a key whose value is `undefined` or
        // `null`, apart from `None` of a key it lacks, as JS's read alone
        // couldn't tell (ADRs 0225, 0884).
        let keyed_option = args.len() == 1 && self.gives_boxed_option(def_id, generic_args);
        let value = match (js_form(self.tcx, def_id), this) {
            // A JS `Map`'s or `WeakMap`'s `get(key)`, whose `has(key)`
            // tells the key is there.
            (JsForm::Call(_), Some(this)) if keyed_option && super::bindings::reads_a_key(self.tcx, def_id) => {
                Expr::call(Expr::helper("$someGet"), vec![this, args.remove(0)])
            }
            // A method or a property is on `this`: it can't be an import.
            // Its method, looked up when it's called: of a JS value of any
            // shape, one a getter may answer (ADR 0750).
            (JsForm::Call(name), Some(this)) if !name.contains('#') => {
                let method = Expr::member(this, name).binding();
                let method = match method_may_be_a_getters(self.tcx, def_id) {
                    true => method.answered_by_a_getter(),
                    false => method,
                };
                Expr::call(method.or_at(fun_span), args)
            }
            (JsForm::Call(name), None) => Expr::call(self.binding_ref(&name, named).or_at(fun_span), args),
            (JsForm::New(name), None) => Expr::new_(self.js_ref(&name).or_at(fun_span), args),
            // A constructor its receiver holds, a window's, `new
            // window.ErrorEvent(..)`: each part of its path a member, as a
            // method's is (ADR 0891).
            (JsForm::New(name), Some(this)) if !name.contains('#') => {
                let class = name.split('.').fold(this, Expr::member);
                Expr::new_(class.or_at(fun_span), args)
            }
            // A property's read, which a getter may answer (ADR 0672); not
            // one that runs nothing, a data property's, a function's
            // `prototype` (rd181).
            (JsForm::Get(name), Some(this)) if args.is_empty() && !name.contains('#') => {
                match super::bindings::runs_nothing(self.tcx, def_id) {
                    true => Expr::member(this, name),
                    false => Expr::member(this, name).answered_by_a_getter(),
                }
            }
            // A class's static property, read at each call: `Notification.permission`.
            (JsForm::Get(name), None) if args.is_empty() => self.js_ref(&name).or_at(fun_span).answered_by_a_getter(),
            (JsForm::Set(name), Some(this)) if args.len() == 1 && !name.contains('#') => {
                let value = args.remove(0);
                // A setter may answer it, which may do anything, throw
                // too: never a write that only makes a value (ADR 0660,
                // amended).
                let property = Expr::member(this, name).answered_by_a_setter();
                out.push(StmtKind::Assign(property, value).at(self.js_span(span)));
                Expr::undefined()
            }
            // A property, written, of a global or an import: `process.exitCode
            // = 1`, `EventEmitter.captureRejections = true`. Not an import
            // itself, which JS can't assign.
            (JsForm::Set(name), None)
                if args.len() == 1 && name.rsplit('#').next().is_some_and(|member| member.contains('.')) =>
            {
                let value = args.remove(0);
                let property = self.js_ref(&name).or_at(fun_span).answered_by_a_setter();
                out.push(StmtKind::Assign(property, value).at(self.js_span(span)));
                Expr::undefined()
            }
            (JsForm::This, Some(this)) if args.is_empty() => this,
            // A `Dict`'s, a JSON `null`'s `Some(None)`, which only an own
            // key has.
            (JsForm::GetIndex, Some(this)) if keyed_option => {
                Expr::call(Expr::helper("$dictGet"), vec![this, args.remove(0)])
            }
            (JsForm::GetIndex, Some(this)) if args.len() == 1 => keyed(this, args.remove(0)).answered_by_a_getter(),
            (JsForm::In, Some(this)) if args.len() == 1 => Expr::bin(Op::In, args.remove(0), this),
            (JsForm::Truthy, Some(this)) if args.is_empty() => {
                Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, this))
            }
            // `||` skips its right side where it isn't needed, and Rust
            // evaluates it either way: one with effects runs first, in
            // order, as `Option::unwrap_or`'s `??` does (rd181).
            (JsForm::Or, Some(mut this)) if args.len() == 1 => {
                let mut other = args.remove(0);
                if other.has_effects() {
                    if this.has_effects() {
                        this = self.spill("value", this, out);
                    }
                    other = self.spill("fallback", other, out);
                }
                Expr::bin(Op::Or, this, other)
            }
            (JsForm::StrictEquals, Some(this)) if args.len() == 1 => Expr::bin(Op::Eq, this, args.remove(0)),
            (JsForm::LooseEquals, Some(this)) if args.len() == 1 => Expr::bin(Op::LooseEq, this, args.remove(0)),
            (JsForm::InstanceOfGiven, Some(this)) if args.len() == 1 => Expr::bin(Op::InstanceOf, this, args.remove(0)),
            (JsForm::TypeOf, Some(this)) if args.is_empty() => Expr::unary(UnaryOp::Typeof, this),
            (JsForm::Text, Some(this)) if args.is_empty() => Expr::bin(Op::Add, Expr::str(""), this),
            (JsForm::SetIndex, Some(this)) if args.len() == 2 => {
                let (key, value) = (args.remove(0), args.remove(0));
                out.push(StmtKind::Assign(keyed(this, key), value).at(self.js_span(span)));
                Expr::undefined()
            }
            (JsForm::CallThis, Some(this)) => Expr::call(this, args),
            (JsForm::KeysIn, Some(this)) if args.is_empty() => super::iterators::keys_in(this),
            (JsForm::NewThis, Some(this)) => Expr::new_(this, args),
            (JsForm::Null, None) if args.is_empty() => Expr::null(),
            // A static method of the class `this` is, called on it,
            // `localPromise.resolve(null)`: the static's own name, of its
            // binding, `Promise.resolve`'s (rd235). Its own value, the
            // first argument, is no argument JS is given.
            (JsForm::Static, Some(this)) if !args.is_empty() => {
                let Some(name) = static_member(self.tcx, generic_args) else {
                    let what = "`Class::call_static` of what isn't a binding's static method, `js::promise::resolve`,";
                    return Err(self.unsupported(span, what));
                };
                args.remove(0);
                Expr::call(Expr::member(this, name).or_at(fun_span), args)
            }
            (JsForm::Class, None) if args.is_empty() => {
                let ty = generic_args.types().next();
                let constructor = ty.and_then(|ty| match ty.kind() {
                    ty::Adt(adt, _) => super::classes::constructor_of(self.tcx, adt.did()),
                    _ => None,
                });
                // The crate's class, its constructor function; or a
                // binding's, the global class TypeScript names (ADR 0911).
                match (constructor, ty.and_then(|ty| self.recognition().class_value(ty))) {
                    (Some(constructor), _) => match self.fn_ref(constructor).kind {
                        js::ExprKind::Member(class, _) => *class,
                        _ => unreachable!("a class's constructor is a member of its class"),
                    },
                    (None, Some(class)) => self.js_ref(&class),
                    (None, None) => {
                        let what = "`js::class::<T>()` of a `T` that's no `#[rust_js::class]` struct, nor a class TypeScript names, `#[rust_js::types = \"AbortController\"]`";
                        return Err(self.unsupported(span, what));
                    }
                }
            }
            // The value where it's an object or an array, as one value:
            // `typeof value === "object" && value !== null ? value :
            // undefined`, which an `if let` of it tests (ADR 0910).
            (JsForm::TypeofObject, Some(this)) if args.is_empty() => {
                let sig = self
                    .tcx
                    .fn_sig(def_id)
                    .instantiate(self.tcx, generic_args)
                    .skip_normalization();
                let kind = sig.skip_binder().inputs()[0];
                let this = match this.reads_same() {
                    true => this,
                    false => self.spill("kind", this, out),
                };
                let test = self.object_test(kind, &this);
                Expr::cond(test, this.defined(), Expr::undefined())
            }
            (JsForm::Throw, Some(this)) if args.is_empty() => {
                out.push(StmtKind::Throw(this).at(self.js_span(span)));
                Expr::undefined()
            }
            (JsForm::Delete, Some(this)) if args.len() == 1 => {
                Expr::unary(UnaryOp::Delete, keyed(this, args.remove(0)))
            }
            (JsForm::WithThis, Some(closure)) if args.is_empty() => {
                self.function_with_this(closure, generic_args.types().nth(1), span)?
            }
            (JsForm::InstanceOf(class), Some(this)) if args.is_empty() => {
                Expr::bin(Op::InstanceOf, this, self.js_ref(&class))
            }
            _ => {
                let what = format!(
                    "the `#[link_name]` of `{}` with this signature",
                    self.tcx.def_path_str(def_id)
                );
                return Err(self.unsupported(span, &what));
            }
        };
        // A call that runs none of the crate's code, `set.has(fiber)` of a
        // weak set's, sets none of its variables (ADR 0277, amended).
        let value = match value.kind {
            js::ExprKind::Call(..) | js::ExprKind::New(..)
                if super::analysis::binding_call_runs_nothing(&self.recognition(), def_id, generic_args) =>
            {
                value.runs_nothing()
            }
            _ => value,
        };
        let value = match value.kind {
            js::ExprKind::Call(..) if super::bindings::iterates_receiver(self.tcx, def_id) => value.receiver_iterated(),
            _ => value,
        };
        Ok(self.catching(def_id, generic_args, value))
    }

    /// Does binding `def_id`, of `generic_args`, give an `Option` of a
    /// value that may look like `None`, whose `Some` is boxed (ADR 0051)?
    fn gives_boxed_option(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        let output = self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, generic_args)
            .skip_binder()
            .output();
        self.option_of(output).is_some_and(|value| self.boxed_payload(value))
    }

    /// `js::import!(f)` of the function `id`: its module's export, a
    /// binding's or the crate's, imported when it's run, or the module.
    pub(super) fn dynamic_import_of(&mut self, id: DefId, module: bool, span: Span) -> R<Expr> {
        let tcx = self.tcx;
        // What holds it is called with its slice whole, where it takes
        // JS's rest arguments (ADR 0664).
        if self.takes_rest(id) {
            return Err(self.unsupported(span, "importing a function whose last parameter is `#[rust_js::rest]`"));
        }
        let (from, export) = if is_binding(tcx, id) {
            let path = super::bindings::js_path(tcx, id).unwrap_or_default();
            match super::bindings::js_import(&path) {
                Some(((from, export), "")) => (js::ImportFrom::Specifier(from), export),
                _ => return Err(self.unsupported(span, "importing a binding that isn't a JS module's export")),
            }
        } else if let Some(info) = id.as_local().and(self.krate.fns.get(&id))
            && info.owner.is_none()
            && tcx.visibility(id).is_public()
        {
            let export = super::default_exports::exported_as(tcx, id, info.exported());
            (js::ImportFrom::Module(super::module_path(tcx, info.module)), export)
        } else {
            return Err(self.unsupported(
                span,
                "importing what isn't a pub function of the crate's, or a binding's",
            ));
        };
        if module {
            if export != "default" {
                return Err(self.unsupported(span, "importing a module by what isn't its default export"));
            }
            return Ok(Expr::import(from));
        }
        let m = "m".to_string();
        let read = Expr::arrow(
            vec![m.clone().into()],
            vec![StmtKind::Return(Some(Expr::member(Expr::var(&m), export))).at(js::Span::NONE)],
        );
        Ok(Expr::call(Expr::member(Expr::import(from), "then"), vec![read]))
    }
}

/// `this[key]`, or `this.name` of a key written that's a name, as a person
/// writes it (ADR 0225): `js::get(value, "name")` is `value.name`.
pub(super) fn keyed(this: Expr, key: Expr) -> Expr {
    match &key.kind {
        js::ExprKind::Str(name)
            if name.starts_with(|c: char| c.is_alphabetic() || c == '_' || c == '$')
                && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') =>
        {
            Expr::member(this, name.clone())
        }
        _ => Expr::index(this, key),
    }
}

/// Whether a call of binding `def_id` looks its method up on a JS value of
/// any shape, `js::Unknown`, `obj.send(a, b)`: once the receiver is read,
/// before its other arguments run, where Rust runs them first, and a
/// getter may answer the lookup, or an argument replace the method (#187,
/// #188). A receiver of a declared type, webapi's `Element` say, has its
/// class's method, which nothing the program does changes (ADR 0750). The
/// one answer, of the lookup's `js::Known` and of what's read past it.
pub(in crate::lower) fn method_may_be_a_getters(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let receiver = || {
        let inputs = tcx.fn_sig(def_id).instantiate_identity().skip_normalization();
        inputs.skip_binder().inputs().first().copied()
    };
    is_binding(tcx, def_id)
        && is_method(tcx, def_id)
        && matches!(js_form(tcx, def_id), JsForm::Call(name) if !name.contains('#'))
        && receiver().is_some_and(
            |ty| matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if super::declarations::any_shape(tcx, adt.did())),
        )
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Each of `args` that's a binding's function given as the value it
    /// is, to a parameter of a function of any shape, `F: js::AnyFunction<M>`
    /// or `js::AnyShape<M>`, which JS calls with what Rust gives it alone, or
    /// not at all: its JS function itself, as a `fn` of it is (ADR 0602),
    /// where its arrow only passes its parameters on.
    /// `function::bind(console::time_stamp, ..)` binds `console.timeStamp`,
    /// and `function::name(&console::time_stamp)` is its name (ADR 0907,
    /// amended). A closure the program writes stays the function it is.
    fn functions_themselves(
        &mut self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        args: &mut [Expr],
        span: Span,
    ) -> R<()> {
        let tcx = self.tcx;
        let sig = tcx.fn_sig(def_id);
        let declared = sig
            .instantiate_identity()
            .skip_normalization()
            .skip_binder()
            .inputs()
            .to_vec();
        let given = sig
            .instantiate(tcx, generic_args)
            .skip_normalization()
            .skip_binder()
            .inputs()
            .to_vec();
        let of_any_shape = |param: ty::Ty<'tcx>| {
            (tcx.clauses_of(def_id).instantiate_identity(tcx).into_iter()).any(|(clause, _)| {
                matches!(clause.skip_normalization().kind().skip_binder(), ty::ClauseKind::Trait(predicate)
                    if predicate.self_ty() == param
                        && super::recognition::is_function_value_trait(tcx, predicate.def_id()))
            })
        };
        for ((arg, declared), given) in args.iter_mut().zip(declared).zip(given) {
            let binding_fn = match *given.peel_refs().kind() {
                ty::FnDef(f, f_args) if super::bindings::is_binding(tcx, f) => Some((f, f_args)),
                _ => None,
            };
            if let Some((f, f_args)) = binding_fn
                && matches!(declared.peel_refs().kind(), ty::Param(_))
                && of_any_shape(declared.peel_refs())
            {
                let itself = self.binding_fn_pointer(f, f_args.skip_binder(), span, span)?;
                *arg = itself;
            }
        }
        Ok(())
    }

    /// `js::function::with_this(closure)`: the closure, an arrow of its
    /// `this` and of the tuple of its parameters, `parameters`, as the JS
    /// function it makes, `function (a, b) { .. }`, each read of its `this`
    /// JS's (ADR 0972). Its tuple's items are the function's parameters,
    /// `(this$, [a, b]) => ..`'s `a` and `b`; a tuple it reads whole, the
    /// array of them. A function nested in it reads its `this` through
    /// `const this$ = this;`, as one that sets it, `let this$ = this;`.
    /// Anonymous: preparation names it where it's stored (ADR 0972).
    fn function_with_this(&mut self, closure: Expr, parameters: Option<ty::Ty<'tcx>>, span: Span) -> R<Expr> {
        let refuse =
            |cx: &Self, why: &str| Err(cx.tcx.dcx().span_err(span, format!("`js::function::with_this` {why}")));
        let js::ExprKind::Arrow(mut params, mut body) = closure.kind else {
            return refuse(
                self,
                "takes a closure written in its call, `with_this(move |this, (a, b)| ..)`, not a variable that holds one",
            );
        };
        let items = match parameters.map(|ty| ty.kind()) {
            Some(ty::Tuple(items)) => *items,
            _ => return refuse(self, "is given a closure of its `this` and a tuple of its parameters"),
        };
        let arity = items.len();
        // Rest arguments as a value, its last, are JS's rest parameter,
        // `function (first, ...rest)`, as a closure's are (ADR 0851): what
        // its type says each call gives spread. One before its last is
        // refused of its function's type already.
        let gathers = (items.last()).is_some_and(|&ty| super::bindings::is_rest_value(self.tcx, ty));
        // A `this` of its maker's read already, which the function's own
        // would be: none is, as a class's `self` is its `this` only once
        // its method is lowered, through each function nested in it.
        if js::reads_this(&body) {
            return Err(self.unsupported(
                span,
                "`js::function::with_this` of a closure that reads its maker's `this`",
            ));
        }
        let tuple = (params.len() > 1).then(|| params.remove(1));
        let this = (!params.is_empty()).then(|| params.remove(0));
        let mut own = Vec::new();
        match tuple {
            Some(js::Pattern::Array(items)) => {
                for item in items {
                    own.push(match item {
                        Some(pattern) => pattern,
                        None => js::Pattern::Name(self.fresh("_")),
                    });
                }
            }
            Some(js::Pattern::Name(tuple)) => {
                let items: Vec<String> = (0..arity).map(|_| self.fresh(&tuple)).collect();
                let whole = match arity {
                    0 => Expr::undefined(),
                    _ => Expr::array(items.iter().map(|item| Expr::var(item)).collect()),
                };
                body.insert(0, StmtKind::Const(tuple, whole).at(js::Span::NONE));
                own.extend(items.into_iter().map(js::Pattern::Name));
            }
            Some(_) => return refuse(self, "is given a closure whose parameters aren't a tuple"),
            None => {}
        }
        if gathers && let Some(js::Pattern::Name(last)) = own.last() {
            let last = last.clone();
            *own.last_mut().expect("its last") = js::Pattern::Rest(last);
        }
        match this {
            Some(js::Pattern::Name(this)) => {
                // Set, or a place lent: a variable of its own, `this` given first.
                if js::Writes::of(&body).may_replace(&this) {
                    body.insert(0, StmtKind::Let(this, Some(Expr::this())).at(js::Span::NONE));
                } else if js::read_as_this(&mut body, &this) {
                    body.insert(0, StmtKind::Const(this, Expr::this()).at(js::Span::NONE));
                }
            }
            Some(_) => return refuse(self, "is given a closure whose `this` is a pattern, not a name"),
            None => {}
        }
        let function = js::Function {
            name: String::new(),
            params: own,
            body,
            export: false,
            is_async: false,
            noinline: false,
            span: self.js_span(span),
            name_span: js::Span::NONE,
            param_spans: Vec::new(),
        };
        Ok(Expr {
            facts: js::Facts::NONE,
            kind: js::ExprKind::Function(Box::new(function)),
            span: closure.span,
        })
    }
}

/// The member a static's binding names of its class, `resolve` of
/// `Promise.resolve`, the function `Class::call_static` is given: a
/// binding's, no method's, of a class's member.
fn static_member<'tcx>(tcx: TyCtxt<'tcx>, generic_args: ty::GenericArgsRef<'tcx>) -> Option<String> {
    let f = generic_args.types().find_map(|ty| match *ty.kind() {
        ty::FnDef(f, _) => Some(f),
        _ => None,
    })?;
    if !is_binding(tcx, f) || is_method(tcx, f) {
        return None;
    }
    match js_form(tcx, f) {
        JsForm::Call(name) if !name.contains('#') => {
            let (class, member) = name.rsplit_once('.')?;
            (!class.is_empty() && crate::names::is_identifier(member)).then(|| member.to_string())
        }
        _ => None,
    }
}
