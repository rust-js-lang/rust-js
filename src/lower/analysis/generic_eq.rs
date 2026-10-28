//! What generic code compares by `Iterator::eq` or `ne`, of types that
//! hold its type parameters, carried to each caller's types (#386). std's
//! `SpecIterEq` of two `TrustedLen` iterators compares their lengths first
//! and steps nothing of unequal ones; generic code's JS, given values that
//! can't report their lengths, compares them item by item. Where that's
//! seen, a closure or a function they hold, or items with a destructor,
//! the caller's call is refused, as `iterators_eq` refuses one it can't
//! count (ADR 0998).

use crate::lower::recognition::{Recognition, StdItem, TypeFact, fact_trait, std_item, trait_method};
use crate::lower::{Body, fn_def};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use std::collections::{HashMap, HashSet};

/// Refuses each call that gives generic code, which compares them by
/// `eq` or `ne`, two `TrustedLen` iterators whose comparing runs code.
pub(super) fn refuse_generic_trusted_eqs<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    drops: impl Fn(Ty<'tcx>) -> bool,
) {
    let Some(trusted_len) = fact_trait(tcx, TypeFact::TrustedLen) else {
        return;
    };
    let iterator = std_item(tcx, StdItem::Iterator);
    let compares = [trait_method(tcx, iterator, "eq"), trait_method(tcx, iterator, "ne")];
    // Each body's own, by its function: the pairs it compares that hold its
    // type parameters, which each caller's call makes of its types.
    let mut of_body: HashMap<rustc_span::def_id::DefId, HashSet<(Ty<'tcx>, Ty<'tcx>)>> = HashMap::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            let mut types = args.types();
            if compares.contains(&callee)
                && let (Some(a), Some(b)) = (types.next(), types.next())
                && (a.has_param() || b.has_param())
            {
                of_body.entry(body.def_id).or_default().insert((a, b));
            }
        }
    }
    let runs_code = |ty: Ty<'tcx>| {
        ty.walk()
            .any(|part| part.as_type().is_some_and(|t| t.is_closure() || t.is_fn()))
            || recognition.iterator_item(ty).is_some_and(&drops)
    };
    let mut refused = HashSet::new();
    // Through what calls each caller in turn, as far as 32 calls deep, as a
    // function calling itself with a type made of its own would go on.
    for _ in 0..32 {
        let mut more = Vec::new();
        for body in all_bodies {
            let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
            for expr in body.thir.exprs.iter() {
                let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                    continue;
                };
                let Some(own) = of_body.get(&callee) else { continue };
                for &(a, b) in own {
                    let made = |ty: Ty<'tcx>| {
                        let ty = ty::EarlyBinder::bind(tcx, ty)
                            .instantiate(tcx, args)
                            .skip_normalization();
                        tcx.erase_and_anonymize_regions(ty)
                    };
                    let (a, b) = (made(a), made(b));
                    if a.has_param() || b.has_param() {
                        if !of_body.get(&body.def_id).is_some_and(|known| known.contains(&(a, b))) {
                            more.push((body.def_id, (a, b)));
                        }
                        continue;
                    }
                    let b = crate::lower::recognition::iterator_type_of(tcx, typing_env, b);
                    let trusted = [a, b].iter().all(|&ty| recognition.implements_trait(ty, trusted_len));
                    if trusted && (runs_code(a) || runs_code(b)) && refused.insert(expr.span) {
                        crate::lower::refusal::refuse(
                            tcx,
                            expr.span,
                            crate::lower::refusal::Refused::Because(
                                "giving generic code two iterators it compares by `eq`, which std asks the lengths of first",
                                "rust-js's generic code compares them item by item, and stepping them runs code std's wouldn't run (#386)",
                            ),
                        );
                    }
                }
            }
        }
        if more.is_empty() {
            break;
        }
        for (owner, pair) in more {
            of_body.entry(owner).or_default().insert(pair);
        }
    }
}
