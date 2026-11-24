//! The checks of what a JS caller gives a function's parameters (#2403,
//! ADRs 1192, amended, and 1194), built where its parameters are final,
//! after tidy, one rule for each: one its JS takes whole, `count(order)`, is
//! checked whole by that name (`checks.rs`' `whole_check`); one its JS
//! takes apart, a pattern's, `Props { title, children }: Props`, `(w, h):
//! (u32, u32)`, or a named one tidy takes apart, `props: ImageProps` of a
//! body that starts `let ImageProps { alt, rest } = props`, keeps its shape
//! where it's given, `Card({ title, children })`, and each part it binds
//! is checked by its path, the parameter named as its `.d.ts` names it:
//!
//! ```js
//! export function Card({ title, children }) {
//!   $checkStr(title, "Card(props.title)");
//! ```
//!
//! Rust reads only what a pattern binds, so that's the whole check: a part
//! JS binds that the body never reads, `lang: _` that keeps `lang` out of
//! `...rest`, isn't checked, unless dropping it runs code. A `null` given
//! takes JS's own `TypeError` as it's taken apart, before Rust runs. A
//! function whose JS takes one apart isn't split (ADR 1193): its entry
//! couldn't give the value whole to its body.

use rust_js_program::js::{self, Expr};
use rustc_middle::ty::Ty;
use rustc_span::def_id::DefId;

use crate::declarations::contract::Contract;
use crate::signatures::{Form, JsSignature, Origin};
use crate::{FnCx, Shape};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The checks of what a JS caller gives `item`, `params` as its JS takes
    /// them and `body` as it reads them, first in it: at no place of the
    /// Rust, the boundary's own code, so a comment above a parameter stays
    /// above it, never above its check. None of a closure's body, a copy only
    /// Rust calls, a function no JS caller can call, `@rust-js/std`'s, a
    /// binding's, whose JS is JS's own, nor one that passes them on to a
    /// function that checks them (`checks_its_parameters`).
    pub fn parameter_checks(
        &self,
        item: DefId,
        (params, body): (&[js::Pattern], &[js::Stmt]),
        mir: &rustc_middle::mir::Body<'tcx>,
    ) -> Vec<js::Stmt> {
        if item != self.item || self.copy.is_some() || !self.checks_its_parameters(item, self.frame.thir) {
            return Vec::new();
        }
        let signature = self.js_signature(item);
        // Each JS parameter's Rust index, as the lowering made it, never its
        // place: a `Formatter` is none of its JS's, and its options, `options`,
        // are a parameter after the others (ADRs 0058, 0137).
        let origins = &self.param_origins;
        if origins.len() != params.len() {
            self.unplaced(item, "its parameters lowered apart from what records them");
        }
        // A class's constructor's state flattened into its keys (ADR 0761),
        // given as them, not as a value whole.
        let flattened = crate::classes::flattened_parameter(self.tcx, item).map(|(at, _)| at);
        let function = self.checked_function_name(item);
        // One its body asks a `js` type test of first, which takes any value.
        let tested = self.type_tested(mir);
        let mut out = Vec::new();
        for param in &signature.params {
            let Origin::Rust(at) = param.origin else { continue };
            if tested.contains(&at) {
                continue;
            }
            // Its JS parameter, by its Rust index, as it was lowered. A checked
            // one its JS has none of would run unchecked: rust-js's bug, so an
            // internal error, never a function left unchecked.
            let Some(index) = origins.iter().position(|&of| of == Some(at)) else {
                if self.parameter_check(param.form).is_some() || self.callback(&signature, param.form).is_some() {
                    self.unplaced(item, &format!("its parameter `{}` has no place in its JS", param.name));
                }
                continue;
            };
            let pattern = &params[index];
            if flattened == Some(at) {
                continue;
            }
            let name = param.name.trim_end_matches('$');
            if let js::Pattern::Name(given) = pattern {
                let place = format!("{function}({name})");
                out.extend(self.whole_check((&signature, param), given, place));
                continue;
            }
            let Form::Value { ty, .. } = param.form else { continue };
            // Its path, inside its function's parentheses, closed where it's
            // checked: `Card(props.title)`.
            let place = format!("{function}({name}");
            self.part_checks((&signature, pattern, body), ty, place, &mut out);
        }
        folded(&function, out)
    }

    /// A library's function `def_id` a module JS may call gives JS by
    /// `alias`, whose own JS checks nothing JS gives it (`Item::checked`):
    /// the module's own forwarder in its re-export's place, which checks what
    /// JS gives it, then calls the library's, `export function
    /// serverDecode(body, manifest) { $checks(..); return Server.decode(body,
    /// manifest); }` (#2403 (Y), the lead's decision). None where nothing it
    /// takes is checked, or it takes what no forwarder passes on as it is, a
    /// rest's items or what its JS takes after its parameters.
    pub fn library_forwarder(
        &self,
        def_id: DefId,
        (local, alias): (&str, &str),
        callee: Option<Expr>,
    ) -> Option<js::Function> {
        if crate::bindings::takes_rest(self.tcx, def_id) {
            return None;
        }
        let signature = self.js_signature(def_id);
        let (mut names, mut checks) = (Vec::new(), Vec::new());
        for param in &signature.params {
            let Origin::Rust(_) = param.origin else { return None };
            let name = js::Name::declared(param.name.clone());
            let place = format!("{alias}({})", param.name.trim_end_matches('$'));
            checks.extend(self.whole_check((&signature, param), &name, place));
            names.push(name);
        }
        if checks.is_empty() {
            return None;
        }
        let callee = callee.unwrap_or_else(|| self.fn_ref(def_id));
        let call = Expr::call(callee, names.iter().map(Expr::var).collect());
        let mut body = folded(alias, checks);
        body.push(js::StmtKind::Return(Some(call)).at(js::Span::NONE));
        Some(js::Function {
            name: js::Name::module(local),
            param_spans: vec![js::Span::NONE; names.len()],
            params: names.into_iter().map(js::Pattern::Name).collect(),
            body,
            // By its own name, or by an alias of its module's, `export { f$1 as
            // f }`, where an import has its name (`library_forwarders`).
            export: local == alias,
            is_async: false,
            noinline: false,
            span: js::Span::NONE,
            name_span: js::Span::NONE,
            set_apart: false,
        })
    }

    /// The checks of what `pattern` binds of a value of `ty`, at `place`,
    /// its path not yet closed, `Card(props`.
    fn part_checks(
        &self,
        (signature, pattern, body): (&JsSignature<'tcx>, &js::Pattern, &[js::Stmt]),
        ty: Ty<'tcx>,
        place: String,
        out: &mut Vec<js::Stmt>,
    ) {
        match pattern {
            // One the body never reads, which JS binds to keep it out of a
            // `...rest`, `lang: _`: not Rust's, unless its drop runs code.
            js::Pattern::Name(name) if js::mentions_of(body, name) == 0 && !self.has_drops(erased(self.tcx, ty)) => {}
            js::Pattern::Name(name) => out.extend(self.value_check(signature, name, ty, place)),
            js::Pattern::Array(items) => {
                let item_tys: Vec<Ty<'tcx>> = match self.contract(ty) {
                    Contract::Shared(inner) => return self.part_checks((signature, pattern, body), inner, place, out),
                    Contract::Tuple(tys) => tys.to_vec(),
                    Contract::Items(item) => vec![item; items.len()],
                    _ => match self.shape(ty) {
                        Shape::Array(tys) => tys,
                        _ => return,
                    },
                };
                for (at, item) in items.iter().enumerate() {
                    let (Some(item), Some(&item_ty)) = (item, item_tys.get(at)) else {
                        continue;
                    };
                    self.part_checks((signature, item, body), item_ty, format!("{place}[{at}]"), out);
                }
            }
            js::Pattern::Object(keys, _) => {
                if let Contract::Shared(inner) = self.contract(ty) {
                    return self.part_checks((signature, pattern, body), inner, place, out);
                }
                let Shape::Object(fields) = self.shape(ty) else { return };
                for (key, part, _) in keys {
                    let field = (fields.iter()).find(|(field, _)| field.symbol_item().is_none() && field.text() == key);
                    let Some(&(_, field_ty)) = field else { continue };
                    self.part_checks((signature, part, body), field_ty, format!("{place}.{key}"), out);
                }
            }
            js::Pattern::Rest(_) | js::Pattern::Defaulted(..) => {}
        }
    }

    /// The check of `name`, a value of `ty`, at `place`, as a parameter's of
    /// that type is: `$checkStr(title, "Card(props.title)")`; a callback's,
    /// wrapped where what it gives back is read (ADR 1195).
    fn value_check(
        &self,
        signature: &JsSignature<'tcx>,
        name: &js::Name,
        ty: Ty<'tcx>,
        place: String,
    ) -> Option<js::Stmt> {
        if let Some(callback) = self.callback_of(signature, ty) {
            let mut args = vec![Expr::var(name), Expr::str(format!("{place})"))];
            let call = |args| Expr::call(Expr::helper("$checkCallback"), args);
            return Some(match callback.output {
                Some(output) => {
                    args.push(output);
                    js::StmtKind::Assign(Expr::var(name), call(args)).at(js::Span::NONE)
                }
                None => js::StmtKind::Expr(call(args)).at(js::Span::NONE),
            });
        }
        let (ty, none) = match self.contract(ty) {
            Contract::Option(inner) if !self.boxed_payload(inner) => (inner, Some("nullish")),
            _ => (ty, None),
        };
        let check = self.shallow_check(ty)?;
        let mut args = vec![Expr::var(name)];
        if let Some(whole) = check.whole {
            args.insert(0, whole);
        }
        if let Some(names) = check.names {
            args.push(Expr::array(names.into_iter().map(Expr::str).collect()));
        }
        args.push(Expr::str(format!("{place})")));
        args.extend(none.map(Expr::str));
        Some(js::StmtKind::Expr(Expr::call(Expr::helper(check.helper), args)).at(js::Span::NONE))
    }

    /// rust-js's bug of `item`'s checks, which it can't place, `why`: an
    /// internal error, never a JS-facing function left unchecked.
    fn unplaced(&self, item: DefId, why: &str) -> ! {
        self.tcx.dcx().span_bug(
            self.tcx.def_span(item),
            format!(
                "rust-js: `{}`'s checks of what JS gives it can't be placed: {why} (#2403)",
                self.tcx.def_path_str(item)
            ),
        )
    }
}

/// The checks a function starts with, `out`, those of a value and its
/// check alone folded into one `$checks(place, value, check, ..)` (#2403,
/// the lead's (A)), its place `function(a, b)` written once, where that's
/// shorter: of two or more, or of a whole value's, `$checkWhole`'s. One
/// that says its `None`, of a fieldless enum's names, or a callback's, which
/// its parameter is given back wrapped, stays a statement of its own, as a
/// lone leaf's does, `$checkU32(n, "double(n)")`.
fn folded(function: &str, out: Vec<js::Stmt>) -> Vec<js::Stmt> {
    // A check of `value` alone: its value, its check, and its parameter
    // inside `function(..)`.
    let alone = |stmt: &js::Stmt| -> Option<(Expr, Expr, String, bool)> {
        let js::StmtKind::Expr(call) = &stmt.kind else {
            return None;
        };
        let found = call.helper_call_of_any(super::checks::CHECKS)?;
        let place = |e: &Expr| -> Option<String> {
            let js::ExprKind::Str(text) = &e.kind else { return None };
            text.strip_prefix(function)?
                .strip_prefix('(')?
                .strip_suffix(')')
                .map(str::to_owned)
        };
        // A callback's, given back wrapped, a fieldless enum's, of its names, or
        // one that says its `None`: a statement of its own.
        if matches!(found.name(), "$checkCallback" | "$checkOneOf" | "$checks")
            || (found.name() != "$checkUnit" && found.given("none"))
        {
            return None;
        }
        let (value, at) = (found.arg("value")?.clone(), place(found.arg("where")?)?);
        match found.name() {
            "$checkWhole" => Some((value, found.arg("check")?.clone(), at, true)),
            name => Some((value, Expr::helper(name), at, false)),
        }
    };
    let found: Vec<_> = out.iter().map(alone).collect();
    let folding: Vec<_> = found.iter().flatten().collect();
    if folding.len() < 2 && !folding.iter().any(|(.., whole)| *whole) {
        return out;
    }
    let params: Vec<&str> = folding.iter().map(|(_, _, param, _)| param.as_str()).collect();
    let mut args = vec![Expr::str(format!("{function}({})", params.join(", ")))];
    for (value, check, ..) in &folding {
        args.push(value.clone());
        args.push(check.clone());
    }
    let call = js::StmtKind::Expr(Expr::call(Expr::helper("$checks"), args)).at(js::Span::NONE);
    // The folded call where its first was, the others as they were.
    let first = found.iter().position(Option::is_some).unwrap_or(0);
    let mut kept: Vec<js::Stmt> = (out.into_iter().zip(&found))
        .filter(|(_, found)| found.is_none())
        .map(|(stmt, _)| stmt)
        .collect();
    kept.insert(first.min(kept.len()), call);
    kept
}

/// `ty` of a signature's, each of its regions erased, its function's bound
/// ones too, as a query of its drops asks.
fn erased<'tcx>(tcx: rustc_middle::ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    rustc_middle::ty::fold_regions(tcx, ty, |_, _| tcx.lifetimes.re_erased)
}
