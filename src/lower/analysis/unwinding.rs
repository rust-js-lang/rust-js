//! Which of the crate's functions never unwind (ADR 0902): each whose MIR
//! has no way to panic, read once from it, so a call of one throws only
//! where what it's given does (`Known::NeverThrows`), and a drop after it
//! needs no `finally`.

use crate::lower::Body;
use crate::lower::bindings::is_binding;
use crate::lower::recognition::{Recognition, is_std_crate};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// The crate's functions that never unwind: of no type parameters, no
/// binding nor trait's method a dictionary may call another impl of, each
/// whose every way on can't panic (`steps_on`), each call of another of
/// these or of std's that can't. Found from the most, each that may taken
/// out until none is.
pub(super) fn never_unwinding<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&Body<'tcx>],
) -> HashSet<DefId> {
    let candidates: Vec<(DefId, &mir::Body<'tcx>)> = (bodies.iter())
        .filter(|body| candidate(tcx, body.def_id))
        .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
        .collect();
    let mut never: HashSet<DefId> = candidates.iter().map(|&(id, _)| id).collect();
    loop {
        let unwinding: Vec<DefId> = (candidates.iter())
            .filter(|(id, mir)| never.contains(id) && !steps_on(tcx, recognition, &never, mir))
            .map(|&(id, _)| id)
            .collect();
        if unwinding.is_empty() {
            return never;
        }
        for id in unwinding {
            never.remove(&id);
        }
    }
}

/// Whether `id` may be one: the crate's own function, of no type
/// parameters, no binding's.
fn candidate(tcx: TyCtxt<'_>, id: DefId) -> bool {
    id.is_local()
        && matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn)
        && !is_binding(tcx, id)
        && !tcx.generics_of(id).requires_monomorphization(tcx)
}

/// Whether each block of `mir` but its cleanup goes on without a panic:
/// a jump, a test, a return, or a call of one of `never`'s or of std's
/// formatting and printing of what can't fail to show. Not an assert, a
/// bound's or a divisor's, nor a drop, nor what's unreachable.
fn steps_on<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    never: &HashSet<DefId>,
    mir: &mir::Body<'tcx>,
) -> bool {
    mir.basic_blocks.iter().filter(|block| !block.is_cleanup).all(|block| {
        let Some(terminator) = &block.terminator else {
            return true;
        };
        match &terminator.kind {
            TerminatorKind::Goto { .. }
            | TerminatorKind::SwitchInt { .. }
            | TerminatorKind::Return
            | TerminatorKind::FalseEdge { .. }
            | TerminatorKind::FalseUnwind { .. } => true,
            TerminatorKind::Call { func, .. } => match func.const_fn_def() {
                Some((callee, args)) => never.contains(&callee) || std_never_unwinds(tcx, recognition, callee, args),
                None => false,
            },
            _ => false,
        }
    })
}

/// Whether std's `callee`, given `args`, never panics as rust-js runs it:
/// a print, whose stream never fails (ADR 0132), the `Arguments` it's
/// given, and each argument's `Display` or `Debug` of a number, a `bool`,
/// a `char` or text, which shows any (ADR 0901).
fn std_never_unwinds<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    if !is_std_crate(tcx, callee) {
        return false;
    }
    let path = tcx.def_path_str(callee);
    let name = tcx.item_name(callee);
    let name = name.as_str();
    match () {
        _ if crate::lower::recognition::print_of(tcx, callee).is_some() => true,
        _ if path.contains("fmt::Arguments") => matches!(name, "new" | "new_const" | "new_v1" | "from_str"),
        _ if path.contains("fmt::rt::Argument") => {
            matches!(name, "new_display" | "new_debug") && args.types().all(|ty| shown_whole(recognition, ty))
        }
        _ => false,
    }
}

/// Whether showing a value of `ty` can't fail: a number's, a `bool`'s, a
/// `char`'s or text's, through references.
fn shown_whole<'tcx>(recognition: &Recognition<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
    let ty = ty.peel_refs();
    ty.is_primitive() || ty.is_str() || recognition.is_lang_adt(ty, LangItem::String)
}
