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
}

impl<'tcx> ReadsArguments<'tcx> {
    /// Whether a call of a `dyn Fn`, a generic one or a pointer, given
    /// `args`, its arguments' tuple, may call one that reads it.
    pub(in crate::lower) fn may_call(&self, tcx: TyCtxt<'tcx>, args: ty::Ty<'tcx>) -> bool {
        self.with_this.contains(&tcx.erase_and_anonymize_regions(args))
    }
}

/// The functions of `all_bodies` whose own body reads `js::arguments()`,
/// each read elsewhere refused.
pub(in crate::lower) fn reads_arguments<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> ReadsArguments<'tcx> {
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
    reading
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
