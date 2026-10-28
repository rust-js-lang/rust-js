//! What generic code asks of std whose answer only its callers' types
//! decide, of types that hold its type parameters, carried to each
//! caller's types through generic callers too. Generic code's JS is one
//! for every caller, so where std's answer would differ from it, and
//! that's seen, the caller's call is refused:
//! - `eq` or `ne` of two `TrustedLen` iterators, which std's `SpecIterEq`
//!   compares the lengths of first, stepping nothing of unequal ones; JS
//!   given values that can't report their lengths compares them item by
//!   item (ADR 1014), as `iterators_eq` refuses one it can't count (ADR
//!   0998).
//! - A zip folded, or adapted to be, whose fold std takes to the shorter
//!   side, `zip_folds_to_the_shorter`'s, where JS steps it as `next()`
//!   does, its first side once more (ADRs 0973, 0997); and one stepped by
//!   `next()`, of sides std reads by index, whose `next()` stops at the
//!   shorter too.
//!
//! Each where stepping runs code: a closure or a function, or items with a
//! destructor.

use crate::lower::recognition::{
    Recognition, StdItem, TypeFact, fact_trait, std_item, trait_method, zip_folds_to_the_shorter, zipped_sources,
};
use crate::lower::{Body, fn_def};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use std::collections::{HashMap, HashSet};

/// What a body asks of std of types holding its type parameters.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Asked<'tcx> {
    /// `a.eq(b)` or `a.ne(b)`, `b` an `IntoIterator`.
    Eq(Ty<'tcx>, Ty<'tcx>),
    /// A zip of this type folded, or adapted to be.
    ZipFold(Ty<'tcx>),
    /// A zip of this type stepped by `next()`.
    ZipStep(Ty<'tcx>),
}

/// Refuses each call that gives generic code what it asks of std that its
/// JS answers otherwise, where that's seen.
pub(super) fn refuse_what_callers_decide<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    drops: impl Fn(Ty<'tcx>) -> bool,
) {
    let iterator = std_item(tcx, StdItem::Iterator);
    let compares = [trait_method(tcx, iterator, "eq"), trait_method(tcx, iterator, "ne")];
    // What steps a zip by `next()`, and what asks it nothing it steps.
    let next = trait_method(tcx, iterator, "next");
    let asks_nothing = ["size_hint", "by_ref"].map(|name| trait_method(tcx, iterator, name));
    // Each body's own, by its function: what it asks that holds its type
    // parameters, which each caller's call makes of its types.
    let mut of_body: HashMap<rustc_span::def_id::DefId, HashSet<Asked<'tcx>>> = HashMap::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            let mut types = args.types();
            let Some(this) = types.next() else { continue };
            if compares.contains(&callee)
                && let Some(other) = types.next()
                && (this.has_param() || other.has_param())
            {
                of_body.entry(body.def_id).or_default().insert(Asked::Eq(this, other));
            }
            if tcx.trait_of_assoc(callee) == Some(iterator) && !asks_nothing.contains(&callee) {
                let zips = (this.walk())
                    .filter_map(|part| part.as_type())
                    .filter(|ty| ty.has_param() && zipped_sources(tcx, *ty).is_some());
                let asked = match callee == next {
                    true => Asked::ZipStep,
                    false => Asked::ZipFold,
                };
                of_body.entry(body.def_id).or_default().extend(zips.map(asked));
            }
        }
    }
    let trusted_len = fact_trait(tcx, TypeFact::TrustedLen);
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
                let made = |ty: Ty<'tcx>| {
                    let ty = ty::EarlyBinder::bind(tcx, ty)
                        .instantiate(tcx, args)
                        .skip_normalization();
                    tcx.erase_and_anonymize_regions(ty)
                };
                for &asked in own {
                    let asked = match asked {
                        Asked::Eq(a, b) => Asked::Eq(made(a), made(b)),
                        Asked::ZipFold(zip) => Asked::ZipFold(made(zip)),
                        Asked::ZipStep(zip) => Asked::ZipStep(made(zip)),
                    };
                    let generic = match asked {
                        Asked::Eq(a, b) => a.has_param() || b.has_param(),
                        Asked::ZipFold(zip) | Asked::ZipStep(zip) => zip.has_param(),
                    };
                    if generic {
                        if !of_body.get(&body.def_id).is_some_and(|known| known.contains(&asked)) {
                            more.push((body.def_id, asked));
                        }
                        continue;
                    }
                    let (what, why) = match asked {
                        Asked::Eq(a, b) => {
                            let b = crate::lower::recognition::iterator_type_of(tcx, typing_env, b);
                            let trusted = trusted_len.is_some_and(|trusted| {
                                [a, b].iter().all(|&ty| recognition.implements_trait(ty, trusted))
                            });
                            if !trusted || !(runs_code(a) || runs_code(b)) {
                                continue;
                            }
                            (
                                "giving generic code two iterators it compares by `eq`, which std asks the lengths of first",
                                "rust-js's generic code compares them item by item, and stepping them runs code std's wouldn't run (#386, ADR 1014)",
                            )
                        }
                        Asked::ZipFold(zip) | Asked::ZipStep(zip) => {
                            let by_index = zip_folds_to_the_shorter(tcx, zip);
                            let stops = match asked {
                                Asked::ZipStep(_) => by_index == Some(true),
                                _ => by_index.is_some(),
                            };
                            if !stops {
                                continue;
                            }
                            (
                                "giving generic code a zip it folds or steps, which std stops at its shorter side",
                                "rust-js's generic code steps it as `next()` does, its first side once more, which runs code std's wouldn't run (ADR 1014)",
                            )
                        }
                    };
                    if refused.insert(expr.span) {
                        crate::lower::refusal::refuse(
                            tcx,
                            expr.span,
                            crate::lower::refusal::Refused::Because(what, why),
                        );
                    }
                }
            }
        }
        if more.is_empty() {
            break;
        }
        for (owner, asked) in more {
            of_body.entry(owner).or_default().insert(asked);
        }
    }
}
