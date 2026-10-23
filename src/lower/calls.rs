//! A call and what calls share: `Call`, a call made, `apply`, calling a
//! function value with what's given, `catching`, a JS call that may throw
//! made a `Result` (ADR 0035), and `given_to_js`, a value made what JS
//! takes. Which call it is, MIR's lowering says
//! (`mir/calls.rs`, `mir/std_calls.rs`).

use super::recognition::Catching;

use super::FnCx;
use crate::js;
use crate::js::{Expr, Stmt, StmtKind};

use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// A call being lowered: what's called, which function that is, its
/// arguments, and whether its value is used.
#[derive(Clone, Copy)]
pub(super) struct Call<'c, 'tcx> {
    pub(super) def_id: DefId,
    pub(super) generic_args: ty::GenericArgsRef<'tcx>,
    /// Each argument's type.
    pub(super) tys: &'c [Ty<'tcx>],
    pub(super) span: Span,
}

/// Arguments given to JS: a trailing `undefined` is one it's given, which
/// no pass leaves out, as what it calls may count its arguments,
/// `Math.max(1, undefined)` (ADR 0330).
pub(super) fn given_to_js(args: &mut [Expr]) {
    for arg in args.iter_mut().rev() {
        if !matches!(arg.kind, js::ExprKind::Undefined) {
            break;
        }
        arg.kind = js::ExprKind::GivenUndefined;
    }
}

/// A call's arguments, each of type `tys`: a trailing `()`, which is no value,
/// is no argument, `resolve()` of `resolve(())`, as JS leaves one out; only
/// `arguments.length` tells (ADR 0441). One that does something to make is
/// kept, so it's still made.
pub(super) fn units_left_out(args: &mut Vec<Expr>, tys: &[ty::Ty<'_>]) {
    while args.len() <= tys.len()
        && let Some(last) = args.last()
        && tys[args.len() - 1].is_unit()
        && !last.has_effects()
    {
        args.pop();
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A JS call that says, in Rust, that it may throw (ADR 0035): one
    /// returning a `Result` runs in a `try`, `$try(() => f(x))`, and one
    /// returning a `Promise<Result<..>>` settles either way, `$settle(p)`.
    pub(super) fn catching(&mut self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, value: Expr) -> Expr {
        match self.recognition().catching(def_id, args) {
            Catching::Result => Expr::tried(value),
            Catching::PromiseResult => Expr::call(Expr::helper("$settle"), vec![value]),
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
                // Of variables alone: a part taken apart in turn isn't one.
                js::Pattern::Array(items) => (items.iter().enumerate())
                    .filter_map(|(i, item)| Some((i, item.as_ref()?)))
                    .map(|(i, item)| Some((item.name()?, Expr::index(arg.clone(), Expr::int(i as i128)))))
                    .collect(),
                js::Pattern::Object(fields, None) if fields.iter().all(|(_, _, default)| default.is_none()) => (fields
                    .iter())
                .map(|(key, field, _)| {
                    Some((
                        field.name()?,
                        super::binding_calls::keyed(arg.clone(), Expr::str(key.clone())),
                    ))
                })
                .collect(),
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
