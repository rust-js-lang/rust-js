//! Whether formatting may print to stdout, the one owner of it (#1490, ADR
//! 1130): Rust's stdout is a `LineWriter`, so what a writer prints while
//! `println!` formats lands inside the line formatted so far, `a inner 1`,
//! and a writer's text written before its print comes before it. Each of
//! the crate's writers, a function given a `Formatter`, may print where
//! what it may run (`may_run`) reaches a body that prints to stdout, or
//! where that can't be followed; those, and every body they may run, are
//! what may run while a line is formatted.

use crate::collections::IndexSet;
use crate::lower::Body;
use crate::lower::recognition::Recognition;
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty;
use rustc_span::def_id::DefId;

use super::may_run::Runs;

/// What `prints` found of a crate.
#[derive(Default)]
pub(in crate::lower) struct Prints {
    /// The writers that may print to stdout as they format, whose text is
    /// written through the line they're formatting into (`$sink()`).
    pub(in crate::lower) writers: IndexSet<DefId>,
    /// The bodies that may run while a line is formatted: those writers and
    /// what each may run. A print of one is written through stdout's line
    /// (`$out`), and a writer's text it makes for anything but that line,
    /// `format!`'s, is made apart from it (`$formatting`).
    pub(in crate::lower) while_formatting: IndexSet<DefId>,
    /// The crate's types one of whose writers may print.
    pub(in crate::lower) types: IndexSet<DefId>,
    /// Of those, the ones one of whose writers that print may stop
    /// part-way, by a panic or an `Err`, in a program that may catch the
    /// panic and print on (`stops_shown`, #1521, #1530).
    pub(in crate::lower) stopping: IndexSet<DefId>,
}

impl Prints {
    /// Does `writer` write its text through the line it formats into?
    pub(in crate::lower) fn writer_prints(&self, writer: DefId) -> bool {
        self.writers.contains(&writer)
    }

    /// May `body` run while a line is formatted?
    pub(in crate::lower) fn runs_while_formatting(&self, body: DefId) -> bool {
        self.while_formatting.contains(&body)
    }

    /// May showing a value of `ty` print as it's formatted: a type of the
    /// crate's one of whose writers may, or one made of one, `Vec<Loud>`;
    /// or a type only a caller, or a `dyn`, says, where any writer may.
    pub(in crate::lower) fn shown_may_print(&self, ty: ty::Ty<'_>) -> bool {
        !self.writers.is_empty()
            && ty
                .walk()
                .filter_map(|part| part.as_type())
                .any(|part| match part.kind() {
                    ty::Adt(adt, _) => self.types.contains(&adt.did()),
                    ty::Param(_) | ty::Dynamic(..) | ty::Alias(..) => true,
                    _ => false,
                })
    }

    /// The type of the crate's whose writer may print and stop part-way,
    /// where a panic may be caught, shown in `ty`, `Loud` or one made of
    /// it, `Vec<Loud>`. A `Debug` that shows it among other parts, a std
    /// value's (`stops_composed`, #1521) or a derived `Debug`'s field or a
    /// builder's step (#1495), makes each part's text whole before its own
    /// reaches the line, so a part that stops loses the parts before it,
    /// `Holder { first: !Grid(1), second: !Grid(` where Rust's stdout keeps
    /// them: refused (#1530). One that may print and never stops is shown
    /// as the line has it, each print marked where it ran (#1521).
    pub(in crate::lower) fn stops_shown<'tcx>(&self, ty: ty::Ty<'tcx>) -> Option<ty::Ty<'tcx>> {
        (ty.walk().filter_map(|part| part.as_type()))
            .find(|part| part.ty_adt_def().is_some_and(|adt| self.stopping.contains(&adt.did())))
    }

    /// `stops_shown` of a std value's `Debug`, `Vec<Loud>`'s, `Option<Loud>`'s
    /// or `(Loud, u8)`'s (#1521). Of a type of the crate's, whose writer is
    /// the crate's own, none: its derived `Debug`'s fields are asked where
    /// it shows them (`stops_shown`, #1495).
    pub(in crate::lower) fn stops_composed<'tcx>(&self, ty: ty::Ty<'tcx>) -> Option<ty::Ty<'tcx>> {
        match ty.ty_adt_def().is_some_and(|adt| adt.did().is_local()) {
            true => None,
            false => self.stops_shown(ty),
        }
    }
}

/// The crate's writers that may print to stdout as they format, and what
/// may run while one does; and the types of those that may stop part-way,
/// those that may unwind, of all but `never_unwinding` (ADR 0902), or
/// return an `Err`, `failing` (ADR 0187), where the crate catches a panic.
pub(super) fn prints<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    runs: &Runs<'_, 'tcx>,
    (never_unwinding, failing): (&IndexSet<DefId>, &IndexSet<DefId>),
) -> Prints {
    let printing: IndexSet<DefId> = (all_bodies.iter())
        .filter(|body| prints_to_stdout(recognition, body))
        .map(|body| body.def_id)
        .collect();
    let mut found = Prints::default();
    if printing.is_empty() {
        return found;
    }
    let mut unfollowed = false;
    // The crate's writers: each function or closure given a `Formatter`,
    // not a `const`'s body.
    let writers: Vec<&Body<'tcx>> = (all_bodies.iter().copied())
        .filter(|body| {
            matches!(
                recognition.tcx.def_kind(body.def_id),
                DefKind::Fn | DefKind::AssocFn | DefKind::Closure
            ) && recognition.formatter_param(body.def_id).is_some()
        })
        .collect();
    for body in &writers {
        let writer = body.def_id;
        match runs.of(writer) {
            Some(run) => {
                if printing.contains(&writer) || run.iter().any(|id| printing.contains(id)) {
                    found.writers.insert(writer);
                    found.while_formatting.insert(writer);
                    found.while_formatting.extend(run);
                }
            }
            // What it may run can't be followed: it may print, and what it
            // runs may be any body that prints.
            None => {
                unfollowed = true;
                found.writers.insert(writer);
                found.while_formatting.insert(writer);
            }
        }
    }
    if unfollowed {
        found.while_formatting.extend(printing.iter().copied());
    }
    let tcx = recognition.tcx;
    let type_of = |writer: DefId| {
        let imp = tcx.impl_of_assoc(writer)?;
        Some(
            tcx.type_of(imp)
                .instantiate_identity()
                .skip_normalization()
                .ty_adt_def()?
                .did(),
        )
    };
    found.types = found.writers.iter().filter_map(|&writer| type_of(writer)).collect();
    if all_bodies.iter().any(|body| catches_unwinding(recognition, body)) {
        found.stopping = (found.writers.iter())
            .filter(|&writer| !never_unwinding.contains(writer) || failing.contains(writer))
            // A `Debug`'s, which std's values show their parts by.
            .filter(|&&writer| {
                (tcx.trait_impl_of_assoc(writer).map(|imp| tcx.impl_trait_id(imp))).is_some_and(|of| {
                    of == crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::Debug)
                })
            })
            .filter_map(|&writer| type_of(writer))
            .collect();
    }
    // A writer that shows a value of a type one of whose writers may print,
    // a derived `Debug` of a field's, which std's builders format through a
    // `&dyn Debug` what may run doesn't follow: it may print too, to a fixed
    // point.
    loop {
        let more: Vec<DefId> = (writers.iter())
            .filter(|body| !found.writers.contains(&body.def_id))
            .filter(|body| {
                body.thir.exprs.iter().any(|expr| {
                    (expr.ty.walk().filter_map(|part| part.as_type()))
                        .any(|part| part.ty_adt_def().is_some_and(|adt| found.types.contains(&adt.did())))
                })
            })
            .map(|body| body.def_id)
            .collect();
        if more.is_empty() {
            return found;
        }
        for writer in more {
            found.writers.insert(writer);
            found.while_formatting.insert(writer);
            found.while_formatting.extend(runs.of(writer).unwrap_or_default());
            found.types.extend(type_of(writer));
        }
    }
}

/// Whether `body` calls `catch_unwind`, past which a program a panic
/// stopped may print on (#1496, #1521).
fn catches_unwinding<'tcx>(recognition: &Recognition<'_, 'tcx>, body: &Body<'tcx>) -> bool {
    body.thir.exprs.iter().any(|expr| {
        let ExprKind::Call { fun, .. } = expr.kind else {
            return false;
        };
        let ty::FnDef(id, _) = *body.thir[fun].ty.kind() else {
            return false;
        };
        crate::lower::recognition::unwind_op(recognition.tcx, id)
            == Some(crate::lower::std_types::unwind::UnwindOp::Catch)
    })
}

/// Whether `body` prints to stdout itself: `print!` or `println!`, or a
/// write to `io::stdout()`.
fn prints_to_stdout<'tcx>(recognition: &Recognition<'_, 'tcx>, body: &Body<'tcx>) -> bool {
    body.thir.exprs.iter().any(|expr| {
        let ExprKind::Call { fun, .. } = expr.kind else {
            return false;
        };
        let ty::FnDef(id, args) = *body.thir[fun].ty.kind() else {
            return false;
        };
        recognition.writes_stdout(id, args.skip_binder())
    })
}
