//! References to items: what a function, a binding or a std function is
//! in JS, whether it's the crate's, and which impl a call runs (ADRs 0049,
//! 0100).

use super::bindings::{Holder, JsForm, is_binding, is_method, js_form};
use super::fn_def;
use super::recognition::Std;
use super::representation::Num;
use super::std_types::number::NumOp;
use super::std_types::text::TextOp;
use super::{FnCx, R, camel_case};
use crate::js;
use crate::js::{Expr, ExprKind, Op, Pattern, StmtKind};

use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A read of an item `fn_ref` names: a thread-local made where it's
    /// first used, the crate's or a library's, is its call, `START()`
    /// (ADR 0037, amended); anything else, its name.
    pub(super) fn item_read(&self, def_id: DefId) -> Expr {
        let place = self.fn_ref(def_id);
        match self.made_where_used(def_id) {
            true => Expr {
                known: js::Known::LocalRead,
                ..Expr::call(place, Vec::new())
            },
            false => place,
        }
    }

    /// Of a read of a thread-local made where it's first used, `key()`,
    /// `item_read`'s: what it calls, `key`, which `initWith(cell)` makes of
    /// the cell given, its initializer never run, where it isn't made yet
    /// (ADR 0037, amended, #252).
    pub(in crate::lower) fn lazy_key_getter(read: &Expr) -> Option<&Expr> {
        match &read.kind {
            ExprKind::Call(getter, args) if read.known == js::Known::LocalRead && args.is_empty() => Some(getter),
            _ => None,
        }
    }

    /// Whether `def_id` is a thread-local made where it's first used, whose
    /// initializer runs the crate's code or may panic (ADR 0037, amended).
    pub(in crate::lower) fn made_where_used(&self, def_id: DefId) -> bool {
        def_id
            .as_local()
            .is_some_and(|local| self.krate.lazy_locals.contains(&local))
            || self.krate.foreign.item(def_id).is_some_and(|item| item.calling.lazy)
    }

    pub(super) fn fn_ref(&self, def_id: DefId) -> Expr {
        // A library's, imported by the name it chose (ADR 0100).
        if let Some(item) = self.krate.foreign.item(def_id) {
            let export = (item.from.clone(), item.export.clone());
            self.dependencies
                .borrow_mut()
                .package_uses
                .insert((self.module, export.clone()));
            let reference = Expr::var(&self.krate.imports[&self.module][&export]);
            return match &item.member {
                Some(method) => Expr::member(reference, method.clone()),
                None => reference.item(),
            };
        }
        self.dependencies.borrow_mut().uses.push((self.item, def_id));
        // A function a block makes and gives: itself, there (ADR 0296).
        if self.krate.named_expressions.contains(&def_id) {
            return Expr {
                known: js::Known::Nothing,
                kind: js::ExprKind::FunctionHole(def_id.index.as_u32()),
                span: js::Span::NONE,
            };
        }
        let target = &self.krate.fns[&def_id];
        if target.module != self.module {
            self.dependencies.borrow_mut().references.insert((self.module, def_id));
        }
        let export = target.owner.as_ref().unwrap_or(&target.name);
        let reference = if target.module != self.module {
            Expr {
                known: js::Known::Nothing,
                kind: js::ExprKind::Symbol(super::module_symbol(target.module, export)),
                span: js::Span::NONE,
            }
        } else {
            Expr::var(export)
        };
        // A class's constant, its prototype's (ADR 0761).
        if super::classes::member(self.tcx, def_id) == Some(super::classes::Member::Const) {
            Expr::member(Expr::member(reference, "prototype"), target.name.clone())
        } else if target.owner.is_some() {
            Expr::member(reference, target.name.clone())
        } else {
            reference.item()
        }
    }

    /// A JS global (`console.log`) or an explicit package import
    /// (`node:path#posix.join` is `posix.join`, ADR 0028).
    pub(super) fn js_ref(&self, path: &str) -> Expr {
        super::path_read(path, self.krate.required, |export| {
            self.dependencies
                .borrow_mut()
                .package_uses
                .insert((self.module, export.clone()));
            self.krate.imports[&self.module][export].clone()
        })
    }

    /// What a binding's `path` reads where the caller names it, at
    /// `named` (ADR 0633): through the module that holds it, where the
    /// caller names it through that module, that module's export, imported;
    /// else in place, as `js_ref` reads it.
    pub(super) fn binding_ref(&self, path: &str, named: Span) -> Expr {
        let Some(read) = self.krate.held_reads.get(&named) else {
            return self.js_ref(path);
        };
        match &read.holder {
            Holder::Module(module) if *module == self.module => Expr::var(&read.name),
            Holder::Module(module) => {
                let default = read.import == "default";
                (self.dependencies.borrow_mut().held).insert((
                    self.module,
                    *module,
                    read.name.clone(),
                    default,
                    read.binding,
                ));
                Expr {
                    known: js::Known::Nothing,
                    kind: js::ExprKind::Symbol(super::module_symbol(*module, &read.name)),
                    span: js::Span::NONE,
                }
            }
            Holder::Library(from) => {
                let export = (from.clone(), read.import.clone());
                let name = self.krate.imports[&self.module][&export].clone();
                self.dependencies
                    .borrow_mut()
                    .package_uses
                    .insert((self.module, export));
                Expr::var(&name)
            }
        }
    }

    /// A binding as a value, `.map(encode)` (ADR 0039): an arrow of its own
    /// parameters, calling it as a call would, so JS gives it no more than
    /// Rust does. `[..].map(parseInt)` would give `parseInt` each index too.
    pub(super) fn binding_value(
        &mut self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        named: Span,
        span: Span,
    ) -> R<Expr> {
        // One in an `extern` block is refused by its call, `binding_values`,
        // the one place that asks (ADR 0373).
        let inputs = self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, args)
            .skip_normalization()
            .skip_binder()
            .inputs()
            .to_vec();
        let idents = self.tcx.fn_arg_idents(def_id);
        // Named as the binding names them, and `this` after its type:
        // `(signal) => signal.aborted`.
        let params: Vec<String> = inputs
            .iter()
            .enumerate()
            .map(|(i, input)| {
                let name = match idents.get(i).copied().flatten().map(|ident| ident.name.to_string()) {
                    Some(name) if name != "this" && !name.starts_with('_') => camel_case(&name),
                    _ => match input.peel_refs().kind() {
                        ty::Adt(adt, _) => super::lower_first(self.tcx.item_name(adt.did()).as_str()),
                        _ => format!("arg{i}"),
                    },
                };
                self.fresh(&name)
            })
            .collect();
        // Each given as its call gives it (ADR 0604): a nullable one's
        // `None`, `null`.
        let values: Vec<Expr> = (params.iter().zip(&inputs).enumerate())
            .map(|(i, (name, &ty))| self.binding_arg(def_id, (i, false), Expr::var(name), ty))
            .collect();
        let mut made = Vec::new();
        let body = self.binding_values((def_id, args), values, (self.js_span(span), named), span, &mut made)?;
        // One that's a statement, a setter's, is no value.
        if !made.is_empty() {
            let what = format!("`{}` as a value", self.tcx.def_path_str(def_id));
            return Err(self.unsupported(span, &what));
        }
        // Rest arguments as a value, its last, gathered as they're spread,
        // `(...data) => console.log(...data)` (ADR 0667).
        let rest = super::bindings::is_variadic(self.tcx, def_id)
            && inputs
                .last()
                .is_some_and(|&ty| super::bindings::is_rest_value(self.tcx, ty));
        let count = params.len();
        let params = (params.into_iter().enumerate())
            .map(|(i, name)| match rest && i + 1 == count {
                true => Pattern::Rest(name),
                false => name.into(),
            })
            .collect();
        Ok(Expr::arrow(
            params,
            vec![StmtKind::Return(Some(body)).at(self.js_span(span))],
        ))
    }

    /// A binding as a `fn` (ADR 0602): the JS function itself, read where
    /// it's taken, as a Rust `fn` is fixed once taken, `const
    /// localSetImmediate = setImmediate`, where its arrow (`binding_value`)
    /// only passes its parameters on; else that arrow. What hands a `fn` to
    /// JS gives it no more than Rust does, `.map((item) => f(item))`. It's
    /// read where it's `named`, through the module that holds it (ADR 0633).
    pub(super) fn binding_fn_pointer(
        &mut self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        named: Span,
        span: Span,
    ) -> R<Expr> {
        let arrow = self.binding_value(def_id, args, named, span)?;
        let ExprKind::Arrow(params, body) = &arrow.kind else {
            return Ok(arrow);
        };
        // Each parameter, and whether it's rest arguments, spread on.
        let names: Option<Vec<(&str, bool)>> = (params.iter())
            .map(|p| match p {
                Pattern::Name(name) => Some((name.as_str(), false)),
                Pattern::Rest(name) => Some((name.as_str(), true)),
                _ => None,
            })
            .collect();
        let passed = |given: &Expr, (name, rest): (&str, bool)| match (&given.kind, rest) {
            (ExprKind::Var(v), false) => v == name,
            (ExprKind::Spread(spread), true) => matches!(&spread.kind, ExprKind::Var(v) if v == name),
            _ => false,
        };
        if let (Some(names), [stmt]) = (names, body.as_slice())
            && let StmtKind::Return(Some(call)) = &stmt.kind
            && let ExprKind::Call(callee, given) = &call.kind
            && given.len() == names.len()
            && given.iter().zip(names).all(|(g, n)| passed(g, n))
        {
            return Ok((**callee).clone());
        }
        Ok(arrow)
    }

    /// The JS component binding `def_id` names, if it names one.
    pub(super) fn binding_component_of(&self, def_id: DefId) -> Option<Expr> {
        match js_form(self.tcx, def_id) {
            JsForm::Call(name) if is_binding(self.tcx, def_id) && !is_method(self.tcx, def_id) => {
                Some(self.js_ref(&name))
            }
            _ => None,
        }
    }

    /// A function or its type's method object, imported by name when it lives
    /// in another module. The linker resolves collisions after lowering.
    /// Is `def_id` a Rust function, `const` or `static` rust-js compiled, read
    /// by its name: the crate's own, or one a library exports (ADRs 0100,
    /// 0770)?
    pub(super) fn is_rust_fn(&self, def_id: DefId) -> bool {
        self.krate.fns.contains_key(&def_id) || self.krate.foreign.item(def_id).is_some()
    }

    /// Does this body read `def_id`, an item rust-js compiled that's made
    /// where its module loads, a `const` or a numbered enum's variant (ADRs
    /// 0031, 0722), by its name? Each body does but one run where its own
    /// module loads (`CrateFacts::loads`), of another of the crate's
    /// modules: an import cycle may load this one first, before that one
    /// has made it, so it reads the value. A library's is made before the
    /// crate loads, and its own module's before what reads it
    /// (`js::in_load_order`).
    pub(super) fn reads_by_name(&self, def_id: DefId) -> bool {
        let module = self.krate.fns.get(&def_id).map(|info| info.module);
        !self.krate.loads.contains(&self.item) || module.is_none_or(|module| module == self.module)
    }

    /// Is `def_id` a trait's function that a call of runs through the trait:
    /// one of a trait the crate's own impls are called through, or that
    /// resolves to a function rust-js compiled.
    pub(super) fn is_rust_trait_fn(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        self.tcx.trait_of_assoc(def_id).is_some_and(|id| {
            super::traits::operational(self.tcx, self.krate.foreign, id)
                || self
                    .resolve_instance(def_id, args)
                    .ok()
                    .flatten()
                    .is_some_and(|i| self.is_rust_fn(i.def_id()))
        })
    }

    /// `resolve_instance_in` this body's typing environment.
    pub(super) fn resolve_instance(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Result<Option<ty::Instance<'tcx>>, rustc_span::ErrorGuaranteed> {
        resolve_instance_in(self.tcx, self.typing_env, def_id, args)
    }

    /// `Into::<U>::into` of a `T` as `<U as From<T>>::from`, and
    /// `TryInto` as `TryFrom`, if that's a hand-written impl.
    pub(super) fn resolve_into(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
        let (method, args, implementation) = self.recognition().resolve_into(def_id, args)?;
        self.is_rust_fn(implementation).then_some((method, args))
    }

    /// The function a call runs: the crate's impl a trait method resolves to,
    /// or the `From` an `Into` does, else the one named.
    pub(super) fn callee(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> (DefId, ty::GenericArgsRef<'tcx>) {
        self.impl_method(def_id, args)
            .ok()
            .flatten()
            .or_else(|| self.resolve_into(def_id, args))
            .unwrap_or((def_id, args))
    }

    /// A std function taken as a value, `str::trim` in `.map(str::trim)`:
    /// an arrow of one parameter, doing what a call does. `None` for one
    /// that isn't one of these.
    pub(super) fn std_fn_value(&mut self, known: Std, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let Some((def_id, args)) = fn_def(ty) else {
            return Ok(None);
        };
        let js_span = self.js_span(span);
        // `drop`: what dropping its value runs, and `forget`: nothing (ADR
        // 0098). `size_of::<T>` is a call's arrow, `() => 2`, or of a type
        // parameter, what its caller gives, `() => TSize` (ADR 0145).
        match known {
            Std::Drop | Std::Forget => {
                let mut body = Vec::new();
                if known == Std::Drop {
                    self.drop_value(Expr::var("value"), args.type_at(0), span, &mut body)?;
                }
                return Ok(Some(Expr::arrow(vec!["value".into()], body)));
            }
            _ => {}
        }
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, args)
            .skip_normalization()
            .skip_binder();
        let [input] = sig.inputs() else {
            return Ok(None);
        };
        let input = input.peel_refs();
        let name = self.parameter_name(input);
        let x = Expr::var(name);
        let body = match known {
            Std::Trim { start, end } => self.trimmed(x, start, end),
            Std::Method(method) => Expr::call(Expr::member(x, method), vec![]),
            // A counted `Rc`'s `new` or deref (ADR 0320).
            Std::Same if let Some(counted) = self.counted_same(def_id, args, x.clone()) => counted,
            Std::Same | Std::Format => x,
            // A guard's `deref`, std's `Deref` of a `Ref<T>` (ADR 0451).
            Std::GuardValue { mutable } => self.guard_value(x, input, mutable),
            Std::Leak => self.leaked(x, sig.output()),
            Std::IsSome => Expr::bin(Op::LooseNe, x, Expr::null()),
            Std::IsNone => Expr::bin(Op::LooseEq, x, Expr::null()),
            Std::ToBig => Expr::call(Expr::var("BigInt"), vec![x]),
            Std::Cast { .. } => self.number_from(x, input, sig.output(), span)?,
            Std::ToString => self.display_string(x, input, span)?,
            Std::Text(TextOp::Is(regex)) => Expr::call(Expr::member(Expr::regex(regex), "test"), vec![x]),
            // An `f32`'s rounded to one, as its call is, but for those that are
            // exact already (ADR 0122).
            Std::Number(NumOp::Math(function)) => {
                let value = Expr::call(Expr::member(Expr::var("Math"), function), vec![x]);
                match Num::of(input) {
                    Some(num @ Num::F32) if !matches!(function, "floor" | "ceil" | "trunc" | "abs") => num.wrap(value),
                    _ => value,
                }
            }
            // `i32::abs`, wrapped as its call is: `i32::MIN`'s is itself (ADR 0125).
            Std::Number(NumOp::Abs) if let Some(num) = Num::of(input).filter(|n| !n.big()) => {
                num.wrap(Expr::call(Expr::member(Expr::var("Math"), "abs"), vec![x]))
            }
            _ => return Ok(None),
        };
        Ok(Some(Expr::arrow(
            vec![name.into()],
            vec![StmtKind::Return(Some(body)).at(js_span)],
        )))
    }

    /// What an arrow's one parameter of `input` is called: `(s) => ..` of a
    /// string, `c` of a `char`, `n` of a number.
    pub(super) fn parameter_name(&self, input: Ty<'tcx>) -> &'static str {
        if input.is_char() {
            "c"
        } else if self.is_string_like(input) {
            "s"
        } else if Num::of(input).is_some() {
            "n"
        } else {
            "x"
        }
    }

    /// The crate's own impl method a trait method call resolves to, if it
    /// does: in a copied default, `Self` is the impl's type (ADR 0049).
    pub(super) fn impl_method(
        &self,
        id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> R<Option<(DefId, ty::GenericArgsRef<'tcx>)>> {
        let generic_args = self.in_impl_terms(generic_args);
        Ok(self
            .resolve_self_instance(id, generic_args)?
            .filter(|instance| {
                self.is_rust_fn(instance.def_id()) && self.tcx.trait_of_assoc(instance.def_id()).is_none()
            })
            .map(|instance| (instance.def_id(), instance.args)))
    }

    /// Whether a call of `def_id` with `generic_args` runs a function of the
    /// crate's, its impl's or its trait's default: in a copied default as
    /// its impl calls it, `self.next()` of a `Self` that's the crate's
    /// iterator, its own `next` (ADRs 0049, 0492). The one answer, of what's
    /// std's and what's the crate's.
    pub(super) fn resolves_to_crate(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> R<bool> {
        // Not `Iterator`'s of a compiled std iterator, `Peekable`'s `next` or
        // `count`: its struct is a JS iterator, stepped as one, and sized by
        // the hint it carries; but a provided method compiled, `fuse()`'s,
        // of one too (ADR 0561).
        if super::recognition::stepped_as_js_iterator(self.tcx, def_id, self.in_impl_terms(generic_args)) {
            return Ok(false);
        }
        let resolved = self.resolve_self_instance(def_id, self.in_impl_terms(generic_args))?;
        // A std-private trait's method whose impl the caller's type picks,
        // `FuseImpl::next`'s: the impls' methods, compiled (ADR 0561).
        if resolved.is_none()
            && let Some(spec) = super::recognition::fused_specialization(
                self.tcx,
                self.impl_typing_env().unwrap_or(self.typing_env),
                def_id,
                self.in_impl_terms(generic_args),
            )
        {
            return Ok(self.is_rust_fn(spec.selected.0) && self.is_rust_fn(spec.fused));
        }
        // Or it's a std trait's provided method compiled into `@rust-js/std`,
        // `peekable`'s, a function over `Self`: called on a type parameter,
        // which resolves to nothing, or on a type, which resolves to the
        // method itself; never one an impl writes itself (ADRs 0185, 0561).
        Ok(resolved.is_some_and(|i| self.is_rust_fn(i.def_id()))
            || (super::recognition::is_compiled_std(self.tcx, def_id) && resolved.is_none_or(|i| i.def_id() == def_id)))
    }

    /// `ty`, of a function's own generics, in a call of it, `generic_args`.
    pub(super) fn instantiated(&self, ty: Ty<'tcx>, generic_args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
        let ty = ty::EarlyBinder::bind(self.tcx, ty)
            .instantiate(self.tcx, generic_args)
            .skip_normalization();
        self.tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    }
}

/// `Instance::try_resolve` of `def_id` with `args`, normalized first, as it
/// requires, or `None` where they can't be in `typing_env`: an associated
/// type only a caller knows isn't one to resolve with (ADR 0106). rustc's
/// resolution panics on arguments it can't normalize. The one way the
/// front end finds what a call or a constant is, in whichever environment
/// its asker is.
pub(super) fn resolve_instance_in<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Result<Option<ty::Instance<'tcx>>, rustc_span::ErrorGuaranteed> {
    let Ok(args) = tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(args)) else {
        return Ok(None);
    };
    ty::Instance::try_resolve(tcx, typing_env, def_id, args)
}
