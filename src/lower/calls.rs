//! Calls to local functions, JavaScript bindings, closures and standard operations.

use super::bindings::{JsForm, is_binding, is_method, is_omitted, is_variadic, js_form, nullable_params};
use super::combinators::Comb;
use super::combinators::StepOp;
use super::display::append_written;
use super::drops::Drops;
use super::fn_def;
use super::recognition::{
    Catching, FmtResultAnswer, Std, StdItem, StreamOp, TypeFact, fmt_result_answer, is_std_def, std_item, trait_method,
};
use super::std_types::lazy::LazyOp;
use super::std_types::number::NumOp;
use super::std_types::once::OnceOp;
use super::{Dest, FnCx, R};
use crate::js;
use crate::js::{Expr, Op, Prop, Stmt, StmtKind, UnaryOp};
use crate::runtime::Helper;
use rustc_ast::{LitKind, Mutability};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::find_attr;
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::format_args::without_newline;

/// A call being lowered: what's called, which function that is, its
/// arguments, and whether its value is used.
#[derive(Clone, Copy)]
pub(super) struct Call<'c, 'tcx> {
    pub(super) fun: ExprId,
    pub(super) def_id: DefId,
    pub(super) generic_args: ty::GenericArgsRef<'tcx>,
    pub(super) args: &'c [ExprId],
    pub(super) discarded: bool,
    pub(super) span: Span,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call to one of our functions (local or imported by name),
    /// to JS (ADR 0021), or to one of the std functions rust-js knows (ADR 0023).
    pub(super) fn call(
        &mut self,
        fun: ExprId,
        args: &[ExprId],
        discarded: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let fun_span = self.js_span(self.thir[fun].span);
        let f = &self.thir[self.strip(fun)];
        let (ExprKind::ZstLiteral { .. }, Some((def_id, generic_args))) = (&f.kind, fn_def(f.ty)) else {
            if matches!(f.ty.kind(), ty::FnDef(..) | ty::FnPtr(..)) {
                let mut operands = vec![fun];
                operands.extend_from_slice(args);
                let mut values = self.operands(&operands, out)?;
                let callee = values.remove(0);
                return Ok(Expr::call(callee, values));
            }
            return Err(self.unsupported(f.span, "calling this"));
        };
        // serde_json's `Value` and what makes one (ADR 0083).
        if let Some(value) = self.json_call(def_id, generic_args, args, span, out)? {
            return Ok(value);
        }
        // `x.into()` is the `From::from(x)` it calls, when that's the crate's
        // own (ADR 0052).
        let (def_id, generic_args) = self
            .resolve_into(def_id, generic_args)
            .unwrap_or((def_id, generic_args));
        if let Some(collection) = self.collection_as_iterable(def_id, generic_args) {
            let what = format!("a `{collection}`, whose `IntoIterator` is the crate's, where any `IntoIterator` goes");
            return Err(self.unsupported(span, &what));
        }
        if let Some(ty) = self.borrowed_by_user(def_id, generic_args)? {
            let path = self.tcx.def_path_str(def_id);
            return Err(self.unsupported(span, &format!("`{path}` of a `{ty}` as what its own `Borrow` gives")));
        }
        let call = Call {
            fun,
            def_id,
            generic_args,
            args,
            discarded,
            span,
        };
        if let Some(value) = self.special_call(call, fun_span, out)? {
            return Ok(value);
        }
        let Some(known) = self.std_fn(fun) else {
            let path = self.tcx.def_path_str(def_id);
            // A library's, that its manifest doesn't list (ADR 0100).
            if let Some(why) = self.krate.foreign.unlisted(def_id) {
                return Err(self.tcx.dcx().span_err(self.thir[fun].span, why));
            }
            return Err(self.unsupported(self.thir[fun].span, &format!("calling `{path}`")));
        };
        self.std_call(known, call, out)
    }

    /// `import(spec).then((m) => m.item)` of `js::import!(item)`, `item` a
    /// function of the crate's, pub, or a binding's of a JS module; and
    /// `import(spec)` of `js::import_module!(item)`, whose module's default
    /// export it is (ADR 0304). It's read nowhere else, so nothing imports
    /// its module statically.
    fn dynamic_import(&mut self, item: ExprId, module: bool, span: Span) -> R<Expr> {
        let tcx = self.tcx;
        let e = &self.thir[self.strip(item)];
        let Some((id, _)) = fn_def(e.ty).filter(|_| matches!(e.kind, ExprKind::ZstLiteral { .. })) else {
            return Err(self.unsupported(span, "importing what isn't a function"));
        };
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
            let default = super::bindings::default_exports(tcx, info.module)
                .iter()
                .any(|&(exported, _)| exported == Some(id));
            let export = if default {
                "default".to_string()
            } else {
                info.name.clone()
            };
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

    /// A type whose `Borrow` is the crate's that std's function would take
    /// as the value itself, for what it borrows as (ADR 0167): not the
    /// crate's own `borrow()` or `borrow_mut()`, which is the crate's.
    fn borrowed_by_user(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> R<Option<Ty<'tcx>>> {
        let crates = self
            .resolve_instance(def_id, generic_args)?
            .is_some_and(|instance| self.is_rust_fn(instance.def_id()));
        Ok(match self.is_rust_fn(def_id) || crates {
            true => None,
            false => self.recognition().borrowed_by_user(def_id, generic_args),
        })
    }

    /// A collection of the crate's that a call takes as any `IntoIterator`:
    /// generic code, std's or the crate's, iterates what it's given as JS
    /// does, which never calls the collection's own `into_iter` (ADR 0160).
    /// Not `into_iter` itself, which is the collection's.
    fn collection_as_iterable(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> Option<Ty<'tcx>> {
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        if self.tcx.trait_of_assoc(def_id) == Some(into_iterator) {
            return None;
        }
        let clauses = self.tcx.clauses_of(def_id).instantiate(self.tcx, generic_args);
        clauses.clauses.iter().find_map(|clause| {
            // A bound of any lifetime, `for<'a> &'a T: IntoIterator`, is of an
            // erased one: a type rustc can select an impl for.
            let bound = self
                .tcx
                .instantiate_bound_regions_with_erased(clause.skip_normalization().as_trait_clause()?)
                .trait_ref;
            (bound.def_id == into_iterator)
                .then(|| bound.self_ty())
                .filter(|&ty| self.user_into_iter(ty).is_some())
        })
    }

    /// A call of what isn't one of std's functions: the crate's own, a
    /// binding's (ADR 0021), a closure's, a trait method's, and what rust-js
    /// writes its own way. `None` if it's std's.
    fn special_call(&mut self, call: Call<'_, 'tcx>, fun_span: js::Span, out: &mut Vec<Stmt>) -> R<Option<Expr>> {
        let Call {
            fun,
            def_id,
            generic_args,
            args,
            discarded,
            span,
        } = call;
        // A function giving each variant its own name gives what it's given,
        // `section.as_str()` is `section` (ADR 0264).
        if let [arg] = args
            && self.gives_own_name(def_id)
        {
            return Ok(Some(self.expr(*arg, out)?));
        }
        // A prop `jsx!` isn't given: none, which JSX leaves out (ADR 0213).
        if is_omitted(self.tcx, def_id) {
            return Ok(Some(Expr::undefined()));
        }
        // A `From` into an untagged enum, or `into()` to one: the value (ADR 0214).
        if let [value] = args
            && self.recognition().converts_to_untagged(def_id, generic_args)
        {
            return self.expr(*value, out).map(Some);
        }
        if let Some(callee) = self.boxed_callee(def_id, generic_args, args, span)? {
            return self.call_with_boxes(callee, args, discarded, span, out).map(Some);
        }
        // `x == &mut 1` or `p < q` of `&mut`s to values JS can't change in
        // place: of what they point at, whatever each `&mut` is (ADR 0099).
        if let Some(trait_id) = self.tcx.trait_of_assoc(def_id)
            && (self.tcx.is_lang_item(trait_id, LangItem::PartialEq)
                || self.tcx.is_lang_item(trait_id, LangItem::PartialOrd)
                || is_std_def(self.tcx, trait_id, StdItem::Ord))
            && generic_args.types().next().is_some_and(|t| self.is_cell(t))
        {
            let pointees = self
                .tcx
                .mk_args_from_iter(generic_args.iter().map(|arg| match arg.as_type() {
                    Some(t) if self.is_cell(t) => t.builtin_deref(true).unwrap_or(t).into(),
                    _ => arg,
                }));
            let values = args
                .iter()
                .map(|&a| self.pointee_value(a, span, out))
                .collect::<R<Vec<_>>>()?;
            if let Some(compared) = self.trait_call(def_id, pointees, values.clone(), span, out)? {
                return Ok(Some(compared));
            }
            // `p < q` of numbers: the operator, as of `&i32`s.
            if let Some(Std::Operator(op)) = self.std_fn(fun)
                && let [left, right] =
                    <[Expr; 2]>::try_from(values).map_err(|_| self.unsupported(span, "comparing `&mut`s"))?
            {
                let ty = pointees.types().next().expect("a comparison has a type");
                return self.binary(op, left, right, None, ty, span).map(Some);
            }
            return Err(self.unsupported(span, "comparing `&mut`s"));
        }
        // `Box<dyn Error>` from an error or a message, by std's `From`, or
        // the `Into` it gives (ADR 0141): `{ value, impl }`.
        if let Some((to, from)) = self.converted(def_id, generic_args)
            && let Some(dictionary) = self.dyn_error_from(to, from, span)?
        {
            let value = self.expr(args[0], out)?;
            return Ok(Some(Expr::object(vec![
                Prop::Field("value".into(), value),
                Prop::Field("impl".into(), dictionary),
            ])));
        }
        if let Some(written) = self.write_call(def_id, generic_args, args, span, out)? {
            return Ok(Some(written));
        }
        if let Some(written) = self.debug_builder(def_id, args, span, out)? {
            return Ok(Some(written));
        }
        if self.is_rust_fn(def_id) && self.tcx.trait_of_assoc(def_id).is_none() {
            let callee = self.fn_ref(def_id);
            let mut values = self.operands(args, out)?;
            // An iterator of the crate's own, given where a generic one goes,
            // is a JS iterator (ADR 0061).
            let inputs = self
                .tcx
                .fn_sig(def_id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs()
                .to_vec();
            for ((value, &arg), input) in values.iter_mut().zip(args).zip(inputs) {
                let taken = std::mem::replace(value, Expr::undefined());
                *value = self.iterator_arg((def_id, input), (arg, taken), self.thir[arg].ty, span, out)?;
            }
            let mut args = values;
            args.extend(self.evidence_args(def_id, generic_args, span)?);
            let called = Expr::call(callee.or_at(fun_span), args);
            return Ok(Some(self.fmt_result_value(def_id, generic_args, called)));
        }
        if is_binding(self.tcx, def_id) {
            // An `#[eii]` function is declared in an `extern` block too, but
            // it's Rust's, linked to its implementation, not JS's.
            if find_attr!(self.tcx, def_id, RustcEiiForeignItem) {
                return Err(self.unsupported(span, "externally implementable items, `#[eii]`,"));
            }
            match js_form(self.tcx, def_id) {
                JsForm::Jsx(tag) => return self.jsx(&tag, args, span, out).map(Some),
                JsForm::Prop(name) => return self.jsx_prop(name.as_deref(), args, span, out).map(Some),
                JsForm::Object(keys) => return self.object_binding(&keys, args, span, out).map(Some),
                JsForm::Import { module } => return self.dynamic_import(args[0], module, span).map(Some),
                _ => {}
            }
            let mut values = self.operands(args, out)?;
            // `()` given to JS is what a tuple is, an array (ADR 0020):
            // `use_effect(f, ())` is `useEffect(f, [])`.
            for (value, &arg) in values.iter_mut().zip(args) {
                if matches!(self.thir[self.strip(arg)].kind, ExprKind::Tuple { ref fields } if fields.is_empty()) {
                    *value = Expr::array(Vec::new());
                }
            }
            // A parameter it names `#[rust_js::nullable(..)]` is `T | null`:
            // its `None` is `null` (ADR 0275).
            let nullable = nullable_params(self.tcx, def_id);
            if !nullable.is_empty() {
                let idents = self.tcx.fn_arg_idents(def_id);
                for ((value, &arg), ident) in values.iter_mut().zip(args).zip(idents) {
                    if ident.is_some_and(|ident| nullable.contains(&ident.name)) {
                        let given = std::mem::replace(value, Expr::undefined());
                        *value = self.nullable(given, arg);
                    }
                }
            }
            let mut args = values;
            let this = is_method(self.tcx, def_id).then(|| args.remove(0));
            // Its last argument, a slice, is JS's rest arguments: what an
            // array written out holds, or the slice spread (ADR 0221).
            if is_variadic(self.tcx, def_id) {
                let inputs = self.tcx.fn_sig(def_id).skip_binder().skip_binder().inputs();
                if !inputs.last().is_some_and(|ty| ty.peel_refs().is_slice()) {
                    return Err(self.unsupported(
                        span,
                        "a `#[rust_js::variadic]` binding whose last parameter isn't a slice",
                    ));
                }
                match args.pop() {
                    Some(Expr {
                        kind: js::ExprKind::Array(items),
                        ..
                    }) => args.extend(items),
                    Some(last) => args.push(Expr::spread(last)),
                    None => {}
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
                        *arg = Expr::bin(Op::And, test.tested(), (**value).clone());
                    }
                }
            }
            let value = match (js_form(self.tcx, def_id), this) {
                // A method or a property is on `this`: it can't be an import.
                (JsForm::Call(name), Some(this)) if !name.contains('#') => {
                    Expr::call(Expr::member(this, name).or_at(fun_span), args)
                }
                (JsForm::Call(name), None) => Expr::call(self.js_ref(&name).or_at(fun_span), args),
                (JsForm::New(name), None) => Expr::new_(self.js_ref(&name).or_at(fun_span), args),
                (JsForm::Get(name), Some(this)) if args.is_empty() && !name.contains('#') => Expr::member(this, name),
                // A class's static property, read at each call: `Notification.permission`.
                (JsForm::Get(name), None) if args.is_empty() => self.js_ref(&name).or_at(fun_span),
                (JsForm::Set(name), Some(this)) if args.len() == 1 && !name.contains('#') => {
                    let value = args.remove(0);
                    out.push(StmtKind::Assign(Expr::member(this, name), value).at(self.js_span(span)));
                    Expr::undefined()
                }
                (JsForm::This, Some(this)) if args.is_empty() => this,
                // Of a value that's an `Option` itself, a JSON `null`'s
                // `Some(None)`, which only an own key has (ADR 0225).
                (JsForm::GetIndex, Some(this))
                    if args.len() == 1
                        && self
                            .option_of(
                                self.tcx
                                    .fn_sig(def_id)
                                    .instantiate(self.tcx, generic_args)
                                    .skip_binder()
                                    .output(),
                            )
                            .is_some_and(|value| self.boxed_payload(value)) =>
                {
                    self.runtime.insert(Helper::DictGet);
                    Expr::call(Expr::var("$dictGet"), vec![this, args.remove(0)])
                }
                (JsForm::GetIndex, Some(this)) if args.len() == 1 => keyed(this, args.remove(0)),
                (JsForm::In, Some(this)) if args.len() == 1 => Expr::bin(Op::In, args.remove(0), this),
                (JsForm::Truthy, Some(this)) if args.is_empty() => {
                    Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, this))
                }
                (JsForm::TypeOf, Some(this)) if args.is_empty() => Expr::unary(UnaryOp::Typeof, this),
                (JsForm::Text, Some(this)) if args.is_empty() => Expr::bin(Op::Add, this, Expr::str("")),
                (JsForm::SetIndex, Some(this)) if args.len() == 2 => {
                    let (key, value) = (args.remove(0), args.remove(0));
                    out.push(StmtKind::Assign(keyed(this, key), value).at(self.js_span(span)));
                    Expr::undefined()
                }
                (JsForm::CallThis, Some(this)) => Expr::call(this, args),
                (JsForm::InstanceOf(class), Some(this)) if args.is_empty() => {
                    Expr::bin(Op::InstanceOf, this, self.js_ref(&class))
                }
                _ => {
                    let what = format!(
                        "the `#[link_name]` of `{}` with this signature",
                        self.tcx.def_path_str(def_id)
                    );
                    return Err(self.unsupported(self.thir[fun].span, &what));
                }
            };
            return Ok(Some(self.catching(def_id, generic_args, value)));
        }
        // Calling a closure, `f(a, b)`, is `Fn::call(&f, (a, b))`: in JS, `f(a, b)`.
        if let Some(fn_trait) = self.tcx.trait_of_assoc(def_id)
            && (self.tcx.fn_trait_kind_from_def_id(fn_trait).is_some()
                || self.tcx.async_fn_trait_kind_from_def_id(fn_trait).is_some())
        {
            let [callee, ExprKind::Tuple { fields }] = [args[0], args[1]].map(|a| &self.thir[self.strip(a)].kind)
            else {
                return Err(self.unsupported(span, "this closure call"));
            };
            let callee = match *callee {
                // `&f` or `&mut f`: the closure itself.
                ExprKind::Borrow { arg, .. } => arg,
                _ => args[0],
            };
            let mut list = vec![callee];
            list.extend(fields.iter().copied());
            let mut values = self.operands(&list, out)?;
            let callee = values.remove(0);
            return Ok(Some(Expr::call(callee, values)));
        }
        // `it.clone()` of a `$iter`, a local stepped through (ADR 0181).
        if let Some(copy) = self.cloned_stepping(def_id, args, span, out)? {
            return Ok(Some(copy));
        }
        if self
            .tcx
            .trait_of_assoc(def_id)
            .is_some_and(|id| super::traits::operational(self.tcx, self.krate.foreign, id))
            || (self.tcx.trait_of_assoc(def_id).is_some()
                && self
                    .resolve_instance(def_id, generic_args)?
                    .is_some_and(|i| self.is_rust_fn(i.def_id())))
        {
            let mut pending = Vec::new();
            let values = self.operands(args, &mut pending)?;
            if let Some(call) = self.trait_call(def_id, generic_args, values, span, &mut pending)? {
                out.extend(pending);
                return Ok(Some(call));
            }
        }
        // An `Ok` `fmt::Result` is nothing in JS (ADR 0054): its `unwrap()` is
        // `()`, after what made it ran (ADR 0148). One that may be an `Err`
        // is `undefined` or the `fmt::Error` (ADR 0187).
        if args
            .first()
            .is_some_and(|&a| self.is_fmt_result(self.thir[a].ty.peel_refs()))
        {
            let Some(answer) = fmt_result_answer(self.tcx, def_id) else {
                return Err(self.unsupported(span, "methods of a `fmt::Result`"));
            };
            let constant = match answer {
                FmtResultAnswer::Unwrap | FmtResultAnswer::Expect => Expr::undefined(),
                FmtResultAnswer::Is(ok) => Expr::bool(ok),
            };
            if !self.krate.any_failing {
                for &arg in args {
                    self.stmt(arg, &Dest::Discard, out)?;
                }
                return Ok(Some(constant));
            }
            let result = self.expr(args[0], out)?;
            let message = match args.get(1) {
                Some(&message) => Some(self.expr(message, out)?),
                None => None,
            };
            // Only what `$fmtTry` caught may be an `Err`, or a variable it's in.
            let may_be_err = match &result.kind {
                js::ExprKind::Call(callee, _) => matches!(&callee.kind, js::ExprKind::Var(name) if name == "$fmtTry"),
                js::ExprKind::Var(_) => true,
                _ => false,
            };
            if !may_be_err {
                for value in std::iter::once(result).chain(message) {
                    if value.has_effects() {
                        out.push(StmtKind::Expr(value).at(self.js_span(span)));
                    }
                }
                return Ok(Some(constant));
            }
            let result = if result.reads_same() {
                result
            } else {
                self.spill("result", result, out)
            };
            let ok = Expr::bin(Op::Eq, result.clone(), Expr::undefined());
            return Ok(Some(match answer {
                FmtResultAnswer::Is(true) => ok,
                FmtResultAnswer::Is(false) => Expr::bin(Op::Ne, result, Expr::undefined()),
                FmtResultAnswer::Unwrap | FmtResultAnswer::Expect => {
                    let message = message.unwrap_or_else(|| Expr::str("called `Result::unwrap()` on an `Err` value"));
                    self.runtime.insert(Helper::FmtError);
                    Expr::call(Expr::var("$fmtExpect"), vec![result, message])
                }
            }));
        }
        // `f.alternate()`, `f.width()` and the like: what this writer's
        // `Formatter` was given (ADRs 0137, 0143).
        if let Some(query) = super::recognition::formatter_query(self.tcx, def_id) {
            return self
                .formatter_answer(query)
                .ok_or_else(|| self.unsupported(span, "asking a `Formatter` here"))
                .map(Some);
        }
        // `cmp::max(a, b)` of what isn't a number: `Ord::max(a, b)`'s (ADR 0136).
        if self.std_fn(fun).is_none()
            && let Some(name) = match self.tcx.def_path_str(def_id).as_str() {
                "std::cmp::max" | "core::cmp::max" => Some("max"),
                "std::cmp::min" | "core::cmp::min" => Some("min"),
                _ => None,
            }
        {
            let ord = std_item(self.tcx, StdItem::Ord);
            let method = trait_method(self.tcx, ord, name);
            let values = self.operands(args, out)?;
            if let Some(call) = self.trait_call(method, generic_args, values, span, out)? {
                return Ok(Some(call));
            }
        }
        Ok(None)
    }

    /// A call of one of std's functions rust-js knows (ADR 0023), `known`.
    /// `std::ptr::eq(a, b)` of JS objects, `webapi::Element`s say: whether
    /// they're one, `a === b` (ADR 0285). Of a Rust value it's an error:
    /// an unchanged copy is the one JS object it was copied from, and a
    /// number is its value, so a place's address isn't anything of JS's.
    fn ptr_eq(&mut self, of: Ty<'tcx>, args: &[ExprId], span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        if !self.is_js_object(of) {
            return Err(self.tcx.dcx().span_err(
                span,
                format!(
                    "`std::ptr::eq` of a `{of}`: rust-js shares a copy no one changes and writes a number as its value, so where a Rust value is has no JS counterpart; compare the values, or references to JS objects"
                ),
            ));
        }
        let a = self.expr(self.pointed_to(args[0]), out)?;
        let b = self.expr(self.pointed_to(args[1]), out)?;
        Ok(Expr::bin(Op::Eq, a, b))
    }

    /// What an argument rustc made a raw pointer of is: `r` of `&raw const
    /// *r`, its `&T` given as a `*const T`, the JS object `r` is.
    fn pointed_to(&self, arg: ExprId) -> ExprId {
        match self.thir[self.strip(arg)].kind {
            ExprKind::RawBorrow { arg, .. } => arg,
            _ => arg,
        }
    }

    fn std_call(&mut self, known: Std, call: Call<'_, 'tcx>, out: &mut Vec<Stmt>) -> R<Expr> {
        let Call {
            fun,
            def_id,
            generic_args,
            args,
            discarded,
            span,
        } = call;
        if known == Std::PtrEq {
            return self.ptr_eq(generic_args.type_at(0), args, span, out);
        }
        if known == Std::OptionFlatten
            && let Some(read) = self.flattened_read(args[0], out)?
        {
            return Ok(read);
        }
        // One that takes a value with a destructor, or changes a place that
        // holds one, must keep or give back what it takes: these do. Another
        // might drop it, which JS wouldn't (ADR 0098). A value whose drops
        // rust-js can't follow, a `vec::IntoIter` of them say, might hold one.
        // `collect()` of a chain that owns its items drains it (ADR 0098).
        if known == Std::Collect
            && let Some(&receiver) = args.first()
        {
            self.mark_owned_drain(receiver);
        }
        let drained = args.first().is_some_and(|&a| self.is_drained(a));
        let holds_drops = |ty: Ty<'tcx>| self.drops(ty) != Drops::Nothing;
        let takes_drops = args.iter().any(|&a| match *self.thir[a].ty.kind() {
            ty::Ref(_, inner, Mutability::Mut) => holds_drops(inner),
            ty::Ref(..) => false,
            _ => holds_drops(self.thir[a].ty),
        });
        // Its value moves into the `Ok`; its function, run only for a `None`,
        // would be dropped unrun, so it mustn't hold one.
        let keeps_value = known == Std::Comb(Comb::OkOrElse) && !holds_drops(self.thir[args[1]].ty);
        if takes_drops
            && !drained
            && !keeps_value
            && !matches!(
                known,
                Std::Drop
                    | Std::Forget
                    | Std::Swap
                    | Std::Replace
                    | Std::Push
                    | Std::Same
                    // Moves its value into the function, which owns it then.
                    | Std::OptionMap
                    | Std::Comb(Comb::ResultMap | Comb::Filter | Comb::MapOr)
                    | Std::VecMacro
                    | Std::Unwrap
                    | Std::UnwrapUnchecked
                    | Std::UnwrapOk
                    | Std::Method("pop")
                    | Std::Index
                    | Std::Len
                    | Std::IsEmpty
            )
        {
            // Of a type parameter's only, a generic iterator's chrono folds:
            // one its callers give none of (ADR 0190).
            let held: Vec<Ty<'tcx>> = args
                .iter()
                .filter_map(|&a| match *self.thir[a].ty.kind() {
                    ty::Ref(_, inner, Mutability::Mut) => Some(inner),
                    ty::Ref(..) => None,
                    _ => Some(self.thir[a].ty),
                })
                .filter(|&ty| self.drops(ty) != Drops::Nothing)
                .collect();
            if !held
                .iter()
                .all(|&ty| self.drop_query().drops_but_params(ty) == Drops::Nothing)
            {
                let path = self.tcx.def_path_str(def_id);
                return Err(self.unsupported(span, &format!("`{path}` of a value with a destructor")));
            }
            for ty in held {
                self.require_no_drops(ty);
            }
        }
        // A std function that makes an `Option` of a generic `T` must box it
        // (ADR 0051); these do, and others aren't supported.
        // Normalized, so an iterator's `Self::Item` is the item's type.
        let output = self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, generic_args)
            .skip_normalization()
            .skip_binder()
            .output();
        let output = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
            .unwrap_or(output);
        // A `&mut` it made itself, to a value JS can't change in place, is the
        // item, not a cell (ADR 0099): only a pattern takes it apart.
        // A leaked `String`'s `&mut str` is the string: nothing writes a
        // `str` in place (ADR 0157), so it's a `&str` (ADR 0238).
        let leaked_text = matches!(self.std_fn(fun), Some(Std::Same))
            && matches!(output.kind(), ty::Ref(_, text, Mutability::Mut) if text.is_str());
        if let Some(cell) = self.makes_items(output, generic_args, args)
            && !self.is_item_call(fun)
            && !leaked_text
        {
            // Of numbers or strings in a collection: a handle on each (ADR 0152).
            if let Some(handles) = self.item_handles(known, args, span, out)? {
                return Ok(handles);
            }
            // A call whose `&mut` is its own cell gives a handle already.
            if !known.gives_its_cell() {
                let path = self.tcx.def_path_str(def_id);
                return Err(self.unsupported(span, &format!("a `{cell}` from `{path}` used as a value")));
            }
        }
        // One whose `Some` is boxed where it looks like `None` (ADR 0051): only
        // these make one.
        let boxed = self.option_of(output).is_some_and(|inner| self.boxed_payload(inner));
        if boxed
            && !matches!(
                known,
                Std::Same
                    | Std::OptionMap
                    | Std::Method("pop")
                    | Std::First
                    | Std::SliceLast
                    | Std::SliceGet
                    | Std::OptionCloned
                    | Std::Comb(Comb::Then | Comb::ThenSome)
                    // An `Option` of the same type, or the closure's own.
                    | Std::Comb(Comb::Filter | Comb::Or | Comb::OrElse | Comb::AndThen)
                    | Std::Last
                    // `as_ref()`: the same value, box and all.
                    | Std::Pointee
                    | Std::ResultOk
                    | Std::ArrayMethod("find")
                    | Std::Extreme(_)
                    | Std::Step(StepOp::Next | StepOp::Peek)
                    // The old value, as it's kept: boxed already.
                    | Std::OptionTake
                    // A `OnceCell`'s, kept as `$some` makes it, and a `LazyCell`'s.
                    | Std::Once(OnceOp::Get | OnceOp::Take)
                    | Std::Lazy(LazyOp::Get)
                    | Std::OptionReplace
                    // The inner `Option`, box and all.
                    | Std::OptionFlatten
            )
        {
            return Err(self.unsupported(span, "this call, for an `Option` of what could look like `None`"));
        }
        if let Std::Map(op) = known {
            return self.map_call(op, args, generic_args, discarded, span, out);
        }
        if let Std::Range(op) = known {
            return self.range_call(op, args, span, out);
        }
        // A standard stream holds nothing JS needs, and a write to one is
        // `print!`'s (ADR 0132).
        if let Std::Stream(op) = known {
            let mut values = self.operands(args, out)?.into_iter();
            if op != StreamOp::Open
                && let Some(on) = values.next()
                && on.has_effects()
            {
                out.push(StmtKind::Expr(on).at(self.js_span(span)));
            }
            return match op {
                StreamOp::Write { error } => {
                    let text = values.next().expect("a write's text");
                    Ok(match without_newline(text) {
                        Ok(line) => Expr::call(
                            Expr::member(Expr::var("console"), if error { "error" } else { "log" }),
                            vec![line],
                        ),
                        Err(text) => {
                            self.runtime.insert(Helper::Print);
                            Expr::call(Expr::var(if error { "$eprint" } else { "$print" }), vec![text])
                        }
                    })
                }
                _ => Ok(Expr::undefined()),
            };
        }
        // rustc's name for the type, as a string; of a type parameter, the
        // one its caller gave (ADR 0145).
        if let Std::TypeName { of_val } = known {
            if of_val {
                let value = self.expr(args[0], out)?;
                if value.has_effects() {
                    out.push(StmtKind::Expr(value).at(self.js_span(span)));
                }
            }
            return self.type_fact_value(generic_args.type_at(0), TypeFact::Name, span);
        }
        if let Std::Comb(comb) = known {
            return self.comb_call(comb, args, generic_args, span, out);
        }
        if let Std::Text(op) = known {
            return self.text_call(op, args, generic_args, span, out);
        }
        if known == Std::Number(NumOp::ToIntUnchecked) {
            let value = self.expr(args[0], out)?;
            return self.cast(value, self.thir[args[0]].ty, output, span);
        }
        if let Std::Number(op) = known {
            // `i32::from_str_radix(s, 16)`'s is what its `Result` holds.
            let ty = match (op, output.kind()) {
                (NumOp::FromStrRadix, ty::Adt(_, result)) => result.type_at(0),
                // `u32::from_be_bytes(b)`'s and `f64::from_bits(b)`'s is what it makes.
                (NumOp::FromBytes { .. } | NumOp::FloatFromBytes { .. } | NumOp::FromBits, _) => output,
                _ => self.thir[args[0]].ty.peel_refs(),
            };
            return self.number_call(op, args, ty, span, out);
        }
        if let Std::ToJson(pretty) = known {
            let ty = generic_args.type_at(0);
            let value = self.expr(args[0], out)?;
            return self.json_text(value, ty, pretty, span);
        }
        if let Std::FromJson = known {
            let ty = generic_args.types().next().expect("`from_str::<T>`");
            let text = self.expr(args[0], out)?;
            return self.json_value(text, ty, span);
        }
        if let Std::Step(op) = known {
            return self.step_call(op, fun, args, generic_args, span, out);
        }
        if let Std::Heap(op) = known {
            return self.heap_call(op, args, span, out);
        }
        if let Std::Cow(op) = known {
            return self.cow_call(op, args, span, out);
        }
        if known == Std::IterLen {
            return self.iter_len(args[0], span, out);
        }
        if let Std::SizeHint(exact) = known {
            return self.size_hint(exact, args[0], span, out);
        }
        if known == Std::NonZeroNew {
            let ty = self.thir[args[0]].ty;
            let num = self.num(ty, span)?;
            let n = self.expr(args[0], out)?;
            // Of a constant, `NonZero::new(7)`: its answer.
            if let Some(value) = n.as_int().or_else(|| n.as_bigint()) {
                return Ok(if value == 0 { Expr::undefined() } else { n });
            }
            let n = if n.reads_same() { n } else { self.spill("n", n, out) };
            let zero = Expr::bin(Op::Eq, n.clone(), num.literal(0));
            return Ok(Expr::cond(zero, Expr::undefined(), n));
        }
        // A `for` loop's is `&mut it` (`lower_for`); a chain of one would take
        // what's left of `it`, which an array's doesn't know.
        if known == Std::IterByRef {
            return Err(self.unsupported(span, "`by_ref()` but as what a `for` loop iterates"));
        }
        if known == Std::GenericSizeHint {
            let it = self.expr(args[0], out)?;
            self.runtime.insert(Helper::SizeHint);
            return Ok(Expr::call(Expr::var("$sizeHint"), vec![it]));
        }
        if known == Std::UserWrite {
            return self.user_write(def_id, args, generic_args, span, out);
        }
        if known == Std::ExactLen {
            return self.exact_len(args[0], span, out);
        }
        if known == Std::DequeRemove {
            let [items, at]: [Expr; 2] = self.operands(args, out)?.try_into().ok().expect("a deque and an index");
            self.runtime.insert(Helper::RemoveOpt);
            return Ok(Expr::call(Expr::var("$removeOpt"), vec![items, at]));
        }
        if known == Std::FromElem {
            return self.vec_of_copies(args, span, out);
        }
        // `vec![a, b]` is `box_assume_init_into_vec_unsafe(write_box_via_move(<box>, [a, b]))`.
        if known == Std::VecMacro {
            let ExprKind::Call { args: ref inner, .. } = self.thir[self.strip(args[0])].kind else {
                return Err(self.unsupported(span, "this `vec!`"));
            };
            return self.expr(inner[1], out);
        }
        if known.takes_iterator() || matches!(known, Std::Sort | Std::SortByKey) {
            return self.iterator_call(known, args, generic_args, boxed, span, out);
        }
        if let Std::StringEdit(edit) = known {
            return self.string_edit(edit, args, span, out);
        }
        // Its capacity is the engine's: what it's given runs, for what it does.
        if known == Std::StringWithCapacity {
            for &arg in args {
                self.stmt(arg, &Dest::Discard, out)?;
            }
            return Ok(Expr::str(""));
        }
        // `s.push_str(t)`: JS strings don't change, so `s` gets a new one.
        if known == Std::PushStr {
            let ExprKind::Borrow { arg: place, .. } = self.thir[self.strip(args[0])].kind else {
                return Err(self.unsupported(span, "`push_str` on this"));
            };
            // A `write!` of what may fail: piece by piece, so what came
            // before is written, and its `fmt::Result` the error, if any
            // (ADR 0187).
            let fails = self.formats_may_fail(args[1]);
            let mut written = Vec::new();
            let into = if fails { &mut written } else { &mut *out };
            let value = self.expr(args[1], into)?;
            let js_span = self.js_span(span);
            // A place is written where it is: `t` can't change the `s` it's
            // pushed to, which Rust has borrowed.
            if self.slot_place(place).is_none() && self.map_slot(place).is_none() && self.place(place).is_some() {
                let target = self.assignee(place)?;
                append_written(&target, value, fails, js_span, into);
                // What a writer wrote before it failed is the string's, as
                // Rust's writes it there: `t += $fmtPartial(error)`.
                if fails {
                    let caught = self.fresh("error");
                    let partial = Expr::call(Expr::var("$fmtPartial"), vec![Expr::var(&caught)]);
                    let handler = vec![
                        StmtKind::Assign(target.clone(), Expr::bin(Op::Add, target, partial)).at(js_span),
                        StmtKind::Throw(Expr::var(&caught)).at(js_span),
                    ];
                    let body = std::mem::take(&mut written);
                    written.push(StmtKind::TryCatch(body, Some(caught), handler).at(js_span));
                }
            } else {
                // Else where `+=` would write: a map's slot, or what a call's cell
                // points at, `pick(&mut a, &mut b).push_str(t)` (ADR 0099).
                if fails {
                    return Err(self.unsupported(span, "a `write!` here of what may fail"));
                }
                let (target, value) = self.prepare_assignment_target(place, true, value, span, into)?;
                let appended = Expr::bin(Op::Add, target.read(), value);
                target.write(appended, js_span, into);
            }
            return Ok(match fails {
                true => self.fmt_try(written),
                false => Expr::undefined(),
            });
        }
        if let Std::WrappingOp(op, assign) = known {
            return self.wrapping_op(op, assign, args, span, out);
        }
        if let Std::AssignOperator(op) = known {
            let ExprKind::Borrow { arg: place, .. } = self.thir[self.strip(args[0])].kind else {
                return Err(self.unsupported(span, "this assignment"));
            };
            if let Some(target) = self.slot_place(place) {
                let value = self.expr(args[1], out)?;
                let value = self.shift_amount(op, value, place, args[1]);
                let ty = self.thir[place].ty;
                let value = self.binary(op, target.read(), value, None, ty, span)?;
                target.write(value, self.js_span(span), out);
                return Ok(Expr::undefined());
            }
            // `*m.entry(k).or_insert(0) += n` with a `&u32` `n`: as with a `u32` (ADR 0059).
            if let Some(slot) = self.map_slot(place) {
                // A trait call evaluates its receiver before its argument.
                let target = self.prepare_map_place(slot, true, span, out)?;
                let value = self.expr(args[1], out)?;
                let value = self.shift_amount(op, value, place, args[1]);
                let ty = self.thir[place].ty;
                let value = self.binary(op, target.read(), value, None, ty, span)?;
                target.write(value, self.js_span(span), out);
                return Ok(Expr::undefined());
            }
            let value = self.expr(args[1], out)?;
            let value = self.shift_amount(op, value, place, args[1]);
            let target = self.assignee(place)?;
            let ty = self.thir[place].ty;
            let current = self.binary(op, target.clone(), value, None, ty, span)?;
            let js_span = self.js_span(span);
            out.push(StmtKind::Assign(target, current).at(js_span));
            return Ok(Expr::undefined());
        }
        if known == Std::FmtNew {
            // `format_arguments::new(template, &args)`, the template a byte string.
            let ExprKind::Literal { lit, .. } = self.thir[self.strip_refs(args[0])].kind else {
                return Err(self.unsupported(span, "this format string"));
            };
            let LitKind::ByteStr(ref bytes, _) = lit.node else {
                return Err(self.unsupported(span, "this format string"));
            };
            let items = self.expr(args[1], out)?;
            return self.format(bytes.as_byte_str(), items, span);
        }
        if known == Std::AssertFailed {
            // `assert_failed(kind, &left, &right, None or Some(message))`.
            let message = match self.thir[self.strip(args[3])].kind {
                ExprKind::Adt(ref option) => option.fields.first().map(|f| f.expr),
                _ => return Err(self.unsupported(span, "this assertion")),
            };
            let mut list = args[..3].to_vec();
            list.extend(message);
            let mut values = self.operands(&list, out)?;
            // The two values' `{:?}`, by their types (ADR 0060).
            for i in [1, 2] {
                let ty = self.thir[args[i]].ty;
                let value = std::mem::replace(&mut values[i], Expr::undefined());
                values[i] = self.debug_string(value, ty, span)?;
            }
            self.runtime.insert(Helper::AssertFailed);
            return Ok(Expr::call(Expr::var("$assertFailed"), values));
        }
        if matches!(
            known,
            Std::Swap | Std::Replace | Std::OptionTake | Std::OptionReplace | Std::MemTake
        ) {
            return self.swap_or_replace(known, args, discarded, span, out);
        }
        // `format!`, `to_string()` and `print!` of what may fail: what they
        // write, in a `try`, a `fmt::Error` std's panic (ADR 0187).
        let fails = match known {
            Std::Format | Std::Print { .. } => self.formats_may_fail(args[0]),
            Std::ToString => self.fmt_may_fail(self.display_trait(), generic_args.type_at(0)),
            _ => false,
        };
        if fails {
            return self.failing_consumer(known, call, out);
        }
        let mut values = self.operands(args, out)?.into_iter();
        if let Some(js) = self.vec_call(known, call, &mut values, boxed, out)? {
            return Ok(js);
        }
        if let Some(js) = self.option_call(known, call, &mut values, boxed, out)? {
            return Ok(js);
        }
        if let Some(js) = self.cell_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.once_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.lazy_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.channel_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.numeric_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.iter_source_call(known, call, &mut values)? {
            return Ok(js);
        }
        if let Some(js) = self.string_call(known, call, &mut values)? {
            return Ok(js);
        }
        if let Some(js) = self.print_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(match known {
            Std::Swap | Std::Replace | Std::OptionTake | Std::OptionReplace | Std::MemTake => {
                unreachable!("lowered from their places, above")
            }
            // An `Rc` is the JS reference itself: the garbage collector does
            // its counting, so a clone is the same object.
            Std::Same | Std::Format => arg(),
            Std::Pointee => self.through_refs(arg(), self.thir[args[0]].ty).0,
            Std::Last
            | Std::Cloned
            | Std::Fuse
            | Std::ArrayMethod(_)
            | Std::Enumerate
            | Std::Rev
            | Std::Skip
            | Std::Take
            | Std::Fold
            | Std::Sum
            | Std::CollectString
            | Std::CollectFallible
            | Std::Collect
            | Std::Position
            | Std::Extreme(_)
            | Std::Sort
            | Std::SortByKey => {
                unreachable!("handled above")
            }
            Std::PushStr | Std::AssignOperator(_) | Std::StringEdit(_) | Std::StringWithCapacity => {
                unreachable!("handled above")
            }
            Std::VecMacro | Std::FmtNew | Std::AssertFailed => unreachable!("handled above"),
            Std::Map(_)
            | Std::Range(_)
            | Std::Stream(_)
            | Std::TypeName { .. }
            | Std::Comb(_)
            | Std::IterComb(_)
            | Std::Text(_)
            | Std::Number(_)
            | Std::FromElem
            | Std::Heap(_)
            | Std::IterLen
            | Std::ExactLen
            | Std::SizeHint(_)
            | Std::WrappingOp(..)
            | Std::GenericSizeHint
            | Std::IterByRef
            | Std::NonZeroNew
            | Std::UserWrite
            | Std::DequeRemove
            | Std::Step(_)
            | Std::ToJson(_)
            | Std::FromJson => {
                unreachable!("handled above")
            }
            Std::Method(_)
            | Std::First
            | Std::SliceLast
            | Std::SliceGet
            | Std::ToVec
            | Std::SortBy
            | Std::IsEmpty
            | Std::VecNew
            | Std::Nothing
            | Std::Append
            | Std::Push
            | Std::Len
            | Std::Index
            | Std::Clear
            | Std::Retain => unreachable!("lowered by vec_call"),
            Std::Then
            | Std::ThenWith
            | Std::IsOk(_)
            | Std::UnwrapOk
            | Std::UnwrapErr
            | Std::ResultOk
            | Std::ResultOr
            | Std::IsSome
            | Std::IsNone
            | Std::Unwrap
            | Std::UnwrapUnchecked
            | Std::UnwrapOr
            | Std::OptionMap
            | Std::OptionIter
            | Std::OptionCloned
            | Std::OptionFlatten => unreachable!("lowered by option_call"),
            Std::Channel(_) => unreachable!("lowered by channel_call"),
            Std::CellNew
            | Std::CellGet
            | Std::CellSet
            | Std::CellReplace
            | Std::CellTake
            | Std::CellReplaceWith
            | Std::Borrow
            | Std::Lock
            | Std::Drop
            | Std::Forget
            | Std::AtomicLoad
            | Std::AtomicStore
            | Std::AtomicSwap
            | Std::AtomicFetch(_)
            | Std::AtomicFetchMax(_)
            | Std::AtomicCompareExchange
            | Std::LocalWith
            | Std::LocalBorrow => unreachable!("lowered by cell_call"),
            Std::Once(_) => unreachable!("lowered by once_call"),
            Std::Lazy(_) => unreachable!("lowered by lazy_call"),
            Std::Cow(_) => unreachable!("lowered by cow_call"),
            Std::PtrEq => unreachable!("lowered by ptr_eq"),
            Std::ToBig
            | Std::Duration(_)
            | Std::SliceToArray { .. }
            | Std::TryFromInt { .. }
            | Std::FromDigit
            | Std::FromU32
            | Std::Cmp
            | Std::MaxOf(_)
            | Std::Operator(_)
            | Std::UnaryOperator(_)
            | Std::Reverse
            | Std::SizeOf
            | Std::AlignOf
            | Std::SizeOfVal => unreachable!("lowered by numeric_call"),
            Std::IterSource(_) => unreachable!("lowered by iter_source_call"),
            Std::Concat
            | Std::StripPrefix
            | Std::StripSuffix
            | Std::SplitOnce
            | Std::RsplitOnce
            | Std::Chars
            | Std::StringNew
            | Std::Trim { .. }
            | Std::AsciiCase { .. }
            | Std::AsciiEq
            | Std::ToString => unreachable!("lowered by string_call"),
            Std::Panic
            | Std::PanicFmt
            | Std::PanicDisplay
            | Std::BeginPanic
            | Std::Print { .. }
            | Std::FmtStr
            | Std::FmtDisplay
            | Std::FmtDebug
            | Std::FmtRadix(_)
            | Std::FmtExp(_)
            | Std::FmtPointer
            | Std::FmtUsize => unreachable!("lowered by print_call"),
        })
    }

    /// What `From::from` or `Into::into` makes, and from what.
    fn converted(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        let tr = self.tcx.trait_of_assoc(def_id)?;
        let mut types = args.types();
        let (first, second) = (types.next()?, types.next()?);
        if is_std_def(self.tcx, tr, StdItem::From) {
            Some((first, second))
        } else if is_std_def(self.tcx, tr, StdItem::Into) {
            Some((second, first))
        } else {
            None
        }
    }

    /// A JS call that says, in Rust, that it may throw (ADR 0035): one
    /// returning a `Result` runs in a `try`, `$try(() => f(x))`, and one
    /// returning a `Promise<Result<..>>` settles either way, `$settle(p)`.
    pub(super) fn catching(&mut self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, value: Expr) -> Expr {
        match self.recognition().catching(def_id, args) {
            Catching::Result => {
                self.runtime.insert(Helper::Try);
                let span = value.span;
                let thunk = Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(value)).at(span)]);
                Expr::call(Expr::var("$try"), vec![thunk])
            }
            Catching::PromiseResult => {
                self.runtime.insert(Helper::Settle);
                Expr::call(Expr::var("$settle"), vec![value])
            }
            Catching::Direct => value,
        }
    }
}

/// `f(args)`, with a closure that only returns put in place, its parameters
/// the arguments: `((s) => s.value)(START)` is `START.value`. Only for
/// arguments that read the same however often they're read.
pub(super) fn apply(f: Expr, args: Vec<Expr>) -> Expr {
    if let js::ExprKind::Arrow(params, body) = &f.kind
        && let [
            js::Stmt {
                kind: StmtKind::Return(Some(value)),
                ..
            },
        ] = body.as_slice()
        && let Some(inlined) = inlined(params, &args, value)
    {
        return inlined;
    }
    Expr::call(f, args)
}

/// `f(args)` as a statement, in `out`: a closure of one statement put in
/// place, `((it) => { it.f(); })(linter)` being `linter.f();`, or
/// `apply`'s. What it gives, `undefined` of one put in place.
pub(super) fn apply_in(f: Expr, args: Vec<Expr>, out: &mut Vec<Stmt>) -> Expr {
    if let js::ExprKind::Arrow(params, body) = &f.kind
        && let [
            js::Stmt {
                kind: StmtKind::Expr(value),
                span,
            },
        ] = body.as_slice()
        && let Some(inlined) = inlined(params, &args, value)
    {
        out.push(StmtKind::Expr(inlined).at(*span));
        return Expr::undefined();
    }
    apply(f, args)
}

/// `value`, of a closure of `params`, with each the part of `args` it
/// binds, if each reads the same however often it's read.
fn inlined(params: &[js::Pattern], args: &[Expr], value: &Expr) -> Option<Expr> {
    if params.len() <= args.len() && args.iter().all(Expr::reads_same) {
        // Each variable a parameter binds, and the part of its argument it
        // is: a destructured one's, `[, age]`'s `age` of `item`, `item[1]`.
        let parts: Option<Vec<(&str, Expr)>> = params
            .iter()
            .zip(args)
            .map(|(p, arg)| match p {
                js::Pattern::Name(name) => Some(vec![(name.as_str(), arg.clone())]),
                js::Pattern::Array(items) => Some(
                    (items.iter().enumerate())
                        .filter_map(|(i, name)| {
                            Some((name.as_deref()?, Expr::index(arg.clone(), Expr::int(i as i128))))
                        })
                        .collect(),
                ),
                js::Pattern::Object(fields, None) if fields.iter().all(|(_, _, default)| default.is_none()) => Some(
                    (fields.iter())
                        .map(|(key, name, _)| (name.as_str(), keyed(arg.clone(), Expr::str(key.clone()))))
                        .collect(),
                ),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.into_iter().flatten().collect());
        return parts.and_then(|parts| {
            value.substitute(&|name: &str| parts.iter().find(|(n, _)| *n == name).map(|(_, part)| part.clone()))
        });
    }
    None
}

/// `this[key]`, or `this.name` of a key written that's a name, as a person
/// writes it (ADR 0225): `js::get(value, "name")` is `value.name`.
fn keyed(this: Expr, key: Expr) -> Expr {
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
