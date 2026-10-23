//! A call of a binding, of its arguments' values (ADR 0364): JS's
//! function, method, property or operator it names (ADRs 0019, 0020).

use super::bindings::{JsForm, is_binding, is_method, is_optional_param, is_rest_value, is_variadic, js_form};
use super::calls::{given_to_js, spread_rest};
use super::recognition::StdItem;
use super::{FnCx, R};
use crate::js;
use crate::js::{Expr, Op, StmtKind, UnaryOp};
use rustc_middle::ty;
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
        let this = is_method(self.tcx, def_id).then(|| args.remove(0));
        // Its last argument is JS's rest arguments (ADRs 0221, 0640): a
        // tuple, `impl js::Rest<T>`'s or an emitter's event's, its items,
        // `()` none; a slice or a `Vec` of them, spread, or what an array
        // written out holds.
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
            if !last.is_some_and(collection) && !tuple && !rest {
                return Err(self.unsupported(
                    span,
                    "a `#[rust_js::variadic]` binding whose last parameter isn't rest arguments: a tuple, a slice, a `Vec` or a `&dyn js::Rest<T>`",
                ));
            }
            spread_rest(&mut args);
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
        let value = match (js_form(self.tcx, def_id), this) {
            // A method or a property is on `this`: it can't be an import.
            (JsForm::Call(name), Some(this)) if !name.contains('#') => {
                Expr::call(Expr::member(this, name).or_at(fun_span), args)
            }
            (JsForm::Call(name), None) => Expr::call(self.binding_ref(&name, named).or_at(fun_span), args),
            (JsForm::New(name), None) => Expr::new_(self.js_ref(&name).or_at(fun_span), args),
            // A property's read, which a getter may answer (ADR 0672).
            (JsForm::Get(name), Some(this)) if args.is_empty() && !name.contains('#') => {
                Expr::member(this, name).answered_by_a_getter()
            }
            // A class's static property, read at each call: `Notification.permission`.
            (JsForm::Get(name), None) if args.is_empty() => self.js_ref(&name).or_at(fun_span).answered_by_a_getter(),
            (JsForm::Set(name), Some(this)) if args.len() == 1 && !name.contains('#') => {
                let value = args.remove(0);
                out.push(StmtKind::Assign(Expr::member(this, name), value).at(self.js_span(span)));
                Expr::undefined()
            }
            // A property, written, of a global or an import: `process.exitCode
            // = 1`, `EventEmitter.captureRejections = true`. Not an import
            // itself, which JS can't assign.
            (JsForm::Set(name), None)
                if args.len() == 1 && name.rsplit('#').next().is_some_and(|member| member.contains('.')) =>
            {
                let value = args.remove(0);
                out.push(StmtKind::Assign(self.js_ref(&name).or_at(fun_span), value).at(self.js_span(span)));
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
                Expr::call(Expr::helper("$dictGet"), vec![this, args.remove(0)])
            }
            (JsForm::GetIndex, Some(this)) if args.len() == 1 => keyed(this, args.remove(0)).answered_by_a_getter(),
            (JsForm::In, Some(this)) if args.len() == 1 => Expr::bin(Op::In, args.remove(0), this),
            (JsForm::Truthy, Some(this)) if args.is_empty() => {
                Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, this))
            }
            (JsForm::TypeOf, Some(this)) if args.is_empty() => Expr::unary(UnaryOp::Typeof, this),
            (JsForm::Text, Some(this)) if args.is_empty() => Expr::bin(Op::Add, Expr::str(""), this),
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
                return Err(self.unsupported(span, &what));
            }
        };
        Ok(self.catching(def_id, generic_args, value))
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
