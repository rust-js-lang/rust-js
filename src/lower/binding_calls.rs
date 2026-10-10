//! A call of a binding, of its arguments' values: JS's function, method,
//! property or operator it names (ADRs 0019, 0020), what THIR's lowering
//! and MIR's share (ADR 0364).

use super::bindings::{JsForm, is_method, is_variadic, js_form};
use super::calls::given_to_js;
use super::{FnCx, R};
use crate::js;
use crate::js::{Expr, Op, StmtKind, UnaryOp};
use crate::runtime::Helper;
use rustc_middle::ty;
use rustc_span::Span;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `def_id`'s call, given `args`, each as JS is given it: `fun_span`,
    /// where its name is written.
    pub(in crate::lower) fn binding_values(
        &mut self,
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        mut args: Vec<Expr>,
        fun_span: js::Span,
        span: Span,
        out: &mut Vec<js::Stmt>,
    ) -> R<Expr> {
        let this = is_method(self.tcx, def_id).then(|| args.remove(0));
        // Its last argument, a slice, is JS's rest arguments: what an
        // array written out holds, or the slice spread (ADR 0221).
        // Or a tuple, as an emitter's event's arguments are, of its type at
        // this call: `()`, an empty array, none.
        if is_variadic(self.tcx, def_id) {
            let last = (self.tcx.fn_sig(def_id).instantiate(self.tcx, generic_args))
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
            if !last.is_some_and(|ty| ty.peel_refs().is_slice()) && !tuple {
                return Err(self.unsupported(
                    span,
                    "a `#[rust_js::variadic]` binding whose last parameter isn't a slice or a tuple",
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
        given_to_js(&mut args);
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
                return Err(self.unsupported(span, &what));
            }
        };
        Ok(self.catching(def_id, generic_args, value))
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
