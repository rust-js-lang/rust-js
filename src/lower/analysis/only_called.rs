//! The crate's functions only calls reach (ADR 0577): each one no JS
//! caller sees, never named as a value, a trait's method or exported, by
//! the calls of it the crate's code makes. What each call gives it is all
//! it's ever given, so what's known of its parameters is what's known
//! where it's called: their ranges (ADR 0577), and a `dyn Fn` the crate's
//! own function fills at each (`passed_on`'s `own_callbacks`, rd616).
//! A query of each body's names, the crate graph's (`call_graph::Names`,
//! ADR 1148), each named as a call's callee or as a value.
//! A function a namespace's or a prototype's method may be, as its JS
//! (`passed_on::forwarded_function`, #566, rd715), is reached as the
//! method is: only calls reach it only where only calls reach the method,
//! else its JS callers, its uses as a value and its export are the
//! function's too (#1675).

use crate::collections::{IndexMap, IndexSet};
use crate::lower::call_graph::{Names, Naming};
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// A call of a function only calls reach: the body that makes it, and its
/// callee's span there, `f` of `f(x)` or of `x.f(y)`.
pub(in crate::lower) type Call = (DefId, Span);

/// Each function only calls reach, by its calls (`Call`), one entry a call. The bodies are those the crate emits, each body's
/// names once (`Names`): a call in a body never lowered never runs. Of
/// `bodies`, the methods that may be a function (`forwarded_function`).
pub(in crate::lower) fn only_called<'tcx>(
    tcx: TyCtxt<'tcx>,
    (names, bodies): (&Names<'tcx>, &[&crate::lower::Body<'tcx>]),
    library: bool,
) -> IndexMap<DefId, Vec<Call>> {
    let mut calls: IndexMap<DefId, Vec<Call>> = IndexMap::default();
    // Named as a value: a function pointer, a callback, what anything may
    // call with anything. And what a `function::bind` binds each parameter
    // of, whose calls give it what's bound where the bound function is
    // called, no call of its own here (`call_at`'s), so a value too.
    let mut values: IndexSet<DefId> = named_as_values(names).into_keys().collect();
    // Of each body the crate emits, none a constant only rustc evaluates,
    // whose calls JS never makes, which no body of the names is.
    for (body, named) in names.iter() {
        for &(callee, _, span, naming) in named {
            match naming {
                Naming::Called => calls.entry(callee).or_default().push((body, span)),
                Naming::Bound => {
                    values.insert(callee);
                }
                Naming::Value => {}
            }
        }
    }
    // A module's default export JS imports, though Rust's `pub` doesn't
    // say so: `js::export_default!(page)` of a private `fn page`.
    let defaults: IndexSet<DefId> = (crate::lower::default_exports::defaults(tcx).into_iter())
        .map(|(_, item)| item)
        .collect();
    calls.retain(|&def_id, _| {
        // Not one code rust-js doesn't compile may call, another crate or
        // JS, whose module exports a `pub fn` though its module is private:
        // its parameters are what that code gives too (#2025).
        def_id.is_local()
            && !crate::lower::library::seen_outside(tcx, library, def_id)
            && !values.contains(&def_id)
            && !defaults.contains(&def_id)
            && (tcx.def_kind(def_id) == DefKind::Fn
                || tcx.def_kind(def_id) == DefKind::AssocFn
                    && tcx.trait_of_assoc(def_id).is_none()
                    && tcx.trait_impl_of_assoc(def_id).is_none())
    });
    // A function a method may be, reached as that method is: of one only
    // calls don't reach, by whatever reaches the method (#1675).
    let forwarding: Vec<(DefId, DefId)> = (bodies.iter())
        .filter_map(|body| {
            let (target, _) = crate::lower::passed_on::forwarded_function(tcx, body.def_id, &body.thir, None)?;
            Some((body.def_id, target))
        })
        .collect();
    for (method, target) in forwarding {
        if !calls.contains_key(&method) {
            calls.swap_remove(&target);
        }
    }
    calls
}

/// Each function the crate's bodies name as a value (`Naming::Value`), by
/// where: what anything may call with anything, the one catalog of it
/// (`call_graph::Names`), which `only_called` and `nulled_params` ask
/// (#1675).
pub(in crate::lower) fn named_as_values(names: &Names<'_>) -> IndexMap<DefId, Vec<Span>> {
    let mut values: IndexMap<DefId, Vec<Span>> = IndexMap::default();
    for (_, named) in names.iter() {
        for &(callee, _, span, naming) in named {
            if naming == Naming::Value {
                values.entry(callee).or_default().push(span);
            }
        }
    }
    values
}

/// The call of `body`'s HIR whose callee is at `callee`, a name's span the
/// crate graph's names give (`only_called::Call`): `f` of `f(x)`, or the
/// method's name of `x.f(y)`, as THIR names each. The one there, or none:
/// none found, or two, a macro's, gives the call nothing (#1671 change 4).
pub(in crate::lower) fn call_at<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: DefId,
    callee: rustc_span::Span,
) -> Option<&'tcx rustc_hir::Expr<'tcx>> {
    use rustc_hir::intravisit::{self, Visitor};
    struct Calls<'tcx>(rustc_span::Span, Vec<&'tcx rustc_hir::Expr<'tcx>>);
    impl<'tcx> Visitor<'tcx> for Calls<'tcx> {
        fn visit_expr(&mut self, e: &'tcx rustc_hir::Expr<'tcx>) {
            let at = match e.kind {
                rustc_hir::ExprKind::Call(f, _) => Some(f.span),
                rustc_hir::ExprKind::MethodCall(segment, ..) => Some(segment.ident.span),
                _ => None,
            };
            if at == Some(self.0) {
                self.1.push(e);
            }
            intravisit::walk_expr(self, e);
        }
    }
    let body = tcx.hir_maybe_body_owned_by(body.as_local()?)?;
    let mut calls = Calls(callee, Vec::new());
    calls.visit_body(body);
    match calls.1.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}
