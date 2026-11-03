//! Which of the crate's functions read `js::arguments()`, JS's `arguments`
//! (rd332, the user's decision): each argument its caller gave, as many as
//! it gave, which each call rust-js writes of such a function gives it, an
//! `undefined` it'd leave out too (`calls::reads_arguments`), and which no
//! closure forwarding to it is (`passed_on`). Read only where its JS is a
//! `function` whose arguments are the Rust's: a function of the crate's,
//! not generic, a class's constructor, method or static, or a namespace's
//! function. Anywhere else, a compile error: a closure, an arrow, whose
//! `arguments` is the function's around it; a generic function, given its
//! type parameters' dictionaries too; a trait's method, called through its
//! dictionary; and a method of what's no class, given its value first.
//! A closure `function::with_this` makes is a JS `function`, whose
//! `arguments` is its own (rd509): read there, each call of a `dyn Fn` of
//! its parameters gives every argument, as no forwarding closure is it.
//!
//! And which read `js::this()`, JS's `this` (rd683): the `this` its JS
//! caller gave it, where each call rust-js writes gives none, `None`, as
//! Rust's does, so a call through a module's namespace is `(0, M.f)()`
//! (`calls`), and no closure forwarding to it is it. Read only in a
//! function of the crate's that isn't generic, whose JS is a `function`
//! called by name; elsewhere a compile error, as is the function's use as
//! a value but to call it, which JS might call on what holds it, through
//! `function::call` and `function::apply`, which give it the `this` they're
//! given, too.

use crate::lower::Body;
use crate::lower::refusal::{Refused, refuse};
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// What reads `js::arguments()`: the functions, and of the closures
/// `function::with_this` makes, each the tuple of its parameters, which
/// the `dyn Fn` a call of one calls takes (rd509).
#[derive(Default)]
pub(in crate::lower) struct ReadsArguments<'tcx> {
    pub functions: HashSet<DefId>,
    pub with_this: HashSet<ty::Ty<'tcx>>,
    /// Each closure `function::with_this` makes that reads it.
    pub made: HashSet<DefId>,
    /// The functions whose own body reads `js::this()` (rd683).
    pub this: HashSet<DefId>,
}

impl<'tcx> ReadsArguments<'tcx> {
    /// Whether a call of a `dyn Fn`, a generic one or a pointer, given
    /// `args`, its arguments' tuple, may call one that reads it: one the
    /// crate makes, or one a library does, as its contract says, which it
    /// may hand the crate (ADR 1048, amended, #553).
    pub(in crate::lower) fn may_call(
        &self,
        tcx: TyCtxt<'tcx>,
        foreign: &crate::lower::library::Foreign<'_, 'tcx>,
        args: ty::Ty<'tcx>,
    ) -> bool {
        self.with_this.contains(&tcx.erase_and_anonymize_regions(args)) || foreign.may_call_reading_arguments(args)
    }

    /// Whether `def_id`'s own body reads more of its call than its
    /// parameters, `js::arguments()` or `js::this()`, so no closure that
    /// forwards to it is it, which its JS caller would give them (rd332,
    /// rd683).
    pub(in crate::lower) fn reads_its_call(&self, def_id: DefId) -> bool {
        self.functions.contains(&def_id) || self.this.contains(&def_id)
    }
}

/// The functions of `all_bodies` whose own body reads `js::arguments()`
/// or `js::this()`, each read elsewhere refused, a static's or a
/// constant's of `initializers` too, and each use of one that reads
/// `this` but to call it, or of another crate's that does, as its contract
/// says (`foreign`'s).
pub(in crate::lower) fn reads_arguments<'tcx>(
    tcx: TyCtxt<'tcx>,
    (all_bodies, initializers): (&[&Body<'tcx>], &[Body<'tcx>]),
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
) -> ReadsArguments<'tcx> {
    // Each closure `function::with_this` makes, and its parameters' tuple.
    let mut functions_made: HashMap<DefId, ty::Ty<'tcx>> = HashMap::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let ty::FnDef(id, _) = *body.thir[fun].ty.kind()
                && crate::lower::recognition::is_with_this(tcx, id)
                && let [made] = args[..]
                && let ExprKind::Closure(ref closure) =
                    body.thir[crate::lower::body_queries::strip(&body.thir, made)].kind
                && let ty::Closure(_, closure_args) = *body.thir[made].ty.kind()
            {
                // Its inputs, one tuple, `(this, (a, b))`.
                let sig = tcx.instantiate_bound_regions_with_erased(closure_args.as_closure().sig());
                if let Some(&inputs) = sig.inputs().first()
                    && let ty::Tuple(inputs) = *inputs.kind()
                    && let [_, params] = inputs[..]
                {
                    functions_made.insert(closure.closure_id.to_def_id(), tcx.erase_and_anonymize_regions(params));
                }
            }
        }
    }
    let mut reading = ReadsArguments::default();
    for body in all_bodies {
        let Some(span) = (body.thir.exprs.iter()).find_map(|expr| match expr.kind {
            ExprKind::Call { fun, .. } if is_arguments(tcx, body.thir[fun].ty) => Some(expr.span),
            _ => None,
        }) else {
            continue;
        };
        if let Some(&params) = functions_made.get(&body.def_id) {
            reading.with_this.insert(params);
            reading.made.insert(body.def_id);
            continue;
        }
        match not_its_own(tcx, body.def_id) {
            Some(why) => _ = refuse(tcx, span, Refused::Because("`js::arguments()` here", why)),
            None => _ = reading.functions.insert(body.def_id),
        }
    }
    let anywhere: Vec<&Body<'tcx>> = all_bodies.iter().copied().chain(initializers).collect();
    for body in &anywhere {
        let Some(span) = (body.thir.exprs.iter()).find_map(|expr| match expr.kind {
            ExprKind::Call { fun, .. } if is_this(tcx, body.thir[fun].ty) => Some(expr.span),
            _ => None,
        }) else {
            continue;
        };
        match not_its_own_this(tcx, body.def_id, functions_made.contains_key(&body.def_id)) {
            Some(why) => _ = refuse(tcx, span, Refused::Because("`js::this()` here", why)),
            None => _ = reading.this.insert(body.def_id),
        }
    }
    for body in &anywhere {
        for span in this_values(tcx, body, &|id| {
            reading.this.contains(&id) || !id.is_local() && foreign.reads_this(id)
        }) {
            _ = refuse(
                tcx,
                span,
                Refused::Because(
                    "a function that reads `js::this()` as a value",
                    "JS would give it the `this` of what holds it where it's called, and Rust gives none: call it by \
                     its name, or `function::call(f, this_arg, ..)`, or make it a closure, `function::with_this(..)`",
                ),
            );
        }
    }
    reading
}

/// Where `body` uses `js::this` or a function `reads_this` says reads it
/// but to call it: as a call's function, or given to `function::call` or
/// `function::apply`, which call it with the `this` they're given.
fn this_values(tcx: TyCtxt<'_>, body: &Body<'_>, reads_this: &dyn Fn(DefId) -> bool) -> Vec<rustc_span::Span> {
    use crate::lower::body_queries::strip;
    let thir = &body.thir;
    let mut called = HashSet::new();
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        called.insert(strip(thir, fun));
        if let ty::FnDef(id, _) = *thir[fun].ty.kind()
            && (crate::lower::recognition::is_function_call(tcx, id)
                || crate::lower::recognition::is_function_apply(tcx, id))
            && let Some(&first) = args.first()
        {
            // The function itself, through each borrow and reborrow of it.
            let mut f = strip(thir, first);
            while let ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } = thir[f].kind {
                f = strip(thir, arg);
            }
            called.insert(f);
        }
    }
    (thir.exprs.iter_enumerated())
        .filter(|&(id, expr)| {
            matches!(expr.kind, ExprKind::ZstLiteral { .. })
                && matches!(*expr.ty.kind(), ty::FnDef(f, _) if reads_this(f) || is_this(tcx, expr.ty))
                && !called.contains(&id)
        })
        .map(|(_, expr)| expr.span)
        .collect()
}

/// Whether `callee`, a call's function's type, is `js::this`.
fn is_this(tcx: TyCtxt<'_>, callee: ty::Ty<'_>) -> bool {
    matches!(*callee.kind(), ty::FnDef(id, _)
        if crate::lower::recognition::js_root_fn(tcx, id).is_some_and(|name| name.as_str() == "this"))
}

/// Why `def_id`'s JS's `this` isn't the one JS calls it by name with, if it
/// isn't: of a closure `function::with_this` makes, `with_this`, its first
/// parameter's.
fn not_its_own_this(tcx: TyCtxt<'_>, def_id: DefId, with_this: bool) -> Option<&'static str> {
    let generic = || {
        (tcx.generics_of(def_id).own_params.iter())
            .any(|param| !matches!(param.kind, ty::GenericParamDefKind::Lifetime))
    };
    if with_this {
        Some("a `function::with_this` closure's `this` is its first parameter, `|this, (a, b)|`: read that")
    } else if tcx.is_closure_like(def_id) {
        Some(
            "a closure is an arrow function, whose `this` is the function's around it: read it in that function, \
             `let this = js::this();`, and give the closure what it needs, or make the closure a `function`, \
             `function::with_this(|this, (a, b)| ..)`",
        )
    } else if tcx.def_kind(def_id) == DefKind::AssocFn {
        Some(
            "a method's `this` is its `self`, or its type of a static, which JS calls it through: read it in a \
             function of the crate's, or make the type a `#[rust_js::class]` and read `self`",
        )
    } else if tcx.def_kind(def_id) != DefKind::Fn {
        Some("only a function is called with a `this`: read it in one")
    } else if generic() {
        Some(
            "a generic function's JS is given its type parameters' dictionaries too, which no JS caller gives; read \
             it in one that isn't generic",
        )
    } else {
        None
    }
}

/// Whether `callee`, a call's function's type, is `js::arguments`.
fn is_arguments(tcx: TyCtxt<'_>, callee: ty::Ty<'_>) -> bool {
    matches!(*callee.kind(), ty::FnDef(id, _)
        if crate::lower::recognition::js_root_fn(tcx, id).is_some_and(|name| name.as_str() == "arguments"))
}

/// Why `def_id`'s JS's `arguments` isn't what its Rust is given, if it isn't.
fn not_its_own(tcx: TyCtxt<'_>, def_id: DefId) -> Option<&'static str> {
    use crate::lower::classes::Member;
    if tcx.is_closure_like(def_id) {
        return Some(
            "a closure is an arrow function, whose `arguments` is the function's around it: read it in that function, \
             and give the closure what it needs, or make the closure a `function`, `function::with_this(..)`",
        );
    }
    let generic = (tcx.generics_of(def_id).own_params.iter())
        .chain(
            tcx.opt_parent(def_id)
                .filter(|_| tcx.def_kind(def_id) == DefKind::AssocFn)
                .iter()
                .flat_map(|&p| tcx.generics_of(p).own_params.iter()),
        )
        .any(|param| !matches!(param.kind, ty::GenericParamDefKind::Lifetime));
    if generic {
        return Some(
            "a generic function's JS is given its type parameters' dictionaries too; read it in one that isn't generic",
        );
    }
    if tcx.def_kind(def_id) != DefKind::AssocFn {
        return None;
    }
    if tcx.trait_of_assoc(def_id).is_some() || tcx.trait_impl_of_assoc(def_id).is_some() {
        return Some(
            "a trait's method is called through its dictionary; read it in a function of the crate's, or a class's method",
        );
    }
    match crate::lower::classes::member(tcx, def_id) {
        Some(Member::Constructor | Member::Method | Member::Static | Member::Getter | Member::Namespace) => None,
        Some(_) => Some("an object's method is its literal's own property; read it in a class's method"),
        None if tcx.associated_item(def_id).is_method() => Some(
            "a method of what's no class is its type's function, given the value first; make the type a \
             `#[rust_js::class]`, whose method's `this` it is",
        ),
        None => None,
    }
}
