//! A call and what calls share: `Call`, a call made, `apply`, calling a
//! function value with what's given, `catching`, a JS call that may throw
//! made a `Result` (ADR 0035), and `given_to_js`, a value made what JS
//! takes. Which call it is, MIR's lowering says
//! (`mir/calls.rs`, `mir/std_calls.rs`).

use super::recognition::Catching;

use super::{FnCx, R};
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
    /// Of a call of a `RefCell`'s, `x.f.borrow()`, or of `drop(guard)` of a
    /// guard of one, whether it's a struct's field's no borrow of which can
    /// meet another, never checked (ADR 0863, amended).
    pub(super) quiet: bool,
    /// Of a call that writes a thread-local's cell, whether the thread-local
    /// is `#[rust_js::nullable]`, its `None` stored `null` (ADR 0586,
    /// amended).
    pub(super) null_for_none: bool,
    pub(super) span: Span,
}

/// What MIR says of a call's arguments that their values don't, as `Call`
/// carries it: `nulled`, `quiet` and `null_for_none`.
#[derive(Clone, Copy, Default)]
pub(super) struct Given<'c> {
    pub(super) nulled: &'c [bool],
    pub(super) quiet: bool,
    pub(super) null_for_none: bool,
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

    /// Refused, a call each owner says JS can't make as Rust does: of a
    /// string's bytes' pointer (`pointers.rs`), rest arguments as a value
    /// given as a type parameter's (`check_rest_unspread`), and a counted
    /// `Rc` given to a binding (`std_types/rc.rs`).
    pub(super) fn check_call(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        destination: rustc_middle::mir::Place<'tcx>,
        span: Span,
    ) -> R<()> {
        self.check_text_pointer(body, (def_id, generic_args), destination, span)?;
        self.check_rest_unspread(def_id, generic_args, span)?;
        self.check_counted_given_to_js(def_id, generic_args, span)
    }

    /// Refused, a call giving rest arguments as a value, `&dyn js::Rest<T>`,
    /// as a type parameter's value, `call::<&dyn Rest<T>, _>(f, args)`:
    /// what its body calls with one, `f(a)` of an `F: Fn(A)`, gives JS's
    /// rest parameter one array, where only a call through a type that says
    /// so spreads it (ADR 0851). A binding's `impl Rest<T>` takes them spread.
    fn check_rest_unspread(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>, span: Span) -> R<()> {
        if !super::bindings::is_binding(self.tcx, def_id)
            && generic_args
                .types()
                .any(|ty| super::bindings::is_rest_value(self.tcx, ty))
        {
            let refused = super::refusal::Refused::Because(
                "rest arguments as a value, `&dyn js::Rest<T>`, as a type parameter's value",
                "a call of an `F: Fn(A)` would give them as one array",
            );
            return Err(super::refusal::refuse(self.tcx, span, refused));
        }
        Ok(())
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
    // One argument read once, first: a thread-local made where it's first
    // used, `x()` (ADR 0037, amended), given a closure that reads it, its
    // `value` or a path of it, `|l| l.get()`: the path of the call.
    if let ([js::Pattern::Name(param)], [arg]) = (params, args)
        && matches!(&arg.kind, js::ExprKind::Call(callee, given)
            if given.is_empty() && matches!(callee.kind, js::ExprKind::Var(_) | js::ExprKind::Symbol(_)))
        && path_of(value, param)
    {
        return value.substitute(&|name: &str| (name == param).then(|| arg.clone()));
    }
    None
}

/// Whether `value` is `param` or a path of its fields, `param.value`, read
/// the same however often it's read.
fn path_of(value: &Expr, param: &str) -> bool {
    match &value.kind {
        js::ExprKind::Var(name) => name == param,
        js::ExprKind::Member(of, _) => path_of(of, param),
        js::ExprKind::Index(of, at) => path_of(of, param) && at.reads_same(),
        _ => false,
    }
}

/// Whether a call of `def_id` gives `null` for `None`, never `undefined`:
/// a function, a binding too, whose result is `#[rust_js::nullable]`, its
/// body writing it so (ADRs 0578, 0638). Not a trait's method, whose impl
/// the call may run, which says nothing of it (ADR 0841, amended).
pub(super) fn gives_null_for_none(tcx: rustc_middle::ty::TyCtxt<'_>, def_id: rustc_span::def_id::DefId) -> bool {
    super::bindings::nullable_result(tcx, def_id) && tcx.trait_of_assoc(def_id).is_none()
}

/// `value`, what a call of `f` gave, marked never `undefined` where `f` is
/// a function item that `gives_null_for_none`: `and_then(name_of)`'s, which
/// a nullable place stores as it is, as `map`'s `Some` (ADR 0841, amended).
/// A closure's value isn't asked.
pub(super) fn given_by<'tcx>(tcx: ty::TyCtxt<'tcx>, f: Ty<'tcx>, value: Expr) -> Expr {
    match f.kind() {
        ty::FnDef(def_id, _) if gives_null_for_none(tcx, *def_id) => value.defined(),
        _ => value,
    }
}
