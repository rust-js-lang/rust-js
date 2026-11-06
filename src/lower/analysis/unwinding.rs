//! Which of the crate's functions and closures never unwind (ADR 0902):
//! each whose MIR has no way to panic, read once from it, so a call of one
//! throws only where what it's given does (`Known::NeverThrows`), a drop
//! after it needs no `finally`, and an iterator whose closure stage it is
//! steps without a panic (`steps_may_panic`, #701).

use crate::lower::Body;
use crate::lower::bindings::is_binding;
use crate::lower::recognition::{Recognition, is_std_crate};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// The crate's functions and closures that never unwind: of no type parameters, no
/// binding nor trait's method a dictionary may call another impl of, each
/// whose every way on can't panic (`steps_on`), each call of another of
/// these or of std's that can't. Found from the most, each that may taken
/// out until none is.
pub(super) fn never_unwinding<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&Body<'tcx>],
    runs: &dyn Fn(Ty<'tcx>, DefId) -> bool,
) -> HashSet<DefId> {
    // Each one's `Drop`s that drop nothing, of what JS drops nothing of or
    // of what's moved out by then (`mir::dead_drops`, #717).
    let candidates: Vec<(DefId, &mir::Body<'tcx>, HashSet<mir::BasicBlock>)> = (bodies.iter())
        .filter(|body| candidate(tcx, body.def_id))
        .filter_map(|body| {
            let mir = &body.mir.as_ref()?.body;
            let mut dead = crate::lower::mir::dead_drops(tcx, mir, &|ty| runs(ty, body.def_id));
            // A closure of a generic function's: a drop of what's of its
            // type parameters may run anything, so none is dead.
            dead.retain(|&block| {
                !matches!(&mir.basic_blocks[block].terminator().kind,
                    TerminatorKind::Drop { place, .. } if place.ty(mir, tcx).ty.has_param())
            });
            Some((body.def_id, mir, dead))
        })
        .collect();
    let mut never: HashSet<DefId> = candidates.iter().map(|&(id, ..)| id).collect();
    loop {
        let unwinding: Vec<DefId> = (candidates.iter())
            .filter(|(id, mir, dead)| never.contains(id) && !steps_on(tcx, recognition, &never, (mir, dead)))
            .map(|&(id, ..)| id)
            .collect();
        if unwinding.is_empty() {
            return never;
        }
        for id in unwinding {
            never.remove(&id);
        }
    }
}

/// Whether `id` may be one: the crate's own function or closure, no
/// binding's, a function of no type parameters (#701). A closure's captures
/// are read in its MIR as its fields, so a call, a drop or an assert of what
/// it captured is a block `steps_on` reads; one written in a generic
/// function may be one too, as each call and drop of what's of its type
/// parameters is no way on `steps_on` takes (#847): a guard's closure,
/// `Defer(|| DEPTH.set(previous))` in a function given `impl FnOnce`. Not a
/// coroutine, an `async` block's, which no stage is.
fn candidate(tcx: TyCtxt<'_>, id: DefId) -> bool {
    id.is_local()
        && match tcx.def_kind(id) {
            DefKind::Fn | DefKind::AssocFn => !tcx.generics_of(id).requires_monomorphization(tcx),
            DefKind::Closure => !tcx.is_coroutine(id),
            _ => false,
        }
        && !is_binding(tcx, id)
}

/// Whether each block of `mir` but its cleanup goes on without a panic:
/// a jump, a test, a return, a `Drop` of `dead`'s, which drops nothing
/// (#717), or a call of one of `never`'s or of std's formatting and
/// printing of what can't fail to show. Not an assert, a bound's or a
/// divisor's, nor another drop, nor what's unreachable.
fn steps_on<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    never: &HashSet<DefId>,
    (mir, dead): (&mir::Body<'tcx>, &HashSet<mir::BasicBlock>),
) -> bool {
    mir.basic_blocks
        .iter_enumerated()
        .filter(|(_, block)| !block.is_cleanup)
        .all(|(at, block)| {
            let Some(terminator) = &block.terminator else {
                return true;
            };
            match &terminator.kind {
                TerminatorKind::Drop { .. } => dead.contains(&at),
                TerminatorKind::Goto { .. }
                | TerminatorKind::SwitchInt { .. }
                | TerminatorKind::Return
                | TerminatorKind::FalseEdge { .. }
                | TerminatorKind::FalseUnwind { .. } => true,
                TerminatorKind::Call { func, .. } => match func.const_fn_def() {
                    Some((callee, args)) => {
                        never.contains(&callee) || std_never_unwinds(tcx, recognition, callee, args)
                    }
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
        // A thread-local's or a `Cell`'s set or replace of a value that
        // drops nothing: a write of a module's `let` or of the cell's place,
        // as rust-js runs it, where native Rust's thread-local may panic once
        // destroyed, which a JS module never is. So a guard that restores
        // one, `DEPTH.set(self.0)`, drops in a `finally` (#847). Not its
        // `get` yet: a scope whose calls then never throw is MIR's blocks,
        // `bb18: { .. }`, where tidy makes no `finally` of its early returns
        // (`drop_guard_early_returns`), #861's to make.
        _ if matches!(name, "set" | "replace") && of_a_cell(tcx, callee) => {
            args.types().all(|ty| ty.is_trivially_pure_clone_copy())
        }
        _ => false,
    }
}

/// Whether `method` is a `Cell`'s own, or a thread-local `Cell`'s,
/// `LocalKey<Cell<T>>`'s; not a `RefCell`'s, whose `set` borrows.
fn of_a_cell(tcx: TyCtxt<'_>, method: DefId) -> bool {
    let Some(imp) = tcx.inherent_impl_of_assoc(method) else {
        return false;
    };
    let is = |ty: Ty<'_>, name: &str| matches!(ty.kind(), ty::Adt(adt, _) if is_std_crate(tcx, adt.did()) && tcx.item_name(adt.did()).as_str() == name);
    let own = tcx.type_of(imp).instantiate_identity().skip_normalization();
    is(own, "Cell")
        || matches!(own.kind(), ty::Adt(_, local) if is(own, "LocalKey")
            && local.types().next().is_some_and(|cell| is(cell, "Cell")))
}

/// Whether showing a value of `ty` can't fail: a number's, a `bool`'s, a
/// `char`'s or text's, through references.
fn shown_whole<'tcx>(recognition: &Recognition<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
    let ty = ty.peel_refs();
    ty.is_primitive() || ty.is_str() || recognition.is_lang_adt(ty, LangItem::String)
}
