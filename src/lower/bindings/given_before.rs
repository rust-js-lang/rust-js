//! `#[rust_js::given_before]`: a trait whose value JS finds by identity
//! among those it was given before, as `removeEventListener` finds the
//! listener `addEventListener` was given (ADR 1021, amended). A closure
//! written in place where it's given is a new function, which JS finds
//! nothing of: it compiles, as rustc makes a closure of no captures
//! `'static`, and removes nothing, Scala.js's listener leak. So it's refused.

use crate::lower::Body;
use crate::lower::body_queries::strip;
use rustc_middle::thir::{ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// Whether trait `def_id` is `#[rust_js::given_before]`.
fn given_before(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("given_before")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// Report each closure written in place as a value of a
/// `#[rust_js::given_before]` trait, by value, borrowed, or the field of
/// what's made there, `EventListener(&|e| ..)`: of any call, a binding's
/// or a function of the crate's whose parameter's bound is the trait.
/// False if there was one.
pub(in crate::lower) fn reject_made_in_place<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> bool {
    let mut valid = true;
    for body in bodies {
        let thir = &body.thir;
        for expr in thir.exprs.iter() {
            let ExprKind::Call { fun, ref args, .. } = expr.kind else {
                continue;
            };
            let ty::FnDef(callee, _) = *thir[strip(thir, fun)].ty.kind() else {
                continue;
            };
            let inputs = tcx.fn_sig(callee).skip_binder().skip_binder().inputs();
            // The type parameters a `#[rust_js::given_before]` trait bounds.
            let bound: Vec<_> = (tcx.clauses_of(callee).instantiate_identity(tcx).into_iter())
                .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
                .filter(|bound| given_before(tcx, bound.def_id()))
                .map(|bound| bound.self_ty().skip_binder())
                .collect();
            for (&arg, input) in args.iter().zip(inputs) {
                if !bound.contains(input) {
                    continue;
                }
                if let Some(closure) = written_in_place(thir, arg) {
                    let name = tcx.item_name(callee);
                    tcx.dcx()
                        .struct_span_err(
                            thir[closure].span,
                            format!(
                                "rust-js: a closure written in place is a new function, which `{name}` never finds \
                                 among those given before"
                            ),
                        )
                        .with_help(
                            "hold the function `webapi::listener(..)` makes, or pass a named `fn`, and give the same \
                             one to both (ADR 1021)",
                        )
                        .emit();
                    valid = false;
                }
            }
        }
    }
    valid
}

/// The closure `e` is, as it's written where it's given: itself, borrowed
/// or reborrowed, coerced, or the one field of what's made there.
fn written_in_place(thir: &Thir<'_>, mut e: ExprId) -> Option<ExprId> {
    loop {
        e = strip(thir, e);
        match &thir[e].kind {
            ExprKind::Closure(_) => return Some(e),
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = *arg,
            ExprKind::PointerCoercion { source, .. } => e = *source,
            ExprKind::Adt(adt) if adt.fields.len() == 1 => e = adt.fields[0].expr,
            _ => return None,
        }
    }
}
