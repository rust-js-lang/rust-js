//! Whether an arrow that only passes on what it's given is the function it
//! calls, by where it's given (ADR 0605, amended): an array method's
//! callback (ADR 0279), an event's handler (ADR 0198), a binding's `FnOnce`
//! callback (ADR 0262's exception), and an arrow rust-js makes of a
//! function it takes as a value. Of a closure the program writes, what it
//! passes its call on to is MIR's fact, read once where it's made
//! (`forwarded_to`), which each taker reads.

use rustc_hir::def::DefKind;
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
    /// A closure that captures nothing as a `fn` (ADR 0125), React's
    /// `ContextOnlyDispatcher`'s `use_callback: |_, _|
    /// throw_invalid_hook_error()`: Rust compares no two `fn`s reliably,
    /// and what a JS caller gives it more, a function of the crate's never
    /// reads, so it may be the function of the crate's it calls with what
    /// it's given, or none of it (ADR 0605, amended).
    FnPointer,
    /// A function as a value, `.map(str::len)`, a trait's method's,
    /// `ToString::to_string` (ADR 0538), an arrow of its call, which JS may
    /// give more than its parameters, an array method's index: its call's
    /// function only where that reads no more than it's given, not a
    /// runtime helper, which may read an optional parameter; an arrow made
    /// here of as many parameters too.
    FnValue,
    /// Another callback a binding or a function of the crate's takes, an
    /// `Fn`'s, a listener's, `el.addEventListener("load", ping)`: only a
    /// closure that ignores what it's given (`Forwarding`'s `ignores`) is
    /// the function it calls, as any taker's is (rd196, rd212, ADR 0262's
    /// exception, the user's decision).
    Callback,
}

/// What a closure MIR makes passes its call on to, where it's written: its
/// fact, which each taker reads (`forwarded_to`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) struct Forwarding {
    pub(in crate::lower) span: js::Span,
    pub(in crate::lower) callee: Callee,
    /// Whether what it calls has a signature of its own, not the
    /// closure's: of other parameters or another result, which Rust can't
    /// give where the closure goes, `|_, _| throw_invalid_hook_error()`
    /// of a `fn() -> !` (ADR 0605, amended).
    pub(in crate::lower) adapts: bool,
    /// Whether what it calls is a function item, the crate's or a
    /// library's rust-js compiled, not a closure: a function that reads no
    /// `this`, which `binds` may bind (ADR 0605, amended).
    pub(in crate::lower) item: bool,
    /// Whether its arrow, as it's made, is that function bound to what it
    /// captured by value (`binds`), which `bound` writes once the body is
    /// tidied, where nothing calls the arrow in place any more.
    pub(in crate::lower) binds: bool,
    /// Whether it ignores what it's given, reading none of its parameters,
    /// and calls, with nothing, a function that reads no `this` and none
    /// of what it's given more (`Callee::Own`), giving what that gives:
    /// `move |_| ping()` of a bound function, `|_| layout_callback()`. A JS
    /// caller may call that function with any `this` and arguments, as it
    /// would the closure, so the closure is that function wherever it's
    /// given, `then(ping, ping)` (rd196, rd212, ADR 0262's exception, the
    /// user's decision): only its `.length` and its identity tell them
    /// apart.
    pub(in crate::lower) ignores: bool,
}

/// What a closure's one call calls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum Callee {
    /// A function of the crate's, or of a library rust-js compiled, that
    /// takes no rest arguments, a closure, a fn pointer only those fill
    /// (`FnPointers::rust_only`), or such a function bound, `function::bind`'s
    /// (`bound_own`): its JS takes the arguments Rust gives it alone, and
    /// reads no `this`.
    Own,
    /// A generic `impl Fn`'s, which a JS caller may give a JS function of
    /// more parameters than its type says.
    Generic,
    /// A `dyn Fn`, a prop's, a JS caller's function as its type says.
    Dyn,
    /// A binding's global function, not a method, of no rest arguments:
    /// JS's own, which a JS caller of the closure's `fn` may give more and
    /// read what it gives, the one taker that takes it, a `fn`'s, as the
    /// user decided (rd158, ADR 0262's exception).
    Host,
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
    /// once, a parameter of a type parameter called by `FnOnce` alone
    /// (`recognition::function_kind`), `impl FnOnce(T) -> U`, or a union's
    /// whose `Function` member is, `js::set_timeout`'s `H:
    /// IntoTimerHandler`, or a `Box<dyn FnOnce(..)>`. Another, an `Fn` or
    /// `FnMut`'s, a listener's, which JS may keep and compare,
    /// `removeEventListener`'s, is a `Callback`, which takes only a closure
    /// that ignores what it's given.
    pub(in crate::lower) fn parameter(tcx: TyCtxt<'_>, def_id: DefId, i: usize) -> Option<Taker> {
        let ty = *tcx.fn_sig(def_id).skip_binder().skip_binder().inputs().get(i)?;
        let only_once = |id: DefId| tcx.fn_trait_kind_from_def_id(id) == Some(ty::ClosureKind::FnOnce);
        let once = match *ty.kind() {
            ty::Param(_) => crate::lower::recognition::function_kind(tcx, def_id, ty) == Some(ty::ClosureKind::FnOnce),
            ty::Adt(adt, args) if adt.is_box() => match args.type_at(0).kind() {
                ty::Dynamic(traits, ..) => traits.principal_def_id().is_some_and(only_once),
                _ => false,
            },
            _ => false,
        };
        Some(match once {
            true => Taker::OnceCallback,
            false => Taker::Callback,
        })
    }

    /// Whether the arrow may pass on to a closure's call of `callee`.
    fn takes(self, callee: Callee) -> bool {
        match self {
            Taker::ArrayMethod => matches!(callee, Callee::Own | Callee::Generic),
            Taker::Handler => callee != Callee::Host,
            Taker::OnceCallback => callee == Callee::Own,
            Taker::FnPointer => matches!(callee, Callee::Own | Callee::Host),
            Taker::Made | Taker::FnValue | Taker::Callback => false,
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
            // One that ignores what it's given is the function it calls,
            // whoever takes it (rd196, rd212).
            if !taker.takes(fact.callee) && !fact.ignores {
                return None;
            }
            // A closure as a `fn` of a function Rust could have named there
            // is one the program wrote to be a function of its own.
            if taker == Taker::FnPointer && !fact.adapts {
                return None;
            }
            matches!(
                taker,
                Taker::Handler | Taker::OnceCallback | Taker::FnPointer | Taker::Callback
            )
        }
    };
    let (callee, arity) = passed_to(f, statement)?;
    let named = callee.is_named();
    let takes = match taker {
        Taker::ArrayMethod | Taker::Handler | Taker::OnceCallback | Taker::FnPointer | Taker::Callback => named,
        Taker::Made => named || matches!(callee.kind, js::ExprKind::Member(..)),
        Taker::FnValue => match &callee.kind {
            js::ExprKind::Var(_) => callee.helper_name().is_none(),
            // One read through its module's namespace, `Dep.noop`, which is
            // the binding its named import is (ADR 1018, amended).
            js::ExprKind::Member(..) => callee.is_named(),
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

/// Whether `e`, a call's argument, is one its JS leaves out where it's
/// last (ADRs 0443, 0441): a `None` or a `()`.
fn left_out(tcx: TyCtxt<'_>, thir: &rustc_middle::thir::Thir<'_>, e: rustc_middle::thir::ExprId) -> bool {
    match thir[super::body_queries::strip(thir, e)].kind {
        ExprKind::Tuple { ref fields } => fields.is_empty(),
        ExprKind::Adt(ref adt) => {
            Some(adt.adt_def.variant(adt.variant_index).def_id) == tcx.lang_items().option_none_variant()
        }
        _ => false,
    }
}

/// Whether `f`, a closure the program wrote, `(x, ..) => g(a, .., x, ..)`,
/// as it's made, passes on what it captured by value, `moved`, one at
/// least, or a constant, then each of its own parameters, in order, to
/// `g`, a function item (`fact`'s `item`): what `bound` writes as `g`
/// bound to them, React's `startTransition.bind(null, fiber, queue, true,
/// false)` (ADR 0605, amended). A `bind` reads what it binds once, where
/// it's made, as a closure takes what it captures by value; and what JS
/// gives it more than the closure's parameters, `g` reads no more of than
/// the arrow would, a function of the crate's taking no rest. Only `.name`
/// and `.length` tell them apart, as of the function passed on itself. Not
/// of a closure of no parameters of its own, `() => g(a)`, a thunk as JS
/// writes one, nor of one that captures nothing, `(x) => g("take", x)`.
pub(in crate::lower) fn binds(f: &Expr, fact: &Forwarding, moved: &[String]) -> bool {
    let taken = |e: &Expr| matches!(&e.kind, js::ExprKind::Var(v) if moved.contains(v));
    fact.item
        && fact.callee == Callee::Own
        && bound_parts(f)
            .is_some_and(|(_, bound)| bound.iter().any(taken) && bound.iter().all(|e| taken(e) || e.is_constant()))
}

/// `g.bind(null, a, ..)` of arrow `f`, `(x, ..) => g(a, .., x, ..)`, where
/// its fact in `forwarding` says it `binds`, and it's still of that shape.
pub(in crate::lower) fn bound(f: &Expr, forwarding: &[Forwarding]) -> Option<Expr> {
    (forwarding.iter()).find(|fact| fact.binds && !f.span.is_none() && fact.span == f.span)?;
    let (callee, bound) = bound_parts(f)?;
    let values = std::iter::once(Expr::null()).chain(bound.iter().cloned()).collect();
    Some(Expr {
        span: f.span,
        ..Expr::call(Expr::member(callee.clone(), "bind"), values)
    })
}

/// Of arrow `f`, `(x, ..) => g(a, .., x, ..)`, of one parameter at least
/// and one call of a named function, given values before each of its
/// parameters in order: `g` and those values, each a variable or a
/// constant.
fn bound_parts(f: &Expr) -> Option<(&Expr, &[Expr])> {
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    let call = match body.as_slice() {
        [
            Stmt {
                kind: StmtKind::Return(Some(call)) | StmtKind::Expr(call),
                ..
            },
        ] => call,
        _ => return None,
    };
    let js::ExprKind::Call(callee, args) = &call.kind else {
        return None;
    };
    let named = callee.is_named();
    let leading = (args.len().checked_sub(params.len())).filter(|&k| k > 0 && named && !params.is_empty())?;
    let (bound, given) = args.split_at(leading);
    let mut names = Vec::new();
    for (param, arg) in params.iter().zip(given) {
        match (param, &arg.kind) {
            (js::Pattern::Name(name), js::ExprKind::Var(v)) if v == name => names.push(name.as_str()),
            _ => return None,
        }
    }
    let value = |e: &Expr| matches!(e.kind, js::ExprKind::Var(_)) || e.is_constant();
    (bound.iter().all(value) && !names.iter().any(|name| callee.mentions_var(name))).then_some((&**callee, bound))
}

/// What arrow `f` passes on what it's given to (`js::passes_on`), and how
/// many parameters it has.
fn passed_to(f: &Expr, statement: bool) -> Option<(&Expr, usize)> {
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    js::passes_on(params, body, statement).map(|callee| (callee, params.len()))
}

/// The variable a closure captured that `e` reads, through its borrows
/// and what they point to, `ping` of `(&*ping)()`; not its own parameter.
fn captured(thir: &rustc_middle::thir::Thir<'_>, mut e: rustc_middle::thir::ExprId) -> Option<rustc_hir::HirId> {
    loop {
        match thir[super::body_queries::strip(thir, e)].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
            ExprKind::UpvarRef { var_hir_id, .. } => return Some(var_hir_id.0),
            _ => return None,
        }
    }
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
    /// which reads them all. And whether it adapts (`Forwarding`'s
    /// `adapts`): a function of the crate's whose signature isn't the
    /// closure's; and whether it's a function item (`item`). The closure is
    /// written at `span`.
    pub(in crate::lower) fn forwarded_to(&self, body: &crate::lower::Body<'tcx>, span: js::Span) -> Option<Forwarding> {
        let thir = &body.thir;
        let mut call = super::body_queries::strip(thir, body.root()?);
        // Of a statement alone, `{ js::set_timeout(callback, None); }`, what
        // it gives is none of the closure's: a host function's alone (rd158).
        let mut statement = false;
        if let ExprKind::Block { block } = thir[call].kind {
            match (&thir[block].stmts[..], thir[block].expr) {
                ([], Some(value)) => call = super::body_queries::strip(thir, value),
                (&[only], None) if let rustc_middle::thir::StmtKind::Expr { expr, .. } = thir[only].kind => {
                    call = super::body_queries::strip(thir, expr);
                    statement = true;
                }
                _ => {}
            }
        }
        let ExprKind::Call { fun, ref args, .. } = thir[call].kind else {
            return None;
        };
        // A fn pointer it captured, only the crate's functions fill (ADR
        // 0330), called with nothing: `move |_| f()`.
        if let ty::FnPtr(..) = thir[fun].ty.kind() {
            let own = self.krate.cell_borrows.pointers().rust_only(self.tcx, thir[fun].ty);
            return (own && args.is_empty() && captured(thir, fun).is_some()).then_some(Forwarding {
                span,
                callee: Callee::Own,
                adapts: true,
                item: false,
                binds: false,
                ignores: true,
            });
        }
        let (def_id, generic_args) = fn_def(thir[fun].ty)?;
        if statement && !super::bindings::is_binding(self.tcx, def_id) {
            return None;
        }
        // A call whose last argument is one its JS leaves out, a `None` or a
        // `()`, `fiber(lanes, None)`'s `fiber(lanes)`, isn't the function,
        // whose JS caller would fill it, `[5].map(fiber)`'s index (ADR 0443,
        // amended); but a host function's, the user's rd158 exception.
        let leaves_out =
            |args: &[rustc_middle::thir::ExprId]| args.last().is_some_and(|&last| left_out(self.tcx, thir, last));
        match self.tcx.trait_of_assoc(def_id) {
            Some(fn_trait) if self.tcx.fn_trait_kind_from_def_id(fn_trait).is_some() => {
                // What it calls, a variable it captured, not its own
                // parameter, `move |_| ping()`'s `ping`.
                let captured = captured(thir, args[0]);
                let ty = thir[args[0]].ty.peel_refs();
                let callee = match ty.kind() {
                    ty::Closure(..) => Callee::Own,
                    ty::Param(_) => Callee::Generic,
                    // One only the crate's functions fill (ADR 0330).
                    ty::FnPtr(..) if self.krate.cell_borrows.pointers().rust_only(self.tcx, ty) => Callee::Own,
                    // A bound function of the crate's (rd196).
                    ty::Dynamic(..) if captured.is_some_and(|var| self.bound_own(var)) => Callee::Own,
                    ty::Dynamic(..) => Callee::Dyn,
                    ty::Adt(adt, args) if adt.is_box() && matches!(args.type_at(0).kind(), ty::Dynamic(..)) => {
                        Callee::Dyn
                    }
                    _ => return None,
                };
                let given = match thir[super::body_queries::strip(thir, args[1])].kind {
                    ExprKind::Tuple { ref fields } if leaves_out(fields) => return None,
                    ExprKind::Tuple { ref fields } => fields.is_empty(),
                    _ => false,
                };
                Some(Forwarding {
                    span,
                    callee,
                    adapts: false,
                    item: false,
                    binds: false,
                    ignores: callee == Callee::Own && given && captured.is_some(),
                })
            }
            // A binding's global function, JS's own, called as a function,
            // not of `this` (rd158).
            _ if super::bindings::is_binding(self.tcx, def_id) => (matches!(
                super::bindings::js_form(self.tcx, def_id),
                super::bindings::JsForm::Call(_)
            ) && !super::bindings::is_method(self.tcx, def_id)
                && !super::bindings::is_variadic(self.tcx, def_id))
            .then(|| Forwarding {
                span,
                callee: Callee::Host,
                adapts: self.adapts(body.def_id, (def_id, generic_args)),
                item: false,
                binds: false,
                ignores: false,
            }),
            // The crate's, or a library's, whose JS rust-js compiled from
            // its Rust as it compiles the crate's (ADR 0605, amended). One
            // of rest arguments reads all JS gives it, so only where the
            // closure gathers them all too, and the arrow's shape then
            // says it spreads them all (`js::passes_on`).
            _ => ((def_id.is_local() || self.krate.foreign.is_library(def_id.krate))
                && !leaves_out(args)
                && (!self.takes_rest(def_id) || self.gathers_rest(body.def_id)))
            .then(|| Forwarding {
                span,
                callee: Callee::Own,
                adapts: self.adapts(body.def_id, (def_id, generic_args)),
                item: true,
                binds: false,
                ignores: args.is_empty(),
            }),
        }
    }

    /// Whether variable `var`, never changed, is a function of the crate's
    /// bound, `let ping = function::bind(on_unsuspend, Some(state), ())`:
    /// what reads no `this`, the one it's bound to, and none of what it's
    /// given more, as the function it binds doesn't (`own_target`). A JS
    /// function bound may read what it's given more.
    fn bound_own(&self, var: rustc_hir::HirId) -> bool {
        self.let_value(var).is_some_and(|init| self.bound_value(init, 0))
    }

    /// What variable `var` is, of a `let` that never changes it: its
    /// initializer.
    fn let_value(&self, var: rustc_hir::HirId) -> Option<&'tcx rustc_hir::Expr<'tcx>> {
        use rustc_hir as hir;
        let hir::Node::Pat(pat) = self.tcx.hir_node(var) else {
            return None;
        };
        let hir::PatKind::Binding(hir::BindingMode(hir::ByRef::No, rustc_hir::Mutability::Not), _, _, None) = pat.kind
        else {
            return None;
        };
        match self.tcx.parent_hir_node(var) {
            hir::Node::LetStmt(local) => local.init,
            _ => None,
        }
    }

    /// Whether `e` is a bound function of what `own_target` says, through
    /// variables that never change, at `depth`.
    fn bound_value(&self, e: &'tcx rustc_hir::Expr<'tcx>, depth: usize) -> bool {
        use rustc_hir as hir;
        if depth > 8 {
            return false;
        }
        let results = self.tcx.typeck(e.hir_id.owner.def_id);
        match e.peel_drop_temps().kind {
            hir::ExprKind::Path(ref qpath) => match results.qpath_res(qpath, e.hir_id) {
                hir::def::Res::Local(var) => self
                    .let_value(var)
                    .is_some_and(|init| self.bound_value(init, depth + 1)),
                _ => false,
            },
            hir::ExprKind::Call(callee, [target, ..]) => match *results.expr_ty_adjusted(callee).kind() {
                ty::FnDef(f, _) if super::recognition::is_function_bind(self.tcx, f) => {
                    self.own_target(target, depth + 1)
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// Whether `e`, a function `function::bind` binds, reads none of what
    /// it's given more than its parameters: a function of the crate's, or
    /// a library's rust-js compiled, of no rest arguments; a closure; a fn
    /// pointer only those fill (ADR 0330); a function `function::with_this`
    /// makes, whose `this` is the one bound; or one bound itself. Through
    /// variables that never change, and a thread-local's value,
    /// `ON_UNSUSPEND.with(|f| *f)`.
    fn own_target(&self, e: &'tcx rustc_hir::Expr<'tcx>, depth: usize) -> bool {
        use rustc_hir as hir;
        if depth > 8 {
            return false;
        }
        let tcx = self.tcx;
        let results = tcx.typeck(e.hir_id.owner.def_id);
        let ty = results.expr_ty(e);
        match *ty.kind() {
            ty::FnDef(f, _) => {
                return (f.is_local() || self.krate.foreign.is_library(f.krate))
                    && !super::bindings::is_binding(tcx, f)
                    && !self.takes_rest(f);
            }
            ty::Closure(..) => return true,
            ty::FnPtr(..) => return self.krate.cell_borrows.pointers().rust_only(tcx, ty),
            _ => {}
        }
        match e.peel_drop_temps().kind {
            hir::ExprKind::Path(ref qpath) => match results.qpath_res(qpath, e.hir_id) {
                hir::def::Res::Local(var) => self.let_value(var).is_some_and(|init| self.own_target(init, depth + 1)),
                _ => false,
            },
            hir::ExprKind::Call(callee, _) => match *results.expr_ty_adjusted(callee).kind() {
                ty::FnDef(f, _) if super::recognition::is_function_bind(tcx, f) => self.bound_value(e, depth + 1),
                ty::FnDef(f, _) => matches!(super::bindings::js_form(tcx, f), super::bindings::JsForm::WithThis),
                _ => false,
            },
            // A thread-local's value, `KEY.with(|f| *f)`: its initializer.
            hir::ExprKind::MethodCall(segment, key, [read], _) if segment.ident.name.as_str() == "with" => {
                let Some(with) = results.type_dependent_def_id(e.hir_id) else {
                    return false;
                };
                let is_key = tcx.opt_parent(with).is_some_and(|imp| {
                    matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                        ty::Adt(adt, _) if super::recognition::is_std_def(tcx, adt.did(), super::recognition::StdItem::LocalKey))
                });
                let hir::ExprKind::Path(ref key_path) = key.peel_borrows().kind else {
                    return false;
                };
                let hir::def::Res::Def(_, key) = results.qpath_res(key_path, key.hir_id) else {
                    return false;
                };
                let hir::ExprKind::Closure(closure) = read.kind else {
                    return false;
                };
                let read_body = tcx.hir_body(closure.body);
                let copied = matches!((read_body.params, read_body.value.peel_blocks().kind),
                    ([param], hir::ExprKind::Unary(hir::UnOp::Deref, of))
                        if matches!(of.kind, hir::ExprKind::Path(hir::QPath::Resolved(None, path))
                            if path.res == hir::def::Res::Local(param.pat.hir_id)));
                let Some(key) = key.as_local().filter(|_| is_key && copied) else {
                    return false;
                };
                let init = tcx.hir_body_owners().find(|&owner| {
                    tcx.def_kind(owner) == DefKind::Fn && super::analysis::init_of(tcx, owner) == Some(key)
                });
                init.is_some_and(|init| self.own_target(tcx.hir_body_owned_by(init).value.peel_blocks(), depth + 1))
            }
            _ => false,
        }
    }

    /// Whether closure `closure`'s last parameter is rest arguments as a
    /// value, `&dyn js::Rest<T>`, JS's rest parameter (ADR 0851), which
    /// gathers all JS gives it.
    fn gathers_rest(&self, closure: DefId) -> bool {
        let tcx = self.tcx;
        let ty::Closure(_, closure_args) = *tcx.type_of(closure).instantiate_identity().skip_normalization().kind()
        else {
            return false;
        };
        let sig = tcx.instantiate_bound_regions_with_erased(closure_args.as_closure().sig());
        let rest = |last: &ty::Ty<'tcx>| super::bindings::is_rest_value(tcx, *last);
        match sig.inputs() {
            [tupled] => match tupled.kind() {
                ty::Tuple(items) => items.last().is_some_and(rest),
                _ => false,
            },
            _ => false,
        }
    }

    /// Whether `callee`'s signature isn't closure `closure`'s, as Rust
    /// would have it where the closure goes: other parameters, or another
    /// result, `!` of a `u32`'s.
    fn adapts(&self, closure: DefId, (callee, args): (DefId, ty::GenericArgsRef<'tcx>)) -> bool {
        let tcx = self.tcx;
        let ty::Closure(_, closure_args) = *tcx.type_of(closure).instantiate_identity().skip_normalization().kind()
        else {
            return true;
        };
        let theirs = tcx.instantiate_bound_regions_with_erased(closure_args.as_closure().sig());
        let called =
            tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(callee).instantiate(tcx, args).skip_normalization());
        let inputs = match theirs.inputs() {
            [tupled] => match tupled.kind() {
                ty::Tuple(items) => items.to_vec(),
                _ => return true,
            },
            other => other.to_vec(),
        };
        let erased = |ty: ty::Ty<'tcx>| tcx.erase_and_anonymize_regions(ty);
        inputs.len() != called.inputs().len()
            || (inputs.iter().zip(called.inputs())).any(|(&a, &b)| erased(a) != erased(b))
            || erased(theirs.output()) != erased(called.output())
    }
}
