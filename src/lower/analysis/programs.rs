//! What a crate may not do of a program it uses, one compiled for its
//! tests as `rust-js build` compiles it (`--tested`), whose contract says
//! what it decided as a program (ADR 1076, amended, #597): each is refused,
//! as native Rust's answer would differ from the JS's.

use std::collections::HashSet;

use rustc_hir::def_id::DefId;
use rustc_middle::ty::{Ty, TyCtxt};

use crate::lower::Body;
use crate::lower::library::Foreign;

/// Refuses, and says so, a `dyn` of a program's own trait made of what
/// drops, of what `drops`, or of a type parameter's value where
/// `params_may_drop`, where none of the program's implementors drops, so
/// its code drops nothing of one (ADR 0320).
pub(super) fn refuses_dropping_dyn<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (foreign, params_may_drop): (&Foreign<'_, 'tcx>, bool),
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    for tr in tcx.all_traits_including_private().filter(|tr| !tr.is_local()) {
        let Some(program) = foreign.drops_no_dyn_of(tr) else {
            continue;
        };
        if let Some((span, from)) = super::closures::dropping_dyn_of(tcx, all_bodies, (tr, params_may_drop), &drops) {
            let tr = tcx.def_path_str(tr);
            tcx.dcx().span_err(
                span,
                format!(
                    "rust-js: this `dyn {tr}` holds a `{from}`, whose drop runs code, but `{program}`, a program compiled as `rust-js build` compiles it, drops nothing of a `dyn {tr}`, as none of its own implementors drops (ADR 1076)"
                ),
            );
            return true;
        }
    }
    false
}

/// Refuses each trait constant the crate's generic code reads, of
/// `generic_consts`, that a program's dictionary of an impl of its trait
/// lacks, carrying only those its own generic code reads (ADR 0106).
pub(super) fn refuse_lacked_consts(tcx: TyCtxt<'_>, foreign: &Foreign<'_, '_>, generic_consts: &HashSet<DefId>) {
    for &konst in generic_consts {
        if let Some((program, ty)) = foreign.lacks_dictionary_const(konst) {
            tcx.dcx().err(format!(
                "rust-js: this crate reads `{}` of a type parameter, which may be `{program}`'s `{ty}`, but `{program}` is a program compiled as `rust-js build` compiles it, whose dictionary of `{ty}` carries only the constants its own generic code reads, and none reads it (ADR 1076): read it of `{ty}` itself, `<{ty} as {}>::{}`",
                tcx.def_path_str(konst),
                tcx.def_path_str(tcx.parent(konst)),
                tcx.item_name(konst),
            ));
        }
    }
}
