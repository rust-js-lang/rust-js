//! The functions JS gives again what it gave them, which it keeps: a
//! component, which React renders again with the props it kept, twice in
//! `<StrictMode>`; a reducer, which React calls again with the action it
//! kept; and each callback a binding says so of (#1723, ADR 0020,
//! amended). Rust's function owns what it's given by value; its JS shares
//! JS's object, so what it changes in place of one JS sees again, where
//! Rust's next call gets its own (`copies/kept.rs`, which copies it).

use crate::collections::IndexSet;
use crate::lower::Body;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

/// The crate's functions and closures JS gives again what it gave them,
/// by `#[rust_js::gives_again]`: of a trait, each whose return is of it,
/// React's `Rendered`, a component's; of a binding, each it's given as a
/// parameter it names, `use_reducer`'s `reducer`. Refused, `Err`, where a
/// binding is given one rust-js can't name, a function pointer or a `dyn`
/// whose function could be any.
pub(in crate::lower) fn given_again<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
) -> Result<IndexSet<DefId>, ()> {
    let rendered: Vec<DefId> = (tcx.all_traits_including_private())
        .filter(|&tr| crate::lower::bindings::gives_again(tcx, tr).is_some())
        .collect();
    let mut again = IndexSet::default();
    let mut valid = true;
    for body in all_bodies {
        let id = body.def_id;
        if !id.is_local() {
            continue;
        }
        // A component's: what it returns is React's `Rendered`.
        if let Some(mir) = &body.mir {
            let output = mir.body.local_decls[rustc_middle::mir::RETURN_PLACE].ty;
            let env = ty::TypingEnv::post_analysis(tcx, id);
            if rendered
                .iter()
                .any(|&tr| crate::lower::recognition::trait_holds_in(tcx, env, output, tr) == Some(true))
            {
                again.insert(id);
            }
        }
        // A callback a binding names, given where it's called.
        for expr in body.thir.exprs.iter() {
            let ExprKind::Call {
                ty: callee, ref args, ..
            } = expr.kind
            else {
                continue;
            };
            let ty::FnDef(binding, _) = *callee.kind() else {
                continue;
            };
            let Some(named) = crate::lower::bindings::gives_again(tcx, binding) else {
                continue;
            };
            let idents = tcx.fn_arg_idents(binding);
            for (index, &arg) in args.iter().enumerate() {
                let given = idents.get(index).copied().flatten().map(|ident| ident.name);
                if !given.is_some_and(|name| named.contains(&name)) {
                    continue;
                }
                let arg_ty = body.thir[arg].ty;
                match *arg_ty.kind() {
                    ty::FnDef(f, _) | ty::Closure(f, _) if f.is_local() => {
                        again.insert(f);
                    }
                    // One whose arguments given by value hold no object,
                    // numbers or text, which nothing changes in place.
                    _ if callable_inputs(arg_ty)
                        .is_some_and(|inputs| !inputs.into_iter().any(|t| may_hold_object(tcx, t))) => {}
                    // Another crate's function is compiled where it is, and
                    // what it changes in place of what it's given is its
                    // own (ADR 0914): refused as any other it can't see.
                    _ => {
                        tcx.dcx().span_err(
                            body.thir[arg].span,
                            format!(
                                "rust-js: `{}` gives this function again what it gives it, which it keeps, so what the function changes in place of it is copied first (#1723): give a function or a closure of this crate, which rust-js sees, not `{arg_ty}`; one rust-js can't see is #1727",
                                tcx.item_name(binding)
                            ),
                        );
                        valid = false;
                    }
                }
            }
        }
    }
    match valid {
        true => Ok(again),
        false => Err(()),
    }
}

/// The argument types a function value of `ty` takes: a function pointer's,
/// or a `dyn Fn`'s, boxed or not. `None` where they can't be read here.
fn callable_inputs<'tcx>(ty: ty::Ty<'tcx>) -> Option<Vec<ty::Ty<'tcx>>> {
    let ty = match ty.kind() {
        ty::Adt(adt, args) if adt.is_box() => args.type_at(0),
        ty::Ref(_, inner, _) => *inner,
        _ => ty,
    };
    match ty.kind() {
        ty::FnPtr(sig, header) => Some(sig.with(*header).skip_binder().inputs().to_vec()),
        ty::Dynamic(preds, ..) => {
            let principal = preds.principal()?.skip_binder();
            let tuple = principal.args.types().next()?;
            match tuple.kind() {
                ty::Tuple(parts) => Some(parts.iter().collect()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether a value of `ty` given by value may hold an object JS can change
/// in place: not a number, a `bool`, a `char`, text, which JS holds as an
/// immutable string, nor a reference, through which nothing changes.
fn may_hold_object<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never | ty::Ref(..) => false,
        ty::Adt(adt, _) => !tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::String),
        _ => true,
    }
}
