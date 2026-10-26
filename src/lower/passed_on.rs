//! Whether an arrow that only passes on what it's given is the function it
//! calls, by where it's given (ADR 0605, amended): an array method's
//! callback (ADR 0279), an event's handler (ADR 0198), a binding's `FnOnce`
//! callback (ADR 0262's exception), and an arrow rust-js makes of a
//! function it takes as a value. Of a closure the program writes, what it
//! passes its call on to is MIR's fact, read once where it's made
//! (`forwarded_to`), which each taker reads.

use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

use super::{FnCx, fn_def};
use crate::js::{self, Expr, Stmt, StmtKind};

/// Where an arrow is given, which says whether it may be the function it
/// passes on to, and what it must pass on to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum Taker {
    /// An array method's callback, `xs.map(f)` (ADR 0279): the method
    /// gives the index and the array too, which a function of the crate's,
    /// or a closure, has no parameter for, and the callback's value is
    /// read. A binding's JS function may read them, `parseInt`'s radix.
    ArrayMethod,
    /// An event's handler, `onClick={f}` (ADR 0198): React ignores what it
    /// gives, and gives it the event, which a `dyn Fn` prop, a JS
    /// caller's, may read as the original's would. Of a component's
    /// optional handler passed on, `(e) => { if (f != null) { f(e); } }`,
    /// too.
    Handler,
    /// A binding's `FnOnce` callback, `p.then(f)`, `setTimeout(f)` (ADR
    /// 0262's exception): JS calls it once and never compares it, so its
    /// identity, the one thing that tells it from a fresh closure, can't
    /// matter. What it gives is read, and JS gives it what the binding
    /// gives the callback, which only a function of the crate's, or a
    /// closure, is known to read no more of than the closure gives it.
    OnceCallback,
    /// An arrow rust-js makes of a function it calls with the arrow's
    /// parameters alone, which what's given the arrow gives no more: a
    /// dictionary's `fmt` (ADR 0058), `next` or `clone`, a binding as a
    /// `fn` (ADR 0602), and the crate's function as a value with only the
    /// drops it needs. A namespace's or a dictionary's member too,
    /// `Math.max`, which reads no `this`.
    Made,
    /// A function as a value, `.map(str::len)`, a trait's method's,
    /// `ToString::to_string` (ADR 0538), an arrow of its call, which JS may
    /// give more than its parameters, an array method's index: its call's
    /// function only where that reads no more than it's given, not a
    /// runtime helper, which may read an optional parameter; an arrow made
    /// here of as many parameters too.
    FnValue,
}

/// What a closure MIR makes passes its call on to, where it's written: its
/// fact, which each taker reads (`forwarded_to`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) struct Forwarding {
    pub(in crate::lower) span: js::Span,
    pub(in crate::lower) callee: Callee,
}

/// What a closure's one call calls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum Callee {
    /// A function of the crate's that takes no rest arguments, or a
    /// closure: its JS takes the arguments Rust gives it alone.
    Own,
    /// A generic `impl Fn`'s, which a JS caller may give a JS function of
    /// more parameters than its type says.
    Generic,
    /// A `dyn Fn`, a prop's, a JS caller's function as its type says.
    Dyn,
}

impl Taker {
    /// The taker of array method `name`'s callback.
    pub(in crate::lower) fn array_method(name: &str) -> Option<Taker> {
        ["map", "filter", "some", "every", "find", "forEach"]
            .contains(&name)
            .then_some(Taker::ArrayMethod)
    }

    /// The taker of prop `name`'s value: an event's, `on` and a capital,
    /// alone, as anywhere else a function's identity, or what it gives,
    /// may be what's used.
    pub(in crate::lower) fn prop(name: &str) -> Option<Taker> {
        js::is_handler_name(name).then_some(Taker::Handler)
    }

    /// The taker of binding `def_id`'s `i`th argument: a callback JS calls
    /// once, a parameter of a type parameter whose only `Fn` bound is
    /// `FnOnce`, `impl FnOnce(T) -> U`, or a `Box<dyn FnOnce(..)>`. Not an
    /// `Fn` or `FnMut`'s, a listener's, which JS may keep and compare,
    /// `removeEventListener`'s.
    pub(in crate::lower) fn parameter(tcx: TyCtxt<'_>, def_id: DefId, i: usize) -> Option<Taker> {
        let ty = *tcx.fn_sig(def_id).skip_binder().skip_binder().inputs().get(i)?;
        let only_once = |id: DefId| tcx.fn_trait_kind_from_def_id(id) == Some(ty::ClosureKind::FnOnce);
        let once = match *ty.kind() {
            ty::Param(_) => {
                let mut bounds = (tcx.param_env(def_id).caller_bounds().iter())
                    .filter_map(|clause| clause.as_trait_clause())
                    .filter(|clause| clause.self_ty().skip_binder() == ty)
                    .map(|clause| clause.def_id())
                    .filter(|&id| tcx.fn_trait_kind_from_def_id(id).is_some())
                    .peekable();
                bounds.peek().is_some() && bounds.all(only_once)
            }
            ty::Adt(adt, args) if adt.is_box() => match args.type_at(0).kind() {
                ty::Dynamic(traits, ..) => traits.principal_def_id().is_some_and(only_once),
                _ => false,
            },
            _ => false,
        };
        once.then_some(Taker::OnceCallback)
    }

    /// Whether the arrow may pass on to a closure's call of `callee`.
    fn takes(self, callee: Callee) -> bool {
        match self {
            Taker::ArrayMethod => matches!(callee, Callee::Own | Callee::Generic),
            Taker::Handler => true,
            Taker::OnceCallback => callee == Callee::Own,
            Taker::Made | Taker::FnValue => false,
        }
    }
}

/// `g` of `f`, an arrow that passes on what it's given, in order, to `g`,
/// where `taker` may take `g` in its place: of a closure the program
/// writes, as its MIR fact in `forwarding` says what `g` is.
pub(in crate::lower) fn passed_on(f: &Expr, taker: Taker, forwarding: &[Forwarding]) -> Option<Expr> {
    if taker == Taker::Handler
        && let Some(handler) = optional_handler(f)
    {
        return Some(handler);
    }
    // What it may be a statement of, `() => { g(); }`: where nothing reads
    // what the arrow gives, or where `g` is the crate's or a closure, of
    // which MIR writes a statement only of a call that gives `()`, whose
    // JS gives `undefined` as the arrow does, or `!`, which never returns.
    let statement = match taker {
        Taker::Made | Taker::FnValue => false,
        _ => {
            let fact = (forwarding.iter()).find(|fact| !f.span.is_none() && fact.span == f.span)?;
            if !taker.takes(fact.callee) {
                return None;
            }
            taker == Taker::Handler || taker == Taker::OnceCallback
        }
    };
    let (callee, arity) = passed_to(f, statement)?;
    let named = matches!(callee.kind, js::ExprKind::Var(_) | js::ExprKind::Symbol(_));
    let takes = match taker {
        Taker::ArrayMethod | Taker::Handler | Taker::OnceCallback => named,
        Taker::Made => named || matches!(callee.kind, js::ExprKind::Member(..)),
        Taker::FnValue => match &callee.kind {
            js::ExprKind::Var(_) => callee.helper_name().is_none(),
            js::ExprKind::Arrow(taken, _) => {
                taken.len() == arity && taken.iter().all(|p| matches!(p, js::Pattern::Name(_)))
            }
            _ => false,
        },
    };
    takes.then(|| Expr {
        span: if f.span.is_none() { callee.span } else { f.span },
        ..callee.clone()
    })
}

/// What arrow `f` passes on what it's given to (`js::passes_on`), and how
/// many parameters it has.
fn passed_to(f: &Expr, statement: bool) -> Option<(&Expr, usize)> {
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    js::passes_on(params, body, statement).map(|callee| (callee, params.len()))
}

/// `f` of `(e) => { if (f != null) { f(e); } }`: a handler that calls
/// another, if there is one, with what it's given (ADR 0198).
fn optional_handler(value: &Expr) -> Option<Expr> {
    let js::ExprKind::Arrow(params, body) = &value.kind else {
        return None;
    };
    let [js::Pattern::Name(param)] = params.as_slice() else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::If(test, then, None),
            ..
        },
    ] = body.as_slice()
    else {
        return None;
    };
    let tested = js::null_test(test, true)?;
    let js::ExprKind::Var(handler) = &tested.kind else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::Expr(call),
            ..
        },
    ] = then.as_slice()
    else {
        return None;
    };
    let js::ExprKind::Call(callee, args) = &call.kind else {
        return None;
    };
    let given = matches!(args.as_slice(), [arg] if matches!(&arg.kind, js::ExprKind::Var(a) if a == param));
    let called = matches!(&callee.kind, js::ExprKind::Var(c) if c == handler);
    (given && called && handler != param).then(|| tested.clone())
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a closure's body, one call, calls, where it's a function of the
    /// crate's, a closure, a generic `impl Fn`'s or a `dyn Fn`'s: its
    /// `Forwarding`'s callee. `None` of a binding's, whose JS function
    /// may read more than it's given, and of a function of rest arguments,
    /// which reads them all.
    pub(in crate::lower) fn forwarded_to(&self, body: &crate::lower::Body<'tcx>) -> Option<Callee> {
        let thir = &body.thir;
        let mut call = super::body_queries::strip(thir, body.root()?);
        if let ExprKind::Block { block } = thir[call].kind
            && thir[block].stmts.is_empty()
            && let Some(value) = thir[block].expr
        {
            call = super::body_queries::strip(thir, value);
        }
        let ExprKind::Call { fun, ref args, .. } = thir[call].kind else {
            return None;
        };
        let (def_id, _) = fn_def(thir[fun].ty)?;
        match self.tcx.trait_of_assoc(def_id) {
            Some(fn_trait) if self.tcx.fn_trait_kind_from_def_id(fn_trait).is_some() => {
                match thir[args[0]].ty.peel_refs().kind() {
                    ty::Closure(..) => Some(Callee::Own),
                    ty::Param(_) => Some(Callee::Generic),
                    ty::Dynamic(..) => Some(Callee::Dyn),
                    ty::Adt(adt, args) if adt.is_box() && matches!(args.type_at(0).kind(), ty::Dynamic(..)) => {
                        Some(Callee::Dyn)
                    }
                    _ => None,
                }
            }
            _ => (def_id.is_local() && !super::bindings::is_binding(self.tcx, def_id) && !self.takes_rest(def_id))
                .then_some(Callee::Own),
        }
    }
}
