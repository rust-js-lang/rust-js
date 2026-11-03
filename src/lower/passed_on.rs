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
use crate::lower::recognition::StdItem;
use std::collections::{HashMap, HashSet};

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
    /// A closure where it's made, wherever its value goes, a `let`'s, a
    /// field's, a box's, and another callback a binding or a function of
    /// the crate's takes, an `Fn`'s, a listener's,
    /// `el.addEventListener("load", ping)`: only one that's the function it
    /// calls whoever takes it, that ignores what it's given (`Forwarding`'s
    /// `ignores`) or passes it all on (`eta`) (rd196, rd212, ADR 0262's
    /// exception, the user's decision).
    Closure,
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
    /// Whether it passes all it's given, in order, to a namespace's
    /// function through the value it's called on, which reads no `this`
    /// (ADR 1037), and gives back what that gives: `move |f, a|
    /// renderer.batched_updates(f, a)`, which Rust can name no other way.
    /// Then it's that function wherever it's made, `renderer.batchedUpdates`,
    /// read there, as Rust's `move` closure holds that value (the user's
    /// decision, 2026-10-31, ADR 0262's exception, as `ignores`): only its
    /// `.length`, its identity and what more a JS caller gives, which that
    /// function never reads, tell them apart. Not one of a function Rust
    /// could have named there, which the program wrote to be a function of
    /// its own, React's DEV dispatchers' `readContext(context) { return
    /// readContext(context); }` (ADR 0605, amended).
    pub(in crate::lower) eta: bool,
    /// Whether what it calls is a variable that never changes, of a
    /// function that reads no `this` (`bound_own`): a bound one, or a local
    /// one `function::with_this` makes of a closure that ignores its `this`,
    /// React's `handleNavigateComplete`, which a `let` of the closure reads
    /// in its place (`held_functions`, rd284b).
    pub(in crate::lower) fixed: bool,
    /// Whether its call is a statement, `{ preload(href, ..); }`, of a
    /// function item that gives `()` or never returns (rd660): never bound
    /// (`binds`), which the user decided of a closure that gives back its
    /// call, as upstream's `.then((s) => { onActionSuccess(queue, node, s);
    /// })` keeps its arrow.
    pub(in crate::lower) statement: bool,
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
    /// `removeEventListener`'s, is a `Closure`'s, which takes only a closure
    /// that's the function it calls wherever it's given.
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
            false => Taker::Closure,
        })
    }

    /// Whether the arrow may pass on to a closure's call of `callee`.
    fn takes(self, callee: Callee) -> bool {
        match self {
            Taker::ArrayMethod => matches!(callee, Callee::Own | Callee::Generic),
            Taker::Handler => callee != Callee::Host,
            Taker::OnceCallback => callee == Callee::Own,
            Taker::FnPointer => matches!(callee, Callee::Own | Callee::Host),
            Taker::Made | Taker::FnValue | Taker::Closure => false,
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
    let mut eta = false;
    let statement = match taker {
        Taker::Made | Taker::FnValue => false,
        _ => {
            let fact = (forwarding.iter()).find(|fact| !f.span.is_none() && fact.span == f.span)?;
            // One that ignores what it's given, or passes it all on, is the
            // function it calls, whoever takes it (rd196, rd212, the user's
            // decisions).
            eta = fact.eta;
            if !taker.takes(fact.callee) && !fact.ignores && !fact.eta {
                return None;
            }
            // A closure as a `fn` of a function Rust could have named there
            // is one the program wrote to be a function of its own.
            if taker == Taker::FnPointer && !fact.adapts {
                return None;
            }
            matches!(
                taker,
                Taker::Handler | Taker::OnceCallback | Taker::FnPointer | Taker::Closure
            )
        }
    };
    let (callee, arity) = passed_to(f, statement)?;
    // Of one that passes it all on, a namespace's function through the
    // value it's called on, `renderer.batchedUpdates`, read where it's made,
    // as Rust's `move` closure holds that value.
    let named = callee.is_named()
        || eta
            && matches!(&callee.kind, js::ExprKind::Member(of, _) if matches!(of.kind, js::ExprKind::Var(_) | js::ExprKind::Symbol(_)));
    let takes = match taker {
        Taker::ArrayMethod | Taker::Handler | Taker::OnceCallback | Taker::FnPointer | Taker::Closure => named,
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

/// Of `method`, whose body is `thir`, a `#[rust_js::namespace]`'s method
/// (ADR 1037) or a constructor function's prototype's (ADR 0761): the
/// function of the crate's that it only passes its parameters on to, past
/// its `self`, in order, each as it is, `react_fiber_work_loop::
/// batched_updates(f, a)`, of its own parameters' types, its result's and
/// generic parameters, and those as it's given them (rd292b, ADR 1037
/// amended). It's that function, as a closure that only passes on what
/// it's given is the function it calls (rd196), where the two take what JS
/// gives them alike (`pipeline::reached::forwarded`). A namespace's, of
/// another module's function: its module re-exports it by the method's
/// name, `export { batchedUpdates } from "./ReactFiberWorkLoop.js"`; of its
/// own module's, the function its module exports by the method's name,
/// `exported` (#566); and a call of it on the namespace is that function's.
/// Through any other value, `NoopRenderer.batchedUpdates(..)`, it's what
/// that value has, as any of its methods is. A prototype's, of a function
/// of its own module: the prototype's property is that function,
/// `ReactDOMHydrationRoot.prototype.unstable_scheduleHydration =
/// scheduleHydration;`, where its JS is that call alone (rd715,
/// `pipeline::reached::passed_on_methods`).
pub(in crate::lower) fn forwarded_function<'tcx>(
    tcx: TyCtxt<'tcx>,
    method: DefId,
    thir: &rustc_middle::thir::Thir<'tcx>,
    exported: &dyn Fn(DefId) -> Option<String>,
) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
    let prototype = super::classes::member(tcx, method) == Some(super::classes::Member::Method)
        && (tcx.inherent_impl_of_assoc(method))
            .and_then(|imp| super::classes::class_of(tcx, method).filter(|_| imp.is_local()))
            .is_some_and(|class| !super::classes::is_es_class(tcx, class));
    if super::classes::member(tcx, method) != Some(super::classes::Member::Namespace) && !prototype {
        return None;
    }
    let (target, args) = super::body_queries::forwards_to(thir, 1)?;
    let module = |id: DefId| id.as_local().map(|local| tcx.parent_module_from_def_id(local));
    // Of a namespace's own module, the function exported by the method's
    // name, which the namespace's property is (#566).
    let own_unexported = !prototype
        && module(target) == module(method)
        && (!tcx.visibility(target).is_public()
            || exported(target).as_deref() != Some(super::classes::property(tcx, method).as_str()));
    // A prototype's, of its own module's function, a declaration its
    // module's load reads, hoisted, where another module's may not be
    // loaded yet in a cycle (rd715).
    let elsewhere = prototype && module(target) != module(method);
    if tcx.def_kind(target) != DefKind::Fn
        || super::bindings::is_binding(tcx, target)
        || !target.is_local()
        || own_unexported
        || elsewhere
    {
        return None;
    }
    // Its own generic parameters, in order, its regions aside.
    let own = ty::GenericArgs::identity_for_item(tcx, method);
    let kept = |args: ty::GenericArgsRef<'tcx>| args.iter().filter(|arg| arg.as_region().is_none()).collect::<Vec<_>>();
    if kept(args) != kept(own) {
        return None;
    }
    // Its parameters, as the function's of its own.
    let erased =
        |sig: ty::PolyFnSig<'tcx>| tcx.erase_and_anonymize_regions(tcx.instantiate_bound_regions_with_erased(sig));
    let mine = erased(tcx.fn_sig(method).instantiate_identity().skip_normalization());
    let theirs = erased(tcx.fn_sig(target).instantiate(tcx, args).skip_normalization());
    (mine.inputs().get(1..) == Some(theirs.inputs()) && mine.output() == theirs.output()).then_some((target, args))
}

/// Of `function`, a free function of the crate's of no type or const
/// parameters, whose body is `thir`: the plain function, of another of
/// the crate's modules or a library's, that it only passes its parameters
/// on to, in order, each as it is, at one instantiation, giving back what
/// that gives: `react_flight_action_server::decode_action::<ServerManifest>
/// (body, server_manifest)` (rd575, ADR 1064). Its parameters' types and
/// its result's are that instantiation's, so no conversion is the call's
/// to make; not a method, a closure, a constructor or a binding. It's a
/// re-export of that function where its JS is that call alone, of the
/// function's own JS, which takes no more (`pipeline::forwarders::found`,
/// `forwarded_call`).
pub(in crate::lower) fn forwarder<'tcx>(
    tcx: TyCtxt<'tcx>,
    function: DefId,
    thir: &rustc_middle::thir::Thir<'tcx>,
) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
    let generic = (tcx.generics_of(function).own_params.iter())
        .any(|param| !matches!(param.kind, ty::GenericParamDefKind::Lifetime));
    if tcx.def_kind(function) != DefKind::Fn || generic {
        return None;
    }
    let (target, args) = super::body_queries::forwards_to(thir, 0)?;
    let module = |id: DefId| id.as_local().map(|local| tcx.parent_module_from_def_id(local));
    if tcx.def_kind(target) != DefKind::Fn
        || super::bindings::is_binding(tcx, target)
        || (target.is_local() && module(target) == module(function))
    {
        return None;
    }
    // Its parameters and result, the function's at what it's given.
    let erased =
        |sig: ty::PolyFnSig<'tcx>| tcx.erase_and_anonymize_regions(tcx.instantiate_bound_regions_with_erased(sig));
    let mine = erased(tcx.fn_sig(function).instantiate_identity().skip_normalization());
    let theirs = erased(tcx.fn_sig(target).instantiate(tcx, args).skip_normalization());
    (mine.inputs_and_output == theirs.inputs_and_output).then_some((target, args))
}

/// What `function`, the JS of a `forwarder`, calls, where it's that call
/// alone, given its parameters, in order, and nothing more, of a function
/// it names: `decodeAction$1` of `function decodeAction(body,
/// serverManifest) { return decodeAction$1(body, serverManifest); }`, and
/// of what gives nothing, `statement`, `ping(n);` too. Any conversion its
/// call needs, a box, a dictionary, a `Some`'s or a result's, is JS of its
/// own, and it stays a function; so does one its `written` Rust has a
/// comment in, which a re-export has no place for, one kept out of line
/// (ADR 0649), and one its module doesn't export (rd575, ADR 1064).
pub(in crate::lower) fn forwarded_call<'f>(
    function: &'f js::Function,
    statement: bool,
    written: Option<&str>,
) -> Option<&'f Expr> {
    let commented = written.is_none_or(|text| text.contains("//") || text.contains("/*"));
    if !function.export || function.noinline || commented {
        return None;
    }
    js::passes_on(&function.params, &function.body, statement).filter(|callee| callee.is_named())
}

/// In `body`, `const onSuccess = handleNavigateComplete;` of a closure
/// that's the function of the program's it calls where it's made, one that
/// ignores what it's given (`Forwarding`'s `ignores`):
/// the function, read where the variable is, the variable gone, as a `let`
/// names nothing more (rd284, ADR 0605, amended). A function of the
/// program's, a declaration or an import, is never assigned.
pub(in crate::lower) fn held_functions(body: &mut Vec<Stmt>, forwarding: &[Forwarding]) {
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            if let StmtKind::Const(name, value) = &stmts[i].kind
                && matches!(value.kind, js::ExprKind::Var(_) | js::ExprKind::Symbol(_))
                && (forwarding.iter()).any(|fact| {
                    !value.span.is_none() && fact.span == value.span && (fact.fixed || value.facts.has(js::Known::Item))
                })
            {
                let (name, value) = (name.clone(), value.clone());
                stmts.remove(i);
                js::each_expr_mut(&mut stmts[i..], &mut |e| {
                    if matches!(&e.kind, js::ExprKind::Var(v) if *v == name) {
                        *e = Expr {
                            span: e.span,
                            ..value.clone()
                        };
                    }
                });
                continue;
            }
            i += 1;
        }
    });
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
/// and `.length` tell them apart, as of the function passed on itself. So
/// of a closure of no parameters of its own too, alien-signals'
/// `computedOper.bind(null, this$)` (#502), and of one of rest arguments
/// it spreads on to a function's rest, `signalOper.bind(null, this$)`. Not
/// of one that captures nothing, `(x) => g("take", x)`, nor of one that
/// borrows what it passes, whose place a borrowed capture is (rd660); nor
/// of a thunk its maker says is `kept` an arrow: one `handed_on` as a
/// callback, which JS writes in place, Fizz's `scheduleWork(() =>
/// performWork(request))`, or one of parameters JS leaves out, `move |_|
/// ping_task(&request, task)`, which ignores what it's given.
pub(in crate::lower) fn binds(f: &Expr, fact: &Forwarding, moved: &[String], kept: bool) -> bool {
    let taken = |e: &Expr| matches!(&e.kind, js::ExprKind::Var(v) if moved.contains(v));
    fact.item
        && fact.callee == Callee::Own
        && !fact.statement
        && !kept
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

/// Of arrow `f`, `(x, ..) => g(a, .., x, ..)`, of one call of a named
/// function given values before each of its parameters in order, rest
/// arguments spread on, `(...xs) => g(a, ...xs)`, or before none, `() =>
/// g(a)` (`js::passes_on_after`): `g` and those values, each a variable or
/// a constant (`binds` asks one at least to be what the closure took).
fn bound_parts(f: &Expr) -> Option<(&Expr, &[Expr])> {
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    let (callee, bound) = js::passes_on_after(params, body, true)?;
    let value = |e: &Expr| matches!(e.kind, js::ExprKind::Var(_)) || e.is_constant();
    (callee.is_named() && bound.iter().all(value)).then_some((callee, bound))
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
/// and what they point to, `ping` of `(&*ping)()`, a `Box`'s or an `Rc`'s
/// too, `on_end` of `on_end()` of an `Rc<dyn Fn()>`; not its own parameter.
fn captured<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &rustc_middle::thir::Thir<'tcx>,
    mut e: rustc_middle::thir::ExprId,
) -> Option<rustc_hir::HirId> {
    let deref = |fun: rustc_middle::thir::ExprId| {
        fn_def(thir[fun].ty).is_some_and(|(id, args)| super::recognition::is_rc_deref(tcx, id, args))
    };
    loop {
        match thir[super::body_queries::strip(thir, e)].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
            ExprKind::Call {
                fun,
                ref args,
                from_hir_call: false,
                ..
            } if args.len() == 1 && deref(fun) => e = args[0],
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
    /// Whether the closure written at `span` is made an `Rc`'s or an
    /// `Arc`'s, `Rc::new(|| one())`, whose pointer `Rc::ptr_eq` compares
    /// (ADR 0959), where the crate compares an `Rc<dyn Fn>` it doesn't
    /// count (`Counted::compared_uncounted`, rd313): a function of its own
    /// wherever it's made, as two such are two `Rc`s to Rust. Where it
    /// compares none, nothing asks an uncounted one's identity, and it's
    /// the function it passes on to, as a `Box`'s is; a counted one's is
    /// its box's.
    pub(in crate::lower) fn counted_closure(&self, span: rustc_span::Span) -> bool {
        if !self.krate.counted.compared_uncounted {
            return false;
        }
        let thir = self.frame.thir;
        thir.exprs.iter().any(|e| {
            let ExprKind::Call { fun, ref args, .. } = e.kind else {
                return false;
            };
            let counted = fn_def(thir[fun].ty)
                .and_then(|(id, _)| self.tcx.impl_of_assoc(id))
                .is_some_and(|imp| {
                    let of = self.tcx.type_of(imp).instantiate_identity().skip_normalization();
                    self.is_std_type(of, StdItem::Rc) || self.is_std_type(of, StdItem::Arc)
                });
            counted
                && args.iter().any(|&a| {
                    let made = &thir[super::body_queries::strip(thir, a)];
                    matches!(made.kind, ExprKind::Closure(..)) && made.span == span
                })
        })
    }

    /// Whether the closure written at `span` is handed on as a callback: a
    /// call's argument, `schedule_work(move || perform_work(&request))`,
    /// boxed, `Box::new(move || start_flowing(&request, ..))`, or lent,
    /// `&move || ..`. JS writes each in place, `() => performWork(request)`,
    /// where a thunk isn't bound (`binds`, #502). Not one an `Rc` or an
    /// `Arc` is made of, a function its holders share, React's Flight
    /// client's `close.bind(null, response)`, nor one returned or held as
    /// it is, alien-signals' `computedOper.bind(node)`.
    pub(in crate::lower) fn handed_on(&self, span: rustc_span::Span) -> bool {
        let thir = self.frame.thir;
        // What's given, a block's value too, `Box::new({ let container =
        // Rc::clone(&container); move || get_children(Some(&container)) })`,
        // and what one wrapper holds, `AssertUnwindSafe(move || ..)`.
        let closure = |mut a: rustc_middle::thir::ExprId| loop {
            let made = &thir[super::body_queries::strip(thir, a)];
            match made.kind {
                ExprKind::Block { block } if let Some(value) = thir[block].expr => a = value,
                ExprKind::Adt(ref adt) if let [only] = &adt.fields[..] => a = only.expr,
                _ => return matches!(made.kind, ExprKind::Closure(..)) && made.span == span,
            }
        };
        thir.exprs.iter().any(|e| match e.kind {
            ExprKind::Call { fun, ref args, .. } => {
                args.iter().any(|&a| closure(a))
                    && !fn_def(thir[fun].ty).is_some_and(|(id, _)| super::recognition::is_rc_new(self.tcx, id))
            }
            ExprKind::Borrow { arg, .. } => closure(arg),
            _ => false,
        })
    }

    /// What a closure's body, one call, calls, where it's a function of the
    /// crate's, a closure, a generic `impl Fn`'s or a `dyn Fn`'s: its
    /// `Forwarding`'s callee. `None` of a binding's, whose JS function
    /// may read more than it's given, and of a function of rest arguments,
    /// which reads them all. And whether it adapts (`Forwarding`'s
    /// `adapts`): a function of the crate's whose signature isn't the
    /// closure's; and whether it's a function item (`item`). The closure is
    /// written at `span`, made at `made_at`.
    pub(in crate::lower) fn forwarded_to(
        &self,
        body: &crate::lower::Body<'tcx>,
        (span, made_at): (js::Span, rustc_span::Span),
    ) -> Option<Forwarding> {
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
        // A namespace's method's call whose result only `.flatten()` reads,
        // which its JS leaves out, `Reconciler.flush_sync(f).flatten()`'s
        // `flushSync(f)`, is that call (rd324, ADR 1037): of the one answer
        // its lowering asks, each of its calls in the body so.
        if let ExprKind::Call { fun, ref args, .. } = thir[call].kind
            && let [flattened] = args[..]
            && let Some((flatten, generic)) = fn_def(thir[fun].ty)
            && self.recognition().classify(flatten, generic) == Some(super::Std::OptionFlatten)
            && let inner = super::body_queries::strip(thir, flattened)
            && let ExprKind::Call { fun: method, .. } = thir[inner].kind
            && let Some(called) = fn_def(thir[method].ty)
            && let Some(mir) = &body.mir
            && super::analysis::flattened_by_name(&self.recognition(), &mir.body, called).is_some()
        {
            call = inner;
        }
        // One whose call of a function value it captured isn't all it is,
        // which converts what it's given or gives back (`converting`).
        if let Some(fact) = self.converting(body, (span, made_at), call) {
            return Some(fact);
        }
        let ExprKind::Call { fun, ref args, .. } = thir[call].kind else {
            return None;
        };
        // A fn pointer it captured, only the crate's functions fill (ADR
        // 0330), called with nothing: `move |_| f()`.
        if let ty::FnPtr(..) = thir[fun].ty.kind() {
            let own = self.krate.cell_borrows.pointers().rust_only(self.tcx, thir[fun].ty);
            return (own && args.is_empty() && captured(self.tcx, thir, fun).is_some()).then_some(Forwarding {
                span,
                callee: Callee::Own,
                adapts: true,
                item: false,
                binds: false,
                ignores: true,
                eta: false,
                fixed: false,
                statement: false,
            });
        }
        let (def_id, generic_args) = fn_def(thir[fun].ty)?;
        // A statement of a function item's call that gives `()` or never
        // returns, `{ preload(href, ..); }`, whose JS gives `undefined` as
        // the closure's does (rd660); of one that gives a value, the closure
        // discards it, which the function wouldn't.
        let gives_nothing = || {
            let sig = self
                .tcx
                .fn_sig(def_id)
                .instantiate(self.tcx, generic_args)
                .skip_normalization();
            let output = sig.skip_binder().output();
            output.is_unit() || output.is_never()
        };
        if statement && !super::bindings::is_binding(self.tcx, def_id) && !gives_nothing() {
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
                let (callee, captured, dyn_fn) = self.function_value(thir, args)?;
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
                    eta: false,
                    fixed: dyn_fn && callee == Callee::Own,
                    statement: false,
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
                eta: false,
                fixed: false,
                statement,
            }),
            // The crate's, or a library's, whose JS rust-js compiled from
            // its Rust as it compiles the crate's (ADR 0605, amended). One
            // of rest arguments reads all JS gives it, so only where the
            // closure gathers them all too, and the arrow's shape then
            // says it spreads them all (`js::passes_on`).
            // One that reads `js::arguments()` or `js::this()` reads what
            // JS gives it, never the function (rd332, rd683).
            _ => ((def_id.is_local() || self.krate.foreign.is_library(def_id.krate))
                && !leaves_out(args)
                && !self.reads_its_call(def_id)
                && (!self.takes_rest(def_id) || self.gathers_rest(body.def_id)))
            .then(|| Forwarding {
                span,
                callee: Callee::Own,
                adapts: self.adapts(body.def_id, (def_id, generic_args)),
                item: true,
                binds: false,
                ignores: args.is_empty(),
                // Or a scope struct's method, its function's function
                // closing over that value, where nothing reads a closure's
                // identity, which the two share (ADR 1043, amended, #500).
                eta: super::classes::member(self.tcx, def_id) == Some(super::classes::Member::Namespace)
                    || self.krate.scopes.of_method(def_id).is_some() && !self.krate.counted.points_at_functions,
                fixed: false,
                statement,
            }),
        }
    }

    /// What `args`, those of a call through an `Fn` trait, calls: a
    /// function value, a closure, a generic `impl Fn`'s, a `dyn Fn`'s or a
    /// fn pointer only the crate's functions fill; the variable it captured
    /// that the value is, if it is one, not its own parameter, `move |_|
    /// ping()`'s `ping`; and whether it's a `dyn Fn`.
    fn function_value(
        &self,
        thir: &rustc_middle::thir::Thir<'tcx>,
        args: &[rustc_middle::thir::ExprId],
    ) -> Option<(Callee, Option<rustc_hir::HirId>, bool)> {
        let captured = captured(self.tcx, thir, args[0]);
        let ty = thir[args[0]].ty.peel_refs();
        // A `dyn Fn`, as it is or boxed, `Box<dyn Fn()>`'s own `Fn`.
        let dyn_fn = match ty.kind() {
            ty::Dynamic(..) => true,
            ty::Adt(adt, args) => adt.is_box() && matches!(args.type_at(0).kind(), ty::Dynamic(..)),
            _ => false,
        };
        let callee = match ty.kind() {
            ty::Closure(..) => Callee::Own,
            ty::Param(_) => Callee::Generic,
            // One only the crate's functions fill (ADR 0330).
            ty::FnPtr(..) if self.krate.cell_borrows.pointers().rust_only(self.tcx, ty) => Callee::Own,
            // A bound function of the crate's (rd196), or one each of the
            // crate's calls gives (rd616).
            _ if dyn_fn && captured.is_some_and(|var| self.bound_own(var)) => Callee::Own,
            _ if dyn_fn => Callee::Dyn,
            _ => return None,
        };
        // Not one that may be a `function::with_this` function that reads
        // `js::arguments()`, which counts what JS gives it (rd509).
        if callee != Callee::Own
            && self
                .krate
                .reads_arguments
                .may_call(self.tcx, self.krate.foreign, thir[args[1]].ty)
        {
            return None;
        }
        Some((callee, captured, dyn_fn))
    }

    /// Whether `e` is a call through an `Fn` trait of a function value the
    /// closure captured.
    fn calls_a_function_value(&self, thir: &rustc_middle::thir::Thir<'tcx>, e: rustc_middle::thir::ExprId) -> bool {
        let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
            return false;
        };
        fn_def(thir[fun].ty).is_some_and(|(id, _)| {
            (self.tcx.trait_of_assoc(id)).is_some_and(|tr| self.tcx.fn_trait_kind_from_def_id(tr).is_some())
        }) && !args.is_empty()
            && captured(self.tcx, thir, args[0]).is_some()
    }

    /// A closure whose one call, of a function value it captured, isn't all
    /// it is, its body, `root`, not that call of its parameters as they are:
    /// it converts what it's given, or what the call gives back, as
    /// alien-signals' `computed<T>` boxes its getter, `move |previous: Value|
    /// js::any_of(Rc::new(getter(previous.map(..))))`. Where each
    /// conversion is the value itself in JS, `js::any_of`, an `Rc` that
    /// isn't counted, `js::cast`, its arrow only passes on what it's given,
    /// `(previous) => getter(previous)`, which the arrow's shape says
    /// (`js::passes_on`), and it's that function where it's made (`eta`):
    /// Rust can name it no other way, as its types aren't the function's.
    /// Of a generic `impl Fn`'s or a `dyn Fn`'s, which may be a JS function
    /// reading more than it's given, only where it's boxed into a field,
    /// which Rust alone calls, with what the closure is given
    /// (`boxed_into_a_field`): not one given on, `xs.map(|x| f(x))`, whose
    /// method gives the index too (ADR 0279).
    /// Only where the crate reads no function's identity, as two closures
    /// of one function are one in JS (ADR 1043, amended, #500); and not one
    /// of `()` that discards what the call gives (ADR 0605, amended).
    fn converting(
        &self,
        body: &crate::lower::Body<'tcx>,
        (span, made_at): (js::Span, rustc_span::Span),
        root: rustc_middle::thir::ExprId,
    ) -> Option<Forwarding> {
        if self.krate.counted.points_at_functions || !self.tcx.is_closure_like(body.def_id) {
            return None;
        }
        let thir = &body.thir;
        let mut calls = (thir.exprs.iter_enumerated()).filter(|&(e, _)| self.calls_a_function_value(thir, e));
        let (call, None) = (calls.next()?, calls.next()) else {
            return None;
        };
        let ExprKind::Call { ref args, .. } = call.1.kind else {
            return None;
        };
        // Its parameters, each as it is: the plain call's, which Rust could
        // have named as the function itself, `move |x| f(x)`.
        let params: Vec<_> = (thir.params.iter())
            .filter_map(|param| match param.pat.as_deref()?.kind {
                rustc_middle::thir::PatKind::Binding { var, .. } => Some(var),
                _ => None,
            })
            .collect();
        let as_they_are = match thir[super::body_queries::strip(thir, args[1])].kind {
            // One whose last argument its JS leaves out, `tagged(n, None)`'s
            // `tagged(n)`, isn't the function, whose JS caller would fill
            // it, `[3, 4].map(tagged)`'s index (ADR 0443, amended).
            ExprKind::Tuple { ref fields } if fields.last().is_some_and(|&last| left_out(self.tcx, thir, last)) => {
                return None;
            }
            ExprKind::Tuple { ref fields } => {
                fields.len() == params.len()
                    && (fields.iter().zip(&params)).all(|(&field, &param)| {
                        matches!(thir[super::body_queries::strip(thir, field)].kind, ExprKind::VarRef { id } if id == param)
                    })
            }
            _ => false,
        };
        if call.0 == root && as_they_are {
            return None;
        }
        let (callee, _, _) = self.function_value(thir, args)?;
        if callee != Callee::Own && !self.boxed_into_a_field(made_at) {
            return None;
        }
        let ty::Closure(_, closure_args) = *self
            .tcx
            .type_of(body.def_id)
            .instantiate_identity()
            .skip_normalization()
            .kind()
        else {
            return None;
        };
        let sig = self
            .tcx
            .instantiate_bound_regions_with_erased(closure_args.as_closure().sig());
        if sig.output().is_unit() && !call.1.ty.is_unit() {
            return None;
        }
        Some(Forwarding {
            span,
            callee,
            adapts: false,
            item: false,
            binds: false,
            ignores: false,
            eta: true,
            fixed: false,
            statement: false,
        })
    }

    /// Whether the closure written at `span` is boxed into a struct's
    /// field where it's made, `Node { getter: Box::new(move |previous| ..)
    /// }`: a `dyn Fn` the crate's code calls, which nothing hands on as it's
    /// made.
    fn boxed_into_a_field(&self, span: rustc_span::Span) -> bool {
        let thir = self.frame.thir;
        // Its box made the field's `dyn Fn`, `Box<dyn Fn(Value) -> Value>`.
        let unsized_ = |mut e: rustc_middle::thir::ExprId| loop {
            e = super::body_queries::strip(thir, e);
            match thir[e].kind {
                ExprKind::PointerCoercion { source, .. } => e = source,
                _ => return e,
            }
        };
        let boxed = |e: rustc_middle::thir::ExprId| match thir[unsized_(e)].kind {
            ExprKind::Call { fun, ref args, .. } => {
                fn_def(thir[fun].ty)
                    .is_some_and(|(id, _)| super::recognition::is_std_def(self.tcx, id, StdItem::BoxNew))
                    && args.iter().any(|&a| {
                        let made = &thir[super::body_queries::strip(thir, a)];
                        matches!(made.kind, ExprKind::Closure(..)) && made.span == span
                    })
            }
            _ => false,
        };
        (thir.exprs.iter())
            .any(|e| matches!(&e.kind, ExprKind::Adt(adt) if adt.fields.iter().any(|field| boxed(field.expr))))
    }

    /// Whether variable `var`, never changed, is a function that reads no
    /// `this` and none of what it's given more (`own_callback`): a `let`'s
    /// of one, `let ping = function::bind(on_unsuspend, Some(state), ())`, or
    /// a parameter each of the crate's calls fills with one, React Flight's
    /// Node client's `on_end: Rc<dyn Fn()>` (`own_callbacks`, rd616). A JS
    /// function bound may read what it's given more.
    fn bound_own(&self, var: rustc_hir::HirId) -> bool {
        let own = self.own_callbacks();
        own.contains(&var) || let_value(self.tcx, var).is_some_and(|init| self.own_callback(init, 0, own))
    }

    /// The parameters that are such a function at every call (rd616): of a
    /// function only the crate's calls reach (`analysis::only_called`, ADR
    /// 0577), so what each call gives it is all it's ever given, each a
    /// `dyn Fn`, as it is or through a reference, a `Box`, an `Rc` or an
    /// `Arc`, a binding never changed, that each call fills with what
    /// `own_callback` says is one. Found the first time it's asked, of every
    /// call at once: what a call gives may be another such parameter, which
    /// is one while each of its own calls gives one, as nothing else fills
    /// it.
    fn own_callbacks(&self) -> &'a HashSet<rustc_hir::HirId> {
        self.krate.own_callbacks.get_or_init(|| self.find_own_callbacks())
    }

    fn find_own_callbacks(&self) -> HashSet<rustc_hir::HirId> {
        use rustc_hir as hir;
        let tcx = self.tcx;
        let callback = |ty: ty::Ty<'tcx>| {
            let ty = ty.peel_refs();
            let pointee = ty.boxed_ty().or_else(|| super::recognition::rc_pointee(tcx, ty));
            matches!(pointee.unwrap_or(ty).kind(), ty::Dynamic(..))
        };
        let mut given: HashMap<hir::HirId, Vec<&'tcx hir::Expr<'tcx>>> = HashMap::new();
        for (function, calls) in super::analysis::only_called::only_called(tcx) {
            let Some(body) = function.as_local().and_then(|local| tcx.hir_maybe_body_owned_by(local)) else {
                continue;
            };
            let inputs = tcx
                .fn_sig(function)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs();
            for (i, (param, &ty)) in body.params.iter().zip(inputs).enumerate() {
                let hir::PatKind::Binding(hir::BindingMode::NONE, var, _, None) = param.pat.kind else {
                    continue;
                };
                if !callback(ty) {
                    continue;
                }
                // What each call gives it: a method's receiver is its first.
                let at = |call: hir::HirId| match tcx.hir_node(call) {
                    hir::Node::Expr(hir::Expr {
                        kind: hir::ExprKind::Call(_, args),
                        ..
                    }) => args.get(i),
                    hir::Node::Expr(hir::Expr {
                        kind: hir::ExprKind::MethodCall(_, receiver, args, _),
                        ..
                    }) => match i {
                        0 => Some(*receiver),
                        _ => args.get(i - 1),
                    },
                    _ => None,
                };
                if let Some(values) = calls.iter().map(|&call| at(call)).collect::<Option<Vec<_>>>() {
                    given.insert(var, values);
                }
            }
        }
        // Each, until one a call gives what's none is put out, and with it
        // what it fills.
        let mut own: HashSet<hir::HirId> = given.keys().copied().collect();
        loop {
            let unfilled: Vec<hir::HirId> = (given.iter())
                .filter(|(var, values)| own.contains(var) && !values.iter().all(|e| self.own_callback(e, 0, &own)))
                .map(|(&var, _)| var)
                .collect();
            if unfilled.is_empty() {
                return own;
            }
            for var in unfilled {
                own.remove(&var);
            }
        }
    }

    /// Whether `e` is a function that reads no `this` and none of what it's
    /// given more, through what makes it, a block's value, a borrow, a
    /// `Box`, an `Rc` or an `Arc` of it, or a clone of that `Rc`, and
    /// variables that never change, of which `params`, the parameters
    /// each call fills with one, at `depth`: one `own_target` says is by its
    /// type, a function of the crate's or a closure; or what `bound_value`
    /// says, a function bound, or one `function::with_this` makes of a
    /// closure that ignores its `this` (rd284b). Not one that reads its
    /// `this`, which a JS caller gives, nor its `arguments`.
    fn own_callback(&self, e: &'tcx rustc_hir::Expr<'tcx>, depth: usize, params: &HashSet<rustc_hir::HirId>) -> bool {
        use rustc_hir as hir;
        if depth > 8 {
            return false;
        }
        let tcx = self.tcx;
        let results = tcx.typeck(e.hir_id.owner.def_id);
        if matches!(
            results.expr_ty(e).kind(),
            ty::FnDef(..) | ty::Closure(..) | ty::FnPtr(..)
        ) {
            return self.own_target(e, depth);
        }
        let inner = |e: &'tcx hir::Expr<'tcx>| self.own_callback(e, depth + 1, params);
        match e.peel_drop_temps().kind {
            hir::ExprKind::Block(block, _) => block.expr.is_some_and(inner),
            hir::ExprKind::AddrOf(_, _, of) => inner(of),
            hir::ExprKind::Path(ref qpath) => match results.qpath_res(qpath, e.hir_id) {
                hir::def::Res::Local(var) => params.contains(&var) || let_value(self.tcx, var).is_some_and(inner),
                _ => false,
            },
            // A function bound, or one `with_this` makes that reads no
            // `this` (`bound_value`); or what's made of one, or shared.
            hir::ExprKind::Call(callee, args) => {
                self.bound_value(e, depth + 1)
                    || match (fn_def(results.expr_ty_adjusted(callee)), args) {
                        (Some((f, generic)), [made]) => self.made_or_shared(f, generic) && inner(made),
                        _ => false,
                    }
            }
            hir::ExprKind::MethodCall(_, receiver, [], _) => (results.type_dependent_def_id(e.hir_id))
                .is_some_and(|f| self.made_or_shared(f, results.node_args(e.hir_id)) && inner(receiver)),
            _ => false,
        }
    }

    /// Whether a call of `f`, of `args`, gives what it's given, or another
    /// handle on it: `Box::new`, `Rc::new` or `Arc::new`, or an `Rc`'s or
    /// an `Arc`'s clone.
    fn made_or_shared(&self, f: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        use crate::lower::recognition::{HandleStep, handle_step};
        super::recognition::is_std_def(self.tcx, f, StdItem::BoxNew)
            || matches!(
                handle_step(self.tcx, f, args),
                Some(HandleStep::RcNew | HandleStep::RcClone)
            )
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
                hir::def::Res::Local(var) => {
                    let_value(self.tcx, var).is_some_and(|init| self.bound_value(init, depth + 1))
                }
                _ => false,
            },
            hir::ExprKind::Call(callee, [target, ..]) => match *results.expr_ty_adjusted(callee).kind() {
                ty::FnDef(f, _) if super::recognition::is_function_bind(self.tcx, f) => {
                    self.own_target(target, depth + 1)
                }
                // A function `function::with_this` makes of a closure that
                // ignores its `this`, `move |_, ()| ..`: one that reads none
                // (rd284b).
                // Not one that reads `js::arguments()` (rd509).
                ty::FnDef(f, _)
                    if matches!(super::bindings::js_form(self.tcx, f), super::bindings::JsForm::WithThis) =>
                {
                    ignores_this(self.tcx, target)
                        && !made_closure(target).is_some_and(|c| self.krate.reads_arguments.made.contains(&c))
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
            // Not one that reads `js::arguments()`, which counts what JS
            // gives it more (rd332).
            ty::FnDef(f, _) => {
                return (f.is_local() || self.krate.foreign.is_library(f.krate))
                    && !super::bindings::is_binding(tcx, f)
                    && !self.takes_rest(f)
                    && !self.reads_arguments(f);
            }
            ty::Closure(..) => return true,
            ty::FnPtr(..) => return self.krate.cell_borrows.pointers().rust_only(tcx, ty),
            _ => {}
        }
        match e.peel_drop_temps().kind {
            hir::ExprKind::Path(ref qpath) => match results.qpath_res(qpath, e.hir_id) {
                hir::def::Res::Local(var) => {
                    let_value(self.tcx, var).is_some_and(|init| self.own_target(init, depth + 1))
                }
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

/// The closure `e` makes, through the block that makes it.
fn made_closure(e: &rustc_hir::Expr<'_>) -> Option<DefId> {
    match e.peel_drop_temps().kind {
        rustc_hir::ExprKind::Block(block, _) => block.expr.and_then(made_closure),
        rustc_hir::ExprKind::Closure(closure) => Some(closure.def_id.to_def_id()),
        _ => None,
    }
}

/// Whether `e` is a closure, through the block that makes it, whose first
/// parameter, a `with_this` function's `this`, is `_`.
fn ignores_this(tcx: TyCtxt<'_>, e: &rustc_hir::Expr<'_>) -> bool {
    use rustc_hir as hir;
    match e.peel_drop_temps().kind {
        hir::ExprKind::Block(block, _) => block.expr.is_some_and(|e| ignores_this(tcx, e)),
        hir::ExprKind::Closure(closure) => {
            matches!(tcx.hir_body(closure.body).params.first(), Some(param) if matches!(param.pat.kind, hir::PatKind::Wild))
        }
        _ => false,
    }
}

/// What variable `var` is, of a `let` that never changes it, never `mut`
/// nor bound by reference: its initializer. A value of a `Cell` it holds
/// may still be set through it.
pub(in crate::lower) fn let_value<'tcx>(
    tcx: TyCtxt<'tcx>,
    var: rustc_hir::HirId,
) -> Option<&'tcx rustc_hir::Expr<'tcx>> {
    use rustc_hir as hir;
    let hir::Node::Pat(pat) = tcx.hir_node(var) else {
        return None;
    };
    let hir::PatKind::Binding(hir::BindingMode(hir::ByRef::No, rustc_hir::Mutability::Not), _, _, None) = pat.kind
    else {
        return None;
    };
    match tcx.parent_hir_node(var) {
        hir::Node::LetStmt(local) => local.init,
        _ => None,
    }
}
