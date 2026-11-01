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

use crate::lower::Body;
use crate::lower::refusal::{Refused, refuse};
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// The functions of `all_bodies` whose own body reads `js::arguments()`,
/// each read elsewhere refused.
pub(in crate::lower) fn reads_arguments<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> HashSet<DefId> {
    let mut reading = HashSet::new();
    for body in all_bodies {
        let Some(span) = (body.thir.exprs.iter()).find_map(|expr| match expr.kind {
            ExprKind::Call { fun, .. } if is_arguments(tcx, body.thir[fun].ty) => Some(expr.span),
            _ => None,
        }) else {
            continue;
        };
        match not_its_own(tcx, body.def_id) {
            Some(why) => _ = refuse(tcx, span, Refused::Because("`js::arguments()` here", why)),
            None => _ = reading.insert(body.def_id),
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
             and give the closure what it needs",
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
