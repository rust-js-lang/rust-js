//! A call and what calls share: `Call`, a call made, `apply`, calling a
//! function value with what's given, `catching`, a JS call that may throw
//! made a `Result` (ADR 0035), and `given_to_js`, a value made what JS
//! takes. Which call it is, MIR's lowering says
//! (`mir/calls.rs`, `mir/std_calls.rs`).

use super::recognition::Catching;

use super::FnCx;
use crate::js;
use crate::js::{Expr, StmtKind};

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
    /// Of an argument that's an `Option`, whether it's `null` or a value
    /// already, MIR's `nulled`, where it's asked: a binding's nullable
    /// parameter's, or what a `Cell` is written (ADR 0772); else empty.
    pub(super) nulled: &'c [bool],
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
    /// A call that says, in Rust, that it may throw (ADR 0035): one
    /// returning a `Result` runs in a `try`, `$try(() => f(x))`, and one
    /// returning a `Promise<Result<..>>` settles either way, `$settle(p)`;
    /// a binding by its declared result, and a throwing function of the
    /// crate's, as `analysis::call_throws` says (ADR 0644).
    pub(super) fn catching(&mut self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, value: Expr) -> Expr {
        match super::analysis::call_throws(&self.recognition(), self.krate.throwing, def_id, args) {
            Catching::Result => Expr::tried(value),
            Catching::PromiseResult => Expr::call(Expr::helper("$settle"), vec![value]),
            Catching::Direct => value,
        }
    }

    /// Whether `def_id`, the crate's or a library's, takes JS's rest
    /// arguments: its last parameter a slice marked `#[rust_js::rest]`
    /// (ADR 0664), which its calls spread (`spread_rest`).
    pub(super) fn takes_rest(&self, def_id: DefId) -> bool {
        super::bindings::takes_rest(self.tcx, def_id) || self.krate.foreign.takes_rest(def_id)
    }

    /// A call of `def_id`'s arguments, its rest's spread where it takes
    /// one: `f(code, ...args)` (ADR 0664).
    pub(super) fn rest_given(&self, def_id: DefId, args: &mut Vec<Expr>) {
        if self.takes_rest(def_id) {
            spread_rest(args);
        }
    }
}

/// A call's last argument given as JS's rest arguments (ADRs 0221, 0640,
/// 0664, 0667): an array written out, a slice's or a tuple's, its items,
/// `f(a, b)` of `&[a, b]`, `log("a", 1)` of `&("a", 1)`, and none of `&[]`;
/// any other value, an array held, spread, `f(...items)`. Of a call of a
/// function taking a rest, a variadic binding, and a `fn` pointer whose
/// last parameter is a `&dyn js::Rest<T>`.
pub(super) fn spread_rest(args: &mut Vec<Expr>) {
    match args.pop() {
        Some(Expr {
            kind: js::ExprKind::Array(items),
            ..
        }) => args.extend(items),
        Some(last) => args.push(Expr::spread(last)),
        None => {}
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
